import { isErr } from "@/lib/result";
import type { AppSettings, ClaudeProbe, TexToolchain, WireError } from "@/lib/tauri";
import { describeError } from "@/lib/tauri";
import type { Slice } from "./types";

export type Probe<T> =
  | { readonly state: "unknown" }
  | { readonly state: "ok"; readonly value: T }
  | { readonly state: "error"; readonly error: WireError };

export type SettingsSlice = {
  readonly settings: AppSettings | undefined;
  readonly texProbe: Probe<TexToolchain>;
  readonly claudeProbe: Probe<ClaudeProbe>;
  readonly loadSettings: () => Promise<void>;
  readonly saveSettings: (settings: AppSettings) => Promise<boolean>;
  readonly probeTools: () => Promise<void>;
};

export const createSettingsSlice: Slice<SettingsSlice> = (backend) => (set, get) => ({
  settings: undefined,
  texProbe: { state: "unknown" },
  claudeProbe: { state: "unknown" },
  loadSettings: async () => {
    const result = await backend.settingsGet();

    if (isErr(result)) {
      get().toast("error", `Settings could not be loaded: ${describeError(result.error)}`);

      return;
    }

    set({ settings: result.value });
    await get().probeTools();
  },
  saveSettings: async (settings) => {
    const result = await backend.settingsSet(settings);

    if (isErr(result)) {
      get().toast("error", `Settings were rejected: ${describeError(result.error)}`);

      return false;
    }

    set({ settings: result.value });
    await get().probeTools();

    return true;
  },
  probeTools: async () => {
    const settings = get().settings;
    const [tex, claude] = await Promise.all([
      backend.texProbeToolchain(settings?.texBinDir ?? null),
      backend.claudeProbeBinary(settings?.claudePath ?? null),
    ]);
    set({
      texProbe: isErr(tex)
        ? { state: "error", error: tex.error }
        : { state: "ok", value: tex.value },
      claudeProbe: isErr(claude)
        ? { state: "error", error: claude.error }
        : { state: "ok", value: claude.value },
    });
  },
});
