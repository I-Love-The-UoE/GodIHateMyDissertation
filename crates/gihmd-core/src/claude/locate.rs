use std::path::Path;
use std::path::PathBuf;

use crate::claude::ClaudeError;

fn executable_names() -> Vec<&'static str> {
    if cfg!(windows) {
        vec!["claude.exe", "claude.cmd", "claude"]
    } else {
        vec!["claude"]
    }
}

fn candidates() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();

    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let home = PathBuf::from(home);
        dirs.push(home.join(".local").join("bin"));
        dirs.push(home.join(".claude").join("local"));
        dirs.push(
            home.join(".claude")
                .join("local")
                .join("node_modules")
                .join(".bin"),
        );
    }

    if let Some(appdata) = std::env::var_os("APPDATA") {
        dirs.push(PathBuf::from(appdata).join("npm"));
    }

    dirs.push(PathBuf::from("/opt/homebrew/bin"));
    dirs.push(PathBuf::from("/usr/local/bin"));

    dirs
}

/// An explicit path must exist; otherwise `PATH` and the usual install locations are searched.
pub fn locate_binary(explicit: Option<&Path>) -> Result<PathBuf, ClaudeError> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Ok(path.to_path_buf());
        }

        return Err(ClaudeError::BinaryInvalid {
            path: path.to_path_buf(),
        });
    }

    let mut searched = Vec::new();

    for dir in candidates() {
        for name in executable_names() {
            let candidate = dir.join(name);

            if candidate.is_file() {
                return Ok(candidate);
            }
        }

        searched.push(dir);
    }

    Err(ClaudeError::BinaryNotFound { searched })
}
