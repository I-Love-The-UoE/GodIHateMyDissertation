use std::path::Path;

use gihmd_core::RelPath;
use gihmd_core::git::AuthContext;
use gihmd_core::git::BranchInfo;
use gihmd_core::git::Change;
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
use gihmd_core::git::StatusEntry;
use gihmd_core::git::host_of;
use git2::BranchType;
use git2::Delta;
use git2::DiffOptions;
use git2::FetchOptions;
use git2::IndexAddOption;
use git2::Oid;
use git2::PushOptions;
use git2::Repository;
use git2::Sort;
use git2::Status;
use git2::StatusOptions;
use git2::build::CheckoutBuilder;
use git2::build::RepoBuilder;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::callbacks::remote_callbacks;
use crate::convert::lib_error;
use crate::convert::open_error;
use crate::convert::remote_error;

pub(crate) fn open(root: &Path) -> Result<Repository, GitError> {
    Repository::open(root).map_err(|error| open_error(&error))
}

pub(crate) fn init(root: &Path) -> Result<Repository, GitError> {
    let mut options = git2::RepositoryInitOptions::new();
    options.initial_head("main");

    Repository::init_opts(root, &options).map_err(|error| lib_error(&error))
}

fn staged_change(status: Status) -> Option<Change> {
    if status.contains(Status::INDEX_NEW) {
        Some(Change::Added)
    } else if status.contains(Status::INDEX_MODIFIED) {
        Some(Change::Modified)
    } else if status.contains(Status::INDEX_DELETED) {
        Some(Change::Deleted)
    } else if status.contains(Status::INDEX_RENAMED) {
        Some(Change::Renamed)
    } else if status.contains(Status::INDEX_TYPECHANGE) {
        Some(Change::TypeChange)
    } else {
        None
    }
}

fn unstaged_change(status: Status) -> Option<Change> {
    if status.contains(Status::CONFLICTED) {
        Some(Change::Conflicted)
    } else if status.contains(Status::WT_NEW) {
        Some(Change::Untracked)
    } else if status.contains(Status::WT_MODIFIED) {
        Some(Change::Modified)
    } else if status.contains(Status::WT_DELETED) {
        Some(Change::Deleted)
    } else if status.contains(Status::WT_RENAMED) {
        Some(Change::Renamed)
    } else if status.contains(Status::WT_TYPECHANGE) {
        Some(Change::TypeChange)
    } else {
        None
    }
}

fn head_oid(repo: &Repository) -> Option<Oid> {
    repo.head().ok().and_then(|head| head.target())
}

pub(crate) fn status(repo: &Repository) -> Result<RepoStatus, GitError> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .exclude_submodules(true)
        .renames_head_to_index(true)
        .include_ignored(false);
    let statuses = repo
        .statuses(Some(&mut options))
        .map_err(|error| lib_error(&error))?;
    let mut entries = Vec::new();

    for entry in statuses.iter() {
        let Ok(path) = entry.path() else {
            continue;
        };
        let Ok(path) = RelPath::parse(path) else {
            continue;
        };
        let flags = entry.status();
        let staged = staged_change(flags);
        let unstaged = unstaged_change(flags);

        if staged.is_some() || unstaged.is_some() {
            entries.push(StatusEntry {
                path,
                staged,
                unstaged,
            });
        }
    }

    let head = repo.head().ok();
    let branch = head
        .as_ref()
        .filter(|head| head.is_branch())
        .and_then(|head| head.shorthand().ok().map(str::to_owned));
    let head_id = head.as_ref().and_then(git2::Reference::target);
    let (ahead, behind, upstream) = match &branch {
        Some(name) => upstream_distance(repo, name),
        None => (0, 0, None),
    };

    Ok(RepoStatus {
        branch,
        head: head_id.map(|oid| oid.to_string()),
        ahead,
        behind,
        upstream,
        entries,
    })
}

fn upstream_distance(repo: &Repository, branch: &str) -> (usize, usize, Option<String>) {
    let Ok(local) = repo.find_branch(branch, BranchType::Local) else {
        return (0, 0, None);
    };
    let Ok(upstream) = local.upstream() else {
        return (0, 0, None);
    };
    let name = upstream.name().ok().flatten().map(str::to_owned);
    let (Some(local_oid), Some(upstream_oid)) = (local.get().target(), upstream.get().target())
    else {
        return (0, 0, name);
    };
    let (ahead, behind) = repo
        .graph_ahead_behind(local_oid, upstream_oid)
        .unwrap_or((0, 0));

    (ahead, behind, name)
}

fn pathspecs(paths: &[RelPath]) -> Vec<&str> {
    paths.iter().map(RelPath::as_str).collect()
}

pub(crate) fn stage(repo: &Repository, paths: &[RelPath]) -> Result<(), GitError> {
    let mut index = repo.index().map_err(|error| lib_error(&error))?;
    let specs = pathspecs(paths);
    index
        .add_all(specs.iter(), IndexAddOption::DEFAULT, None)
        .map_err(|error| lib_error(&error))?;
    index
        .update_all(specs.iter(), None)
        .map_err(|error| lib_error(&error))?;
    index.write().map_err(|error| lib_error(&error))?;

    Ok(())
}

pub(crate) fn unstage(repo: &Repository, paths: &[RelPath]) -> Result<(), GitError> {
    let specs = pathspecs(paths);

    if let Some(oid) = head_oid(repo) {
        let commit = repo.find_commit(oid).map_err(|error| lib_error(&error))?;
        repo.reset_default(Some(commit.as_object()), specs.iter())
            .map_err(|error| lib_error(&error))?;

        return Ok(());
    }

    let mut index = repo.index().map_err(|error| lib_error(&error))?;

    for path in paths {
        let _ = index.remove_path(Path::new(path.as_str()));
    }

    index.write().map_err(|error| lib_error(&error))?;

    Ok(())
}

pub(crate) fn discard(repo: &Repository, paths: &[RelPath]) -> Result<(), GitError> {
    let Some(workdir) = repo.workdir() else {
        return Err(GitError::NotARepository);
    };
    let mut builder = CheckoutBuilder::new();
    builder.force().remove_untracked(true);

    for path in paths {
        builder.path(path.as_str());
    }

    repo.checkout_head(Some(&mut builder))
        .map_err(|error| lib_error(&error))?;
    let index = repo.index().map_err(|error| lib_error(&error))?;

    for path in paths {
        if index.get_path(Path::new(path.as_str()), 0).is_none() {
            let full = path.join(workdir);

            if full.is_file() {
                std::fs::remove_file(&full).map_err(|source| GitError::Io { source })?;
            }
        }
    }

    Ok(())
}

pub(crate) fn commit(repo: &Repository, message: &str) -> Result<String, GitError> {
    let signature = repo.signature().ok().ok_or(GitError::IdentityMissing)?;
    let mut index = repo.index().map_err(|error| lib_error(&error))?;
    let tree_id = index.write_tree().map_err(|error| lib_error(&error))?;
    let tree = repo.find_tree(tree_id).map_err(|error| lib_error(&error))?;
    let parent = head_oid(repo)
        .map(|oid| repo.find_commit(oid))
        .transpose()
        .map_err(|error| lib_error(&error))?;

    if let Some(parent) = &parent
        && parent.tree_id() == tree_id
    {
        return Err(GitError::NothingToCommit);
    }

    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    let oid = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )
        .map_err(|error| lib_error(&error))?;

    Ok(oid.to_string())
}

fn commit_info(commit: &git2::Commit<'_>) -> CommitInfo {
    let oid = commit.id().to_string();
    let author = commit.author();

    CommitInfo {
        short_oid: oid.chars().take(7).collect(),
        oid,
        summary: commit.summary().ok().flatten().unwrap_or("").to_owned(),
        body: commit.body().ok().flatten().unwrap_or("").to_owned(),
        author_name: author.name().unwrap_or("").to_owned(),
        author_email: author.email().unwrap_or("").to_owned(),
        time_unix: commit.time().seconds(),
        parents: commit.parent_ids().map(|id| id.to_string()).collect(),
    }
}

pub(crate) fn log(repo: &Repository, query: &LogQuery) -> Result<Vec<CommitInfo>, GitError> {
    let mut walk = repo.revwalk().map_err(|error| lib_error(&error))?;
    walk.set_sorting(Sort::TIME)
        .map_err(|error| lib_error(&error))?;

    if let Some(rev) = &query.rev {
        let object = repo
            .revparse_single(rev)
            .map_err(|error| lib_error(&error))?;
        walk.push(object.id()).map_err(|error| lib_error(&error))?;
    } else {
        if head_oid(repo).is_none() {
            return Ok(Vec::new());
        }

        walk.push_head().map_err(|error| lib_error(&error))?;
    }

    let mut commits = Vec::new();

    for oid in walk.skip(query.skip).take(query.limit) {
        let oid = oid.map_err(|error| lib_error(&error))?;
        let commit = repo.find_commit(oid).map_err(|error| lib_error(&error))?;
        commits.push(commit_info(&commit));
    }

    Ok(commits)
}

fn change_of(delta: Delta) -> Change {
    match delta {
        Delta::Added | Delta::Copied => Change::Added,
        Delta::Deleted => Change::Deleted,
        Delta::Renamed => Change::Renamed,
        Delta::Typechange => Change::TypeChange,
        Delta::Untracked => Change::Untracked,
        Delta::Conflicted => Change::Conflicted,
        Delta::Modified | Delta::Unmodified | Delta::Ignored | Delta::Unreadable => {
            Change::Modified
        }
    }
}

pub(crate) fn commit_files(repo: &Repository, oid: &str) -> Result<Vec<ChangedFile>, GitError> {
    let oid = Oid::from_str(oid).map_err(|error| lib_error(&error))?;
    let commit = repo.find_commit(oid).map_err(|error| lib_error(&error))?;
    let tree = commit.tree().map_err(|error| lib_error(&error))?;
    let parent_tree = commit.parent(0).ok().and_then(|parent| parent.tree().ok());
    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
        .map_err(|error| lib_error(&error))?;
    let mut files = Vec::new();

    for (index, delta) in diff.deltas().enumerate() {
        let Some(path) = delta.new_file().path().or_else(|| delta.old_file().path()) else {
            continue;
        };
        let Ok(path) = RelPath::from_relative(path) else {
            continue;
        };
        let (additions, deletions) = git2::Patch::from_diff(&diff, index)
            .ok()
            .flatten()
            .and_then(|patch| patch.line_stats().ok())
            .map_or((0, 0), |(_, additions, deletions)| (additions, deletions));
        files.push(ChangedFile {
            path,
            change: change_of(delta.status()),
            additions,
            deletions,
        });
    }

    Ok(files)
}

fn blob_text(repo: &Repository, tree: Option<&git2::Tree<'_>>, path: &RelPath) -> (String, bool) {
    let Some(tree) = tree else {
        return (String::new(), false);
    };
    let Ok(entry) = tree.get_path(Path::new(path.as_str())) else {
        return (String::new(), false);
    };
    let Ok(object) = entry.to_object(repo) else {
        return (String::new(), false);
    };
    let Ok(blob) = object.peel_to_blob() else {
        return (String::new(), false);
    };

    if blob.is_binary() {
        return (String::new(), true);
    }

    (String::from_utf8_lossy(blob.content()).into_owned(), false)
}

fn index_text(repo: &Repository, path: &RelPath) -> (String, bool) {
    let Ok(index) = repo.index() else {
        return (String::new(), false);
    };
    let Some(entry) = index.get_path(Path::new(path.as_str()), 0) else {
        return (String::new(), false);
    };
    let Ok(blob) = repo.find_blob(entry.id) else {
        return (String::new(), false);
    };

    if blob.is_binary() {
        return (String::new(), true);
    }

    (String::from_utf8_lossy(blob.content()).into_owned(), false)
}

fn workdir_text(repo: &Repository, path: &RelPath) -> (String, bool) {
    let Some(workdir) = repo.workdir() else {
        return (String::new(), false);
    };
    let Ok(bytes) = std::fs::read(path.join(workdir)) else {
        return (String::new(), false);
    };

    if bytes.contains(&0) {
        return (String::new(), true);
    }

    (String::from_utf8_lossy(&bytes).into_owned(), false)
}

fn head_tree(repo: &Repository) -> Option<git2::Tree<'_>> {
    head_oid(repo)
        .and_then(|oid| repo.find_commit(oid).ok())
        .and_then(|commit| commit.tree().ok())
}

pub(crate) fn diff_file(
    repo: &Repository,
    path: &RelPath,
    kind: &DiffKind,
) -> Result<FileDiff, GitError> {
    let mut options = DiffOptions::new();
    options.pathspec(path.as_str());
    let (old_label, new_label, (old_text, old_binary), (new_text, new_binary), diff) = match kind {
        DiffKind::Unstaged => {
            let diff = repo
                .diff_index_to_workdir(None, Some(&mut options))
                .map_err(|error| lib_error(&error))?;

            (
                "index".to_owned(),
                "working tree".to_owned(),
                index_text(repo, path),
                workdir_text(repo, path),
                diff,
            )
        }
        DiffKind::Staged => {
            let head = head_tree(repo);
            let diff = repo
                .diff_tree_to_index(head.as_ref(), None, Some(&mut options))
                .map_err(|error| lib_error(&error))?;

            (
                "HEAD".to_owned(),
                "index".to_owned(),
                blob_text(repo, head.as_ref(), path),
                index_text(repo, path),
                diff,
            )
        }
        DiffKind::Commit { oid } => {
            let oid = Oid::from_str(oid).map_err(|error| lib_error(&error))?;
            let commit = repo.find_commit(oid).map_err(|error| lib_error(&error))?;
            let tree = commit.tree().map_err(|error| lib_error(&error))?;
            let parent_tree = commit.parent(0).ok().and_then(|parent| parent.tree().ok());
            let diff = repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut options))
                .map_err(|error| lib_error(&error))?;
            let short: String = oid.to_string().chars().take(7).collect();

            (
                format!("{short}^"),
                short,
                blob_text(repo, parent_tree.as_ref(), path),
                blob_text(repo, Some(&tree), path),
                diff,
            )
        }
    };
    let stats = diff.stats().map_err(|error| lib_error(&error))?;

    Ok(FileDiff {
        path: path.clone(),
        old_label,
        new_label,
        old_text,
        new_text,
        binary: old_binary || new_binary,
        additions: stats.insertions(),
        deletions: stats.deletions(),
    })
}

pub(crate) fn branches(repo: &Repository) -> Result<Vec<BranchInfo>, GitError> {
    let mut result = Vec::new();

    for item in repo.branches(None).map_err(|error| lib_error(&error))? {
        let (branch, kind) = item.map_err(|error| lib_error(&error))?;
        let Ok(Some(name)) = branch.name() else {
            continue;
        };
        let upstream = branch
            .upstream()
            .ok()
            .and_then(|up| up.name().ok().flatten().map(str::to_owned));
        result.push(BranchInfo {
            name: name.to_owned(),
            is_head: branch.is_head(),
            upstream,
            remote: kind == BranchType::Remote,
        });
    }

    Ok(result)
}

pub(crate) fn checkout_branch(repo: &Repository, name: &str) -> Result<(), GitError> {
    let branch = repo
        .find_branch(name, BranchType::Local)
        .ok()
        .ok_or_else(|| GitError::BranchMissing {
            name: name.to_owned(),
        })?;
    let Ok(refname) = branch.get().name() else {
        return Err(GitError::BranchMissing {
            name: name.to_owned(),
        });
    };
    let object = repo
        .revparse_single(refname)
        .map_err(|error| lib_error(&error))?;
    let mut builder = CheckoutBuilder::new();
    builder.safe();
    repo.checkout_tree(&object, Some(&mut builder))
        .map_err(|error| checkout_error(&error))?;
    repo.set_head(refname).map_err(|error| lib_error(&error))?;

    Ok(())
}

fn checkout_error(error: &git2::Error) -> GitError {
    if error.code() == git2::ErrorCode::Conflict {
        return GitError::Conflicts { paths: Vec::new() };
    }

    lib_error(error)
}

pub(crate) fn create_branch(repo: &Repository, name: &str, checkout: bool) -> Result<(), GitError> {
    let Some(oid) = head_oid(repo) else {
        return Err(GitError::DetachedHead);
    };
    let commit = repo.find_commit(oid).map_err(|error| lib_error(&error))?;
    repo.branch(name, &commit, false)
        .map_err(|error| lib_error(&error))?;

    if checkout {
        checkout_branch(repo, name)?;
    }

    Ok(())
}

pub(crate) fn remotes(repo: &Repository) -> Result<Vec<RemoteInfo>, GitError> {
    let names = repo.remotes().map_err(|error| lib_error(&error))?;
    let mut result = Vec::new();

    for name in names.iter().filter_map(Result::ok).flatten() {
        let remote = repo.find_remote(name).map_err(|error| lib_error(&error))?;
        result.push(RemoteInfo {
            name: name.to_owned(),
            url: remote.url().unwrap_or("").to_owned(),
            push_url: remote.pushurl().ok().flatten().map(str::to_owned),
        });
    }

    Ok(result)
}

pub(crate) fn add_remote(repo: &Repository, name: &str, url: &str) -> Result<(), GitError> {
    repo.remote(name, url).map_err(|error| lib_error(&error))?;

    Ok(())
}

pub(crate) fn remove_remote(repo: &Repository, name: &str) -> Result<(), GitError> {
    repo.remote_delete(name).map_err(|error| lib_error(&error))
}

fn remote_host(url: &str) -> (String, String) {
    let scheme = if url.starts_with("https://") || url.starts_with("http://") {
        "https"
    } else {
        "ssh"
    };

    (host_of(url).unwrap_or_default(), scheme.to_owned())
}

pub(crate) fn fetch(
    repo: &Repository,
    remote_name: &str,
    auth: AuthContext,
    progress: mpsc::Sender<GitProgress>,
    cancel: CancellationToken,
) -> Result<(), GitError> {
    let mut remote = repo
        .find_remote(remote_name)
        .ok()
        .ok_or_else(|| GitError::RemoteMissing {
            name: remote_name.to_owned(),
        })?;
    let (host, scheme) = remote_host(remote.url().unwrap_or(""));
    let mut options = FetchOptions::new();
    options.remote_callbacks(remote_callbacks(auth, progress, cancel));
    let refspecs: Vec<String> = remote
        .fetch_refspecs()
        .map_err(|error| lib_error(&error))?
        .iter()
        .filter_map(Result::ok)
        .flatten()
        .map(str::to_owned)
        .collect();
    remote
        .fetch(&refspecs, Some(&mut options), None)
        .map_err(|error| remote_error(&error, &host, &scheme))?;

    Ok(())
}

pub(crate) fn pull_fast_forward(
    repo: &Repository,
    remote_name: &str,
    auth: AuthContext,
    progress: mpsc::Sender<GitProgress>,
    cancel: CancellationToken,
) -> Result<PullResult, GitError> {
    fetch(repo, remote_name, auth, progress, cancel)?;
    let head = repo.head().ok().ok_or(GitError::DetachedHead)?;

    if !head.is_branch() {
        return Err(GitError::DetachedHead);
    }

    let Ok(branch_name) = head.shorthand().map(str::to_owned) else {
        return Err(GitError::DetachedHead);
    };
    let local = repo
        .find_branch(&branch_name, BranchType::Local)
        .map_err(|error| lib_error(&error))?;
    let upstream = local.upstream().ok().ok_or(GitError::NonFastForward)?;
    let Some(upstream_oid) = upstream.get().target() else {
        return Err(GitError::NonFastForward);
    };
    let annotated = repo
        .find_annotated_commit(upstream_oid)
        .map_err(|error| lib_error(&error))?;
    let (analysis, _) = repo
        .merge_analysis(&[&annotated])
        .map_err(|error| lib_error(&error))?;

    if analysis.is_up_to_date() {
        return Ok(PullResult::UpToDate);
    }

    if !analysis.is_fast_forward() {
        return Err(GitError::NonFastForward);
    }

    let from = head.target().map(|oid| oid.to_string()).unwrap_or_default();
    let Ok(refname) = head.name() else {
        return Err(GitError::DetachedHead);
    };
    let mut reference = repo
        .find_reference(refname)
        .map_err(|error| lib_error(&error))?;
    reference
        .set_target(upstream_oid, "fast-forward")
        .map_err(|error| lib_error(&error))?;
    let mut builder = CheckoutBuilder::new();
    builder.safe();
    repo.checkout_head(Some(&mut builder))
        .map_err(|error| checkout_error(&error))?;

    Ok(PullResult::FastForwarded {
        from,
        to: upstream_oid.to_string(),
    })
}

pub(crate) fn push(
    repo: &Repository,
    remote_name: &str,
    refspec: Option<String>,
    auth: AuthContext,
    progress: mpsc::Sender<GitProgress>,
    cancel: CancellationToken,
) -> Result<(), GitError> {
    let mut remote = repo
        .find_remote(remote_name)
        .ok()
        .ok_or_else(|| GitError::RemoteMissing {
            name: remote_name.to_owned(),
        })?;
    let (host, scheme) = remote_host(remote.url().unwrap_or(""));
    let head = repo.head().ok().ok_or(GitError::DetachedHead)?;
    let branch_name = head
        .shorthand()
        .ok()
        .filter(|_| head.is_branch())
        .map(str::to_owned);
    let spec = if let Some(spec) = refspec {
        spec
    } else {
        let Some(name) = &branch_name else {
            return Err(GitError::DetachedHead);
        };

        format!("refs/heads/{name}:refs/heads/{name}")
    };
    let mut options = PushOptions::new();
    options.remote_callbacks(remote_callbacks(auth, progress, cancel));
    remote
        .push(&[spec.as_str()], Some(&mut options))
        .map_err(|error| remote_error(&error, &host, &scheme))?;

    if let Some(name) = branch_name
        && let Ok(mut local) = repo.find_branch(&name, BranchType::Local)
        && local.upstream().is_err()
    {
        let _ = local.set_upstream(Some(&format!("{remote_name}/{name}")));
    }

    Ok(())
}

pub(crate) fn clone(
    url: &str,
    dest: &Path,
    auth: AuthContext,
    progress: mpsc::Sender<GitProgress>,
    cancel: CancellationToken,
) -> Result<(), GitError> {
    let (host, scheme) = remote_host(url);
    let mut options = FetchOptions::new();
    options.remote_callbacks(remote_callbacks(auth, progress, cancel));
    RepoBuilder::new()
        .fetch_options(options)
        .clone(url, dest)
        .map_err(|error| remote_error(&error, &host, &scheme))?;

    Ok(())
}
