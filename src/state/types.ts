import type { StateCreator } from "zustand";
import type { Backend } from "@/lib/tauri";
import type { ClaudeSlice } from "./claude-slice";
import type { CompileSlice } from "./compile-slice";
import type { EditorSlice } from "./editor-slice";
import type { GitSlice } from "./git-slice";
import type { ProjectSlice } from "./project-slice";
import type { SettingsSlice } from "./settings-slice";
import type { UiSlice } from "./ui-slice";

export type Store = SettingsSlice &
  ProjectSlice &
  EditorSlice &
  CompileSlice &
  GitSlice &
  ClaudeSlice &
  UiSlice;

export type Slice<T> = (backend: Backend) => StateCreator<Store, [], [], T>;

export type Toast = {
  readonly id: number;
  readonly level: "info" | "warn" | "error";
  readonly text: string;
};
