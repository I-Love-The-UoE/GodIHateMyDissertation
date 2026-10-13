use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProcessError {
    #[error("The program could not be started.")]
    Spawn {
        program: PathBuf,
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: std::io::Error,
    },
    #[error("Reading from or writing to the process failed.")]
    Io {
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: std::io::Error,
    },
    #[error("The process has already exited.")]
    Exited,
    #[error("The process produced output that is not valid UTF-8.")]
    NotUtf8 {
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: std::string::FromUtf8Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    /// Prepended to `PATH` so programs that shell out by name (latexmk, claude) find their siblings.
    pub path_prepend: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitOutcome {
    pub code: Option<i32>,
    pub cancelled: bool,
    pub duration_ms: u64,
}

impl ExitOutcome {
    pub fn new(code: Option<i32>, cancelled: bool, duration: Duration) -> Self {
        Self {
            code,
            cancelled,
            duration_ms: u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captured {
    pub outcome: ExitOutcome,
    pub stdout: String,
    pub stderr: String,
}

/// A child process with line-oriented stdin and stdout, used for the Claude session.
pub trait InteractiveChild: Send + Sync + 'static {
    fn write_line(&self, line: &str) -> impl Future<Output = Result<(), ProcessError>> + Send;

    /// Resolves to `None` once the child closes its stdout.
    fn read_line(&self) -> impl Future<Output = Result<Option<String>, ProcessError>> + Send;

    fn interrupt(&self) -> impl Future<Output = Result<(), ProcessError>> + Send;

    fn terminate(&self) -> impl Future<Output = Result<ExitOutcome, ProcessError>> + Send;
}

pub trait Processes: Send + Sync + 'static {
    type Child: InteractiveChild;

    /// Runs to completion, streaming merged stdout and stderr lines; cancelling kills the whole
    /// process tree.
    fn run(
        &self,
        spec: CommandSpec,
        lines: mpsc::Sender<String>,
        cancel: CancellationToken,
    ) -> impl Future<Output = Result<ExitOutcome, ProcessError>> + Send;

    fn capture(
        &self,
        spec: CommandSpec,
    ) -> impl Future<Output = Result<Captured, ProcessError>> + Send;

    fn spawn_interactive(
        &self,
        spec: CommandSpec,
    ) -> impl Future<Output = Result<Self::Child, ProcessError>> + Send;
}
