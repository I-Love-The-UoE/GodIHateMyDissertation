use serde::Serialize;
use tauri::AppHandle;
use tauri::Emitter;

pub const FS_CHANGED: &str = "fs://changed";
pub const COMPILE_EVENT: &str = "compile://event";
pub const GIT_PROGRESS: &str = "git://progress";
pub const GIT_CHANGED: &str = "git://changed";
pub const CLAUDE_EVENT: &str = "claude://event";
pub const TERMINAL_EVENT: &str = "terminal://event";

pub fn emit<T: Serialize + Clone>(app: &AppHandle, name: &str, payload: T) {
    if let Err(error) = app.emit(name, payload) {
        tracing::warn!(error = %error, event = name, "An event could not be delivered to the window.");
    }
}
