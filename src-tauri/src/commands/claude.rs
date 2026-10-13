use std::fmt::Write;

use gihmd_core::Context;
use gihmd_core::claude::ClaudeSession;
use gihmd_core::claude::PermissionDecision;
use gihmd_core::claude::PermissionMode;
use gihmd_core::claude::SessionConfig;
use gihmd_core::claude::locate_binary;
use gihmd_core::tex::Severity;
use gihmd_core::tex::build_dir_for;
use serde::Serialize;
use tauri::AppHandle;
use tauri::State;
use uuid::Uuid;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartedSession {
    pub session_id: Uuid,
    pub resumed: bool,
}

fn system_prompt(root: &std::path::Path, build_dir: &std::path::Path) -> String {
    format!(
        "You are working inside GodIHateMyDissertation, a LaTeX editor. The user's LaTeX project root is {} \
         and compiled output (PDF, .log, .synctex.gz) lives in {}. The editor compiles with latexmk and shows \
         the user compile diagnostics; when the user attaches diagnostics, fix the LaTeX source rather than the \
         build output. Prefer small, targeted edits to .tex and .bib files.",
        root.display(),
        build_dir.display(),
    )
}

#[tauri::command]
pub async fn claude_start_session(
    app: AppHandle,
    state: State<'_, AppState>,
    resume: bool,
    permission_mode: PermissionMode,
    model: Option<String>,
) -> Result<StartedSession, AppError> {
    let record = state.current_record()?;
    let explicit = state.settings()?.claude_path.clone();
    let claude_path = locate_binary(explicit.as_deref())?;
    let (session_id, resumed) = match (resume, record.claude_session_id) {
        (true, Some(existing)) => (existing, true),
        _ => (Uuid::new_v4(), false),
    };
    let build_dir = build_dir_for(&state.context.paths().cache_dir, &record.root);
    let config = SessionConfig {
        claude_path,
        cwd: record.root.clone(),
        session_id,
        resume: resumed,
        permission_mode,
        model,
        append_system_prompt: system_prompt(&record.root, &build_dir),
        extra_path: Vec::new(),
    };
    let events = state.claude_event_sender(&app)?;
    let session = ClaudeSession::spawn(state.context.processes(), config, events).await?;
    AppState::lock(&state.claude)?.insert(session_id, std::sync::Arc::new(session));
    state.update_record(|record| record.claude_session_id = Some(session_id))?;
    state.persist_projects().await?;
    tracing::info!(session_id = %session_id, resumed, "Started a Claude Code session.");

    Ok(StartedSession {
        session_id,
        resumed,
    })
}

fn diagnostics_preamble(state: &AppState) -> Result<Option<String>, AppError> {
    let latest = AppState::lock(&state.latest_outcome)?;
    let Some(outcome) = latest.as_ref() else {
        return Ok(None);
    };
    let mut text = String::from("Latest compile diagnostics from latexmk:\n```\n");

    for diagnostic in outcome
        .diagnostics
        .iter()
        .filter(|d| d.severity != Severity::BadBox)
        .take(40)
    {
        let location = match (&diagnostic.file, diagnostic.line) {
            (Some(file), Some(line)) => format!("{}:{line}", file.display()),
            (Some(file), None) => file.display().to_string(),
            _ => "unknown".to_owned(),
        };
        let _ = writeln!(
            text,
            "[{:?}] {location}: {}",
            diagnostic.severity, diagnostic.message
        );
    }

    text.push_str("```\n");

    Ok(Some(text))
}

#[tauri::command]
pub async fn claude_send(
    state: State<'_, AppState>,
    session_id: Uuid,
    text: String,
    attach_diagnostics: bool,
) -> Result<(), AppError> {
    let preamble = if attach_diagnostics {
        diagnostics_preamble(&state)?
    } else {
        None
    };
    let session = state.claude_session(session_id)?;
    session.send_user(&text, preamble.as_deref()).await?;

    Ok(())
}

#[tauri::command]
pub async fn claude_permission(
    state: State<'_, AppState>,
    session_id: Uuid,
    request_id: String,
    decision: PermissionDecision,
) -> Result<(), AppError> {
    let session = state.claude_session(session_id)?;
    session.respond_permission(&request_id, decision).await?;

    Ok(())
}

#[tauri::command]
pub async fn claude_interrupt(
    state: State<'_, AppState>,
    session_id: Uuid,
) -> Result<(), AppError> {
    let session = state.claude_session(session_id)?;
    session.interrupt().await?;

    Ok(())
}

#[tauri::command]
pub async fn claude_stop(state: State<'_, AppState>, session_id: Uuid) -> Result<(), AppError> {
    let session = AppState::lock(&state.claude)?.remove(&session_id);

    if let Some(session) = session {
        session.shutdown().await?;
    }

    Ok(())
}

#[tauri::command]
pub async fn claude_sessions(state: State<'_, AppState>) -> Result<Vec<Uuid>, AppError> {
    let ids = AppState::lock(&state.claude)?.keys().copied().collect();

    Ok(ids)
}
