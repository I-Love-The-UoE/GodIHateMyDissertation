use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::config::ConfigError;

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    PdfLatex,
    XeLatex,
    LuaLatex,
}

impl Engine {
    pub fn latexmk_flag(self) -> &'static str {
        match self {
            Self::PdfLatex => "-pdf",
            Self::XeLatex => "-pdfxe",
            Self::LuaLatex => "-pdflua",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// `None` means "resolve on first use and fail loudly if nothing is found".
    pub tex_bin_dir: Option<PathBuf>,
    pub claude_path: Option<PathBuf>,
    pub engine: Engine,
    pub shell_escape: bool,
    pub autocompile: bool,
    pub autocompile_debounce_ms: u64,
    pub user_name: String,
}

impl AppSettings {
    pub fn initial() -> Self {
        Self {
            tex_bin_dir: None,
            claude_path: None,
            engine: Engine::PdfLatex,
            shell_escape: false,
            autocompile: true,
            autocompile_debounce_ms: 1200,
            user_name: "me".to_owned(),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.autocompile_debounce_ms < 100 {
            return Err(ConfigError::Invalid {
                setting: "autocompileDebounceMs".to_owned(),
                detail: "must be at least 100 milliseconds".to_owned(),
            });
        }

        if self.user_name.trim().is_empty() {
            return Err(ConfigError::Invalid {
                setting: "userName".to_owned(),
                detail: "must not be empty".to_owned(),
            });
        }

        Ok(())
    }
}

/// A missing file yields the initial settings (first launch); a malformed file is an error.
pub async fn load_settings(config_dir: &Path) -> Result<AppSettings, ConfigError> {
    let path = config_dir.join(SETTINGS_FILE);

    let text = match tokio::fs::read_to_string(&path).await {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AppSettings::initial());
        }
        Err(source) => return Err(ConfigError::Io { path, source }),
    };

    let settings: AppSettings =
        serde_json::from_str(&text).map_err(|source| ConfigError::Malformed { path, source })?;
    settings.validate()?;

    Ok(settings)
}

pub async fn save_settings(config_dir: &Path, settings: &AppSettings) -> Result<(), ConfigError> {
    settings.validate()?;
    let path = config_dir.join(SETTINGS_FILE);
    tokio::fs::create_dir_all(config_dir)
        .await
        .map_err(|source| ConfigError::Io {
            path: config_dir.to_path_buf(),
            source,
        })?;
    let text = serde_json::to_string_pretty(settings).map_err(|source| ConfigError::Malformed {
        path: path.clone(),
        source,
    })?;

    tokio::fs::write(&path, text)
        .await
        .map_err(|source| ConfigError::Io { path, source })
}
