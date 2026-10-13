use serde::Serialize;

use crate::paths::RelPath;
use crate::secrets::SecretsError;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GitError {
    #[error("The project is not a git repository.")]
    NotARepository,
    #[error("No author identity is configured; set user.name and user.email in git config.")]
    IdentityMissing,
    #[error("Credentials are required for this remote.")]
    CredentialsRequired { host: String, scheme: String },
    #[error("Authentication failed for this remote.")]
    AuthFailed { host: String },
    #[error("The remote host key is not in known_hosts.")]
    UnknownHostKey { host: String },
    #[error("The remote has diverged; the branch cannot be fast-forwarded.")]
    NonFastForward,
    #[error("The working tree has conflicts.")]
    Conflicts { paths: Vec<RelPath> },
    #[error("HEAD is detached.")]
    DetachedHead,
    #[error("The remote does not exist.")]
    RemoteMissing { name: String },
    #[error("The remote URL has no recognisable host.")]
    RemoteUrlInvalid { url: String },
    #[error("The branch does not exist.")]
    BranchMissing { name: String },
    #[error("Nothing is staged to commit.")]
    NothingToCommit,
    #[error("The operation was cancelled.")]
    Cancelled,
    #[error("The keychain could not be used.")]
    Secrets(#[from] SecretsError),
    #[error("libgit2 reported an error.")]
    Lib { code: String, message: String },
    #[error("The repository could not be accessed.")]
    Io {
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: std::io::Error,
    },
    #[error("The git operation thread ended unexpectedly.")]
    Worker,
}
