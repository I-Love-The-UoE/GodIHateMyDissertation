import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import { useStore } from "@/app/services";
import { Button } from "@/components/button";
import { spawn } from "@/lib/async";
import type { PermissionMode } from "@/lib/tauri";
import type { ChatItem } from "@/state/claude-slice";
import { toolResultText } from "@/state/claude-slice";

const MODES: readonly { readonly value: PermissionMode; readonly label: string }[] = [
  { value: "default", label: "Ask before tools" },
  { value: "acceptEdits", label: "Auto-accept edits" },
  { value: "plan", label: "Plan only" },
  { value: "bypassPermissions", label: "Bypass permissions" },
];

function pretty(value: unknown): string {
  if (typeof value === "string") {
    return value;
  }

  return JSON.stringify(value, null, 2);
}

function Item(props: { readonly item: ChatItem }): ReactNode {
  const { item } = props;
  const decidePermission = useStore((state) => state.decidePermission);

  switch (item.kind) {
    case "user":
      return <div className="chat-item chat-user">{item.text}</div>;
    case "assistant":
      return (
        <div className="chat-item chat-assistant">
          {item.text}
          {item.streaming ? <span className="hint"> ▍</span> : null}
        </div>
      );
    case "system":
      return <div className="chat-item chat-system">{item.text}</div>;
    case "tool":
      return (
        <details
          className={`chat-item chat-tool${item.result?.isError === true ? " is-error" : ""}`}
        >
          <summary>
            {item.result === undefined ? "⏳" : item.result.isError ? "✗" : "✓"} {item.name}
          </summary>
          <pre>{pretty(item.input)}</pre>
          {item.result === undefined ? null : (
            <pre>{toolResultText(item.result.content) || pretty(item.result.content)}</pre>
          )}
        </details>
      );
    case "permission":
      return (
        <div className="chat-item chat-permission">
          <div>
            Claude wants to run <strong>{item.toolName}</strong>
          </div>
          <pre style={{ maxHeight: 200, overflow: "auto", fontSize: 12 }}>{pretty(item.input)}</pre>
          {item.decision === undefined ? (
            <div className="actions">
              <Button
                small
                variant="primary"
                onClick={() => spawn(decidePermission(item.id, { behavior: "allow" }))}
              >
                Allow
              </Button>
              <Button
                small
                variant="danger"
                onClick={() =>
                  spawn(
                    decidePermission(item.id, {
                      behavior: "deny",
                      message: "The user denied this action.",
                    }),
                  )
                }
              >
                Deny
              </Button>
            </div>
          ) : (
            <div className="hint">{item.decision === "allow" ? "Allowed." : "Denied."}</div>
          )}
        </div>
      );
    case "result":
      return (
        <div className="chat-result">
          {item.isError ? `error: ${item.subtype}` : item.subtype} · {item.numTurns ?? 0} turns ·{" "}
          {((item.durationMs ?? 0) / 1000).toFixed(1)}s
          {item.costUsd === null ? "" : ` · $${item.costUsd.toFixed(4)}`}
        </div>
      );
    default:
      return item satisfies never;
  }
}

export function ClaudePanel(): ReactNode {
  const chat = useStore((state) => state.chat);
  const sessionState = useStore((state) => state.sessionState);
  const sessionId = useStore((state) => state.sessionId);
  const model = useStore((state) => state.model);
  const permissionMode = useStore((state) => state.permissionMode);
  const attachDiagnostics = useStore((state) => state.attachDiagnostics);
  const claudeProbe = useStore((state) => state.claudeProbe);
  const project = useStore((state) => state.project);
  const startSession = useStore((state) => state.startSession);
  const sendMessage = useStore((state) => state.sendMessage);
  const interruptSession = useStore((state) => state.interruptSession);
  const stopSession = useStore((state) => state.stopSession);
  const setPermissionMode = useStore((state) => state.setPermissionMode);
  const setAttachDiagnostics = useStore((state) => state.setAttachDiagnostics);
  const [draft, setDraft] = useState("");
  const bottomRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (chat.length > 0) {
      bottomRef.current?.scrollIntoView({ block: "end" });
    }
  }, [chat.length]);

  const send = (): void => {
    const text = draft.trim();

    if (text === "" || sessionState === "busy" || sessionState === "starting") {
      return;
    }

    setDraft("");
    spawn(sendMessage(text));
  };

  if (claudeProbe.state === "error") {
    return (
      <div
        className="empty"
        style={{ flexDirection: "column", gap: 8, padding: 20, textAlign: "center" }}
      >
        <span>The Claude Code CLI was not found.</span>
        <span className="hint">
          Install it with the official installer, or set its path in Settings.
        </span>
      </div>
    );
  }

  return (
    <div className="panel">
      <div className="panel-header" style={{ fontWeight: 400 }}>
        <span
          className={`dot ${sessionState === "stopped" ? "dot-idle" : sessionState === "busy" ? "dot-warn" : "dot-ok"}`}
        />
        <span className="hint">
          {sessionState === "stopped" ? "No session" : (model ?? "session")}
        </span>
        <span className="spacer" />
        {sessionId === undefined ? (
          <>
            <Button
              small
              onClick={() => spawn(startSession(false))}
              disabled={sessionState === "starting"}
            >
              New session
            </Button>
            {project?.record.claudeSessionId === null ? null : (
              <Button
                small
                onClick={() => spawn(startSession(true))}
                disabled={sessionState === "starting"}
              >
                Resume
              </Button>
            )}
          </>
        ) : (
          <>
            <Button
              small
              onClick={() => spawn(interruptSession())}
              disabled={sessionState !== "busy"}
            >
              Interrupt
            </Button>
            <Button small variant="ghost" onClick={() => spawn(stopSession())}>
              Stop
            </Button>
          </>
        )}
      </div>
      <div className="panel-body">
        <div className="chat">
          {chat.length === 0 ? (
            <div className="chat-system">
              Ask Claude to explain, fix or extend your LaTeX. It can read and edit files in this
              project.
            </div>
          ) : null}
          {chat.map((item) => (
            <Item key={item.id} item={item} />
          ))}
          <div ref={bottomRef} />
        </div>
      </div>
      <div className="composer">
        <textarea
          placeholder="Message Claude Code… (Enter to send, Shift+Enter for a new line)"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              send();
            }
          }}
        />
        <div className="row">
          <select
            value={permissionMode}
            onChange={(event) => setPermissionMode(event.target.value as PermissionMode)}
            disabled={sessionId !== undefined}
          >
            {MODES.map((mode) => (
              <option key={mode.value} value={mode.value}>
                {mode.label}
              </option>
            ))}
          </select>
          <label className="hint" style={{ display: "flex", gap: 4, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={attachDiagnostics}
              onChange={(event) => setAttachDiagnostics(event.target.checked)}
            />
            attach compile diagnostics
          </label>
          <span className="spacer" />
          <Button
            variant="primary"
            small
            onClick={send}
            disabled={draft.trim() === "" || sessionState === "busy" || sessionState === "starting"}
          >
            Send
          </Button>
        </div>
      </div>
    </div>
  );
}
