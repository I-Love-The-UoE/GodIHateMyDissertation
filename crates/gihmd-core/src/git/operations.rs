use std::path::Path;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::context::Context;
use crate::git::AuthContext;
use crate::git::GitError;
use crate::git::GitProgress;
use crate::git::PullResult;
use crate::git::Repository;
use crate::secrets::Secrets;

/// Extracts the host from `https://host/..`, `ssh://user@host/..` and `user@host:path` URLs.
pub fn host_of(url: &str) -> Option<String> {
    if let Some((_, rest)) = url.split_once("://") {
        let authority = rest.split('/').next()?;
        let host = authority.rsplit('@').next()?;
        let host = host.split(':').next()?;

        return (!host.is_empty()).then(|| host.to_owned());
    }

    if let Some((authority, _)) = url.split_once(':') {
        let host = authority.rsplit('@').next()?;

        return (!host.is_empty()).then(|| host.to_owned());
    }

    None
}

fn is_https(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

async fn auth_for_url<C: Context>(ctx: &C, url: &str) -> Result<AuthContext, GitError> {
    if !is_https(url) {
        return Ok(AuthContext::default());
    }

    let host = host_of(url).ok_or_else(|| GitError::RemoteUrlInvalid {
        url: url.to_owned(),
    })?;
    let https = ctx.secrets().lookup(&host).await?;

    Ok(AuthContext { https })
}

async fn auth_for_remote<C: Context>(
    ctx: &C,
    root: &Path,
    remote: &str,
) -> Result<AuthContext, GitError> {
    let remotes = ctx.repository().remotes(root).await?;
    let info = remotes
        .into_iter()
        .find(|info| info.name == remote)
        .ok_or_else(|| GitError::RemoteMissing {
            name: remote.to_owned(),
        })?;

    auth_for_url(ctx, &info.url).await
}

pub async fn fetch<C: Context>(
    ctx: &C,
    root: &Path,
    remote: &str,
    progress: mpsc::Sender<GitProgress>,
    cancel: CancellationToken,
) -> Result<(), GitError> {
    let auth = auth_for_remote(ctx, root, remote).await?;

    ctx.repository()
        .fetch(root, remote, auth, progress, cancel)
        .await
}

pub async fn pull<C: Context>(
    ctx: &C,
    root: &Path,
    remote: &str,
    progress: mpsc::Sender<GitProgress>,
    cancel: CancellationToken,
) -> Result<PullResult, GitError> {
    let auth = auth_for_remote(ctx, root, remote).await?;

    ctx.repository()
        .pull_fast_forward(root, remote, auth, progress, cancel)
        .await
}

pub async fn push<C: Context>(
    ctx: &C,
    root: &Path,
    remote: &str,
    refspec: Option<String>,
    progress: mpsc::Sender<GitProgress>,
    cancel: CancellationToken,
) -> Result<(), GitError> {
    let auth = auth_for_remote(ctx, root, remote).await?;

    ctx.repository()
        .push(root, remote, refspec, auth, progress, cancel)
        .await
}

pub async fn clone_repository<C: Context>(
    ctx: &C,
    url: &str,
    dest: &Path,
    progress: mpsc::Sender<GitProgress>,
    cancel: CancellationToken,
) -> Result<(), GitError> {
    let auth = auth_for_url(ctx, url).await?;

    ctx.repository()
        .clone(url, dest, auth, progress, cancel)
        .await
}
