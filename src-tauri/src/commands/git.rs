use std::path::PathBuf;

use gihmd_core::Context;
use gihmd_core::RelPath;
use gihmd_core::Secrets;
use gihmd_core::StoredCredential;
use gihmd_core::git::BranchInfo;
use gihmd_core::git::ChangedFile;
use gihmd_core::git::CommitInfo;
use gihmd_core::git::DiffKind;
use gihmd_core::git::FileDiff;
use gihmd_core::git::GitProgress;
use gihmd_core::git::LogQuery;
use gihmd_core::git::PullResult;
use gihmd_core::git::RemoteInfo;
use gihmd_core::git::RepoStatus;
use gihmd_core::git::Repository;
use serde::Serialize;
use tauri::AppHandle;
use tauri::State;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::events;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    operation: String,
    progress: GitProgress,
}

fn changed(app: &AppHandle) {
    events::emit(app, events::GIT_CHANGED, ());
}

fn progress_channel(app: &AppHandle, operation: &str) -> mpsc::Sender<GitProgress> {
    let (sender, mut receiver) = mpsc::channel::<GitProgress>(64);
    let handle = app.clone();
    let operation = operation.to_owned();
    drop(tokio::spawn(async move {
        while let Some(progress) = receiver.recv().await {
            events::emit(
                &handle,
                events::GIT_PROGRESS,
                ProgressPayload {
                    operation: operation.clone(),
                    progress,
                },
            );
        }
    }));

    sender
}

fn new_cancel(state: &AppState) -> Result<CancellationToken, AppError> {
    let token = CancellationToken::new();
    *AppState::lock(&state.git_cancel)? = Some(token.clone());

    Ok(token)
}

#[tauri::command]
pub async fn git_open_or_none(state: State<'_, AppState>) -> Result<Option<RepoStatus>, AppError> {
    let record = state.current_record()?;

    Ok(state
        .context
        .repository()
        .open_or_none(&record.root)
        .await?)
}

#[tauri::command]
pub async fn git_init(app: AppHandle, state: State<'_, AppState>) -> Result<RepoStatus, AppError> {
    let record = state.current_record()?;
    let status = state.context.repository().init(&record.root).await?;
    changed(&app);

    Ok(status)
}

#[tauri::command]
pub async fn git_status(state: State<'_, AppState>) -> Result<RepoStatus, AppError> {
    let record = state.current_record()?;

    Ok(state.context.repository().status(&record.root).await?)
}

#[tauri::command]
pub async fn git_stage(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<RelPath>,
) -> Result<RepoStatus, AppError> {
    let record = state.current_record()?;
    state
        .context
        .repository()
        .stage(&record.root, &paths)
        .await?;
    changed(&app);

    Ok(state.context.repository().status(&record.root).await?)
}

#[tauri::command]
pub async fn git_unstage(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<RelPath>,
) -> Result<RepoStatus, AppError> {
    let record = state.current_record()?;
    state
        .context
        .repository()
        .unstage(&record.root, &paths)
        .await?;
    changed(&app);

    Ok(state.context.repository().status(&record.root).await?)
}

#[tauri::command]
pub async fn git_discard(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<RelPath>,
) -> Result<RepoStatus, AppError> {
    let record = state.current_record()?;
    state
        .context
        .repository()
        .discard(&record.root, &paths)
        .await?;
    changed(&app);

    Ok(state.context.repository().status(&record.root).await?)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitOutcome {
    pub oid: String,
    pub status: RepoStatus,
}

#[tauri::command]
pub async fn git_commit(
    app: AppHandle,
    state: State<'_, AppState>,
    message: String,
) -> Result<CommitOutcome, AppError> {
    let record = state.current_record()?;
    let oid = state
        .context
        .repository()
        .commit(&record.root, &message)
        .await?;
    changed(&app);
    let status = state.context.repository().status(&record.root).await?;

    Ok(CommitOutcome { oid, status })
}

#[tauri::command]
pub async fn git_log(
    state: State<'_, AppState>,
    query: LogQuery,
) -> Result<Vec<CommitInfo>, AppError> {
    let record = state.current_record()?;

    Ok(state.context.repository().log(&record.root, query).await?)
}

#[tauri::command]
pub async fn git_commit_files(
    state: State<'_, AppState>,
    oid: String,
) -> Result<Vec<ChangedFile>, AppError> {
    let record = state.current_record()?;

    Ok(state
        .context
        .repository()
        .commit_files(&record.root, &oid)
        .await?)
}

#[tauri::command]
pub async fn git_diff_file(
    state: State<'_, AppState>,
    path: RelPath,
    kind: DiffKind,
) -> Result<FileDiff, AppError> {
    let record = state.current_record()?;

    Ok(state
        .context
        .repository()
        .diff_file(&record.root, &path, kind)
        .await?)
}

#[tauri::command]
pub async fn git_branches(state: State<'_, AppState>) -> Result<Vec<BranchInfo>, AppError> {
    let record = state.current_record()?;

    Ok(state.context.repository().branches(&record.root).await?)
}

#[tauri::command]
pub async fn git_create_branch(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    checkout: bool,
) -> Result<Vec<BranchInfo>, AppError> {
    let record = state.current_record()?;
    state
        .context
        .repository()
        .create_branch(&record.root, &name, checkout)
        .await?;
    changed(&app);

    Ok(state.context.repository().branches(&record.root).await?)
}

#[tauri::command]
pub async fn git_checkout_branch(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<RepoStatus, AppError> {
    let record = state.current_record()?;
    state
        .context
        .repository()
        .checkout_branch(&record.root, &name)
        .await?;
    changed(&app);

    Ok(state.context.repository().status(&record.root).await?)
}

#[tauri::command]
pub async fn git_remotes(state: State<'_, AppState>) -> Result<Vec<RemoteInfo>, AppError> {
    let record = state.current_record()?;

    Ok(state.context.repository().remotes(&record.root).await?)
}

#[tauri::command]
pub async fn git_add_remote(
    state: State<'_, AppState>,
    name: String,
    url: String,
) -> Result<Vec<RemoteInfo>, AppError> {
    let record = state.current_record()?;
    state
        .context
        .repository()
        .add_remote(&record.root, &name, &url)
        .await?;

    Ok(state.context.repository().remotes(&record.root).await?)
}

#[tauri::command]
pub async fn git_remove_remote(
    state: State<'_, AppState>,
    name: String,
) -> Result<Vec<RemoteInfo>, AppError> {
    let record = state.current_record()?;
    state
        .context
        .repository()
        .remove_remote(&record.root, &name)
        .await?;

    Ok(state.context.repository().remotes(&record.root).await?)
}

#[tauri::command]
pub async fn git_fetch(
    app: AppHandle,
    state: State<'_, AppState>,
    remote: String,
) -> Result<RepoStatus, AppError> {
    let record = state.current_record()?;
    let cancel = new_cancel(&state)?;
    gihmd_core::git::fetch(
        state.context.as_ref(),
        &record.root,
        &remote,
        progress_channel(&app, "fetch"),
        cancel,
    )
    .await?;
    changed(&app);

    Ok(state.context.repository().status(&record.root).await?)
}

#[tauri::command]
pub async fn git_pull(
    app: AppHandle,
    state: State<'_, AppState>,
    remote: String,
) -> Result<PullResult, AppError> {
    let record = state.current_record()?;
    let cancel = new_cancel(&state)?;
    let result = gihmd_core::git::pull(
        state.context.as_ref(),
        &record.root,
        &remote,
        progress_channel(&app, "pull"),
        cancel,
    )
    .await?;
    changed(&app);

    Ok(result)
}

#[tauri::command]
pub async fn git_push(
    app: AppHandle,
    state: State<'_, AppState>,
    remote: String,
    refspec: Option<String>,
) -> Result<RepoStatus, AppError> {
    let record = state.current_record()?;
    let cancel = new_cancel(&state)?;
    gihmd_core::git::push(
        state.context.as_ref(),
        &record.root,
        &remote,
        refspec,
        progress_channel(&app, "push"),
        cancel,
    )
    .await?;
    changed(&app);

    Ok(state.context.repository().status(&record.root).await?)
}

#[tauri::command]
pub async fn git_clone(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    dest: PathBuf,
) -> Result<PathBuf, AppError> {
    let cancel = new_cancel(&state)?;
    gihmd_core::git::clone_repository(
        state.context.as_ref(),
        &url,
        &dest,
        progress_channel(&app, "clone"),
        cancel,
    )
    .await?;

    Ok(dest)
}

#[tauri::command]
pub async fn git_cancel(state: State<'_, AppState>) -> Result<(), AppError> {
    if let Some(token) = AppState::lock(&state.git_cancel)?.as_ref() {
        token.cancel();
    }

    Ok(())
}

#[tauri::command]
pub async fn git_credential_store(
    state: State<'_, AppState>,
    host: String,
    username: String,
    secret: String,
) -> Result<(), AppError> {
    state
        .context
        .secrets()
        .store(&host, &StoredCredential { username, secret })
        .await?;

    Ok(())
}

#[tauri::command]
pub async fn git_credential_forget(
    state: State<'_, AppState>,
    host: String,
) -> Result<(), AppError> {
    state.context.secrets().forget(&host).await?;

    Ok(())
}
