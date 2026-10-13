import { invoke } from "@tauri-apps/api/core";
import type { Result } from "@/lib/result";
import { err, ok } from "@/lib/result";
import type {
  AppSettings,
  BranchInfo,
  ChangedFile,
  ClaudeProbe,
  CommitInfo,
  CommitOutcome,
  CompileOutcome,
  CompilePolicy,
  DiffKind,
  FileContents,
  FileDiff,
  FileNode,
  FileStat,
  ForwardHit,
  InverseHit,
  LogQuery,
  MainFileDetection,
  OpenedProject,
  PermissionDecision,
  PermissionMode,
  ProjectIndex,
  ProjectRecord,
  PullResult,
  RemoteInfo,
  RepoStatus,
  StartedSession,
  TexToolchain,
  WireError,
} from "./types";
import { isWireError } from "./types";

export type Call<T> = Promise<Result<T, WireError>>;

async function call<T>(command: string, args: Record<string, unknown> = {}): Call<T> {
  try {
    return ok(await invoke<T>(command, args));
  } catch (rejection: unknown) {
    if (!isWireError(rejection)) {
      throw rejection; // NOTE: Not a backend error value; this is a bug and must surface.
    }

    return err(rejection);
  }
}

export type Backend = {
  readonly settingsGet: () => Call<AppSettings>;
  readonly settingsSet: (settings: AppSettings) => Call<AppSettings>;
  readonly texProbeToolchain: (explicit: string | null) => Call<TexToolchain>;
  readonly claudeProbeBinary: (explicit: string | null) => Call<ClaudeProbe>;
  readonly projectRecent: () => Call<ProjectRecord[]>;
  readonly projectOpen: (root: string) => Call<OpenedProject>;
  readonly projectCreate: (root: string) => Call<OpenedProject>;
  readonly projectClose: () => Call<void>;
  readonly projectForget: (root: string) => Call<ProjectRecord[]>;
  readonly projectSetMainFile: (path: string) => Call<ProjectRecord>;
  readonly projectSetTabs: (
    openTabs: readonly string[],
    activeTab: string | null,
  ) => Call<ProjectRecord>;
  readonly projectDetectMainFile: () => Call<MainFileDetection>;
  readonly projectTree: () => Call<FileNode>;
  readonly projectIndex: () => Call<ProjectIndex>;
  readonly fileRead: (path: string) => Call<FileContents>;
  readonly fileWrite: (
    path: string,
    text: string,
    expectedMtimeMs: number | null,
  ) => Call<FileStat>;
  readonly fileCreate: (path: string) => Call<FileNode>;
  readonly dirCreate: (path: string) => Call<FileNode>;
  readonly pathRename: (from: string, to: string) => Call<FileNode>;
  readonly pathDelete: (path: string) => Call<FileNode>;
  readonly pathMove: (from: string, toDir: string | null) => Call<FileNode>;
  readonly texCompile: (policy: CompilePolicy) => Call<number>;
  readonly texCancel: () => Call<void>;
  readonly texSynctexForward: (file: string, line: number, column: number) => Call<ForwardHit[]>;
  readonly texSynctexInverse: (page: number, x: number, y: number) => Call<InverseHit[]>;
  readonly texReadLog: () => Call<string>;
  readonly texLatestOutcome: () => Call<CompileOutcome | null>;
  readonly gitOpenOrNone: () => Call<RepoStatus | null>;
  readonly gitInit: () => Call<RepoStatus>;
  readonly gitStatus: () => Call<RepoStatus>;
  readonly gitStage: (paths: readonly string[]) => Call<RepoStatus>;
  readonly gitUnstage: (paths: readonly string[]) => Call<RepoStatus>;
  readonly gitDiscard: (paths: readonly string[]) => Call<RepoStatus>;
  readonly gitCommit: (message: string) => Call<CommitOutcome>;
  readonly gitLog: (query: LogQuery) => Call<CommitInfo[]>;
  readonly gitCommitFiles: (oid: string) => Call<ChangedFile[]>;
  readonly gitDiffFile: (path: string, kind: DiffKind) => Call<FileDiff>;
  readonly gitBranches: () => Call<BranchInfo[]>;
  readonly gitCreateBranch: (name: string, checkout: boolean) => Call<BranchInfo[]>;
  readonly gitCheckoutBranch: (name: string) => Call<RepoStatus>;
  readonly gitRemotes: () => Call<RemoteInfo[]>;
  readonly gitAddRemote: (name: string, url: string) => Call<RemoteInfo[]>;
  readonly gitRemoveRemote: (name: string) => Call<RemoteInfo[]>;
  readonly gitFetch: (remote: string) => Call<RepoStatus>;
  readonly gitPull: (remote: string) => Call<PullResult>;
  readonly gitPush: (remote: string, refspec: string | null) => Call<RepoStatus>;
  readonly gitClone: (url: string, dest: string) => Call<string>;
  readonly gitCancel: () => Call<void>;
  readonly gitCredentialStore: (host: string, username: string, secret: string) => Call<void>;
  readonly gitCredentialForget: (host: string) => Call<void>;
  readonly claudeStartSession: (
    resume: boolean,
    permissionMode: PermissionMode,
    model: string | null,
  ) => Call<StartedSession>;
  readonly claudeSend: (sessionId: string, text: string, attachDiagnostics: boolean) => Call<void>;
  readonly claudePermission: (
    sessionId: string,
    requestId: string,
    decision: PermissionDecision,
  ) => Call<void>;
  readonly claudeInterrupt: (sessionId: string) => Call<void>;
  readonly claudeStop: (sessionId: string) => Call<void>;
  readonly claudeSessions: () => Call<string[]>;
  readonly terminalOpen: (cols: number, rows: number) => Call<number>;
  readonly terminalWrite: (id: number, data: string) => Call<void>;
  readonly terminalResize: (id: number, cols: number, rows: number) => Call<void>;
  readonly terminalClose: (id: number) => Call<void>;
};

export function createBackend(): Backend {
  return {
    settingsGet: () => call("settings_get"),
    settingsSet: (settings) => call("settings_set", { settings }),
    texProbeToolchain: (explicit) => call("tex_probe_toolchain", { explicit }),
    claudeProbeBinary: (explicit) => call("claude_probe_binary", { explicit }),
    projectRecent: () => call("project_recent"),
    projectOpen: (root) => call("project_open", { root }),
    projectCreate: (root) => call("project_create", { root }),
    projectClose: () => call("project_close"),
    projectForget: (root) => call("project_forget", { root }),
    projectSetMainFile: (path) => call("project_set_main_file", { path }),
    projectSetTabs: (openTabs, activeTab) => call("project_set_tabs", { openTabs, activeTab }),
    projectDetectMainFile: () => call("project_detect_main_file"),
    projectTree: () => call("project_tree"),
    projectIndex: () => call("project_index"),
    fileRead: (path) => call("file_read", { path }),
    fileWrite: (path, text, expectedMtimeMs) => call("file_write", { path, text, expectedMtimeMs }),
    fileCreate: (path) => call("file_create", { path }),
    dirCreate: (path) => call("dir_create", { path }),
    pathRename: (from, to) => call("path_rename", { from, to }),
    pathDelete: (path) => call("path_delete", { path }),
    pathMove: (from, toDir) => call("path_move", { from, toDir }),
    texCompile: (policy) => call("tex_compile", { policy }),
    texCancel: () => call("tex_cancel"),
    texSynctexForward: (file, line, column) => call("tex_synctex_forward", { file, line, column }),
    texSynctexInverse: (page, x, y) => call("tex_synctex_inverse", { page, x, y }),
    texReadLog: () => call("tex_read_log"),
    texLatestOutcome: () => call("tex_latest_outcome"),
    gitOpenOrNone: () => call("git_open_or_none"),
    gitInit: () => call("git_init"),
    gitStatus: () => call("git_status"),
    gitStage: (paths) => call("git_stage", { paths }),
    gitUnstage: (paths) => call("git_unstage", { paths }),
    gitDiscard: (paths) => call("git_discard", { paths }),
    gitCommit: (message) => call("git_commit", { message }),
    gitLog: (query) => call("git_log", { query }),
    gitCommitFiles: (oid) => call("git_commit_files", { oid }),
    gitDiffFile: (path, kind) => call("git_diff_file", { path, kind }),
    gitBranches: () => call("git_branches"),
    gitCreateBranch: (name, checkout) => call("git_create_branch", { name, checkout }),
    gitCheckoutBranch: (name) => call("git_checkout_branch", { name }),
    gitRemotes: () => call("git_remotes"),
    gitAddRemote: (name, url) => call("git_add_remote", { name, url }),
    gitRemoveRemote: (name) => call("git_remove_remote", { name }),
    gitFetch: (remote) => call("git_fetch", { remote }),
    gitPull: (remote) => call("git_pull", { remote }),
    gitPush: (remote, refspec) => call("git_push", { remote, refspec }),
    gitClone: (url, dest) => call("git_clone", { url, dest }),
    gitCancel: () => call("git_cancel"),
    gitCredentialStore: (host, username, secret) =>
      call("git_credential_store", { host, username, secret }),
    gitCredentialForget: (host) => call("git_credential_forget", { host }),
    claudeStartSession: (resume, permissionMode, model) =>
      call("claude_start_session", { resume, permissionMode, model }),
    claudeSend: (sessionId, text, attachDiagnostics) =>
      call("claude_send", { sessionId, text, attachDiagnostics }),
    claudePermission: (sessionId, requestId, decision) =>
      call("claude_permission", { sessionId, requestId, decision }),
    claudeInterrupt: (sessionId) => call("claude_interrupt", { sessionId }),
    claudeStop: (sessionId) => call("claude_stop", { sessionId }),
    claudeSessions: () => call("claude_sessions"),
    terminalOpen: (cols, rows) => call("terminal_open", { cols, rows }),
    terminalWrite: (id, data) => call("terminal_write", { id, data }),
    terminalResize: (id, cols, rows) => call("terminal_resize", { id, cols, rows }),
    terminalClose: (id) => call("terminal_close", { id }),
  };
}
