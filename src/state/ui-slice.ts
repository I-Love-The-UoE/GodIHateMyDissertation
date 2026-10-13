import type { Slice, Toast } from "./types";

export type RightTab = "pdf" | "claude";
export type BottomTab = "problems" | "log" | "git" | "terminal";

export type CredentialPrompt = { readonly host: string; readonly retry: () => void };

export type UiSlice = {
  readonly rightTab: RightTab;
  readonly bottomTab: BottomTab;
  readonly bottomOpen: boolean;
  readonly toasts: readonly Toast[];
  readonly settingsOpen: boolean;
  readonly credentialPrompt: CredentialPrompt | undefined;
  readonly setRightTab: (tab: RightTab) => void;
  readonly setBottomTab: (tab: BottomTab) => void;
  readonly toggleBottom: () => void;
  readonly toast: (level: Toast["level"], text: string) => void;
  readonly dismissToast: (id: number) => void;
  readonly setSettingsOpen: (open: boolean) => void;
  readonly setCredentialPrompt: (prompt: CredentialPrompt | undefined) => void;
};

let nextToast = 1;

export const createUiSlice: Slice<UiSlice> = () => (set) => ({
  rightTab: "pdf",
  bottomTab: "problems",
  bottomOpen: true,
  toasts: [],
  settingsOpen: false,
  credentialPrompt: undefined,
  setRightTab: (tab) => set({ rightTab: tab }),
  setBottomTab: (tab) => set({ bottomTab: tab, bottomOpen: true }),
  toggleBottom: () => set((state) => ({ bottomOpen: !state.bottomOpen })),
  toast: (level, text) => {
    const id = nextToast;
    nextToast += 1;
    set((state) => ({ toasts: [...state.toasts, { id, level, text }] }));
  },
  dismissToast: (id) =>
    set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) })),
  setSettingsOpen: (open) => set({ settingsOpen: open }),
  setCredentialPrompt: (prompt) => set({ credentialPrompt: prompt }),
});
