import { spawn } from "@/lib/async";
import { isErr } from "@/lib/result";
import type {
  AppSettings,
  ClaudeProbe,
  GitIdentity,
  ManagedTexStatus,
  TexInstallEvent,
  TexToolchain,
  WireError,
} from "@/lib/tauri";
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
  readonly managedTex: ManagedTexStatus | undefined;
  readonly texInstall: TexInstallEvent | undefined;
  readonly gitIdentity: GitIdentity | null | undefined;
  readonly installTex: () => Promise<void>;
  readonly handleTexInstall: (event: TexInstallEvent) => void;
  readonly loadSettings: () => Promise<void>;
  readonly saveSettings: (settings: AppSettings) => Promise<boolean>;
  readonly probeTools: () => Promise<void>;
};

export const createSettingsSlice: Slice<SettingsSlice> = (backend) => (set, get) => ({
  settings: undefined,
  texProbe: { state: "unknown" },
  claudeProbe: { state: "unknown" },
  managedTex: undefined,
  texInstall: undefined,
  gitIdentity: undefined,
  installTex: async () => {
    set({ texInstall: { phase: "downloading", received: 0, total: null } });
    const result = await backend.texInstallManaged();

    if (isErr(result)) {
      set({ texInstall: { phase: "failed", error: describeError(result.error) } });
    }
  },
  handleTexInstall: (event) => {
    set({ texInstall: event });

    if (event.phase === "finished") {
      get().toast("info", "TeX Live is installed.");
      spawn(get().probeTools());
    }

    if (event.phase === "packages_installed") {
      get().toast("info", `Installed TeX packages: ${event.names.join(", ")}`);
    }

    if (event.phase === "failed") {
      get().toast("error", `TeX install failed: ${event.error}`);
    }
  },
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
