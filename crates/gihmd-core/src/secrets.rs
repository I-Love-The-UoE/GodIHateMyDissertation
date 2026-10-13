use std::future::Future;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SecretsError {
    #[error("The operating system keychain is not available.")]
    NoStorageAccess { detail: String },
    #[error("The keychain rejected the operation.")]
    Platform { detail: String },
    #[error("The stored credential is not in the expected format.")]
    Malformed { host: String, detail: String },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StoredCredential {
    pub username: String,
    pub secret: String,
}

// NOTE: The secret is deliberately kept out of Debug output.
impl std::fmt::Debug for StoredCredential {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StoredCredential")
            .field("username", &self.username)
            .field("secret", &"<redacted>")
            .finish()
    }
}

pub trait Secrets: Send + Sync + 'static {
    fn store(
        &self,
        host: &str,
        credential: &StoredCredential,
    ) -> impl Future<Output = Result<(), SecretsError>> + Send;

    fn lookup(
        &self,
        host: &str,
    ) -> impl Future<Output = Result<Option<StoredCredential>, SecretsError>> + Send;

    fn forget(&self, host: &str) -> impl Future<Output = Result<(), SecretsError>> + Send;
}
