use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;

use gihmd_core::Context;
use gihmd_core::FsEvent;
use gihmd_core::Watcher;
use gihmd_core::claude::ClaudeSession;
use gihmd_core::claude::SessionEvent;
use gihmd_core::config::AppSettings;
use gihmd_core::config::ProjectRecord;
use gihmd_core::config::ProjectRegistry;
use gihmd_core::tex::CompileEvent;
use gihmd_core::tex::CompileOutcome;
use gihmd_core::tex::CompileRequest;
use gihmd_core::tex::Compiler;
use gihmd_core::tex::ManagedTexLive;
use gihmd_core::tex::TexInstallEvent;
use gihmd_core::tex::missing_files;
use gihmd_process::TokioProcesses;
use gihmd_pty::PtySession;
use gihmd_watch::WatchHandle;
use tauri::AppHandle;
use tauri::Manager;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::config;
use crate::context::AppContext;
use crate::error::AppError;
use crate::error::StartupError;
use crate::events;

const EVENT_QUEUE: usize = 256;

#[derive(Debug)]
pub struct OpenProject {
    pub record: ProjectRecord,
    _watch: WatchHandle,
    _forwarder: JoinHandle<()>,
}

#[derive(Debug)]
pub struct AppState {
    pub context: Arc<AppContext>,
    pub projects: Mutex<ProjectRegistry>,
    pub open: Mutex<Option<OpenProject>>,
    pub compiler: Mutex<Option<Compiler<TokioProcesses>>>,
    pub latest_outcome: Mutex<Option<CompileOutcome>>,
    pub git_cancel: Mutex<Option<CancellationToken>>,
    pub claude: Mutex<HashMap<Uuid, Arc<ClaudeSession<TokioProcesses>>>>,
    pub claude_forwarders: Mutex<Vec<JoinHandle<()>>>,
    pub compile_forwarder: Mutex<Option<JoinHandle<()>>>,
    pub terminals: Mutex<HashMap<u64, Arc<PtySession>>>,
    pub next_terminal_id: Mutex<u64>,
    pub managed_tex: ManagedTexLive,
    pub tex_install: Mutex<Option<JoinHandle<()>>>,
    pub last_request: Mutex<Option<CompileRequest>>,
    pub attempted_files: Mutex<HashSet<String>>,
}

impl AppState {
    pub fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, AppError> {
        mutex.lock().ok().ok_or_else(AppError::poisoned)
    }

    pub fn settings(&self) -> Result<RwLockReadGuard<'_, AppSettings>, AppError> {
        self.context
            .settings()
            .read()
            .ok()
            .ok_or_else(AppError::poisoned)
    }

    pub fn settings_mut(&self) -> Result<RwLockWriteGuard<'_, AppSettings>, AppError> {
        self.context
            .settings()
            .write()
            .ok()
            .ok_or_else(AppError::poisoned)
    }

    pub fn claude_session(
        &self,
        session_id: Uuid,
    ) -> Result<Arc<ClaudeSession<TokioProcesses>>, AppError> {
        let sessions = Self::lock(&self.claude)?;

        sessions.get(&session_id).map(Arc::clone).ok_or_else(|| {
            gihmd_core::claude::ClaudeError::SessionMissing {
                session_id: session_id.to_string(),
            }
            .into()
        })
    }

    pub fn current_record(&self) -> Result<ProjectRecord, AppError> {
        let open = Self::lock(&self.open)?;

        open.as_ref()
            .map(|project| project.record.clone())
            .ok_or_else(AppError::no_project)
    }

    pub fn update_record<F: FnOnce(&mut ProjectRecord)>(
        &self,
        update: F,
    ) -> Result<ProjectRecord, AppError> {
        let mut open = Self::lock(&self.open)?;
        let project = open.as_mut().ok_or_else(AppError::no_project)?;
        update(&mut project.record);
        let record = project.record.clone();
        drop(open);
        let mut projects = Self::lock(&self.projects)?;
        projects.update(record.clone());

        Ok(record)
    }

    pub async fn persist_projects(&self) -> Result<(), AppError> {
        let snapshot = Self::lock(&self.projects)?.clone();
        snapshot.save(&self.context.paths().config_dir).await?;

        Ok(())
    }

    pub fn start_watch(&self, app: &AppHandle, record: ProjectRecord) -> Result<(), AppError> {
        let (sender, mut receiver) = mpsc::channel::<Vec<FsEvent>>(EVENT_QUEUE);
        let watch = self.context.watcher().start(&record.root, sender)?;
        let handle = app.clone();
        let forwarder = tokio::spawn(async move {
            while let Some(batch) = receiver.recv().await {
                events::emit(&handle, events::FS_CHANGED, batch);
            }
        });
        let mut open = Self::lock(&self.open)?;
        *open = Some(OpenProject {
            record,
            _watch: watch,
            _forwarder: forwarder,
        });

        Ok(())
    }

    pub fn ensure_compiler(&self, app: &AppHandle) -> Result<(), AppError> {
        let mut compiler = Self::lock(&self.compiler)?;

        if compiler.is_some() {
            return Ok(());
        }

        let toolchain = self.resolve_toolchain()?;
        let (sender, mut receiver) = mpsc::channel::<CompileEvent>(EVENT_QUEUE);
        let handle = app.clone();
        let forwarder = tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                if let CompileEvent::Finished { outcome } = &event {
                    let state = handle.state::<AppState>();

                    if let Err(error) = state.auto_install_packages(&handle, outcome) {
                        tracing::warn!(error = %error, "Automatic package installation could not start.");
                    }

                    tracing::info!(
                        job_id = outcome.job_id,
                        status = ?outcome.status,
                        pdf = outcome.pdf_path.is_some(),
                        duration_ms = outcome.duration_ms,
                        "Compile finished."
                    );
                    let state = handle.state::<AppState>();

                    if let Ok(mut latest) = state.latest_outcome.lock() {
                        *latest = Some(outcome.clone());
                    }
                }

                events::emit(&handle, events::COMPILE_EVENT, event);
            }
        });
        *compiler = Some(Compiler::new(
            self.context.shared_processes(),
            toolchain,
            sender,
        ));
        *Self::lock(&self.compile_forwarder)? = Some(forwarder);

        Ok(())
    }

    pub fn resolve_toolchain(&self) -> Result<gihmd_core::tex::TexToolchain, AppError> {
        let explicit = self.settings()?.tex_bin_dir.clone();

        Ok(gihmd_core::tex::resolve_toolchain(
            explicit.as_deref(),
            self.managed_tex.bin_dir(),
            self.managed_tex.root(),
        )?)
    }

    pub fn tex_install_sender(app: &AppHandle) -> mpsc::Sender<TexInstallEvent> {
        let (sender, mut receiver) = mpsc::channel::<TexInstallEvent>(EVENT_QUEUE);
        let handle = app.clone();
        drop(tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                events::emit(&handle, events::TEX_INSTALL, event);
            }
        }));

        sender
    }

    /// With the managed TeX Live active, a compile that failed on files no installed package
    /// provides triggers a `tlmgr` lookup and install, then a recompile. Each file is tried once
    /// per project session so an unknown file cannot loop.
    pub fn auto_install_packages(
        &self,
        app: &AppHandle,
        outcome: &CompileOutcome,
    ) -> Result<(), AppError> {
        if self.settings()?.tex_bin_dir.is_some() || self.managed_tex.bin_dir().is_none() {
            return Ok(());
        }

        let files: Vec<String> = {
            let mut attempted = Self::lock(&self.attempted_files)?;

            missing_files(&outcome.diagnostics)
                .into_iter()
                .filter(|file| attempted.insert(file.clone()))
                .collect()
        };

        if files.is_empty() {
            return Ok(());
        }

        let Some(request) = Self::lock(&self.last_request)?.clone() else {
            return Ok(());
        };
        let handle = app.clone();
        drop(tokio::spawn(async move {
            let state = handle.state::<AppState>();
            let events = Self::tex_install_sender(&handle);
            let context = Arc::clone(&state.context);

            match state
                .managed_tex
                .ensure_files(context.as_ref(), &files, &events)
                .await
            {
                Ok(unresolved) if unresolved.is_empty() => {
                    let compiler = state.compiler.lock().ok();

                    if let Some(compiler) = compiler.as_ref().and_then(|guard| guard.as_ref()) {
                        compiler.request(CompileRequest {
                            policy: gihmd_core::tex::CompilePolicy::Queue,
                            ..request
                        });
                    }
                }
                Ok(unresolved) => {
                    let error =
                        gihmd_core::tex::TexError::PackagesUnavailable { files: unresolved };
                    let _ = events
                        .send(TexInstallEvent::Failed {
                            error: error.to_string(),
                        })
                        .await;
                }
                Err(error) => {
                    let _ = events
                        .send(TexInstallEvent::Failed {
                            error: error.to_string(),
                        })
                        .await;
                }
            }
        }));

        Ok(())
    }

    pub fn reset_compiler(&self) -> Result<(), AppError> {
        *Self::lock(&self.compiler)? = None;

        if let Some(forwarder) = Self::lock(&self.compile_forwarder)?.take() {
            forwarder.abort();
        }

        Ok(())
    }

    pub fn claude_event_sender(
        &self,
        app: &AppHandle,
    ) -> Result<mpsc::Sender<SessionEvent>, AppError> {
        let (sender, mut receiver) = mpsc::channel::<SessionEvent>(EVENT_QUEUE);
        let handle = app.clone();
        let forwarder = tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                if let SessionEvent::Exited { session_id, .. } = &event {
                    let state = handle.state::<AppState>();

                    if let Ok(mut sessions) = state.claude.lock() {
                        sessions.remove(session_id);
                    }
                }

                events::emit(&handle, events::CLAUDE_EVENT, event);
            }
        });
        Self::lock(&self.claude_forwarders)?.push(forwarder);

        Ok(sender)
    }
}

fn install_tracing(log_dir: &std::path::Path) -> Result<(), StartupError> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    std::fs::create_dir_all(log_dir).map_err(|error| StartupError::Logging {
        detail: error.to_string(),
    })?;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("gihmd.log"))
        .map_err(|error| StartupError::Logging {
            detail: error.to_string(),
        })?;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new("info,gihmd=debug,gihmd_core=debug")
    });
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(file),
        )
        .try_init()
        .map_err(|error| StartupError::Logging {
            detail: error.to_string(),
        })?;

    Ok(())
}

pub fn initialise(app: &tauri::App) -> Result<(), StartupError> {
    let config = config::load(app)?;
    install_tracing(&config.log_dir)?;
    tracing::info!("Configuration loaded.");
    let data_dir = config.paths.data_dir.clone();
    let context = Arc::new(AppContext::new(config.paths, config.settings));
    app.manage(AppState {
        context,
        projects: Mutex::new(config.projects),
        open: Mutex::new(None),
        compiler: Mutex::new(None),
        latest_outcome: Mutex::new(None),
        git_cancel: Mutex::new(None),
        claude: Mutex::new(HashMap::new()),
        claude_forwarders: Mutex::new(Vec::new()),
        compile_forwarder: Mutex::new(None),
        terminals: Mutex::new(HashMap::new()),
        next_terminal_id: Mutex::new(0),
        managed_tex: ManagedTexLive::new(&data_dir),
        tex_install: Mutex::new(None),
        last_request: Mutex::new(None),
        attempted_files: Mutex::new(HashSet::new()),
    });

    Ok(())
}
