use gihmd_core::RelPath;
use gihmd_core::project::FileNode;
use gihmd_core::project::FileStat;
use gihmd_core::project::create_dir;
use gihmd_core::project::create_file;
use gihmd_core::project::delete_path;
use gihmd_core::project::list_tree;
use gihmd_core::project::move_path;
use gihmd_core::project::read_text;
use gihmd_core::project::rename_path;
use gihmd_core::project::write_text;
use serde::Serialize;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContents {
    pub text: String,
    pub stat: FileStat,
}

#[tauri::command]
pub async fn file_read(
    state: State<'_, AppState>,
    path: RelPath,
) -> Result<FileContents, AppError> {
    let record = state.current_record()?;
    let (text, stat) = read_text(&record.root, &path).await?;

    Ok(FileContents { text, stat })
}

#[tauri::command]
pub async fn file_write(
    state: State<'_, AppState>,
    path: RelPath,
    text: String,
    expected_mtime_ms: Option<u64>,
) -> Result<FileStat, AppError> {
    let record = state.current_record()?;

    Ok(write_text(&record.root, &path, &text, expected_mtime_ms).await?)
}

#[tauri::command]
pub async fn file_create(state: State<'_, AppState>, path: RelPath) -> Result<FileNode, AppError> {
    let record = state.current_record()?;
    create_file(&record.root, &path).await?;

    Ok(list_tree(&record.root).await?)
}

#[tauri::command]
pub async fn dir_create(state: State<'_, AppState>, path: RelPath) -> Result<FileNode, AppError> {
    let record = state.current_record()?;
    create_dir(&record.root, &path).await?;

    Ok(list_tree(&record.root).await?)
}

#[tauri::command]
pub async fn path_rename(
    state: State<'_, AppState>,
    from: RelPath,
    to: RelPath,
) -> Result<FileNode, AppError> {
    let record = state.current_record()?;
    rename_path(&record.root, &from, &to).await?;

    Ok(list_tree(&record.root).await?)
}

#[tauri::command]
pub async fn path_delete(state: State<'_, AppState>, path: RelPath) -> Result<FileNode, AppError> {
    let record = state.current_record()?;
    delete_path(&record.root, &path).await?;

    Ok(list_tree(&record.root).await?)
}

#[tauri::command]
pub async fn path_move(
    state: State<'_, AppState>,
    from: RelPath,
    to_dir: Option<RelPath>,
) -> Result<FileNode, AppError> {
    let record = state.current_record()?;
    move_path(&record.root, &from, to_dir.as_ref()).await?;

    Ok(list_tree(&record.root).await?)
}
