use std::fmt::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use std::time::SystemTime;

use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::config::Engine;
use crate::paths::RelPath;
use crate::process::CommandSpec;
use crate::process::Processes;
use crate::tex::Diagnostic;
use crate::tex::TexError;
use crate::tex::TexToolchain;
use crate::tex::parse_log;

const ERRORS_PRESENT_EXIT_CODE: i32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilePolicy {
    /// Run after the current job finishes; a newer queued request replaces an older one.
    Queue,
    /// Kill the current job and start now.
    Preempt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileRequest {
    pub project_root: PathBuf,
    pub main_file: RelPath,
    pub build_dir: PathBuf,
    pub engine: Engine,
    pub shell_escape: bool,
    pub policy: CompilePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CompileStatus {
    Success,
    ErrorsPresent,
    Cancelled,
    Failed { exit_code: Option<i32> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileOutcome {
    pub job_id: u64,
    pub status: CompileStatus,
    pub pdf_path: Option<PathBuf>,
    pub synctex_path: Option<PathBuf>,
    pub log_path: PathBuf,
    pub diagnostics: Vec<Diagnostic>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CompileEvent {
    Started { job_id: u64 },
    OutputLine { job_id: u64, line: String },
    Finished { outcome: CompileOutcome },
    Failed { job_id: u64, error: String },
}

/// Builds live under the app cache so the project tree and git status stay clean.
pub fn build_dir_for(cache_dir: &Path, project_root: &Path) -> PathBuf {
    let digest = sha2::Sha256::digest(project_root.to_string_lossy().as_bytes());
    let hex = digest.iter().take(12).fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");

        hex
    });

    cache_dir.join("builds").join(hex)
}

#[derive(Debug)]
struct Running {
    job_id: u64,
    cancel: CancellationToken,
    task: JoinHandle<()>,
}

#[derive(Debug, Default)]
struct State {
    next_job_id: u64,
    running: Option<Running>,
    pending: Option<CompileRequest>,
}

/// The Drop-free half of the compiler: cloned into job tasks so that starting a queued job never
/// triggers the cancel-on-drop of the public handle.
#[derive(Debug, Clone)]
struct Launcher<P: Processes> {
    processes: Arc<P>,
    toolchain: TexToolchain,
    events: mpsc::Sender<CompileEvent>,
    state: Arc<Mutex<State>>,
}

#[derive(Debug)]
pub struct Compiler<P: Processes> {
    launcher: Launcher<P>,
}

impl<P: Processes> Compiler<P> {
    pub fn new(
        processes: Arc<P>,
        toolchain: TexToolchain,
        events: mpsc::Sender<CompileEvent>,
    ) -> Self {
        Self {
            launcher: Launcher {
                processes,
                toolchain,
                events,
                state: Arc::new(Mutex::new(State::default())),
            },
        }
    }

    pub fn toolchain(&self) -> &TexToolchain {
        &self.launcher.toolchain
    }

    /// Returns the job id that will run this request (immediately or after the current job).
    pub fn request(&self, request: CompileRequest) -> u64 {
        let Ok(mut state) = self.launcher.state.lock() else {
            return 0;
        };

        if let Some(running) = state.running.as_ref() {
            let next_job_id = running.job_id + 1;

            if request.policy == CompilePolicy::Preempt {
                running.cancel.cancel();
            }

            state.pending = Some(request);

            return next_job_id;
        }

        self.launcher.start(&mut state, request)
    }

    pub fn cancel(&self) {
        let Ok(mut state) = self.launcher.state.lock() else {
            return;
        };
        state.pending = None;

        if let Some(running) = state.running.as_ref() {
            running.cancel.cancel();
        }
    }
}

impl<P: Processes> Launcher<P> {
    fn start(&self, state: &mut State, request: CompileRequest) -> u64 {
        state.next_job_id += 1;
        let job_id = state.next_job_id;
        let cancel = CancellationToken::new();
        let processes = Arc::clone(&self.processes);
        let toolchain = self.toolchain.clone();
        let events = self.events.clone();
        let shared = Arc::clone(&self.state);
        let token = cancel.clone();
        let task = tokio::spawn(async move {
            let outcome = run_job(
                processes.as_ref(),
                &toolchain,
                job_id,
                &request,
                &events,
                token,
            )
            .await;

            match outcome {
                Ok(outcome) => {
                    let _ = events.send(CompileEvent::Finished { outcome }).await;
                }
                Err(error) => {
                    tracing::error!(error = %error, "The compile job failed.");
                    let _ = events
                        .send(CompileEvent::Failed {
                            job_id,
                            error: error.to_string(),
                        })
                        .await;
                }
            }

            let next = {
                let Ok(mut state) = shared.lock() else {
                    return;
                };
                state.running = None;

                state.pending.take()
            };

            if let Some(next) = next {
                let launcher = Launcher {
                    processes,
                    toolchain,
                    events,
                    state: shared,
                };
                let Ok(mut state) = launcher.state.lock() else {
                    return;
                };
                launcher.start(&mut state, next);
            }
        });
        state.running = Some(Running {
            job_id,
            cancel,
            task,
        });

        job_id
    }
}

impl<P: Processes> Drop for Compiler<P> {
    fn drop(&mut self) {
        if let Ok(state) = self.launcher.state.lock()
            && let Some(running) = state.running.as_ref()
        {
            running.cancel.cancel();
            running.task.abort();
        }
    }
}

/// TeX resolves classes, styles and `\input` paths against the working directory, so a project
/// is compiled from the main file's own directory, exactly as the user would at a shell.
pub fn compile_dir(project_root: &Path, main_file: &RelPath) -> PathBuf {
    main_file.parent().map_or_else(
        || project_root.to_path_buf(),
        |parent| parent.join(project_root),
    )
}

/// Puts the compile directory and the project root on TeX's search paths (the trailing empty
/// entry keeps the distribution defaults), so templates that mix root-relative `\input` paths
/// with a class file beside the main file still compile. `max_print_line` stops the log from
/// wrapping file names, which the parser relies on.
fn tex_environment(project_root: &Path, main_file: &RelPath) -> Vec<(String, String)> {
    let compile_dir = compile_dir(project_root, main_file);
    let search = std::env::join_paths([
        compile_dir.as_os_str(),
        project_root.as_os_str(),
        std::ffi::OsStr::new(""),
    ])
    .map(|joined| joined.to_string_lossy().into_owned())
    .unwrap_or_default();

    vec![
        ("TEXINPUTS".to_owned(), search.clone()),
        ("BIBINPUTS".to_owned(), search.clone()),
        ("BSTINPUTS".to_owned(), search),
        ("max_print_line".to_owned(), "10000".to_owned()),
        ("error_line".to_owned(), "254".to_owned()),
        ("half_error_line".to_owned(), "238".to_owned()),
    ]
}

fn stem(main_file: &RelPath) -> String {
    let name = main_file.file_name();

    name.rsplit_once('.')
        .map_or_else(|| name.to_owned(), |(stem, _)| stem.to_owned())
}

async fn run_job<P: Processes>(
    processes: &P,
    toolchain: &TexToolchain,
    job_id: u64,
    request: &CompileRequest,
    events: &mpsc::Sender<CompileEvent>,
    cancel: CancellationToken,
) -> Result<CompileOutcome, TexError> {
    tokio::fs::create_dir_all(&request.build_dir)
        .await
        .map_err(|source| TexError::Io {
            path: request.build_dir.clone(),
            source,
        })?;
    let _ = events.send(CompileEvent::Started { job_id }).await;
    let started = Instant::now();
    let started_at = SystemTime::now();
    let mut args = vec![
        request.engine.latexmk_flag().to_owned(),
        // NOTE: `-g` stops latexmk from replaying a previous failure without running TeX when
        // the sources look unchanged; every compile the app requests must really run.
        "-g".to_owned(),
        "-interaction=nonstopmode".to_owned(),
        "-synctex=1".to_owned(),
        "-file-line-error".to_owned(),
        format!("-outdir={}", request.build_dir.display()),
    ];

    if request.shell_escape {
        args.push("-shell-escape".to_owned());
    }

    args.push(request.main_file.file_name().to_owned());
    let spec = CommandSpec {
        program: toolchain.latexmk.clone(),
        args,
        cwd: compile_dir(&request.project_root, &request.main_file),
        env: tex_environment(&request.project_root, &request.main_file),
        path_prepend: vec![toolchain.bin_dir.clone()],
    };
    let (lines_tx, mut lines_rx) = mpsc::channel::<String>(256);
    let forward_events = events.clone();
    let forwarder = tokio::spawn(async move {
        while let Some(line) = lines_rx.recv().await {
            let _ = forward_events
                .send(CompileEvent::OutputLine { job_id, line })
                .await;
        }
    });
    let exit = processes.run(spec, lines_tx, cancel).await?;
    let _ = forwarder.await;
    let base = request.build_dir.join(stem(&request.main_file));
    let log_path = base.with_extension("log");
    let pdf_path = base.with_extension("pdf");
    let synctex_path = base.with_extension("synctex.gz");
    let log_text = tokio::fs::read_to_string(&log_path)
        .await
        .unwrap_or_default();
    let diagnostics = parse_log(
        &log_text,
        &[
            compile_dir(&request.project_root, &request.main_file),
            request.project_root.clone(),
        ],
        &request.build_dir,
    );
    let pdf_metadata = tokio::fs::metadata(&pdf_path).await.ok();
    let fresh_pdf = pdf_metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .is_some_and(|modified| modified >= started_at);
    let status = if exit.cancelled {
        CompileStatus::Cancelled
    } else {
        match exit.code {
            Some(0) => CompileStatus::Success,
            Some(ERRORS_PRESENT_EXIT_CODE) => CompileStatus::ErrorsPresent,
            code => CompileStatus::Failed { exit_code: code },
        }
    };
    // NOTE: latexmk leaves the PDF untouched when nothing changed, so a clean exit publishes any
    // existing PDF; only a cancelled or failed job requires one written during this run.
    let publish_pdf = match status {
        CompileStatus::Success | CompileStatus::ErrorsPresent => pdf_metadata.is_some(),
        CompileStatus::Cancelled | CompileStatus::Failed { .. } => fresh_pdf,
    };

    Ok(CompileOutcome {
        job_id,
        status,
        pdf_path: publish_pdf.then_some(pdf_path),
        synctex_path: publish_pdf.then_some(synctex_path),
        log_path,
        diagnostics,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}
