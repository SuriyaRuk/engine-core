use std::sync::Arc;

use alloy::primitives::U256;
use engine_aa_core::smart_account::{DeterminedSmartAccount, SmartAccount, SmartAccountFromSalt};
use engine_core::{
    chain::{ChainService, RpcCredentials},
    credentials::SigningCredential,
    error::EngineError,
    execution_options::{
        BaseExecutionOptions, QueuedTransaction, SendTransactionRequest, SpecificExecutionOptions,
        WebhookOptions, aa::Erc4337ExecutionOptions,
    },
    transaction::InnerTransaction,
};
use engine_executors::{
    external_bundler::{
        confirm::UserOpConfirmationHandler,
        send::{ExternalBundlerSendHandler, ExternalBundlerSendJobData},
    },
    transaction_registry::TransactionRegistry,
    webhook::WebhookJobHandler,
};
use twmq::{Queue, error::TwmqError};
use engine_core::auth::Auth;
// vault_types removed, replaced with engine_core::auth

use crate::chains::ThirdwebChainService;

pub struct ExecutionRouter {
    pub webhook_queue: Arc<Queue<WebhookJobHandler>>,
    pub external_bundler_send_queue: Arc<Queue<ExternalBundlerSendHandler<ThirdwebChainService>>>,
    pub userop_confirm_queue: Arc<Queue<UserOpConfirmationHandler<ThirdwebChainService>>>,
    pub transaction_registry: Arc<TransactionRegistry>,
    pub chains: Arc<ThirdwebChainService>,
}

impl ExecutionRouter {
    fn generate_random_nonce() -> U256 {
        use rand::Rng;
        let mut rng = rand::rng();
        let rand1 = rng.random::<u64>();
        let rand2 = rng.random::<u64>();
        let rand3 = rng.random::<u64>();

        U256::from_limbs([0, rand1, rand2, rand3])
    }

    /// Convert vault access tokens to signed tokens with ERC4337-specific restrictions
    /// NOTE: This is now a stub implementation since vault functionality was removed
    async fn convert_vault_credential_for_erc4337(
        &self,
        signing_credential: SigningCredential,
        _erc4337_options: &Erc4337ExecutionOptions,
        _base_options: &BaseExecutionOptions,
        _transactions: &[InnerTransaction],
    ) -> Result<(SigningCredential, Option<U256>), EngineError> {
        // Only convert vault access tokens
        match &signing_credential {
            SigningCredential::Vault(Auth::AccessToken { access_token: _ }) => {
                // Return an error since vault functionality is no longer available
                Err(EngineError::InternalError {
                    message: "Vault functionality has been removed and is no longer supported".to_string(),
                })
            }
            _ => Ok((signing_credential, None)),
        }
    }

    pub async fn execute(
        &self,
        execution_request: SendTransactionRequest,
        rpc_credentials: RpcCredentials,
        signing_credential: engine_core::credentials::SigningCredential,
    ) -> Result<Vec<QueuedTransaction>, EngineError> {
        match execution_request.execution_options.specific {
            SpecificExecutionOptions::ERC4337(ref erc4337_execution_options) => {
                // Convert vault access tokens to signed tokens with proper nonce and calldata
                let (converted_credential, pregenerated_nonce) = self
                    .convert_vault_credential_for_erc4337(
                        signing_credential,
                        erc4337_execution_options,
                        &execution_request.execution_options.base,
                        &execution_request.params,
                    )
                    .await?;

                self.execute_external_bundler(
                    &execution_request.execution_options.base,
                    erc4337_execution_options,
                    &execution_request.webhook_options,
                    &execution_request.params,
                    rpc_credentials,
                    converted_credential,
                    pregenerated_nonce,
                )
                .await?;

                let queued_transaction = QueuedTransaction {
                    id: execution_request
                        .execution_options
                        .base
                        .idempotency_key
                        .clone(),
                    batch_index: 0,
                    execution_params: execution_request.execution_options,
                    transaction_params: execution_request.params,
                };

                Ok(vec![queued_transaction])
            }

            SpecificExecutionOptions::Auto(_auto_execution_options) => {
                todo!()
            }
        }
    }

    async fn execute_external_bundler(
        &self,
        base_execution_options: &BaseExecutionOptions,
        erc4337_execution_options: &Erc4337ExecutionOptions,
        webhook_options: &Option<Vec<WebhookOptions>>,
        transactions: &[InnerTransaction],
        rpc_credentials: RpcCredentials,
        signing_credential: SigningCredential,
        pregenerated_nonce: Option<U256>,
    ) -> Result<(), TwmqError> {
        let job_data = ExternalBundlerSendJobData {
            transaction_id: base_execution_options.idempotency_key.clone(),
            chain_id: base_execution_options.chain_id,
            transactions: transactions.to_vec(),
            execution_options: erc4337_execution_options.clone(),
            signing_credential,
            webhook_options: webhook_options.clone(),
            rpc_credentials,
            pregenerated_nonce,
        };

        // Register transaction in registry first
        self.transaction_registry
            .set_transaction_queue(
                &base_execution_options.idempotency_key,
                "external_bundler_send",
            )
            .await
            .map_err(|e| TwmqError::Runtime {
                message: format!("Failed to register transaction: {}", e),
            })?;

        // Create job with transaction ID as the job ID for idempotency
        self.external_bundler_send_queue
            .clone()
            .job(job_data)
            .with_id(&base_execution_options.idempotency_key)
            .push()
            .await?;

        tracing::debug!(
            transaction_id = %base_execution_options.idempotency_key,
            queue = "external_bundler_send",
            "Job queued successfully"
        );

        Ok(())
    }
}
