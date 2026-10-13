import type { UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";
import { spawn } from "@/lib/async";
import type { SessionEvent } from "./claude-protocol";
import type { CompileEvent, FsEvent, GitProgressPayload, TerminalEventPayload } from "./types";

export type Unsubscribe = () => void;

function subscribe<T>(name: string, handler: (payload: T) => void): Unsubscribe {
  let unlisten: UnlistenFn | undefined;
  let cancelled = false;
  spawn(
    listen<T>(name, (event) => handler(event.payload)).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    }),
  );

  return () => {
    cancelled = true;
    unlisten?.();
  };
}

export function onFsChanged(handler: (events: readonly FsEvent[]) => void): Unsubscribe {
  return subscribe("fs://changed", handler);
}

export function onCompileEvent(handler: (event: CompileEvent) => void): Unsubscribe {
  return subscribe("compile://event", handler);
}

export function onGitProgress(handler: (payload: GitProgressPayload) => void): Unsubscribe {
  return subscribe("git://progress", handler);
}

export function onGitChanged(handler: () => void): Unsubscribe {
  return subscribe<null>("git://changed", () => handler());
}

export function onTerminalEvent(handler: (payload: TerminalEventPayload) => void): Unsubscribe {
  return subscribe("terminal://event", handler);
}

export function onClaudeEvent(handler: (event: SessionEvent) => void): Unsubscribe {
  return subscribe("claude://event", handler);
}
