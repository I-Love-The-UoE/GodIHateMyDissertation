use gihmd_core::git::GitError;

pub(crate) const CREDENTIALS_REQUIRED: &str = "gihmd:credentials-required";
pub(crate) const AUTH_EXHAUSTED: &str = "gihmd:auth-exhausted";

pub(crate) fn lib_error(error: &git2::Error) -> GitError {
    GitError::Lib {
        code: format!("{:?}", error.code()),
        message: error.message().to_owned(),
    }
}

pub(crate) fn remote_error(error: &git2::Error, host: &str, scheme: &str) -> GitError {
    let message = error.message();

    if message.contains(CREDENTIALS_REQUIRED) {
        return GitError::CredentialsRequired {
            host: host.to_owned(),
            scheme: scheme.to_owned(),
        };
    }

    if message.contains(AUTH_EXHAUSTED) || error.code() == git2::ErrorCode::Auth {
        return GitError::AuthFailed {
            host: host.to_owned(),
        };
    }

    if error.code() == git2::ErrorCode::Certificate
        || (error.class() == git2::ErrorClass::Ssh && message.to_lowercase().contains("host key"))
    {
        return GitError::UnknownHostKey {
            host: host.to_owned(),
        };
    }

    if error.code() == git2::ErrorCode::User {
        return GitError::Cancelled;
    }

    lib_error(error)
}

pub(crate) fn open_error(error: &git2::Error) -> GitError {
    if error.code() == git2::ErrorCode::NotFound {
        return GitError::NotARepository;
    }

    lib_error(error)
}
