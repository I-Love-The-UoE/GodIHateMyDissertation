import { MergeView } from "@codemirror/merge";
import { EditorState } from "@codemirror/state";
import { EditorView, lineNumbers } from "@codemirror/view";
import { latexLanguage } from "./latex-language";
import { editorStyling } from "./theme";

export type MergeHandle = { readonly destroy: () => void };

export function createMergeView(
  parent: HTMLElement,
  oldText: string,
  newText: string,
): MergeHandle {
  const extensions = [
    lineNumbers(),
    EditorView.editable.of(false),
    EditorState.readOnly.of(true),
    EditorView.lineWrapping,
    latexLanguage(),
    editorStyling(),
  ];
  const view = new MergeView({
    parent,
    a: { doc: oldText, extensions },
    b: { doc: newText, extensions },
    collapseUnchanged: { margin: 3, minSize: 4 },
    highlightChanges: true,
    gutter: true,
  });

  return { destroy: () => view.destroy() };
}
