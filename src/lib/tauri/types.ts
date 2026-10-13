export type Engine = "pdf_latex" | "xe_latex" | "lua_latex";

export type AppSettings = {
  readonly texBinDir: string | null;
  readonly claudePath: string | null;
  readonly engine: Engine;
  readonly shellEscape: boolean;
  readonly autocompile: boolean;
  readonly autocompileDebounceMs: number;
  readonly userName: string;
};

export type TexToolchain = {
  readonly binDir: string;
  readonly latexmk: string;
  readonly synctex: string;
};

export type ClaudeProbe = { readonly path: string; readonly version: string };

export type ProjectRecord = {
  readonly id: string;
  readonly root: string;
  readonly mainFile: string | null;
  readonly lastOpened: { readonly secs_since_epoch: number; readonly nanos_since_epoch: number };
  readonly openTabs: readonly string[];
  readonly activeTab: string | null;
  readonly claudeSessionId: string | null;
};

export type NodeKind = "file" | "dir";

export type FileNode = {
  readonly path: string | null;
  readonly name: string;
  readonly kind: NodeKind;
  readonly children: readonly FileNode[];
};

export type MainFileDetection =
  | { readonly kind: "exactly"; readonly path: string }
  | { readonly kind: "candidates"; readonly paths: readonly string[] }
  | { readonly kind: "none" };

export type OpenedProject = {
  readonly record: ProjectRecord;
  readonly tree: FileNode;
  readonly mainFileDetection: MainFileDetection;
};

export type Symbol = { readonly name: string; readonly file: string; readonly line: number };

export type ProjectIndex = {
  readonly labels: readonly Symbol[];
  readonly citations: readonly Symbol[];
  readonly commands: readonly Symbol[];
  readonly environments: readonly Symbol[];
  readonly texFiles: readonly string[];
  readonly bibFiles: readonly string[];
};

export type FileStat = { readonly mtimeMs: number; readonly size: number };
export type FileContents = { readonly text: string; readonly stat: FileStat };

export type FsEvent =
  | { readonly kind: "created"; readonly path: string }
  | { readonly kind: "modified"; readonly path: string }
  | { readonly kind: "removed"; readonly path: string }
  | { readonly kind: "renamed"; readonly from: string; readonly to: string }
  | { readonly kind: "rescan" };

export type CompilePolicy = "queue" | "preempt";
export type Severity = "error" | "warning" | "bad_box" | "info";

export type Diagnostic = {
  readonly severity: Severity;
  readonly file: string | null;
  readonly line: number | null;
  readonly message: string;
  readonly detail: string | null;
  readonly logLine: number;
};

export type CompileStatus =
  | { readonly kind: "success" }
  | { readonly kind: "errors_present" }
  | { readonly kind: "cancelled" }
  | { readonly kind: "failed"; readonly exitCode: number | null };

export type CompileOutcome = {
  readonly jobId: number;
  readonly status: CompileStatus;
  readonly pdfPath: string | null;
  readonly synctexPath: string | null;
  readonly logPath: string;
  readonly diagnostics: readonly Diagnostic[];
  readonly durationMs: number;
};

export type CompileEvent =
  | { readonly kind: "started"; readonly job_id: number }
  | { readonly kind: "output_line"; readonly job_id: number; readonly line: string }
  | { readonly kind: "finished"; readonly outcome: CompileOutcome }
  | { readonly kind: "failed"; readonly job_id: number; readonly error: string };

export type ForwardHit = {
  readonly page: number;
  readonly x: number;
  readonly y: number;
  readonly h: number;
  readonly v: number;
  readonly width: number;
  readonly height: number;
};

export type InverseHit = { readonly file: string; readonly line: number; readonly column: number };

export type Change =
  | "added"
  | "modified"
  | "deleted"
  | "renamed"
  | "type_change"
  | "untracked"
  | "conflicted";

export type StatusEntry = {
  readonly path: string;
  readonly staged: Change | null;
  readonly unstaged: Change | null;
};

export type RepoStatus = {
  readonly branch: string | null;
  readonly head: string | null;
  readonly ahead: number;
  readonly behind: number;
  readonly upstream: string | null;
  readonly entries: readonly StatusEntry[];
};

export type CommitInfo = {
  readonly oid: string;
  readonly shortOid: string;
  readonly summary: string;
  readonly body: string;
  readonly authorName: string;
  readonly authorEmail: string;
  readonly timeUnix: number;
  readonly parents: readonly string[];
};

export type LogQuery = {
  readonly rev: string | null;
  readonly skip: number;
  readonly limit: number;
};

export type ChangedFile = {
  readonly path: string;
  readonly change: Change;
  readonly additions: number;
  readonly deletions: number;
};

export type DiffKind =
  | { readonly kind: "unstaged" }
  | { readonly kind: "staged" }
  | { readonly kind: "commit"; readonly oid: string };

export type FileDiff = {
  readonly path: string;
  readonly oldLabel: string;
  readonly newLabel: string;
  readonly oldText: string;
  readonly newText: string;
  readonly binary: boolean;
  readonly additions: number;
  readonly deletions: number;
};

export type BranchInfo = {
  readonly name: string;
  readonly isHead: boolean;
  readonly upstream: string | null;
  readonly remote: boolean;
};

export type RemoteInfo = {
  readonly name: string;
  readonly url: string;
  readonly pushUrl: string | null;
};

export type GitProgress =
  | { readonly kind: "counting"; readonly received: number; readonly total: number }
  | {
      readonly kind: "receiving";
      readonly received: number;
      readonly total: number;
      readonly bytes: number;
    }
  | { readonly kind: "resolving"; readonly current: number; readonly total: number }
  | {
      readonly kind: "pushing";
      readonly current: number;
      readonly total: number;
      readonly bytes: number;
    }
  | { readonly kind: "sideband"; readonly text: string };

export type GitProgressPayload = { readonly operation: string; readonly progress: GitProgress };

export type PullResult =
  | { readonly kind: "up_to_date" }
  | { readonly kind: "fast_forwarded"; readonly from: string; readonly to: string };

export type CommitOutcome = { readonly oid: string; readonly status: RepoStatus };

export type PermissionMode = "default" | "acceptEdits" | "plan" | "dontAsk" | "bypassPermissions";

export type PermissionDecision =
  | { readonly behavior: "allow"; readonly updatedInput?: unknown }
  | { readonly behavior: "deny"; readonly message: string };

export type TerminalEventPayload =
  | { readonly kind: "output"; readonly id: number; readonly data: string }
  | { readonly kind: "exited"; readonly id: number; readonly code: number | null };

export type StartedSession = { readonly sessionId: string; readonly resumed: boolean };

export type WireError = {
  readonly domain:
    | "config"
    | "project"
    | "tex"
    | "git"
    | "claude"
    | "watch"
    | "path"
    | "secrets"
    | "process"
    | "terminal"
    | "no_project"
    | "poisoned";
  readonly kind: string;
} & Readonly<Record<string, unknown>>;

export function isWireError(value: unknown): value is WireError {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const candidate = value as { readonly domain?: unknown; readonly kind?: unknown };

  return typeof candidate.domain === "string" && typeof candidate.kind === "string";
}

export function describeError(error: WireError): string {
  const detail = Object.entries(error)
    .filter(([key]) => key !== "domain" && key !== "kind")
    .map(([key, value]) => `${key}=${typeof value === "string" ? value : JSON.stringify(value)}`)
    .join(", ");

  return detail === ""
    ? `${error.domain}/${error.kind}`
    : `${error.domain}/${error.kind} (${detail})`;
}
