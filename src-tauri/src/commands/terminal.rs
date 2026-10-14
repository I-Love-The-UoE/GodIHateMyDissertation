use std::sync::Arc;

use base64::Engine as _;
use gihmd_core::Context;
use gihmd_core::TerminalError;
use gihmd_core::TerminalEvent;
use gihmd_core::TerminalSession;
use gihmd_core::TerminalSpec;
use gihmd_core::Terminals;
use gihmd_core::default_shell;
use gihmd_core::tex::compile_dir;
use serde::Serialize;
use tauri::AppHandle;
use tauri::Manager;
use tauri::State;
use tokio::sync::mpsc;

use crate::error::AppError;
use crate::events;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TerminalPayload {
    Output { id: u64, data: String },
    Exited { id: u64, code: Option<u32> },
}

#[tauri::command]
pub async fn terminal_open(
    app: AppHandle,
    state: State<'_, AppState>,
    cols: u16,
    rows: u16,
) -> Result<u64, AppError> {
    let record = state.current_record()?;
    let (program, args) = default_shell()?;
    let cwd = record.main_file.as_ref().map_or_else(
        || record.root.clone(),
        |main| compile_dir(&record.root, main),
    );
    // NOTE: A TeX distribution that cannot be found is not an error for the terminal itself.
    let path_prepend = state
        .resolve_toolchain()
        .map(|toolchain| vec![toolchain.bin_dir])
        .unwrap_or_default();
    let id = {
        let mut next = AppState::lock(&state.next_terminal_id)?;
        *next += 1;

        *next
    };
    let (sender, mut receiver) = mpsc::channel::<TerminalEvent>(256);
    let session = Arc::new(state.context.terminals().spawn(
        TerminalSpec {
            program,
            args,
            cwd,
            env: Vec::new(),
            path_prepend,
            cols,
            rows,
        },
        sender,
    )?);
    AppState::lock(&state.terminals)?.insert(id, Arc::clone(&session));
    let handle = app.clone();
    drop(tokio::spawn(async move {
        while let Some(event) = receiver.recv().await {
            match event {
                TerminalEvent::Output(bytes) => {
                    let data = base64::engine::general_purpose::STANDARD.encode(bytes);
                    events::emit(
                        &handle,
                        events::TERMINAL_EVENT,
                        TerminalPayload::Output { id, data },
                    );
                }
                TerminalEvent::Exited { code } => {
                    events::emit(
                        &handle,
                        events::TERMINAL_EVENT,
                        TerminalPayload::Exited { id, code },
                    );
                }
            }
        }
    }));
    let waiter = app.clone();
    drop(tokio::task::spawn_blocking(move || {
        let code = session.wait();
        let state = waiter.state::<AppState>();

        if let Ok(mut terminals) = state.terminals.lock() {
            terminals.remove(&id);
        }

        events::emit(
            &waiter,
            events::TERMINAL_EVENT,
            TerminalPayload::Exited { id, code },
        );
    }));
    tracing::info!(id, "Opened a terminal.");

    Ok(id)
}

fn session(state: &AppState, id: u64) -> Result<Arc<gihmd_pty::PtySession>, AppError> {
    let terminals = AppState::lock(&state.terminals)?;

    terminals
        .get(&id)
        .map(Arc::clone)
        .ok_or_else(|| TerminalError::Missing { id }.into())
}

#[tauri::command]
pub async fn terminal_write(
    state: State<'_, AppState>,
    id: u64,
    data: String,
) -> Result<(), AppError> {
    let session = session(&state, id)?;
    session.write(data.as_bytes())?;

    Ok(())
}

#[tauri::command]
pub async fn terminal_resize(
    state: State<'_, AppState>,
    id: u64,
    cols: u16,
    rows: u16,
) -> Result<(), AppError> {
    let session = session(&state, id)?;
    session.resize(cols, rows)?;

    Ok(())
}

#[tauri::command]
pub async fn terminal_close(state: State<'_, AppState>, id: u64) -> Result<(), AppError> {
    let session = AppState::lock(&state.terminals)?.remove(&id);

    if let Some(session) = session {
        session.kill()?;
    }

    Ok(())
}
