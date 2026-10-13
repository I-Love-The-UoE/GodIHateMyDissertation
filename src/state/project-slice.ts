import { isErr } from "@/lib/result";
import type {
  FileNode,
  FsEvent,
  MainFileDetection,
  ProjectIndex,
  ProjectRecord,
} from "@/lib/tauri";
import { describeError } from "@/lib/tauri";
import type { Slice } from "./types";

export type OpenProjectState = {
  readonly record: ProjectRecord;
  readonly tree: FileNode;
  readonly mainFileDetection: MainFileDetection;
};

export type ProjectSlice = {
  readonly project: OpenProjectState | undefined;
  readonly recent: readonly ProjectRecord[];
  readonly index: ProjectIndex | undefined;
  readonly lastFsEvents: readonly FsEvent[];
  readonly loadRecent: () => Promise<void>;
  readonly openProject: (root: string) => Promise<boolean>;
  readonly createProject: (root: string) => Promise<boolean>;
  readonly closeProject: () => Promise<void>;
  readonly forgetProject: (root: string) => Promise<void>;
  readonly setMainFile: (path: string) => Promise<void>;
  readonly refreshTree: () => Promise<void>;
  readonly refreshIndex: () => Promise<void>;
  readonly applyFsEvents: (events: readonly FsEvent[]) => Promise<void>;
  readonly createFile: (path: string) => Promise<boolean>;
  readonly createDir: (path: string) => Promise<boolean>;
  readonly renamePath: (from: string, to: string) => Promise<boolean>;
  readonly deletePath: (path: string) => Promise<boolean>;
  readonly movePath: (from: string, toDir: string | null) => Promise<boolean>;
  readonly persistTabs: (openTabs: readonly string[], activeTab: string | null) => Promise<void>;
};

export const createProjectSlice: Slice<ProjectSlice> = (backend) => (set, get) => {
  const applyTree = (tree: FileNode): void => {
    const project = get().project;

    if (project !== undefined) {
      set({ project: { ...project, tree } });
    }
  };

  const treeAction = async (
    call: () => ReturnType<typeof backend.fileCreate>,
  ): Promise<boolean> => {
    const result = await call();

    if (isErr(result)) {
      get().toast("error", describeError(result.error));

      return false;
    }

    applyTree(result.value);
    await get().refreshIndex();

    return true;
  };

  const opened = async (call: () => ReturnType<typeof backend.projectOpen>): Promise<boolean> => {
    const result = await call();

    if (isErr(result)) {
      get().toast("error", `The project could not be opened: ${describeError(result.error)}`);

      return false;
    }

    set({ project: result.value, index: undefined });
    get().resetEditor(result.value.record);
    get().resetCompile();
    get().resetGit();
    get().resetClaude();
    await Promise.all([get().refreshIndex(), get().loadRecent(), get().refreshGit()]);

    return true;
  };

  return {
    project: undefined,
    recent: [],
    index: undefined,
    lastFsEvents: [],
    loadRecent: async () => {
      const result = await backend.projectRecent();

      if (isErr(result)) {
        get().toast("error", describeError(result.error));

        return;
      }

      set({ recent: result.value });
    },
    openProject: (root) => opened(() => backend.projectOpen(root)),
    createProject: (root) => opened(() => backend.projectCreate(root)),
    closeProject: async () => {
      await get().closeAllTabs();
      const result = await backend.projectClose();

      if (isErr(result)) {
        get().toast("error", describeError(result.error));
      }

      set({ project: undefined, index: undefined });
      get().resetCompile();
      get().resetGit();
      get().resetClaude();
      await get().loadRecent();
    },
    forgetProject: async (root) => {
      const result = await backend.projectForget(root);

      if (isErr(result)) {
        get().toast("error", describeError(result.error));

        return;
      }

      set({ recent: result.value });
    },
    setMainFile: async (path) => {
      const result = await backend.projectSetMainFile(path);

      if (isErr(result)) {
        get().toast("error", describeError(result.error));

        return;
      }

      const project = get().project;

      if (project !== undefined) {
        set({ project: { ...project, record: result.value } });
      }
    },
    refreshTree: async () => {
      const result = await backend.projectTree();

      if (isErr(result)) {
        return;
      }

      applyTree(result.value);
    },
    refreshIndex: async () => {
      const result = await backend.projectIndex();

      if (isErr(result)) {
        return;
      }

      set({ index: result.value });
    },
    applyFsEvents: async (events) => {
      set({ lastFsEvents: events });
      await Promise.all([get().refreshTree(), get().refreshIndex()]);
    },
    createFile: (path) => treeAction(() => backend.fileCreate(path)),
    createDir: (path) => treeAction(() => backend.dirCreate(path)),
    renamePath: (from, to) => treeAction(() => backend.pathRename(from, to)),
    deletePath: (path) => treeAction(() => backend.pathDelete(path)),
    movePath: (from, toDir) => treeAction(() => backend.pathMove(from, toDir)),
    persistTabs: async (openTabs, activeTab) => {
      const result = await backend.projectSetTabs(openTabs, activeTab);

      if (isErr(result)) {
        return;
      }

      const project = get().project;

      if (project !== undefined) {
        set({ project: { ...project, record: result.value } });
      }
    },
  };
};
