import type { ReactNode } from "react";
import { useStore } from "@/app/services";

const LABELS = { error: "error", warning: "warning", bad_box: "bad box", info: "info" } as const;

export function ProblemsPanel(): ReactNode {
  const diagnostics = useStore((state) => state.diagnostics);
  const project = useStore((state) => state.project);
  const requestGoTo = useStore((state) => state.requestGoTo);
  const compileState = useStore((state) => state.compileState);

  if (diagnostics.length === 0) {
    return (
      <div className="empty">{compileState === "idle" ? "No compile yet." : "No problems."}</div>
    );
  }

  const root = project === undefined ? "" : `${project.record.root}/`;

  return (
    <div className="problems">
      {diagnostics.map((diagnostic) => {
        const relative =
          diagnostic.file?.startsWith(root) === true
            ? diagnostic.file.slice(root.length)
            : diagnostic.file;
        const where =
          relative === null
            ? ""
            : `${relative}${diagnostic.line === null ? "" : `:${diagnostic.line}`}`;
        const clickable =
          relative !== null && diagnostic.line !== null && relative.endsWith(".tex");

        return (
          <button
            type="button"
            key={`${diagnostic.logLine}-${diagnostic.message}`}
            className="problem"
            onClick={() => {
              if (clickable && diagnostic.line !== null) {
                requestGoTo(relative, diagnostic.line, 1);
              }
            }}
            title={diagnostic.detail ?? undefined}
          >
            <span className={`sev-${diagnostic.severity}`}>{LABELS[diagnostic.severity]}</span>
            <span>
              {diagnostic.message}
              {where === "" ? null : <span className="where"> — {where}</span>}
            </span>
          </button>
        );
      })}
    </div>
  );
}
