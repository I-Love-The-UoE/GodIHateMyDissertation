import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import type { Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { tags } from "@lezer/highlight";

const editorTheme = EditorView.theme(
  {
    "&": { backgroundColor: "var(--bg-0)", color: "var(--fg-0)", height: "100%" },
    ".cm-content": { fontFamily: "var(--font-mono)", fontSize: "13.5px", padding: "8px 0" },
    ".cm-scroller": { fontFamily: "var(--font-mono)", lineHeight: "1.55" },
    ".cm-gutters": {
      backgroundColor: "var(--bg-1)",
      color: "var(--fg-3)",
      border: "none",
      borderRight: "1px solid var(--border)",
    },
    ".cm-activeLineGutter": { backgroundColor: "var(--bg-2)", color: "var(--fg-1)" },
    ".cm-activeLine": { backgroundColor: "#ffffff08" },
    ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
      backgroundColor: "var(--selection) !important",
    },
    ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent-hover)" },
    ".cm-matchingBracket, &.cm-focused .cm-matchingBracket": {
      backgroundColor: "#3b82f633",
      outline: "1px solid #3b82f688",
    },
    ".cm-tooltip": {
      backgroundColor: "var(--bg-2)",
      border: "1px solid var(--border)",
      color: "var(--fg-0)",
    },
    ".cm-tooltip-autocomplete ul li[aria-selected]": {
      backgroundColor: "var(--accent-muted)",
      color: "var(--fg-0)",
    },
    ".cm-panels": { backgroundColor: "var(--bg-1)", color: "var(--fg-0)" },
    ".cm-panels.cm-panels-bottom": { borderTop: "1px solid var(--border)" },
    ".cm-searchMatch": { backgroundColor: "#f59e0b44" },
    ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "#f59e0b88" },
    ".cm-lintRange-error": { backgroundImage: "none", textDecoration: "underline wavy var(--err)" },
    ".cm-lintRange-warning": {
      backgroundImage: "none",
      textDecoration: "underline wavy var(--warn)",
    },
    ".cm-lintRange-info": {
      backgroundImage: "none",
      textDecoration: "underline dotted var(--info)",
    },
    ".cm-ySelectionInfo": { fontFamily: "var(--font-ui)", fontSize: "11px" },
  },
  { dark: true },
);

const highlight = HighlightStyle.define([
  { tag: tags.tagName, color: "#60a5fa" },
  { tag: tags.keyword, color: "#c084fc" },
  { tag: tags.atom, color: "#2dd4bf" },
  { tag: tags.bracket, color: "#b7c3d8" },
  { tag: tags.comment, color: "#55637d", fontStyle: "italic" },
  { tag: tags.number, color: "#fbbf24" },
  { tag: tags.special(tags.variableName), color: "#93c5fd" },
  { tag: tags.string, color: "#86efac" },
  { tag: tags.invalid, color: "#ef4444", textDecoration: "underline wavy" },
]);

export function editorStyling(): Extension {
  return [editorTheme, syntaxHighlighting(highlight)];
}
