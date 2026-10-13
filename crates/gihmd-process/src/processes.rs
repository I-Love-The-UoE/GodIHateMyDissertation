use std::future::Future;
use std::time::Instant;

use gihmd_core::Captured;
use gihmd_core::CommandSpec;
use gihmd_core::ExitOutcome;
use gihmd_core::ProcessError;
use gihmd_core::Processes;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncRead;
use tokio::io::BufReader;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::child::TokioChild;
use crate::platform;

#[derive(Debug, Default)]
pub struct TokioProcesses;

fn configure(spec: &CommandSpec) -> tokio::process::Command {
    let mut command = platform::command(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .kill_on_drop(true);

    for (key, value) in &spec.env {
        command.env(key, value);
    }

    if !spec.path_prepend.is_empty() {
        let existing = std::env::var_os("PATH").unwrap_or_default();
        let mut paths = spec.path_prepend.clone();
        paths.extend(std::env::split_paths(&existing));

        if let Ok(joined) = std::env::join_paths(paths) {
            command.env("PATH", joined);
        }
    }

    command
}

async fn pump<R: AsyncRead + Unpin>(reader: R, lines: mpsc::Sender<String>) {
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::new();

    loop {
        buffer.clear();

        match reader.read_until(b'\n', &mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&buffer)
                    .trim_end_matches(['\n', '\r'])
                    .to_owned();

                if lines.send(text).await.is_err() {
                    break;
                }
            }
        }
    }
}

impl Processes for TokioProcesses {
    type Child = TokioChild;

    async fn run(
        &self,
        spec: CommandSpec,
        lines: mpsc::Sender<String>,
        cancel: CancellationToken,
    ) -> Result<ExitOutcome, ProcessError> {
        let started = Instant::now();
        let mut child = platform::piped(configure(&spec))
            .spawn()
            .map_err(|source| ProcessError::Spawn {
                program: spec.program.clone(),
                source,
            })?;
        drop(child.stdin.take());
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let mut pumps = tokio::task::JoinSet::new();

        if let Some(stdout) = stdout {
            pumps.spawn(pump(stdout, lines.clone()));
        }

        if let Some(stderr) = stderr {
            pumps.spawn(pump(stderr, lines));
        }

        let cancelled = tokio::select! {
            () = cancel.cancelled() => {
                platform::kill_tree(&mut child).await;

                true
            }
            _ = child.wait() => false,
        };
        let status = child
            .wait()
            .await
            .map_err(|source| ProcessError::Io { source })?;

        while pumps.join_next().await.is_some() {}

        Ok(ExitOutcome::new(
            status.code(),
            cancelled,
            started.elapsed(),
        ))
    }

    async fn capture(&self, spec: CommandSpec) -> Result<Captured, ProcessError> {
        let started = Instant::now();
        let output = platform::piped(configure(&spec))
            .output()
            .await
            .map_err(|source| ProcessError::Spawn {
                program: spec.program.clone(),
                source,
            })?;

        Ok(Captured {
            outcome: ExitOutcome::new(output.status.code(), false, started.elapsed()),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    fn spawn_interactive(
        &self,
        spec: CommandSpec,
    ) -> impl Future<Output = Result<Self::Child, ProcessError>> + Send {
        std::future::ready(spawn_now(&spec))
    }
}

fn spawn_now(spec: &CommandSpec) -> Result<TokioChild, ProcessError> {
    {
        let mut child =
            platform::piped(configure(spec))
                .spawn()
                .map_err(|source| ProcessError::Spawn {
                    program: spec.program.clone(),
                    source,
                })?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(ProcessError::Exited);
        };

        if let Some(stderr) = child.stderr.take() {
            let (lines, mut receiver) = mpsc::channel::<String>(64);
            drop(tokio::spawn(pump(stderr, lines)));
            drop(tokio::spawn(async move {
                while let Some(line) = receiver.recv().await {
                    tracing::debug!(line, "Interactive child wrote to stderr.");
                }
            }));
        }

        Ok(TokioChild::new(child, stdin, stdout))
    }
}

pub fn default() -> TokioProcesses {
    TokioProcesses
}
