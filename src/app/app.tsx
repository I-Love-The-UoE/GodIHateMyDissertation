import type { ReactNode } from "react";
import { useEffect } from "react";
import { ToastList } from "@/components/toast-list";
import { SettingsDialog } from "@/features/settings/settings-dialog";
import { StartScreen } from "@/features/start/start-screen";
import { Workspace } from "@/features/workspace/workspace";
import { spawn } from "@/lib/async";
import {
  onClaudeEvent,
  onCompileEvent,
  onFsChanged,
  onGitChanged,
  onGitProgress,
  onTexInstall,
} from "@/lib/tauri";
import { isLocalProvider } from "@/lib/yjs";
import { useServices, useStore } from "./services";

function useBackendEvents(): void {
  const { store, documents } = useServices();

  useEffect(() => {
    const unsubscribes = [
      onFsChanged((events) => {
        const state = store.getState();
        spawn(state.applyFsEvents(events));

        for (const event of events) {
          const path =
            event.kind === "rescan" ? undefined : event.kind === "renamed" ? event.to : event.path;

          if (path === undefined) {
            continue;
          }

          const document = documents.byPath(path);

          if (
            document !== undefined &&
            isLocalProvider(document.provider) &&
            event.kind !== "removed"
          ) {
            spawn(document.provider.reloadFromDisk(false));
          }
        }

        if (state.repo !== null && state.repo !== undefined) {
          spawn(state.refreshGit());
        }
      }),
      onCompileEvent((event) => store.getState().handleCompileEvent(event)),
      onGitProgress((payload) => store.getState().handleGitProgress(payload)),
      onGitChanged(() => spawn(store.getState().refreshGit())),
      onClaudeEvent((event) => store.getState().handleClaudeEvent(event)),
      onTexInstall((event) => store.getState().handleTexInstall(event)),
    ];

    return () => {
      for (const unsubscribe of unsubscribes) {
        unsubscribe();
      }
    };
  }, [store, documents]);
}

export function App(): ReactNode {
  const { store } = useServices();
  const project = useStore((state) => state.project);
  const toasts = useStore((state) => state.toasts);
  const dismissToast = useStore((state) => state.dismissToast);
  useBackendEvents();

  useEffect(() => {
    const state = store.getState();
    spawn(Promise.all([state.loadSettings(), state.loadRecent()]).then(() => undefined));
  }, [store]);

  return (
    <div className="app">
      <div className="app-body">{project === undefined ? <StartScreen /> : <Workspace />}</div>
      <SettingsDialog />
      <ToastList toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
}
