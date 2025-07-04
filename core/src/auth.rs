use serde::{Deserialize, Serialize};

/// Local authentication types to replace vault-types dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Auth {
    AccessToken { access_token: String },
}

/// Message format for signing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageFormat {
    Text,
    Hex,
}