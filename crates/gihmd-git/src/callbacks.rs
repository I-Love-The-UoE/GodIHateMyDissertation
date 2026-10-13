use gihmd_core::git::AuthContext;
use gihmd_core::git::GitProgress;
use git2::Cred;
use git2::CredentialType;
use git2::RemoteCallbacks;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::convert::AUTH_EXHAUSTED;
use crate::convert::CREDENTIALS_REQUIRED;

pub(crate) fn remote_callbacks<'a>(
    auth: AuthContext,
    progress: mpsc::Sender<GitProgress>,
    cancel: CancellationToken,
) -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    let mut tried_agent = false;
    let mut tried_userpass = false;
    callbacks.credentials(move |_url, username_from_url, allowed| {
        if allowed.contains(CredentialType::SSH_KEY) && !tried_agent {
            tried_agent = true;

            return Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"));
        }

        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) && !tried_userpass {
            tried_userpass = true;

            return match &auth.https {
                Some(credential) => {
                    Cred::userpass_plaintext(&credential.username, &credential.secret)
                }
                None => Err(git2::Error::from_str(CREDENTIALS_REQUIRED)),
            };
        }

        Err(git2::Error::from_str(AUTH_EXHAUSTED))
    });
    let transfer_progress = progress.clone();
    let transfer_cancel = cancel;
    callbacks.transfer_progress(move |stats| {
        let event = if stats.received_objects() < stats.total_objects() {
            GitProgress::Receiving {
                received: stats.received_objects(),
                total: stats.total_objects(),
                bytes: stats.received_bytes(),
            }
        } else if stats.indexed_deltas() < stats.total_deltas() {
            GitProgress::Resolving {
                current: stats.indexed_deltas(),
                total: stats.total_deltas(),
            }
        } else {
            GitProgress::Counting {
                received: stats.received_objects(),
                total: stats.total_objects(),
            }
        };
        let _ = transfer_progress.try_send(event);

        !transfer_cancel.is_cancelled()
    });
    let sideband_progress = progress.clone();
    callbacks.sideband_progress(move |data| {
        let text = String::from_utf8_lossy(data).trim().to_owned();

        if !text.is_empty() {
            let _ = sideband_progress.try_send(GitProgress::Sideband { text });
        }

        true
    });
    callbacks.push_transfer_progress(move |current, total, bytes| {
        let _ = progress.try_send(GitProgress::Pushing {
            current,
            total,
            bytes,
        });
    });
    callbacks.push_update_reference(|_refname, status| match status {
        Some(message) => Err(git2::Error::from_str(message)),
        None => Ok(()),
    });

    callbacks
}
