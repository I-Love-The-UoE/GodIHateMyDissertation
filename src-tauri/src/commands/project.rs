use std::path::PathBuf;

use gihmd_core::RelPath;
use gihmd_core::config::ProjectRecord;
use gihmd_core::normalise;
use gihmd_core::project::FileNode;
use gihmd_core::project::MainFileDetection;
use gihmd_core::project::ProjectError;
use gihmd_core::project::ProjectIndex;
use gihmd_core::project::build_index;
use gihmd_core::project::detect_main_file;
use gihmd_core::project::list_tree;
use gihmd_core::project::write_text;
use serde::Serialize;
use tauri::AppHandle;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

const TEMPLATE: &str = r"\documentclass[11pt]{article}
\usepackage[utf8]{inputenc}
\usepackage{amsmath}
\usepackage{graphicx}
\usepackage{hyperref}

\title{My Dissertation}
\author{Author}
\date{\today}

\begin{document}

\maketitle

\section{Introduction}
Hello, world.

\end{document}
";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedProject {
    pub record: ProjectRecord,
    pub tree: FileNode,
    pub main_file_detection: MainFileDetection,
}

#[tauri::command]
pub async fn project_recent(state: State<'_, AppState>) -> Result<Vec<ProjectRecord>, AppError> {
    let recent = AppState::lock(&state.projects)?.recent();

    Ok(recent)
}

async fn open_at(
    app: &AppHandle,
    state: &AppState,
    root: PathBuf,
) -> Result<OpenedProject, AppError> {
    let root = normalise(&root)?;

    if !root.is_dir() {
        return Err(ProjectError::RootMissing { root }.into());
    }

    close_current(state)?;
    let tree = list_tree(&root).await?;
    let main_file_detection = detect_main_file(&root).await?;
    let mut record = AppState::lock(&state.projects)?.touch(&root).clone();

    if record.main_file.is_none()
        && let MainFileDetection::Exactly { path } = &main_file_detection
    {
        record.main_file = Some(path.clone());
    }

    AppState::lock(&state.projects)?.update(record.clone());
    state.start_watch(app, record.clone())?;
    state.persist_projects().await?;
    tracing::info!(root = %root.display(), "Opened a project.");

    Ok(OpenedProject {
        record,
        tree,
        main_file_detection,
    })
}

fn close_current(state: &AppState) -> Result<(), AppError> {
    let mut open = AppState::lock(&state.open)?;
    *open = None;
    drop(open);

    if let Some(compiler) = AppState::lock(&state.compiler)?.as_ref() {
        compiler.cancel();
    }

    *AppState::lock(&state.latest_outcome)? = None;

    Ok(())
}

#[tauri::command]
pub async fn project_open(
    app: AppHandle,
    state: State<'_, AppState>,
    root: PathBuf,
) -> Result<OpenedProject, AppError> {
    open_at(&app, &state, root).await
}

#[tauri::command]
pub async fn project_create(
    app: AppHandle,
    state: State<'_, AppState>,
    root: PathBuf,
) -> Result<OpenedProject, AppError> {
    tokio::fs::create_dir_all(&root)
        .await
        .map_err(|source| ProjectError::Listing {
            root: root.clone(),
            source,
        })?;
    let main = RelPath::parse("main.tex")?;

    if !main.join(&root).exists() {
        write_text(&root, &main, TEMPLATE, None).await?;
    }

    open_at(&app, &state, root).await
}

#[tauri::command]
pub async fn project_close(state: State<'_, AppState>) -> Result<(), AppError> {
    close_current(&state)?;
    state.persist_projects().await
}

#[tauri::command]
pub async fn project_forget(
    state: State<'_, AppState>,
    root: PathBuf,
) -> Result<Vec<ProjectRecord>, AppError> {
    AppState::lock(&state.projects)?.forget(&root);
    state.persist_projects().await?;
    let recent = AppState::lock(&state.projects)?.recent();

    Ok(recent)
}

#[tauri::command]
pub async fn project_set_main_file(
    state: State<'_, AppState>,
    path: RelPath,
) -> Result<ProjectRecord, AppError> {
    let record = state.update_record(|record| record.main_file = Some(path))?;
    state.persist_projects().await?;

    Ok(record)
}

#[tauri::command]
pub async fn project_set_tabs(
    state: State<'_, AppState>,
    open_tabs: Vec<RelPath>,
    active_tab: Option<RelPath>,
) -> Result<ProjectRecord, AppError> {
    let record = state.update_record(|record| {
        record.open_tabs = open_tabs;
        record.active_tab = active_tab;
    })?;
    state.persist_projects().await?;

    Ok(record)
}

#[tauri::command]
pub async fn project_detect_main_file(
    state: State<'_, AppState>,
) -> Result<MainFileDetection, AppError> {
    let record = state.current_record()?;

    Ok(detect_main_file(&record.root).await?)
}

#[tauri::command]
pub async fn project_tree(state: State<'_, AppState>) -> Result<FileNode, AppError> {
    let record = state.current_record()?;

    Ok(list_tree(&record.root).await?)
}

#[tauri::command]
pub async fn project_index(state: State<'_, AppState>) -> Result<ProjectIndex, AppError> {
    let record = state.current_record()?;

    Ok(build_index(&record.root).await?)
}
