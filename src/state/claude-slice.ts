import { isErr } from "@/lib/result";
import type {
  AssistantMessage,
  PermissionDecision,
  PermissionMode,
  ResultMessage,
  SessionEvent,
  StreamEvent,
  UserEcho,
} from "@/lib/tauri";
import { describeError } from "@/lib/tauri";
import type { Slice } from "./types";

export type ChatItem =
  | { readonly kind: "user"; readonly id: string; readonly text: string }
  | {
      readonly kind: "assistant";
      readonly id: string;
      readonly text: string;
      readonly streaming: boolean;
    }
  | {
      readonly kind: "tool";
      readonly id: string;
      readonly name: string;
      readonly input: unknown;
      readonly result: { readonly content: unknown; readonly isError: boolean } | undefined;
    }
  | {
      readonly kind: "permission";
      readonly id: string;
      readonly toolName: string;
      readonly input: unknown;
      readonly decision: "allow" | "deny" | undefined;
    }
  | {
      readonly kind: "result";
      readonly id: string;
      readonly subtype: string;
      readonly isError: boolean;
      readonly costUsd: number | null;
      readonly durationMs: number | null;
      readonly numTurns: number | null;
    }
  | { readonly kind: "system"; readonly id: string; readonly text: string };

export type ClaudeSlice = {
  readonly sessionId: string | undefined;
  readonly sessionState: "stopped" | "starting" | "idle" | "busy";
  readonly chat: readonly ChatItem[];
  readonly model: string | undefined;
  readonly permissionMode: PermissionMode;
  readonly attachDiagnostics: boolean;
  readonly totalCostUsd: number;
  readonly startSession: (resume: boolean) => Promise<void>;
  readonly sendMessage: (text: string) => Promise<void>;
  readonly decidePermission: (requestId: string, decision: PermissionDecision) => Promise<void>;
  readonly interruptSession: () => Promise<void>;
  readonly stopSession: () => Promise<void>;
  readonly setPermissionMode: (mode: PermissionMode) => void;
  readonly setAttachDiagnostics: (attach: boolean) => void;
  readonly handleClaudeEvent: (event: SessionEvent) => void;
  readonly resetClaude: () => void;
};

let itemCounter = 0;

function nextId(prefix: string): string {
  itemCounter += 1;

  return `${prefix}-${itemCounter}`;
}

function textOfBlocks(content: unknown): string {
  if (typeof content === "string") {
    return content;
  }

  if (!Array.isArray(content)) {
    return "";
  }

  return content
    .map((block: unknown) => {
      if (
        typeof block === "object" &&
        block !== null &&
        "type" in block &&
        block.type === "text" &&
        "text" in block
      ) {
        return String(block.text);
      }

      return "";
    })
    .join("");
}

function appendStreamingText(chat: readonly ChatItem[], text: string): ChatItem[] {
  const last = chat.at(-1);

  if (last !== undefined && last.kind === "assistant" && last.streaming) {
    return [...chat.slice(0, -1), { ...last, text: last.text + text }];
  }

  return [...chat, { kind: "assistant", id: nextId("assistant"), text, streaming: true }];
}

function applyStream(chat: readonly ChatItem[], event: StreamEvent): ChatItem[] {
  if (event.parent_tool_use_id !== null) {
    return [...chat];
  }

  if (event.event.type === "content_block_delta" && event.event.delta?.type === "text_delta") {
    return appendStreamingText(chat, event.event.delta.text ?? "");
  }

  return [...chat];
}

function applyAssistant(chat: readonly ChatItem[], message: AssistantMessage): ChatItem[] {
  if (message.parent_tool_use_id !== null) {
    return [...chat];
  }

  let next = [...chat];
  const text = message.message.content
    .map((block) => (block.type === "text" ? block.text : ""))
    .join("");
  const last = next.at(-1);

  if (last !== undefined && last.kind === "assistant" && last.streaming) {
    next = [
      ...next.slice(0, -1),
      { ...last, text: text === "" ? last.text : text, streaming: false },
    ];
  } else if (text !== "") {
    next.push({ kind: "assistant", id: nextId("assistant"), text, streaming: false });
  }

  for (const block of message.message.content) {
    if (
      block.type === "tool_use" &&
      !next.some((item) => item.kind === "tool" && item.id === block.id)
    ) {
      next.push({
        kind: "tool",
        id: block.id,
        name: block.name,
        input: block.input,
        result: undefined,
      });
    }
  }

  return next;
}

function applyToolResults(chat: readonly ChatItem[], echo: UserEcho): ChatItem[] {
  const content = echo.message.content;

  if (!Array.isArray(content)) {
    return [...chat];
  }

  let next = [...chat];

  for (const block of content) {
    if (
      typeof block !== "object" ||
      block === null ||
      !("type" in block) ||
      block.type !== "tool_result"
    ) {
      continue;
    }

    const toolUseId = "tool_use_id" in block ? String(block.tool_use_id) : "";
    const isError = "is_error" in block && block.is_error === true;
    const body = "content" in block ? block.content : "";
    next = next.map((item) =>
      item.kind === "tool" && item.id === toolUseId
        ? { ...item, result: { content: body, isError } }
        : item,
    );
  }

  return next;
}

function applyResult(chat: readonly ChatItem[], message: ResultMessage): ChatItem[] {
  const finished = chat.map((item) =>
    item.kind === "assistant" && item.streaming ? { ...item, streaming: false } : item,
  );

  return [
    ...finished,
    {
      kind: "result",
      id: nextId("result"),
      subtype: message.subtype,
      isError: message.is_error,
      costUsd: message.total_cost_usd,
      durationMs: message.duration_ms,
      numTurns: message.num_turns,
    },
  ];
}

export const createClaudeSlice: Slice<ClaudeSlice> = (backend) => (set, get) => ({
  sessionId: undefined,
  sessionState: "stopped",
  chat: [],
  model: undefined,
  permissionMode: "default",
  attachDiagnostics: true,
  totalCostUsd: 0,
  startSession: async (resume) => {
    set({ sessionState: "starting" });
    const result = await backend.claudeStartSession(resume, get().permissionMode, null);

    if (isErr(result)) {
      set({ sessionState: "stopped" });
      get().toast("error", `Claude Code could not start: ${describeError(result.error)}`);

      return;
    }

    set({
      sessionId: result.value.sessionId,
      sessionState: "idle",
      chat: [
        {
          kind: "system",
          id: nextId("system"),
          text: result.value.resumed ? "Resumed the previous session." : "Started a new session.",
        },
      ],
    });
  },
  sendMessage: async (text) => {
    const sessionId = get().sessionId;

    if (sessionId === undefined) {
      await get().startSession(false);
    }

    const active = get().sessionId;

    if (active === undefined) {
      return;
    }

    set((state) => ({
      chat: [...state.chat, { kind: "user", id: nextId("user"), text }],
      sessionState: "busy",
    }));
    const result = await backend.claudeSend(active, text, get().attachDiagnostics);

    if (isErr(result)) {
      set({ sessionState: "idle" });
      get().toast("error", describeError(result.error));
    }
  },
  decidePermission: async (requestId, decision) => {
    const sessionId = get().sessionId;

    if (sessionId === undefined) {
      return;
    }

    set((state) => ({
      chat: state.chat.map((item) =>
        item.kind === "permission" && item.id === requestId
          ? { ...item, decision: decision.behavior }
          : item,
      ),
    }));
    const result = await backend.claudePermission(sessionId, requestId, decision);

    if (isErr(result)) {
      get().toast("error", describeError(result.error));
    }
  },
  interruptSession: async () => {
    const sessionId = get().sessionId;

    if (sessionId === undefined) {
      return;
    }

    const result = await backend.claudeInterrupt(sessionId);

    if (isErr(result)) {
      get().toast("error", describeError(result.error));
    }
  },
  stopSession: async () => {
    const sessionId = get().sessionId;

    if (sessionId === undefined) {
      return;
    }

    await backend.claudeStop(sessionId);
    set({ sessionId: undefined, sessionState: "stopped" });
  },
  setPermissionMode: (mode) => set({ permissionMode: mode }),
  setAttachDiagnostics: (attach) => set({ attachDiagnostics: attach }),
  handleClaudeEvent: (event) => {
    if (event.session_id !== get().sessionId) {
      return;
    }

    switch (event.kind) {
      case "exited":
        set((state) => ({
          sessionId: undefined,
          sessionState: "stopped",
          chat: [
            ...state.chat,
            {
              kind: "system",
              id: nextId("system"),
              text: `Session ended (exit code ${event.code ?? "unknown"}).`,
            },
          ],
        }));

        return;
      case "undecodable":
        set((state) => ({
          chat: [
            ...state.chat,
            {
              kind: "system",
              id: nextId("system"),
              text: `Undecodable line: ${event.line.slice(0, 200)}`,
            },
          ],
        }));

        return;
      case "message": {
        const message = event.message;

        switch (message.type) {
          case "system":
            if (message.subtype === "init") {
              set({ model: message.model ?? undefined });
            }

            return;
          case "stream_event":
            set((state) => ({ chat: applyStream(state.chat, message) }));

            return;
          case "assistant":
            set((state) => ({ chat: applyAssistant(state.chat, message) }));

            return;
          case "user":
            set((state) => ({ chat: applyToolResults(state.chat, message) }));

            return;
          case "result":
            set((state) => ({
              chat: applyResult(state.chat, message),
              sessionState: "idle",
              totalCostUsd: message.total_cost_usd ?? state.totalCostUsd,
            }));

            return;
          case "control_request":
            if (message.request.subtype === "can_use_tool") {
              const request = message.request;
              set((state) => ({
                chat: [
                  ...state.chat,
                  {
                    kind: "permission",
                    id: message.request_id,
                    toolName: request.tool_name,
                    input: request.input,
                    decision: undefined,
                  },
                ],
              }));
            }

            return;
          case "control_response":
          case "control_cancel_request":
          case "rate_limit_event":
          case "unknown":
            return;
          default:
            return message satisfies never;
        }
      }
      default:
        return event satisfies never;
    }
  },
  resetClaude: () =>
    set({
      sessionId: undefined,
      sessionState: "stopped",
      chat: [],
      model: undefined,
      totalCostUsd: 0,
    }),
});

export function toolResultText(content: unknown): string {
  return textOfBlocks(content);
}
