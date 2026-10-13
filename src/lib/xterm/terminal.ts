import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { Terminal } from "@xterm/xterm";

export type TerminalView = {
  readonly cols: () => number;
  readonly rows: () => number;
  readonly write: (bytes: Uint8Array) => void;
  readonly writeText: (text: string) => void;
  readonly fit: () => void;
  readonly focus: () => void;
  readonly onInput: (handler: (data: string) => void) => () => void;
  readonly onResize: (handler: (cols: number, rows: number) => void) => () => void;
  readonly destroy: () => void;
};

export function createTerminalView(
  parent: HTMLElement,
  openUrl: (url: string) => void,
): TerminalView {
  const terminal = new Terminal({
    cursorBlink: true,
    fontFamily: '"JetBrains Mono", "Fira Code", "Cascadia Code", ui-monospace, monospace',
    fontSize: 13,
    scrollback: 5000,
    allowProposedApi: false,
    theme: {
      background: "#0b1220",
      foreground: "#e6edf7",
      cursor: "#60a5fa",
      selectionBackground: "#1f3b73",
      black: "#101a2d",
      blue: "#3b82f6",
      brightBlue: "#60a5fa",
      cyan: "#38bdf8",
      green: "#22c55e",
      magenta: "#c084fc",
      red: "#ef4444",
      yellow: "#f59e0b",
      white: "#b7c3d8",
      brightBlack: "#55637d",
      brightWhite: "#ffffff",
    },
  });
  const fit = new FitAddon();
  terminal.loadAddon(fit);
  terminal.loadAddon(new WebLinksAddon((_event, url) => openUrl(url)));
  terminal.open(parent);

  return {
    cols: () => terminal.cols,
    rows: () => terminal.rows,
    write: (bytes) => terminal.write(bytes),
    writeText: (text) => terminal.write(text),
    fit: () => fit.fit(),
    focus: () => terminal.focus(),
    onInput: (handler) => {
      const disposable = terminal.onData(handler);

      return () => disposable.dispose();
    },
    onResize: (handler) => {
      const disposable = terminal.onResize(({ cols, rows }) => handler(cols, rows));

      return () => disposable.dispose();
    },
    destroy: () => terminal.dispose(),
  };
}

export function decodeBase64(data: string): Uint8Array {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);

  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }

  return bytes;
}
