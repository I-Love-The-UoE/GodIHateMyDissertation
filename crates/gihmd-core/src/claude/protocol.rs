use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::claude::ClaudeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    Plan,
    DontAsk,
    BypassPermissions,
}

impl PermissionMode {
    pub fn cli_value(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::AcceptEdits => "acceptEdits",
            Self::Plan => "plan",
            Self::DontAsk => "dontAsk",
            Self::BypassPermissions => "bypassPermissions",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    pub role: String,
    pub content: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "behavior", rename_all = "snake_case")]
pub enum PermissionDecision {
    Allow {
        #[serde(rename = "updatedInput", skip_serializing_if = "Option::is_none")]
        updated_input: Option<Value>,
    },
    Deny {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Outbound {
    User {
        message: UserMessage,
        #[serde(rename = "parent_tool_use_id")]
        parent_tool_use_id: Option<String>,
        #[serde(rename = "session_id", skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
    },
    ControlResponse {
        response: ControlResponse,
    },
    ControlRequest {
        request_id: String,
        request: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlResponse {
    Success { request_id: String, response: Value },
    Error { request_id: String, error: String },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SystemMessage {
    pub subtype: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub tools: Vec<Value>,
    #[serde(default)]
    pub mcp_servers: Vec<Value>,
    #[serde(default, rename = "permissionMode")]
    pub permission_mode: Option<String>,
    #[serde(flatten)]
    pub rest: Value,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        #[serde(default)]
        thinking: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        content: Value,
        #[serde(default)]
        is_error: bool,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AssistantMessage {
    pub message: AssistantBody,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub parent_tool_use_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AssistantBody {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub content: Vec<ContentBlock>,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct UserEcho {
    pub message: UserBody,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub parent_tool_use_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct UserBody {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Value,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ResultMessage {
    pub subtype: String,
    #[serde(default)]
    pub is_error: bool,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub num_turns: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub duration_api_ms: Option<u64>,
    #[serde(default)]
    pub total_cost_usd: Option<f64>,
    #[serde(default)]
    pub usage: Option<Value>,
    #[serde(default, rename = "modelUsage")]
    pub model_usage: Option<Value>,
    #[serde(default)]
    pub permission_denials: Vec<Value>,
    #[serde(default)]
    pub errors: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlRequest {
    CanUseTool {
        tool_name: String,
        input: Value,
        #[serde(default)]
        tool_use_id: Option<String>,
        #[serde(default)]
        permission_suggestions: Option<Value>,
        #[serde(default)]
        blocked_path: Option<String>,
        #[serde(default)]
        decision_reason: Option<Value>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Inbound {
    System(SystemMessage),
    Assistant(AssistantMessage),
    User(UserEcho),
    Result(ResultMessage),
    StreamEvent {
        event: Value,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        parent_tool_use_id: Option<String>,
    },
    ControlRequest {
        request_id: String,
        request: ControlRequest,
    },
    ControlResponse {
        response: Value,
    },
    ControlCancelRequest {
        request_id: String,
    },
    RateLimitEvent {
        #[serde(flatten)]
        rest: Value,
    },
    #[serde(other)]
    Unknown,
}

pub fn encode(message: &Outbound) -> Result<String, ClaudeError> {
    serde_json::to_string(message).map_err(|source| ClaudeError::ProtocolEncode { source })
}

pub fn decode(line: &str) -> Result<Inbound, ClaudeError> {
    serde_json::from_str(line).map_err(|source| ClaudeError::ProtocolDecode {
        line: line.to_owned(),
        source,
    })
}
