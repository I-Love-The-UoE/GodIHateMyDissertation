use gihmd_core::Context;
use gihmd_core::RelPath;
use gihmd_core::project::build_index;
use gihmd_core::tex::CompileOutcome;
use gihmd_core::tex::CompilePolicy;
use gihmd_core::tex::CompileRequest;
use gihmd_core::tex::ForwardHit;
use gihmd_core::tex::InverseHit;
use gihmd_core::tex::TexError;
use gihmd_core::tex::build_dir_for;
use gihmd_core::tex::compile_dir;
use tauri::AppHandle;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn tex_compile(
    app: AppHandle,
    state: State<'_, AppState>,
    policy: CompilePolicy,
) -> Result<u64, AppError> {
    let record = state.current_record()?;
    let main_file = record.main_file.ok_or(TexError::MainFileMissing)?;
    state.ensure_compiler(&app)?;
    let (engine, shell_escape) = {
        let settings = state.settings()?;

        (settings.engine, settings.shell_escape)
    };
    let request = CompileRequest {
        build_dir: build_dir_for(&state.context.paths().cache_dir, &record.root),
        project_root: record.root,
        main_file,
        engine,
        shell_escape,
        policy,
    };
    *AppState::lock(&state.last_request)? = Some(request.clone());
    preinstall_packages(&app, &state, &request).await;
    let compiler = AppState::lock(&state.compiler)?;
    let job_id = compiler
        .as_ref()
        .map_or(0, |compiler| compiler.request(request));

    Ok(job_id)
}

#[tauri::command]
pub async fn tex_cancel(state: State<'_, AppState>) -> Result<(), AppError> {
    if let Some(compiler) = AppState::lock(&state.compiler)?.as_ref() {
        compiler.cancel();
    }

    Ok(())
}

fn latest_pdf(state: &AppState) -> Result<std::path::PathBuf, AppError> {
    let latest = AppState::lock(&state.latest_outcome)?;

    latest
        .as_ref()
        .and_then(|outcome| outcome.pdf_path.clone())
        .ok_or_else(|| TexError::NoPdf.into())
}

#[tauri::command]
pub async fn tex_synctex_forward(
    state: State<'_, AppState>,
    file: RelPath,
    line: u32,
    column: u32,
) -> Result<Vec<ForwardHit>, AppError> {
    let record = state.current_record()?;
    let toolchain = state.resolve_toolchain()?;
    let pdf = latest_pdf(&state)?;
    let main_file = record.main_file.ok_or(TexError::MainFileMissing)?;
    let compile_dir = compile_dir(&record.root, &main_file);
    let absolute = file.join(&record.root);
    let hits = gihmd_core::tex::forward(
        state.context.processes(),
        &toolchain,
        &pdf,
        &compile_dir,
        &absolute,
        line,
        column,
    )
    .await?;

    Ok(hits)
}

#[tauri::command]
pub async fn tex_synctex_inverse(
    state: State<'_, AppState>,
    page: u32,
    x: f64,
    y: f64,
) -> Result<Vec<InverseHit>, AppError> {
    let record = state.current_record()?;
    let toolchain = state.resolve_toolchain()?;
    let pdf = latest_pdf(&state)?;
    let main_file = record.main_file.ok_or(TexError::MainFileMissing)?;
    let compile_dir = compile_dir(&record.root, &main_file);
    let hits = gihmd_core::tex::inverse(
        state.context.processes(),
        &toolchain,
        &pdf,
        &compile_dir,
        page,
        x,
        y,
    )
    .await?;

    Ok(hits)
}

#[tauri::command]
pub async fn tex_read_log(state: State<'_, AppState>) -> Result<String, AppError> {
    let log_path = {
        let latest = AppState::lock(&state.latest_outcome)?;

        latest
            .as_ref()
            .map(|outcome| outcome.log_path.clone())
            .ok_or(TexError::NoPdf)?
    };
    let text = tokio::fs::read_to_string(&log_path)
        .await
        .map_err(|source| TexError::Io {
            path: log_path,
            source,
        })?;

    Ok(text)
}

#[tauri::command]
pub async fn tex_latest_outcome(
    state: State<'_, AppState>,
) -> Result<Option<CompileOutcome>, AppError> {
    let latest = AppState::lock(&state.latest_outcome)?.clone();

    Ok(latest)
}

/// With the managed TeX Live, install every package the project names before the first compile
/// so a fresh template needs one round instead of one per missing file. Failures only log: the
/// compile itself reports what is still missing.
async fn preinstall_packages(app: &AppHandle, state: &AppState, request: &CompileRequest) {
    let managed = state
        .settings()
        .is_ok_and(|settings| settings.tex_bin_dir.is_none())
        && state.managed_tex.bin_dir().is_some();

    if !managed {
        return;
    }

    let Ok(index) = build_index(&request.project_root).await else {
        return;
    };
    let files: Vec<String> = index
        .packages
        .iter()
        .map(|name| format!("{name}.sty"))
        .chain(index.classes.iter().map(|name| format!("{name}.cls")))
        .filter(|file| !index.local_styles.contains(file))
        .collect();
    let events = AppState::tex_install_sender(app);

    match state
        .managed_tex
        .ensure_files(state.context.as_ref(), &files, &events)
        .await
    {
        Ok(unresolved) => {
            if let Ok(mut attempted) = state.attempted_files.lock() {
                attempted.extend(unresolved);
            }
        }
        Err(error) => tracing::warn!(error = %error, "Pre-installing packages failed."),
    }
}
