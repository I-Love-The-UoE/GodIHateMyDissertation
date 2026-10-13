import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { useStore } from "@/app/services";
import { Button } from "@/components/button";
import { Dialog } from "@/components/dialog";
import { spawn } from "@/lib/async";
import type { AppSettings, Engine } from "@/lib/tauri";
import { describeError } from "@/lib/tauri";

const ENGINES: readonly { readonly value: Engine; readonly label: string }[] = [
  { value: "pdf_latex", label: "pdfLaTeX" },
  { value: "xe_latex", label: "XeLaTeX" },
  { value: "lua_latex", label: "LuaLaTeX" },
];

export function SettingsDialog(): ReactNode {
  const open = useStore((state) => state.settingsOpen);
  const setOpen = useStore((state) => state.setSettingsOpen);
  const settings = useStore((state) => state.settings);
  const saveSettings = useStore((state) => state.saveSettings);
  const texProbe = useStore((state) => state.texProbe);
  const claudeProbe = useStore((state) => state.claudeProbe);
  const [draft, setDraft] = useState<AppSettings | undefined>(settings);

  useEffect(() => {
    if (open) {
      setDraft(settings);
    }
  }, [open, settings]);

  if (!open || draft === undefined) {
    return null;
  }

  const update = (patch: Partial<AppSettings>): void => setDraft({ ...draft, ...patch });

  const save = async (): Promise<void> => {
    if (await saveSettings(draft)) {
      setOpen(false);
    }
  };

  return (
    <Dialog
      open
      title="Settings"
      onClose={() => setOpen(false)}
      footer={
        <>
          <Button onClick={() => setOpen(false)}>Cancel</Button>
          <Button variant="primary" onClick={() => spawn(save())}>
            Save
          </Button>
        </>
      }
    >
      <div className="settings-grid">
        <div className="field">
          <label htmlFor="tex-bin">TeX binary directory (blank = auto-detect)</label>
          <input
            id="tex-bin"
            value={draft.texBinDir ?? ""}
            onChange={(event) =>
              update({ texBinDir: event.target.value.trim() === "" ? null : event.target.value })
            }
            placeholder="/usr/bin or /Library/TeX/texbin"
          />
          <span className="hint">
            {texProbe.state === "ok"
              ? `Detected: ${texProbe.value.binDir}`
              : texProbe.state === "error"
                ? describeError(texProbe.error)
                : ""}
          </span>
        </div>
        <div className="field">
          <label htmlFor="claude-path">Claude Code CLI path (blank = auto-detect)</label>
          <input
            id="claude-path"
            value={draft.claudePath ?? ""}
            onChange={(event) =>
              update({ claudePath: event.target.value.trim() === "" ? null : event.target.value })
            }
            placeholder="~/.local/bin/claude"
          />
          <span className="hint">
            {claudeProbe.state === "ok"
              ? `Detected: ${claudeProbe.value.path} (${claudeProbe.value.version})`
              : claudeProbe.state === "error"
                ? describeError(claudeProbe.error)
                : ""}
          </span>
        </div>
        <div className="field">
          <label htmlFor="engine">Engine</label>
          <select
            id="engine"
            value={draft.engine}
            onChange={(event) => update({ engine: event.target.value as Engine })}
          >
            {ENGINES.map((engine) => (
              <option key={engine.value} value={engine.value}>
                {engine.label}
              </option>
            ))}
          </select>
        </div>
        <label className="field" style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
          <input
            type="checkbox"
            checked={draft.shellEscape}
            onChange={(event) => update({ shellEscape: event.target.checked })}
          />
          Enable shell escape (needed by minted and similar packages)
        </label>
        <label className="field" style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
          <input
            type="checkbox"
            checked={draft.autocompile}
            onChange={(event) => update({ autocompile: event.target.checked })}
          />
          Compile automatically while typing
        </label>
        <div className="field">
          <label htmlFor="debounce">Autocompile delay (ms)</label>
          <input
            id="debounce"
            type="number"
            min={100}
            value={draft.autocompileDebounceMs}
            onChange={(event) => update({ autocompileDebounceMs: Number(event.target.value) })}
          />
        </div>
        <div className="field">
          <label htmlFor="user-name">Your name (shown to collaborators later)</label>
          <input
            id="user-name"
            value={draft.userName}
            onChange={(event) => update({ userName: event.target.value })}
          />
        </div>
      </div>
    </Dialog>
  );
}
