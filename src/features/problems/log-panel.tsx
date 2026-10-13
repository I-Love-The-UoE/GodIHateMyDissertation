import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import { useServices, useStore } from "@/app/services";
import { Button } from "@/components/button";
import { spawn } from "@/lib/async";

export function LogPanel(): ReactNode {
  const { backend } = useServices();
  const outputLines = useStore((state) => state.outputLines);
  const compileState = useStore((state) => state.compileState);
  const [rawLog, setRawLog] = useState<string | undefined>(undefined);
  const bottomRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (compileState === "running" && outputLines.length >= 0) {
      setRawLog(undefined);
      bottomRef.current?.scrollIntoView({ block: "end" });
    }
  }, [compileState, outputLines.length]);

  const loadRaw = async (): Promise<void> => {
    const result = await backend.texReadLog();
    setRawLog(result.ok ? result.value : "The log could not be read.");
  };

  return (
    <div className="panel">
      <div className="panel-header" style={{ fontWeight: 400 }}>
        <span className="hint">{rawLog === undefined ? "latexmk output" : "Full .log file"}</span>
        <span className="spacer" />
        {rawLog === undefined ? (
          <Button
            small
            onClick={() => spawn(loadRaw())}
            disabled={compileState === "idle" || compileState === "running"}
          >
            Show full .log
          </Button>
        ) : (
          <Button small onClick={() => setRawLog(undefined)}>
            Show latexmk output
          </Button>
        )}
      </div>
      <div className="panel-body">
        <pre className="log-output">{rawLog ?? outputLines.join("\n")}</pre>
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
