use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;
use serde_json::json;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::claude::ClaudeError;
use crate::claude::Inbound;
use crate::claude::Outbound;
use crate::claude::PermissionDecision;
use crate::claude::PermissionMode;
use crate::claude::UserMessage;
use crate::claude::decode;
use crate::claude::encode;
use crate::claude::protocol::ControlResponse;
use crate::process::CommandSpec;
use crate::process::InteractiveChild;
use crate::process::Processes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionConfig {
    pub claude_path: PathBuf,
    pub cwd: PathBuf,
    pub session_id: Uuid,
    pub resume: bool,
    pub permission_mode: PermissionMode,
    pub model: Option<String>,
    pub append_system_prompt: String,
    pub extra_path: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionEvent {
    Message {
        session_id: Uuid,
        message: Box<Inbound>,
    },
    Undecodable {
        session_id: Uuid,
        line: String,
        error: String,
    },
    Exited {
        session_id: Uuid,
        code: Option<i32>,
    },
}

#[derive(Debug)]
pub struct ClaudeSession<P: Processes> {
    session_id: Uuid,
    child: Arc<P::Child>,
    reader: JoinHandle<()>,
}

fn arguments(config: &SessionConfig) -> Vec<String> {
    let mut args = vec![
        "-p".to_owned(),
        "--input-format".to_owned(),
        "stream-json".to_owned(),
        "--output-format".to_owned(),
        "stream-json".to_owned(),
        "--verbose".to_owned(),
        "--include-partial-messages".to_owned(),
        "--permission-mode".to_owned(),
        config.permission_mode.cli_value().to_owned(),
        "--permission-prompt-tool".to_owned(),
        "stdio".to_owned(),
    ];

    if config.resume {
        args.push("--resume".to_owned());
        args.push(config.session_id.to_string());
    } else {
        args.push("--session-id".to_owned());
        args.push(config.session_id.to_string());
    }

    if let Some(model) = &config.model {
        args.push("--model".to_owned());
        args.push(model.clone());
    }

    if !config.append_system_prompt.is_empty() {
        args.push("--append-system-prompt".to_owned());
        args.push(config.append_system_prompt.clone());
    }

    args
}

impl<P: Processes> ClaudeSession<P> {
    pub async fn spawn(
        processes: &P,
        config: SessionConfig,
        events: mpsc::Sender<SessionEvent>,
    ) -> Result<Self, ClaudeError> {
        let session_id = config.session_id;
        let mut path_prepend = config.extra_path.clone();

        if let Some(dir) = config.claude_path.parent() {
            path_prepend.push(dir.to_path_buf());
        }

        let spec = CommandSpec {
            program: config.claude_path.clone(),
            args: arguments(&config),
            cwd: config.cwd.clone(),
            env: vec![("CLAUDE_CODE_ENTRYPOINT".to_owned(), "gihmd".to_owned())],
            path_prepend,
        };
        let child = Arc::new(processes.spawn_interactive(spec).await?);
        let reader_child = Arc::clone(&child);
        let reader = tokio::spawn(async move {
            loop {
                match reader_child.read_line().await {
                    Ok(Some(line)) => {
                        let trimmed = line.trim();

                        if trimmed.is_empty() {
                            continue;
                        }

                        let event = match decode(trimmed) {
                            Ok(message) => SessionEvent::Message {
                                session_id,
                                message: Box::new(message),
                            },
                            Err(error) => {
                                tracing::warn!(error = %error, "Claude Code sent an undecodable line.");

                                SessionEvent::Undecodable {
                                    session_id,
                                    line: trimmed.to_owned(),
                                    error: error.to_string(),
                                }
                            }
                        };

                        if events.send(event).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        tracing::error!(error = %error, "Reading from Claude Code failed.");

                        break;
                    }
                }
            }

            let code = reader_child
                .terminate()
                .await
                .ok()
                .and_then(|outcome| outcome.code);
            let _ = events.send(SessionEvent::Exited { session_id, code }).await;
        });

        Ok(Self {
            session_id,
            child,
            reader,
        })
    }

    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    async fn send(&self, message: &Outbound) -> Result<(), ClaudeError> {
        let line = encode(message)?;
        self.child.write_line(&line).await?;

        Ok(())
    }

    pub async fn send_user(&self, text: &str, preamble: Option<&str>) -> Result<(), ClaudeError> {
        let content = match preamble {
            Some(preamble) => json!([
                { "type": "text", "text": preamble },
                { "type": "text", "text": text },
            ]),
            None => Value::String(text.to_owned()),
        };

        self.send(&Outbound::User {
            message: UserMessage {
                role: "user".to_owned(),
                content,
            },
            parent_tool_use_id: None,
            session_id: Some(self.session_id.to_string()),
        })
        .await
    }

    pub async fn respond_permission(
        &self,
        request_id: &str,
        decision: PermissionDecision,
    ) -> Result<(), ClaudeError> {
        let response = serde_json::to_value(&decision)
            .map_err(|source| ClaudeError::ProtocolEncode { source })?;

        self.send(&Outbound::ControlResponse {
            response: ControlResponse::Success {
                request_id: request_id.to_owned(),
                response,
            },
        })
        .await
    }

    pub async fn interrupt(&self) -> Result<(), ClaudeError> {
        self.send(&Outbound::ControlRequest {
            request_id: Uuid::new_v4().to_string(),
            request: json!({ "subtype": "interrupt" }),
        })
        .await
    }

    pub async fn shutdown(&self) -> Result<Option<i32>, ClaudeError> {
        let outcome = self.child.terminate().await?;
        self.reader.abort();

        Ok(outcome.code)
    }
}
