import {
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
} from "@codemirror/autocomplete";
import { defaultKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, indentOnInput } from "@codemirror/language";
import type { Diagnostic as LintDiagnostic } from "@codemirror/lint";
import { linter, lintGutter, setDiagnostics } from "@codemirror/lint";
import { highlightSelectionMatches, openSearchPanel, searchKeymap } from "@codemirror/search";
import type { Extension } from "@codemirror/state";
import { Compartment, EditorState } from "@codemirror/state";
import {
  drawSelection,
  dropCursor,
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  rectangularSelection,
} from "@codemirror/view";
import { yCollab, yUndoManagerKeymap } from "y-codemirror.next";
import type { Awareness } from "y-protocols/awareness";
import type * as Y from "yjs";
import type { IndexSource } from "./completion";
import { latexCompletions } from "./completion";
import { latexLanguage } from "./latex-language";
import { editorStyling } from "./theme";

export type EditorDiagnostic = {
  readonly line: number;
  readonly severity: "error" | "warning" | "info" | "hint";
  readonly message: string;
};

export type EditorCommands = {
  readonly save: () => void;
  readonly compile: () => void;
  readonly forwardSearch: () => void;
};

export type EditorHandle = {
  readonly view: EditorView;
  readonly setDocument: (state: EditorState) => void;
  readonly setDiagnostics: (diagnostics: readonly EditorDiagnostic[]) => void;
  readonly goTo: (line: number, column: number) => void;
  readonly cursor: () => { readonly line: number; readonly column: number };
  readonly openSearch: () => void;
  readonly focus: () => void;
  readonly destroy: () => void;
};

export type DocumentStateOptions = {
  readonly text: Y.Text;
  readonly awareness: Awareness;
  readonly undo: Y.UndoManager;
  readonly index: IndexSource;
  readonly commands: EditorCommands;
};

const diagnosticsCompartment = new Compartment();

function toLintDiagnostics(
  state: EditorState,
  diagnostics: readonly EditorDiagnostic[],
): LintDiagnostic[] {
  const result: LintDiagnostic[] = [];

  for (const diagnostic of diagnostics) {
    if (diagnostic.line < 1 || diagnostic.line > state.doc.lines) {
      continue;
    }

    const line = state.doc.line(diagnostic.line);
    result.push({
      from: line.from,
      to: line.to,
      severity: diagnostic.severity,
      message: diagnostic.message,
    });
  }

  return result;
}

export function createDocumentState(options: DocumentStateOptions): EditorState {
  const { text, awareness, undo, index, commands } = options;
  const extensions: Extension[] = [
    lineNumbers(),
    highlightActiveLineGutter(),
    highlightActiveLine(),
    drawSelection(),
    dropCursor(),
    rectangularSelection(),
    indentOnInput(),
    bracketMatching(),
    closeBrackets(),
    highlightSelectionMatches(),
    autocompletion({ override: [latexCompletions(index)], activateOnTyping: true }),
    linter(() => [], { delay: 0 }),
    lintGutter(),
    diagnosticsCompartment.of([]),
    EditorView.lineWrapping,
    latexLanguage(),
    editorStyling(),
    yCollab(text, awareness, { undoManager: undo }),
    keymap.of([
      {
        key: "Mod-s",
        run: () => {
          commands.save();

          return true;
        },
      },
      {
        key: "Mod-Enter",
        run: () => {
          commands.compile();

          return true;
        },
      },
      {
        key: "Mod-Alt-j",
        run: () => {
          commands.forwardSearch();

          return true;
        },
      },
      indentWithTab,
      ...yUndoManagerKeymap,
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...searchKeymap,
      ...completionKeymap,
    ]),
  ];

  return EditorState.create({ doc: text.toString(), extensions });
}

export function createEditor(parent: HTMLElement): EditorHandle {
  const view = new EditorView({
    parent,
    state: EditorState.create({ doc: "", extensions: [editorStyling()] }),
  });

  return {
    view,
    setDocument: (state) => view.setState(state),
    setDiagnostics: (diagnostics) => {
      view.dispatch(setDiagnostics(view.state, toLintDiagnostics(view.state, diagnostics)));
    },
    goTo: (line, column) => {
      const clampedLine = Math.min(Math.max(line, 1), view.state.doc.lines);
      const target = view.state.doc.line(clampedLine);
      const position = Math.min(target.from + Math.max(column - 1, 0), target.to);
      view.dispatch({
        selection: { anchor: position },
        effects: EditorView.scrollIntoView(position, { y: "center" }),
      });
      view.focus();
    },
    cursor: () => {
      const head = view.state.selection.main.head;
      const line = view.state.doc.lineAt(head);

      return { line: line.number, column: head - line.from + 1 };
    },
    openSearch: () => {
      openSearchPanel(view);
    },
    focus: () => view.focus(),
    destroy: () => view.destroy(),
  };
}
