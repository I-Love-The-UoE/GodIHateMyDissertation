use gihmd_core::Secrets;
use gihmd_core::SecretsError;
use gihmd_core::StoredCredential;

const SERVICE: &str = "dev.realistik.gihmd.git";

#[derive(Debug, Default)]
pub struct KeyringSecrets;

fn entry(host: &str) -> Result<keyring::Entry, SecretsError> {
    keyring::Entry::new(SERVICE, host).map_err(|error| platform_error(&error))
}

fn platform_error(error: &keyring::Error) -> SecretsError {
    match error {
        keyring::Error::NoStorageAccess(cause) => SecretsError::NoStorageAccess {
            detail: cause.to_string(),
        },
        other => SecretsError::Platform {
            detail: other.to_string(),
        },
    }
}

impl Secrets for KeyringSecrets {
    async fn store(&self, host: &str, credential: &StoredCredential) -> Result<(), SecretsError> {
        let host = host.to_owned();
        let payload =
            serde_json::to_string(credential).map_err(|error| SecretsError::Platform {
                detail: error.to_string(),
            })?;

        tokio::task::spawn_blocking(move || {
            entry(&host)?
                .set_password(&payload)
                .map_err(|error| platform_error(&error))
        })
        .await
        .map_err(|error| SecretsError::Platform {
            detail: error.to_string(),
        })?
    }

    async fn lookup(&self, host: &str) -> Result<Option<StoredCredential>, SecretsError> {
        let host = host.to_owned();

        tokio::task::spawn_blocking(move || {
            let payload = match entry(&host)?.get_password() {
                Ok(payload) => payload,
                Err(keyring::Error::NoEntry) => return Ok(None),
                Err(error) => return Err(platform_error(&error)),
            };
            let credential: StoredCredential =
                serde_json::from_str(&payload).map_err(|error| SecretsError::Malformed {
                    host: host.clone(),
                    detail: error.to_string(),
                })?;

            Ok(Some(credential))
        })
        .await
        .map_err(|error| SecretsError::Platform {
            detail: error.to_string(),
        })?
    }

    async fn forget(&self, host: &str) -> Result<(), SecretsError> {
        let host = host.to_owned();

        tokio::task::spawn_blocking(move || match entry(&host)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(platform_error(&error)),
        })
        .await
        .map_err(|error| SecretsError::Platform {
            detail: error.to_string(),
        })?
    }
}

pub fn default() -> KeyringSecrets {
    KeyringSecrets
}
