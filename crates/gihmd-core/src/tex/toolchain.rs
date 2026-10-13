use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;

use crate::tex::TexError;

const REQUIRED: &[&str] = &["latexmk", "synctex", "pdflatex"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TexToolchain {
    pub bin_dir: PathBuf,
    pub latexmk: PathBuf,
    pub synctex: PathBuf,
}

fn executable_name(program: &str) -> String {
    if cfg!(windows) {
        format!("{program}.exe")
    } else {
        program.to_owned()
    }
}

fn validate_dir(dir: &Path) -> Result<TexToolchain, Vec<String>> {
    let missing: Vec<String> = REQUIRED
        .iter()
        .filter(|program| !dir.join(executable_name(program)).is_file())
        .map(|program| (*program).to_owned())
        .collect();

    if !missing.is_empty() {
        return Err(missing);
    }

    Ok(TexToolchain {
        bin_dir: dir.to_path_buf(),
        latexmk: dir.join(executable_name("latexmk")),
        synctex: dir.join(executable_name("synctex")),
    })
}

fn path_candidates() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default()
}

fn platform_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("/Library/TeX/texbin"),
        PathBuf::from("/usr/local/texlive/2026/bin/universal-darwin"),
        PathBuf::from("/usr/local/texlive/2025/bin/universal-darwin"),
        PathBuf::from("/usr/local/texlive/2026/bin/x86_64-linux"),
        PathBuf::from("/usr/local/texlive/2025/bin/x86_64-linux"),
        PathBuf::from("/usr/local/texlive/2026/bin/aarch64-linux"),
        PathBuf::from("/usr/local/texlive/2025/bin/aarch64-linux"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("C:\\texlive\\2026\\bin\\windows"),
        PathBuf::from("C:\\texlive\\2025\\bin\\windows"),
    ];

    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(local)
                .join("Programs")
                .join("MiKTeX")
                .join("miktex")
                .join("bin")
                .join("x64"),
        );
    }

    candidates
}

/// An explicit directory must be valid or the error says what is missing; without one, `PATH` and
/// the well-known install locations are probed and `ToolchainNotFound` lists everything searched.
pub fn resolve_toolchain(explicit: Option<&Path>) -> Result<TexToolchain, TexError> {
    if let Some(dir) = explicit {
        return validate_dir(dir).map_err(|missing| TexError::ToolchainInvalid {
            dir: dir.to_path_buf(),
            missing,
        });
    }

    let mut searched = Vec::new();

    for dir in path_candidates().into_iter().chain(platform_candidates()) {
        if let Ok(toolchain) = validate_dir(&dir) {
            return Ok(toolchain);
        }

        searched.push(dir);
    }

    Err(TexError::ToolchainNotFound { searched })
}
