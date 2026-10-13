use std::path::PathBuf;

use serde::Serialize;
use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerminalError {
    #[error("No shell is configured; set SHELL (or COMSPEC on Windows).")]
    ShellMissing,
    #[error("The pseudo-terminal could not be created.")]
    Pty { detail: String },
    #[error("The shell could not be started.")]
    Spawn { program: PathBuf, detail: String },
    #[error("The terminal has already closed.")]
    Closed,
    #[error("No terminal with this id exists.")]
    Missing { id: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    pub path_prepend: Vec<PathBuf>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    Output(Vec<u8>),
    Exited { code: Option<u32> },
}

pub trait TerminalSession: Send + Sync + 'static {
    fn write(&self, bytes: &[u8]) -> Result<(), TerminalError>;

    fn resize(&self, cols: u16, rows: u16) -> Result<(), TerminalError>;

    fn kill(&self) -> Result<(), TerminalError>;
}

pub trait Terminals: Send + Sync + 'static {
    type Session: TerminalSession;

    /// Output and exit are delivered on `events`; the session ends when the shell exits or
    /// `kill` is called.
    fn spawn(
        &self,
        spec: TerminalSpec,
        events: mpsc::Sender<TerminalEvent>,
    ) -> Result<Self::Session, TerminalError>;
}

/// The user's login shell on Unix and the command processor on Windows. POSIX guarantees
/// `/bin/sh`, so that is the only default taken when `SHELL` is unset.
pub fn default_shell() -> Result<(PathBuf, Vec<String>), TerminalError> {
    if cfg!(windows) {
        return std::env::var_os("COMSPEC")
            .map(|comspec| (PathBuf::from(comspec), Vec::new()))
            .ok_or(TerminalError::ShellMissing);
    }

    let shell = std::env::var_os("SHELL").map_or_else(|| PathBuf::from("/bin/sh"), PathBuf::from);

    Ok((shell, vec!["-l".to_owned()]))
}
