//! The one place that knows how process groups and signals differ between operating systems.

use std::path::Path;
use std::process::Stdio;

use tokio::process::Child;
use tokio::process::Command;

#[cfg(unix)]
pub(crate) fn command(program: &Path) -> Command {
    let mut command = Command::new(program);
    command.process_group(0);

    command
}

#[cfg(windows)]
pub(crate) fn command(program: &Path) -> Command {
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = Command::new(program);
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);

    command
}

pub(crate) fn piped(mut command: Command) -> Command {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command
}

#[cfg(unix)]
pub(crate) async fn kill_tree(child: &mut Child) {
    use std::time::Duration;

    if let Some(pid) = child.id()
        && let Ok(pid) = i32::try_from(pid)
    {
        // SAFETY-FREE: libc::killpg is a plain FFI call that the crate marks safe to wrap here
        // because the pid comes from a child this process spawned.
        unsafe_free_killpg(pid, libc::SIGTERM);

        if tokio::time::timeout(Duration::from_secs(2), child.wait())
            .await
            .is_ok()
        {
            return;
        }

        unsafe_free_killpg(pid, libc::SIGKILL);
    }

    let _ = child.kill().await;
}

#[cfg(unix)]
fn unsafe_free_killpg(pid: i32, signal: i32) {
    // NOTE: `kill` on a negative pid targets the whole process group; the tokio Command set the
    // child as its own group leader, so this reaches latexmk's pdflatex/biber children too.
    let _ = std::process::Command::new("kill")
        .arg(format!("-{signal}"))
        .arg("--")
        .arg(format!("-{pid}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(windows)]
pub(crate) async fn kill_tree(child: &mut Child) {
    if let Some(pid) = child.id() {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
    }

    let _ = child.kill().await;
}

#[cfg(unix)]
pub(crate) fn interrupt(child: &Child) {
    if let Some(pid) = child.id() {
        let _ = std::process::Command::new("kill")
            .arg("-INT")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

#[cfg(windows)]
pub(crate) fn interrupt(_child: &Child) {
    // NOTE: Windows has no SIGINT equivalent for a piped child; Claude Code is interrupted through
    // its stdin control protocol instead, which the session layer always sends first.
}
