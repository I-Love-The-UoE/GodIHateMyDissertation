import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import { useServices, useStore } from "@/app/services";
import { Button } from "@/components/button";
import { spawn } from "@/lib/async";
import { debugLog, errorLog } from "@/lib/log";
import { browse, describeError, onTerminalEvent } from "@/lib/tauri";
import type { TerminalView } from "@/lib/xterm";
import { createTerminalView, decodeBase64 } from "@/lib/xterm";

export type TerminalPanelProps = { readonly visible: boolean };

/** One shell per project; the view stays mounted so switching tabs keeps the session. */
export function TerminalPanel(props: TerminalPanelProps): ReactNode {
  const { visible } = props;
  const { backend } = useServices();
  const project = useStore((state) => state.project);
  const toast = useStore((state) => state.toast);
  const hostRef = useRef<HTMLDivElement | null>(null);
  const viewRef = useRef<TerminalView | undefined>(undefined);
  const idRef = useRef<number | undefined>(undefined);
  const [exited, setExited] = useState<number | null | undefined>(undefined);
  const [generation, setGeneration] = useState(0);
  const projectId = project?.record.id;

  useEffect(() => {
    const host = hostRef.current;

    if (host === null || projectId === undefined) {
      return;
    }

    const view = createTerminalView(host, (url) => spawn(browse(url)));
    viewRef.current = view;
    debugLog("Created a terminal view.", { generation });
    view.fit();
    let closed = false;
    const unsubscribe = onTerminalEvent((payload) => {
      if (payload.id !== idRef.current) {
        return;
      }

      if (payload.kind === "output") {
        view.write(decodeBase64(payload.data));
      } else {
        setExited(payload.code);
        view.writeText(`\r\n[process exited with code ${payload.code ?? "unknown"}]\r\n`);
      }
    });
    const open = async (): Promise<void> => {
      const result = await backend.terminalOpen(view.cols(), view.rows());

      if (closed) {
        if (result.ok) {
          await backend.terminalClose(result.value);
        }

        return;
      }

      if (!result.ok) {
        toast("error", `Terminal: ${describeError(result.error)}`);
        view.writeText(`\r\n${describeError(result.error)}\r\n`);

        return;
      }

      idRef.current = result.value;
      setExited(undefined);
      view.focus();
    };
    const stopInput = view.onInput((data) => {
      const id = idRef.current;

      if (id !== undefined) {
        spawn(backend.terminalWrite(id, data));
      }
    });
    const stopResize = view.onResize((cols, rows) => {
      const id = idRef.current;

      if (id !== undefined) {
        spawn(backend.terminalResize(id, cols, rows));
      }
    });
    const observer = new ResizeObserver(() => {
      if (host.clientWidth > 0 && host.clientHeight > 0) {
        view.fit();
      }
    });
    observer.observe(host);
    spawn(
      open().catch((rejection: unknown) => {
        errorLog("Opening the terminal failed.", { rejection });
      }),
    );

    return () => {
      closed = true;
      observer.disconnect();
      stopInput();
      stopResize();
      unsubscribe();
      const id = idRef.current;
      idRef.current = undefined;

      if (id !== undefined) {
        spawn(backend.terminalClose(id));
      }

      view.destroy();
      viewRef.current = undefined;
    };
  }, [backend, projectId, toast, generation]);

  useEffect(() => {
    if (visible) {
      requestAnimationFrame(() => {
        viewRef.current?.fit();
        viewRef.current?.focus();
      });
    }
  }, [visible]);

  return (
    <div className="panel">
      {exited === undefined ? null : (
        <div className="panel-header" style={{ fontWeight: 400 }}>
          <span className="hint">The shell exited.</span>
          <span className="spacer" />
          <Button small variant="primary" onClick={() => setGeneration((value) => value + 1)}>
            New terminal
          </Button>
        </div>
      )}
      <div className="panel-body terminal-host" ref={hostRef} />
    </div>
  );
}
