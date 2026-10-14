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

pub(crate) fn validate_dir(dir: &Path) -> Result<TexToolchain, Vec<String>> {
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

/// A custom directory from settings must be valid or the error says what is missing; otherwise
/// the managed TeX Live is used, and if that is absent the caller shows the setup flow.
pub fn resolve_toolchain(
    explicit: Option<&Path>,
    managed_bin_dir: Option<PathBuf>,
    managed_root: &Path,
) -> Result<TexToolchain, TexError> {
    if let Some(dir) = explicit {
        return validate_dir(dir).map_err(|missing| TexError::ToolchainInvalid {
            dir: dir.to_path_buf(),
            missing,
        });
    }

    let Some(dir) = managed_bin_dir else {
        return Err(TexError::ToolchainNotInstalled {
            root: managed_root.to_path_buf(),
        });
    };

    validate_dir(&dir).map_err(|missing| TexError::ToolchainInvalid { dir, missing })
}
