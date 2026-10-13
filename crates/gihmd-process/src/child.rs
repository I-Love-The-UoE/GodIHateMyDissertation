use std::time::Instant;

use gihmd_core::ExitOutcome;
use gihmd_core::InteractiveChild;
use gihmd_core::ProcessError;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;
use tokio::sync::Mutex;

use crate::platform;

#[derive(Debug)]
pub struct TokioChild {
    child: Mutex<Child>,
    stdin: Mutex<Option<ChildStdin>>,
    stdout: Mutex<BufReader<ChildStdout>>,
    started: Instant,
}

impl TokioChild {
    pub(crate) fn new(child: Child, stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self {
            child: Mutex::new(child),
            stdin: Mutex::new(Some(stdin)),
            stdout: Mutex::new(BufReader::with_capacity(1 << 20, stdout)),
            started: Instant::now(),
        }
    }
}

impl InteractiveChild for TokioChild {
    async fn write_line(&self, line: &str) -> Result<(), ProcessError> {
        let mut guard = self.stdin.lock().await;
        let Some(stdin) = guard.as_mut() else {
            return Err(ProcessError::Exited);
        };
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|source| ProcessError::Io { source })?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|source| ProcessError::Io { source })?;
        stdin
            .flush()
            .await
            .map_err(|source| ProcessError::Io { source })?;

        Ok(())
    }

    async fn read_line(&self) -> Result<Option<String>, ProcessError> {
        let mut reader = self.stdout.lock().await;
        let mut buffer = Vec::new();
        let read = reader
            .read_until(b'\n', &mut buffer)
            .await
            .map_err(|source| ProcessError::Io { source })?;

        if read == 0 {
            return Ok(None);
        }

        let line = String::from_utf8(buffer).map_err(|source| ProcessError::NotUtf8 { source })?;

        Ok(Some(line))
    }

    async fn interrupt(&self) -> Result<(), ProcessError> {
        let child = self.child.lock().await;
        platform::interrupt(&child);

        Ok(())
    }

    async fn terminate(&self) -> Result<ExitOutcome, ProcessError> {
        self.stdin.lock().await.take();
        let mut child = self.child.lock().await;

        if let Ok(Some(status)) = child.try_wait() {
            return Ok(ExitOutcome::new(
                status.code(),
                false,
                self.started.elapsed(),
            ));
        }

        platform::kill_tree(&mut child).await;
        let status = child
            .wait()
            .await
            .map_err(|source| ProcessError::Io { source })?;

        Ok(ExitOutcome::new(
            status.code(),
            true,
            self.started.elapsed(),
        ))
    }
}
