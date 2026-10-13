import { spawn } from "@/lib/async";
import type { ProjectRecord } from "@/lib/tauri";
import type { ExternalChange, SyncStatus } from "@/lib/yjs";
import type { Slice } from "./types";

export type GoTo = {
  readonly path: string;
  readonly line: number;
  readonly column: number;
  readonly nonce: number;
};

export type EditorSlice = {
  readonly tabs: readonly string[];
  readonly activeTab: string | undefined;
  readonly syncStatus: Readonly<Record<string, SyncStatus>>;
  readonly cursor: { readonly line: number; readonly column: number };
  readonly goTo: GoTo | undefined;
  readonly externalChanges: Readonly<Record<string, ExternalChange>>;
  readonly tabCloseRequests: readonly string[];
  readonly openTab: (path: string) => void;
  readonly requestCloseTab: (path: string) => void;
  readonly closeAllTabs: () => Promise<void>;
  readonly tabClosed: (path: string) => void;
  readonly setActiveTab: (path: string) => void;
  readonly setSyncStatus: (path: string, status: SyncStatus) => void;
  readonly setCursor: (line: number, column: number) => void;
  readonly requestGoTo: (path: string, line: number, column: number) => void;
  readonly noteExternalChange: (path: string, change: ExternalChange) => void;
  readonly clearExternalChange: (path: string) => void;
  readonly resetEditor: (record: ProjectRecord) => void;
};

let goToNonce = 0;

export const createEditorSlice: Slice<EditorSlice> = () => (set, get) => ({
  tabs: [],
  activeTab: undefined,
  syncStatus: {},
  cursor: { line: 1, column: 1 },
  goTo: undefined,
  externalChanges: {},
  tabCloseRequests: [],
  openTab: (path) => {
    const tabs = get().tabs.includes(path) ? get().tabs : [...get().tabs, path];
    set({ tabs, activeTab: path });
    spawn(get().persistTabs(tabs, path));
  },
  requestCloseTab: (path) => {
    set((state) => ({ tabCloseRequests: [...state.tabCloseRequests, path] }));
  },
  closeAllTabs: async () => {
    const tabs = get().tabs;
    set({ tabCloseRequests: [...tabs] });
    await Promise.resolve();
  },
  tabClosed: (path) => {
    const tabs = get().tabs.filter((tab) => tab !== path);
    const activeTab = get().activeTab === path ? tabs.at(-1) : get().activeTab;
    const syncStatus = { ...get().syncStatus };
    delete syncStatus[path];
    set((state) => ({
      tabs,
      activeTab,
      syncStatus,
      tabCloseRequests: state.tabCloseRequests.filter((request) => request !== path),
    }));
    spawn(get().persistTabs(tabs, activeTab ?? null));
  },
  setActiveTab: (path) => {
    set({ activeTab: path });
    spawn(get().persistTabs(get().tabs, path));
  },
  setSyncStatus: (path, status) =>
    set((state) => ({ syncStatus: { ...state.syncStatus, [path]: status } })),
  setCursor: (line, column) => set({ cursor: { line, column } }),
  requestGoTo: (path, line, column) => {
    goToNonce += 1;
    set({ goTo: { path, line, column, nonce: goToNonce } });
    get().openTab(path);
  },
  noteExternalChange: (path, change) =>
    set((state) => ({ externalChanges: { ...state.externalChanges, [path]: change } })),
  clearExternalChange: (path) =>
    set((state) => {
      const externalChanges = { ...state.externalChanges };
      delete externalChanges[path];

      return { externalChanges };
    }),
  resetEditor: (record) => {
    set({
      tabs: [...record.openTabs],
      activeTab: record.activeTab ?? record.openTabs.at(-1),
      syncStatus: {},
      goTo: undefined,
      externalChanges: {},
      tabCloseRequests: [],
    });
  },
});
