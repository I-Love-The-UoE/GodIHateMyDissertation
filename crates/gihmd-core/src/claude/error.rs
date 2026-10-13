use std::path::PathBuf;

use serde::Serialize;

use crate::process::ProcessError;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClaudeError {
    #[error("The Claude Code CLI was not found; set its path in settings.")]
    BinaryNotFound { searched: Vec<PathBuf> },
    #[error("The configured Claude Code path is not an executable file.")]
    BinaryInvalid { path: PathBuf },
    #[error("The Claude Code process could not be used.")]
    Process(#[from] ProcessError),
    #[error("A line from Claude Code could not be decoded.")]
    ProtocolDecode {
        line: String,
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: serde_json::Error,
    },
    #[error("A message for Claude Code could not be encoded.")]
    ProtocolEncode {
        #[source]
        #[serde(serialize_with = "crate::display")]
        source: serde_json::Error,
    },
    #[error("No session with this id is running.")]
    SessionMissing { session_id: String },
    #[error("The session has ended.")]
    SessionEnded,
}
