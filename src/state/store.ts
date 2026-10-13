import { create } from "zustand";
import type { Backend } from "@/lib/tauri";
import { createClaudeSlice } from "./claude-slice";
import { createCompileSlice } from "./compile-slice";
import { createEditorSlice } from "./editor-slice";
import { createGitSlice } from "./git-slice";
import { createProjectSlice } from "./project-slice";
import { createSettingsSlice } from "./settings-slice";
import type { Store } from "./types";
import { createUiSlice } from "./ui-slice";

export type AppStore = ReturnType<typeof createAppStore>;

export function createAppStore(backend: Backend) {
  return create<Store>()((...args) => ({
    ...createSettingsSlice(backend)(...args),
    ...createProjectSlice(backend)(...args),
    ...createEditorSlice(backend)(...args),
    ...createCompileSlice(backend)(...args),
    ...createGitSlice(backend)(...args),
    ...createClaudeSlice(backend)(...args),
    ...createUiSlice(backend)(...args),
  }));
}
