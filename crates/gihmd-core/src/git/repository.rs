use std::future::Future;
use std::path::Path;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::git::AuthContext;
use crate::git::BranchInfo;
use crate::git::ChangedFile;
use crate::git::CommitInfo;
use crate::git::DiffKind;
use crate::git::FileDiff;
use crate::git::GitError;
use crate::git::GitProgress;
use crate::git::LogQuery;
use crate::git::PullResult;
use crate::git::RemoteInfo;
use crate::git::RepoStatus;
use crate::paths::RelPath;

pub trait Repository: Send + Sync + 'static {
    fn open_or_none(
        &self,
        root: &Path,
    ) -> impl Future<Output = Result<Option<RepoStatus>, GitError>> + Send;

    fn init(&self, root: &Path) -> impl Future<Output = Result<RepoStatus, GitError>> + Send;

    fn status(&self, root: &Path) -> impl Future<Output = Result<RepoStatus, GitError>> + Send;

    fn stage(
        &self,
        root: &Path,
        paths: &[RelPath],
    ) -> impl Future<Output = Result<(), GitError>> + Send;

    fn unstage(
        &self,
        root: &Path,
        paths: &[RelPath],
    ) -> impl Future<Output = Result<(), GitError>> + Send;

    fn discard(
        &self,
        root: &Path,
        paths: &[RelPath],
    ) -> impl Future<Output = Result<(), GitError>> + Send;

    fn commit(
        &self,
        root: &Path,
        message: &str,
    ) -> impl Future<Output = Result<String, GitError>> + Send;

    fn log(
        &self,
        root: &Path,
        query: LogQuery,
    ) -> impl Future<Output = Result<Vec<CommitInfo>, GitError>> + Send;

    fn commit_files(
        &self,
        root: &Path,
        oid: &str,
    ) -> impl Future<Output = Result<Vec<ChangedFile>, GitError>> + Send;

    fn diff_file(
        &self,
        root: &Path,
        path: &RelPath,
        kind: DiffKind,
    ) -> impl Future<Output = Result<FileDiff, GitError>> + Send;

    fn branches(
        &self,
        root: &Path,
    ) -> impl Future<Output = Result<Vec<BranchInfo>, GitError>> + Send;

    fn create_branch(
        &self,
        root: &Path,
        name: &str,
        checkout: bool,
    ) -> impl Future<Output = Result<(), GitError>> + Send;

    fn checkout_branch(
        &self,
        root: &Path,
        name: &str,
    ) -> impl Future<Output = Result<(), GitError>> + Send;

    fn remotes(
        &self,
        root: &Path,
    ) -> impl Future<Output = Result<Vec<RemoteInfo>, GitError>> + Send;

    fn add_remote(
        &self,
        root: &Path,
        name: &str,
        url: &str,
    ) -> impl Future<Output = Result<(), GitError>> + Send;

    fn remove_remote(
        &self,
        root: &Path,
        name: &str,
    ) -> impl Future<Output = Result<(), GitError>> + Send;

    fn fetch(
        &self,
        root: &Path,
        remote: &str,
        auth: AuthContext,
        progress: mpsc::Sender<GitProgress>,
        cancel: CancellationToken,
    ) -> impl Future<Output = Result<(), GitError>> + Send;

    fn pull_fast_forward(
        &self,
        root: &Path,
        remote: &str,
        auth: AuthContext,
        progress: mpsc::Sender<GitProgress>,
        cancel: CancellationToken,
    ) -> impl Future<Output = Result<PullResult, GitError>> + Send;

    fn push(
        &self,
        root: &Path,
        remote: &str,
        refspec: Option<String>,
        auth: AuthContext,
        progress: mpsc::Sender<GitProgress>,
        cancel: CancellationToken,
    ) -> impl Future<Output = Result<(), GitError>> + Send;

    fn clone(
        &self,
        url: &str,
        dest: &Path,
        auth: AuthContext,
        progress: mpsc::Sender<GitProgress>,
        cancel: CancellationToken,
    ) -> impl Future<Output = Result<(), GitError>> + Send;
}
