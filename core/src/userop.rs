use alloy::{
    hex::FromHex,
    primitives::{Address, Bytes, ChainId},
};
use thirdweb_core::iaw::IAWClient;
// vault_sdk and vault_types removed, replaced with engine_core::auth

use crate::{credentials::SigningCredential, error::EngineError};

// Re-export for convenience
pub use engine_aa_types::VersionedUserOp;

#[derive(Clone)]
pub struct UserOpSigner {
    pub iaw_client: IAWClient,
}

pub struct UserOpSignerParams {
    pub credentials: SigningCredential,
    pub entrypoint: Address,
    pub userop: VersionedUserOp,
    pub signer_address: Address,
    pub chain_id: ChainId,
}

// vault functionality removed - this was used to convert userop to vault input format

impl UserOpSigner {
    pub async fn sign(&self, params: UserOpSignerParams) -> Result<Bytes, EngineError> {
        match &params.credentials {
            SigningCredential::Vault(_auth_method) => {
                // Return an error since vault functionality is no longer available
                Err(EngineError::ValidationError {
                    message: "Vault functionality has been removed and is no longer supported".to_string(),
                })
            }
            SigningCredential::Iaw {
                auth_token,
                thirdweb_auth,
            } => {
                let result = self
                    .iaw_client
                    .sign_userop(
                        auth_token.clone(),
                        thirdweb_auth.clone(),
                        params.userop,
                        params.entrypoint,
                        params.signer_address,
                        params.chain_id,
                    )
                    .await
                    .map_err(|e| EngineError::ValidationError {
                        message: format!("Failed to sign userop: {}", e),
                    })?;

                Ok(Bytes::from_hex(&result.signature).map_err(|_| {
                    EngineError::ValidationError {
                        message: "Bad signature received from IAW".to_string(),
                    }
                })?)
            }
        }
    }
}
