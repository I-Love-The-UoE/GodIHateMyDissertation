export type { EditorState } from "@codemirror/state";
export type { IndexSource } from "./completion";
export type {
  DocumentStateOptions,
  EditorCommands,
  EditorDiagnostic,
  EditorHandle,
} from "./editor";
export { createDocumentState, createEditor } from "./editor";
export type { MergeHandle } from "./merge-view";
export { createMergeView } from "./merge-view";
