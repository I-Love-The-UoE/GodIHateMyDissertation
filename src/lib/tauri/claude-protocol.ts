export type ContentBlock =
  | { readonly type: "text"; readonly text: string }
  | { readonly type: "thinking"; readonly thinking: string }
  | {
      readonly type: "tool_use";
      readonly id: string;
      readonly name: string;
      readonly input: unknown;
    }
  | {
      readonly type: "tool_result";
      readonly tool_use_id: string;
      readonly content: unknown;
      readonly is_error: boolean;
    }
  | { readonly type: "other" };

export type SystemMessage = {
  readonly type: "system";
  readonly subtype: string;
  readonly session_id: string | null;
  readonly model: string | null;
  readonly cwd: string | null;
  readonly tools: readonly unknown[];
  readonly mcp_servers: readonly unknown[];
  readonly permissionMode: string | null;
} & Readonly<Record<string, unknown>>;

export type AssistantMessage = {
  readonly type: "assistant";
  readonly message: {
    readonly id: string | null;
    readonly role: string | null;
    readonly model: string | null;
    readonly content: readonly ContentBlock[];
    readonly stop_reason: string | null;
  };
  readonly session_id: string | null;
  readonly parent_tool_use_id: string | null;
};

export type UserEcho = {
  readonly type: "user";
  readonly message: { readonly role: string | null; readonly content: unknown };
  readonly session_id: string | null;
  readonly parent_tool_use_id: string | null;
};

export type ResultMessage = {
  readonly type: "result";
  readonly subtype: string;
  readonly is_error: boolean;
  readonly session_id: string | null;
  readonly result: string | null;
  readonly num_turns: number | null;
  readonly duration_ms: number | null;
  readonly duration_api_ms: number | null;
  readonly total_cost_usd: number | null;
  readonly usage: unknown;
  readonly modelUsage: Readonly<Record<string, ModelUsage>> | null;
  readonly permission_denials: readonly unknown[];
  readonly errors: readonly unknown[];
};

export type ModelUsage = {
  readonly inputTokens?: number;
  readonly outputTokens?: number;
  readonly cacheReadInputTokens?: number;
  readonly cacheCreationInputTokens?: number;
  readonly costUSD?: number;
};

export type StreamEvent = {
  readonly type: "stream_event";
  readonly event: {
    readonly type: string;
    readonly index?: number;
    readonly delta?: {
      readonly type: string;
      readonly text?: string;
      readonly partial_json?: string;
    };
    readonly content_block?: {
      readonly type: string;
      readonly id?: string;
      readonly name?: string;
    };
  };
  readonly session_id: string | null;
  readonly parent_tool_use_id: string | null;
};

export type ControlRequest =
  | {
      readonly subtype: "can_use_tool";
      readonly tool_name: string;
      readonly input: unknown;
      readonly tool_use_id: string | null;
      readonly permission_suggestions: unknown;
      readonly blocked_path: string | null;
      readonly decision_reason: unknown;
    }
  | { readonly subtype: "other" };

export type Inbound =
  | SystemMessage
  | AssistantMessage
  | UserEcho
  | ResultMessage
  | StreamEvent
  | {
      readonly type: "control_request";
      readonly request_id: string;
      readonly request: ControlRequest;
    }
  | { readonly type: "control_response"; readonly response: unknown }
  | { readonly type: "control_cancel_request"; readonly request_id: string }
  | ({ readonly type: "rate_limit_event" } & Readonly<Record<string, unknown>>)
  | { readonly type: "unknown" };

export type SessionEvent =
  | { readonly kind: "message"; readonly session_id: string; readonly message: Inbound }
  | {
      readonly kind: "undecodable";
      readonly session_id: string;
      readonly line: string;
      readonly error: string;
    }
  | { readonly kind: "exited"; readonly session_id: string; readonly code: number | null };
