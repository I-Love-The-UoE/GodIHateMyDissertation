import { isErr } from "@/lib/result";
import type { CompileEvent, CompileOutcome, CompilePolicy, Diagnostic } from "@/lib/tauri";
import { describeError } from "@/lib/tauri";
import type { Slice } from "./types";

export type CompileState = "idle" | "running" | "success" | "errors" | "failed" | "cancelled";

export type CompileSlice = {
  readonly compileState: CompileState;
  readonly compileJobId: number;
  readonly outcome: CompileOutcome | undefined;
  readonly diagnostics: readonly Diagnostic[];
  readonly outputLines: readonly string[];
  readonly pdf: { readonly path: string; readonly jobId: number } | undefined;
  readonly compile: (policy: CompilePolicy) => Promise<void>;
  readonly cancelCompile: () => Promise<void>;
  readonly handleCompileEvent: (event: CompileEvent) => void;
  readonly loadLatestOutcome: () => Promise<void>;
  readonly resetCompile: () => void;
};

const MAX_OUTPUT_LINES = 2000;

function stateOf(outcome: CompileOutcome): CompileState {
  switch (outcome.status.kind) {
    case "success":
      return "success";
    case "errors_present":
      return "errors";
    case "cancelled":
      return "cancelled";
    case "failed":
      return "failed";
    default:
      return outcome.status satisfies never;
  }
}

export const createCompileSlice: Slice<CompileSlice> = (backend) => (set, get) => {
  const applyOutcome = (outcome: CompileOutcome): void => {
    const pdf =
      outcome.pdfPath === null ? get().pdf : { path: outcome.pdfPath, jobId: outcome.jobId };
    set({ compileState: stateOf(outcome), outcome, diagnostics: outcome.diagnostics, pdf });
  };

  return {
    compileState: "idle",
    compileJobId: 0,
    outcome: undefined,
    diagnostics: [],
    outputLines: [],
    pdf: undefined,
    compile: async (policy) => {
      const result = await backend.texCompile(policy);

      if (isErr(result)) {
        get().toast("error", `Compile could not start: ${describeError(result.error)}`);
        set({ compileState: "failed" });

        return;
      }

      set({ compileJobId: result.value });
    },
    cancelCompile: async () => {
      const result = await backend.texCancel();

      if (isErr(result)) {
        get().toast("error", describeError(result.error));
      }
    },
    handleCompileEvent: (event) => {
      switch (event.kind) {
        case "started":
          set({ compileState: "running", outputLines: [], compileJobId: event.job_id });

          return;
        case "output_line":
          set((state) => ({
            outputLines: [...state.outputLines.slice(-MAX_OUTPUT_LINES), event.line],
          }));

          return;
        case "finished":
          applyOutcome(event.outcome);

          return;
        case "failed":
          set({ compileState: "failed" });
          get().toast("error", `Compile failed: ${event.error}`);

          return;
        default:
          return event satisfies never;
      }
    },
    loadLatestOutcome: async () => {
      const result = await backend.texLatestOutcome();

      if (isErr(result) || result.value === null) {
        return;
      }

      applyOutcome(result.value);
    },
    resetCompile: () =>
      set({
        compileState: "idle",
        compileJobId: 0,
        outcome: undefined,
        diagnostics: [],
        outputLines: [],
        pdf: undefined,
      }),
  };
};
