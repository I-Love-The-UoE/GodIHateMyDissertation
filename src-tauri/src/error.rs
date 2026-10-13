use gihmd_core::config::ConfigError;
use gihmd_core::tex::TexError;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("The application window could not be created.")]
    Tauri(#[from] tauri::Error),
    #[error("The configuration could not be loaded.")]
    Config(#[from] ConfigError),
    #[error("The log subscriber could not be installed.")]
    Logging { detail: String },
}

/// The wire error for every command: the domain tag plus the domain error's own `kind` tag.
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "domain", rename_all = "snake_case")]
pub enum AppError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Project(#[from] gihmd_core::project::ProjectError),
    #[error(transparent)]
    Tex(#[from] TexError),
    #[error(transparent)]
    Git(#[from] gihmd_core::git::GitError),
    #[error(transparent)]
    Claude(#[from] gihmd_core::claude::ClaudeError),
    #[error(transparent)]
    Watch(#[from] gihmd_core::WatchError),
    #[error(transparent)]
    Path(#[from] gihmd_core::PathError),
    #[error(transparent)]
    Secrets(#[from] gihmd_core::SecretsError),
    #[error(transparent)]
    Process(#[from] gihmd_core::ProcessError),
    #[error(transparent)]
    Terminal(#[from] gihmd_core::TerminalError),
    #[error("No project is open.")]
    NoProject { kind: NoProjectKind },
    #[error("The application state is poisoned.")]
    Poisoned { kind: PoisonedKind },
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NoProjectKind {
    NoProject,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PoisonedKind {
    Poisoned,
}

impl AppError {
    pub fn no_project() -> Self {
        Self::NoProject {
            kind: NoProjectKind::NoProject,
        }
    }

    pub fn poisoned() -> Self {
        Self::Poisoned {
            kind: PoisonedKind::Poisoned,
        }
    }
}
