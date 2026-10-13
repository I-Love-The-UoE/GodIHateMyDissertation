use std::io::Read;
use std::io::Write;
use std::sync::Mutex;

use gihmd_core::TerminalError;
use gihmd_core::TerminalEvent;
use gihmd_core::TerminalSession;
use gihmd_core::TerminalSpec;
use gihmd_core::Terminals;
use portable_pty::Child;
use portable_pty::CommandBuilder;
use portable_pty::MasterPty;
use portable_pty::PtySize;
use portable_pty::native_pty_system;
use tokio::sync::mpsc;

const READ_CHUNK: usize = 16 * 1024;

#[derive(Debug, Default)]
pub struct PortableTerminals;

pub struct PtySession {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    child: Mutex<Box<dyn Child + Send + Sync>>,
}

impl std::fmt::Debug for PtySession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("PtySession").finish_non_exhaustive()
    }
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn pump(mut reader: Box<dyn Read + Send>, events: &mpsc::Sender<TerminalEvent>) {
    let mut buffer = vec![0_u8; READ_CHUNK];

    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                if events
                    .blocking_send(TerminalEvent::Output(buffer[..read].to_vec()))
                    .is_err()
                {
                    break;
                }
            }
        }
    }
}

impl Terminals for PortableTerminals {
    type Session = PtySession;

    fn spawn(
        &self,
        spec: TerminalSpec,
        events: mpsc::Sender<TerminalEvent>,
    ) -> Result<Self::Session, TerminalError> {
        let pair = native_pty_system()
            .openpty(size(spec.cols, spec.rows))
            .map_err(|error| TerminalError::Pty {
                detail: error.to_string(),
            })?;
        let mut command = CommandBuilder::new(&spec.program);
        command.args(&spec.args);
        command.cwd(&spec.cwd);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");

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

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| TerminalError::Spawn {
                program: spec.program.clone(),
                detail: error.to_string(),
            })?;
        drop(pair.slave);
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| TerminalError::Pty {
                detail: error.to_string(),
            })?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| TerminalError::Pty {
                detail: error.to_string(),
            })?;
        let reader_events = events.clone();
        drop(tokio::task::spawn_blocking(move || {
            pump(reader, &reader_events);
        }));

        Ok(PtySession {
            master: Mutex::new(pair.master),
            writer: Mutex::new(Some(writer)),
            child: Mutex::new(child),
        })
    }
}

impl PtySession {
    /// Blocks until the shell exits; run it on a blocking task.
    pub fn wait(&self) -> Option<u32> {
        let Ok(mut child) = self.child.lock() else {
            return None;
        };

        child.wait().ok().map(|status| status.exit_code())
    }
}

impl TerminalSession for PtySession {
    fn write(&self, bytes: &[u8]) -> Result<(), TerminalError> {
        let mut guard = self.writer.lock().ok().ok_or(TerminalError::Closed)?;
        let writer = guard.as_mut().ok_or(TerminalError::Closed)?;
        writer
            .write_all(bytes)
            .map_err(|error| TerminalError::Pty {
                detail: error.to_string(),
            })?;
        writer.flush().map_err(|error| TerminalError::Pty {
            detail: error.to_string(),
        })
    }

    fn resize(&self, cols: u16, rows: u16) -> Result<(), TerminalError> {
        let master = self.master.lock().ok().ok_or(TerminalError::Closed)?;

        master
            .resize(size(cols, rows))
            .map_err(|error| TerminalError::Pty {
                detail: error.to_string(),
            })
    }

    fn kill(&self) -> Result<(), TerminalError> {
        self.writer.lock().ok().ok_or(TerminalError::Closed)?.take();
        let mut child = self.child.lock().ok().ok_or(TerminalError::Closed)?;

        child.kill().map_err(|error| TerminalError::Pty {
            detail: error.to_string(),
        })
    }
}

pub fn default() -> PortableTerminals {
    PortableTerminals
}
