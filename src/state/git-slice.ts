import { spawn } from "@/lib/async";
import { isErr } from "@/lib/result";
import type {
  BranchInfo,
  CommitInfo,
  DiffKind,
  FileDiff,
  GitProgressPayload,
  RemoteInfo,
  RepoStatus,
  WireError,
} from "@/lib/tauri";
import { describeError } from "@/lib/tauri";
import type { Slice } from "./types";

export type GitSlice = {
  readonly repo: RepoStatus | null | undefined;
  readonly branches: readonly BranchInfo[];
  readonly remotes: readonly RemoteInfo[];
  readonly history: readonly CommitInfo[];
  readonly diff: FileDiff | undefined;
  readonly gitBusy: string | undefined;
  readonly gitProgress: GitProgressPayload | undefined;
  readonly refreshGit: () => Promise<void>;
  readonly gitInit: () => Promise<void>;
  readonly stage: (paths: readonly string[]) => Promise<void>;
  readonly unstage: (paths: readonly string[]) => Promise<void>;
  readonly discard: (paths: readonly string[]) => Promise<void>;
  readonly commit: (message: string) => Promise<boolean>;
  readonly loadHistory: () => Promise<void>;
  readonly showDiff: (path: string, kind: DiffKind) => Promise<void>;
  readonly clearDiff: () => void;
  readonly createBranch: (name: string, checkout: boolean) => Promise<void>;
  readonly checkoutBranch: (name: string) => Promise<void>;
  readonly addRemote: (name: string, url: string) => Promise<void>;
  readonly removeRemote: (name: string) => Promise<void>;
  readonly fetch: (remote: string) => Promise<void>;
  readonly pull: (remote: string) => Promise<void>;
  readonly push: (remote: string) => Promise<void>;
  readonly cancelGit: () => Promise<void>;
  readonly storeCredential: (host: string, username: string, secret: string) => Promise<boolean>;
  readonly handleGitProgress: (payload: GitProgressPayload) => void;
  readonly resetGit: () => void;
};

export const createGitSlice: Slice<GitSlice> = (backend) => (set, get) => {
  const report = (error: WireError): void => {
    get().toast("error", `Git: ${describeError(error)}`);
  };

  const applyStatus = async (call: () => ReturnType<typeof backend.gitStatus>): Promise<void> => {
    const result = await call();

    if (isErr(result)) {
      report(result.error);

      return;
    }

    set({ repo: result.value });
  };

  const network = async (
    operation: string,
    remote: string,
    call: () => Promise<{ readonly ok: true } | { readonly ok: false; readonly error: WireError }>,
  ): Promise<void> => {
    set({ gitBusy: operation, gitProgress: undefined });
    const result = await call();
    set({ gitBusy: undefined, gitProgress: undefined });

    if (!result.ok) {
      if (
        result.error.kind === "credentials_required" &&
        typeof result.error["host"] === "string"
      ) {
        const host = result.error["host"];
        get().setCredentialPrompt({
          host,
          retry: () => spawn(network(operation, remote, call)),
        });

        return;
      }

      report(result.error);

      return;
    }

    get().toast("info", `${operation} finished.`);
    await get().refreshGit();
  };

  return {
    repo: undefined,
    branches: [],
    remotes: [],
    history: [],
    diff: undefined,
    gitBusy: undefined,
    gitProgress: undefined,
    refreshGit: async () => {
      const status = await backend.gitOpenOrNone();

      if (isErr(status)) {
        report(status.error);

        return;
      }

      if (status.value === null) {
        set({ repo: null, branches: [], remotes: [], history: [] });

        return;
      }

      set({ repo: status.value });
      const [branches, remotes] = await Promise.all([backend.gitBranches(), backend.gitRemotes()]);
      set({
        branches: isErr(branches) ? [] : branches.value,
        remotes: isErr(remotes) ? [] : remotes.value,
      });
      await get().loadHistory();
    },
    gitInit: async () => {
      await applyStatus(() => backend.gitInit());
      await get().refreshGit();
    },
    stage: (paths) => applyStatus(() => backend.gitStage(paths)),
    unstage: (paths) => applyStatus(() => backend.gitUnstage(paths)),
    discard: (paths) => applyStatus(() => backend.gitDiscard(paths)),
    commit: async (message) => {
      const result = await backend.gitCommit(message);

      if (isErr(result)) {
        report(result.error);

        return false;
      }

      set({ repo: result.value.status });
      await get().loadHistory();

      return true;
    },
    loadHistory: async () => {
      const result = await backend.gitLog({ rev: null, skip: 0, limit: 100 });

      if (isErr(result)) {
        return;
      }

      set({ history: result.value });
    },
    showDiff: async (path, kind) => {
      const result = await backend.gitDiffFile(path, kind);

      if (isErr(result)) {
        report(result.error);

        return;
      }

      set({ diff: result.value });
    },
    clearDiff: () => set({ diff: undefined }),
    createBranch: async (name, checkout) => {
      const result = await backend.gitCreateBranch(name, checkout);

      if (isErr(result)) {
        report(result.error);

        return;
      }

      await get().refreshGit();
    },
    checkoutBranch: async (name) => {
      await applyStatus(() => backend.gitCheckoutBranch(name));
      await get().refreshGit();
    },
    addRemote: async (name, url) => {
      const result = await backend.gitAddRemote(name, url);

      if (isErr(result)) {
        report(result.error);

        return;
      }

      set({ remotes: result.value });
    },
    removeRemote: async (name) => {
      const result = await backend.gitRemoveRemote(name);

      if (isErr(result)) {
        report(result.error);

        return;
      }

      set({ remotes: result.value });
    },
    fetch: (remote) => network("Fetch", remote, () => backend.gitFetch(remote)),
    pull: (remote) => network("Pull", remote, () => backend.gitPull(remote)),
    push: (remote) => network("Push", remote, () => backend.gitPush(remote, null)),
    cancelGit: async () => {
      await backend.gitCancel();
    },
    storeCredential: async (host, username, secret) => {
      const result = await backend.gitCredentialStore(host, username, secret);

      if (isErr(result)) {
        report(result.error);

        return false;
      }

      return true;
    },
    handleGitProgress: (payload) => set({ gitProgress: payload }),
    resetGit: () =>
      set({
        repo: undefined,
        branches: [],
        remotes: [],
        history: [],
        diff: undefined,
        gitBusy: undefined,
        gitProgress: undefined,
      }),
  };
};
