use std::path::PathBuf;

use gihmd_core::CommandSpec;
use gihmd_core::Context;
use gihmd_core::Processes;
use gihmd_core::claude::locate_binary;
use gihmd_core::config::AppSettings;
use gihmd_core::config::save_settings;
use gihmd_core::tex::TexToolchain;
use gihmd_core::tex::resolve_toolchain;
use serde::Serialize;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
    let settings = state.settings()?.clone();

    Ok(settings)
}

#[tauri::command]
pub async fn settings_set(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, AppError> {
    settings.validate()?;

    if let Some(dir) = &settings.tex_bin_dir {
        resolve_toolchain(Some(dir))?;
    }

    if let Some(path) = &settings.claude_path {
        locate_binary(Some(path))?;
    }

    save_settings(&state.context.paths().config_dir, &settings).await?;
    let tex_changed = {
        let mut current = state.settings_mut()?;
        let changed = current.tex_bin_dir != settings.tex_bin_dir;
        *current = settings.clone();

        changed
    };

    if tex_changed {
        state.reset_compiler()?;
    }

    Ok(settings)
}

#[tauri::command]
pub async fn tex_probe_toolchain(explicit: Option<PathBuf>) -> Result<TexToolchain, AppError> {
    Ok(resolve_toolchain(explicit.as_deref())?)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeProbe {
    pub path: PathBuf,
    pub version: String,
}

#[tauri::command]
pub async fn claude_probe_binary(
    state: State<'_, AppState>,
    explicit: Option<PathBuf>,
) -> Result<ClaudeProbe, AppError> {
    let path = locate_binary(explicit.as_deref())?;
    let cwd = std::env::temp_dir();
    let spec = CommandSpec {
        program: path.clone(),
        args: vec!["--version".to_owned()],
        cwd,
        env: Vec::new(),
        path_prepend: path
            .parent()
            .map(std::path::Path::to_path_buf)
            .into_iter()
            .collect(),
    };
    let captured = state.context.processes().capture(spec).await?;

    Ok(ClaudeProbe {
        path,
        version: captured.stdout.trim().to_owned(),
    })
}
