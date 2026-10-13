import type { ReactNode } from "react";
import { useServices, useStore } from "@/app/services";
import { Button } from "@/components/button";
import { Tabs } from "@/components/tabs";
import { spawn } from "@/lib/async";
import { isLocalProvider } from "@/lib/yjs";
import { useDocuments } from "./use-documents";

export type EditorPaneProps = {
  readonly onForwardSearch: () => void;
  readonly onCompile: () => void;
  readonly onSave: () => void;
};

export function EditorPane(props: EditorPaneProps): ReactNode {
  const { onForwardSearch, onCompile, onSave } = props;
  const { documents } = useServices();
  const tabs = useStore((state) => state.tabs);
  const activeTab = useStore((state) => state.activeTab);
  const syncStatus = useStore((state) => state.syncStatus);
  const setActiveTab = useStore((state) => state.setActiveTab);
  const requestCloseTab = useStore((state) => state.requestCloseTab);
  const externalChanges = useStore((state) => state.externalChanges);
  const clearExternalChange = useStore((state) => state.clearExternalChange);
  const { hostRef } = useDocuments({
    save: onSave,
    compile: onCompile,
    forwardSearch: onForwardSearch,
  });
  const change = activeTab === undefined ? undefined : externalChanges[activeTab];

  const dismiss = (): void => {
    if (activeTab !== undefined) {
      clearExternalChange(activeTab);
    }
  };

  const discardAndReload = (): void => {
    const document = activeTab === undefined ? undefined : documents.byPath(activeTab);

    if (document !== undefined && isLocalProvider(document.provider)) {
      spawn(document.provider.reloadFromDisk(true));
    }

    dismiss();
  };

  return (
    <div className="panel">
      <Tabs
        items={tabs.map((path) => ({
          id: path,
          label: path.split("/").at(-1) ?? path,
          dirty: syncStatus[path] === "dirty" || syncStatus[path] === "saving",
        }))}
        active={activeTab}
        onSelect={setActiveTab}
        onClose={requestCloseTab}
      />
      {change === undefined ? null : (
        <div className="panel-header" style={{ background: "var(--bg-2)", fontWeight: 400 }}>
          <span>
            {change === "kept"
              ? "This file changed on disk; your editor version was kept and overwrites it."
              : "This file was reloaded from disk."}
          </span>
          <span className="spacer" />
          {change === "kept" ? (
            <Button small onClick={discardAndReload}>
              Discard my edits and reload
            </Button>
          ) : null}
          <Button small variant="ghost" onClick={dismiss}>
            Dismiss
          </Button>
        </div>
      )}
      <div
        className="panel-body editor-host"
        ref={hostRef}
        style={{ display: activeTab === undefined ? "none" : "block" }}
      />
      {activeTab === undefined ? (
        <div className="empty">Open a file from the tree to start editing.</div>
      ) : null}
    </div>
  );
}
