import { StreamLanguage } from "@codemirror/language";
import { stex } from "@codemirror/legacy-modes/mode/stex";
import type { Extension } from "@codemirror/state";

// NOTE: There is no maintained Lezer grammar for LaTeX; the stream mode is isolated here so a
// grammar can replace it in one file.
export function latexLanguage(): Extension {
  return StreamLanguage.define(stex);
}
