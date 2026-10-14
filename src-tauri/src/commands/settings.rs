use std::path::PathBuf;
use std::sync::Arc;

use gihmd_core::CommandSpec;
use gihmd_core::Context;
use gihmd_core::Processes;
use gihmd_core::claude::locate_binary;
use gihmd_core::config::AppSettings;
use gihmd_core::config::save_settings;
use gihmd_core::git::Identity;
use gihmd_core::git::Repository;
use gihmd_core::tex::TexToolchain;
use gihmd_core::tex::resolve_toolchain;
use serde::Serialize;
use tauri::AppHandle;
use tauri::Manager;
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
        resolve_toolchain(Some(dir), None, state.managed_tex.root())?;
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
pub async fn tex_probe_toolchain(
    state: State<'_, AppState>,
    explicit: Option<PathBuf>,
) -> Result<TexToolchain, AppError> {
    Ok(resolve_toolchain(
        explicit.as_deref(),
        state.managed_tex.bin_dir(),
        state.managed_tex.root(),
    )?)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedTexStatus {
    pub root: PathBuf,
    pub installed: bool,
    pub bin_dir: Option<PathBuf>,
    pub perl_available: bool,
    pub custom_dir: Option<PathBuf>,
}

#[tauri::command]
pub async fn tex_managed_status(state: State<'_, AppState>) -> Result<ManagedTexStatus, AppError> {
    let bin_dir = state.managed_tex.bin_dir();
    // NOTE: TeX Live bundles its own Perl on Windows; elsewhere the installer needs the system one.
    let perl_available = cfg!(windows)
        || state
            .context
            .processes()
            .capture(CommandSpec {
                program: PathBuf::from("perl"),
                args: vec!["-v".to_owned()],
                cwd: std::env::temp_dir(),
                env: Vec::new(),
                path_prepend: Vec::new(),
            })
            .await
            .is_ok_and(|captured| captured.outcome.code == Some(0));

    Ok(ManagedTexStatus {
        root: state.managed_tex.root().to_path_buf(),
        installed: bin_dir.is_some(),
        bin_dir,
        perl_available,
        custom_dir: state.settings()?.tex_bin_dir.clone(),
    })
}

#[tauri::command]
pub async fn tex_install_managed(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut running = AppState::lock(&state.tex_install)?;

    if running.as_ref().is_some_and(|task| !task.is_finished()) {
        return Ok(());
    }

    let handle = app.clone();
    *running = Some(tokio::spawn(async move {
        let state = handle.state::<AppState>();
        let events = AppState::tex_install_sender(&handle);
        let context = Arc::clone(&state.context);
        let outcome = state.managed_tex.install(context.as_ref(), events).await;

        if outcome.is_ok()
            && let Err(error) = state.reset_compiler()
        {
            tracing::warn!(error = %error, "The compiler could not be reset after installing TeX Live.");
        }
    }));

    Ok(())
}

#[tauri::command]
pub async fn git_identity(state: State<'_, AppState>) -> Result<Option<Identity>, AppError> {
    Ok(state.context.repository().identity().await?)
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
