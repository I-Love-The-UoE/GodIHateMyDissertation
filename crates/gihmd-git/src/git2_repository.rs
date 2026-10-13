use std::future::Future;
use std::path::Path;
use std::path::PathBuf;

use gihmd_core::RelPath;
use gihmd_core::git::AuthContext;
use gihmd_core::git::BranchInfo;
use gihmd_core::git::ChangedFile;
use gihmd_core::git::CommitInfo;
use gihmd_core::git::DiffKind;
use gihmd_core::git::FileDiff;
use gihmd_core::git::GitError;
use gihmd_core::git::GitProgress;
use gihmd_core::git::LogQuery;
use gihmd_core::git::PullResult;
use gihmd_core::git::RemoteInfo;
use gihmd_core::git::RepoStatus;
use gihmd_core::git::Repository;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::operations;

#[derive(Debug, Default)]
pub struct Git2Repository;

async fn blocking<T, F>(work: F) -> Result<T, GitError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, GitError> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .ok()
        .ok_or(GitError::Worker)?
}

fn with_repo<T, F>(root: PathBuf, work: F) -> impl Future<Output = Result<T, GitError>> + Send
where
    T: Send + 'static,
    F: FnOnce(&git2::Repository) -> Result<T, GitError> + Send + 'static,
{
    blocking(move || {
        let repo = operations::open(&root)?;

        work(&repo)
    })
}

impl Repository for Git2Repository {
    async fn open_or_none(&self, root: &Path) -> Result<Option<RepoStatus>, GitError> {
        let root = root.to_path_buf();

        blocking(move || match operations::open(&root) {
            Ok(repo) => operations::status(&repo).map(Some),
            Err(GitError::NotARepository) => Ok(None),
            Err(error) => Err(error),
        })
        .await
    }

    async fn init(&self, root: &Path) -> Result<RepoStatus, GitError> {
        let root = root.to_path_buf();

        blocking(move || {
            let repo = operations::init(&root)?;

            operations::status(&repo)
        })
        .await
    }

    async fn status(&self, root: &Path) -> Result<RepoStatus, GitError> {
        with_repo(root.to_path_buf(), operations::status).await
    }

    async fn stage(&self, root: &Path, paths: &[RelPath]) -> Result<(), GitError> {
        let paths = paths.to_vec();

        with_repo(root.to_path_buf(), move |repo| {
            operations::stage(repo, &paths)
        })
        .await
    }

    async fn unstage(&self, root: &Path, paths: &[RelPath]) -> Result<(), GitError> {
        let paths = paths.to_vec();

        with_repo(root.to_path_buf(), move |repo| {
            operations::unstage(repo, &paths)
        })
        .await
    }

    async fn discard(&self, root: &Path, paths: &[RelPath]) -> Result<(), GitError> {
        let paths = paths.to_vec();

        with_repo(root.to_path_buf(), move |repo| {
            operations::discard(repo, &paths)
        })
        .await
    }

    async fn commit(&self, root: &Path, message: &str) -> Result<String, GitError> {
        let message = message.to_owned();

        with_repo(root.to_path_buf(), move |repo| {
            operations::commit(repo, &message)
        })
        .await
    }

    async fn log(&self, root: &Path, query: LogQuery) -> Result<Vec<CommitInfo>, GitError> {
        with_repo(root.to_path_buf(), move |repo| {
            operations::log(repo, &query)
        })
        .await
    }

    async fn commit_files(&self, root: &Path, oid: &str) -> Result<Vec<ChangedFile>, GitError> {
        let oid = oid.to_owned();

        with_repo(root.to_path_buf(), move |repo| {
            operations::commit_files(repo, &oid)
        })
        .await
    }

    async fn diff_file(
        &self,
        root: &Path,
        path: &RelPath,
        kind: DiffKind,
    ) -> Result<FileDiff, GitError> {
        let path = path.clone();

        with_repo(root.to_path_buf(), move |repo| {
            operations::diff_file(repo, &path, &kind)
        })
        .await
    }

    async fn branches(&self, root: &Path) -> Result<Vec<BranchInfo>, GitError> {
        with_repo(root.to_path_buf(), operations::branches).await
    }

    async fn create_branch(&self, root: &Path, name: &str, checkout: bool) -> Result<(), GitError> {
        let name = name.to_owned();

        with_repo(root.to_path_buf(), move |repo| {
            operations::create_branch(repo, &name, checkout)
        })
        .await
    }

    async fn checkout_branch(&self, root: &Path, name: &str) -> Result<(), GitError> {
        let name = name.to_owned();

        with_repo(root.to_path_buf(), move |repo| {
            operations::checkout_branch(repo, &name)
        })
        .await
    }

    async fn remotes(&self, root: &Path) -> Result<Vec<RemoteInfo>, GitError> {
        with_repo(root.to_path_buf(), operations::remotes).await
    }

    async fn add_remote(&self, root: &Path, name: &str, url: &str) -> Result<(), GitError> {
        let name = name.to_owned();
        let url = url.to_owned();

        with_repo(root.to_path_buf(), move |repo| {
            operations::add_remote(repo, &name, &url)
        })
        .await
    }

    async fn remove_remote(&self, root: &Path, name: &str) -> Result<(), GitError> {
        let name = name.to_owned();

        with_repo(root.to_path_buf(), move |repo| {
            operations::remove_remote(repo, &name)
        })
        .await
    }

    async fn fetch(
        &self,
        root: &Path,
        remote: &str,
        auth: AuthContext,
        progress: mpsc::Sender<GitProgress>,
        cancel: CancellationToken,
    ) -> Result<(), GitError> {
        let remote = remote.to_owned();

        with_repo(root.to_path_buf(), move |repo| {
            operations::fetch(repo, &remote, auth, progress, cancel)
        })
        .await
    }

    async fn pull_fast_forward(
        &self,
        root: &Path,
        remote: &str,
        auth: AuthContext,
        progress: mpsc::Sender<GitProgress>,
        cancel: CancellationToken,
    ) -> Result<PullResult, GitError> {
        let remote = remote.to_owned();

        with_repo(root.to_path_buf(), move |repo| {
            operations::pull_fast_forward(repo, &remote, auth, progress, cancel)
        })
        .await
    }

    async fn push(
        &self,
        root: &Path,
        remote: &str,
        refspec: Option<String>,
        auth: AuthContext,
        progress: mpsc::Sender<GitProgress>,
        cancel: CancellationToken,
    ) -> Result<(), GitError> {
        let remote = remote.to_owned();

        with_repo(root.to_path_buf(), move |repo| {
            operations::push(repo, &remote, refspec, auth, progress, cancel)
        })
        .await
    }

    async fn clone(
        &self,
        url: &str,
        dest: &Path,
        auth: AuthContext,
        progress: mpsc::Sender<GitProgress>,
        cancel: CancellationToken,
    ) -> Result<(), GitError> {
        let url = url.to_owned();
        let dest = dest.to_path_buf();

        blocking(move || operations::clone(&url, &dest, auth, progress, cancel)).await
    }
}

pub fn default() -> Git2Repository {
    Git2Repository
}
