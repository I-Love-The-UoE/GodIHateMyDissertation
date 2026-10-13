import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import { useServices, useStore } from "@/app/services";
import { Button } from "@/components/button";
import { Split } from "@/components/split";
import { StatusBar } from "@/components/status-bar";
import { Tabs } from "@/components/tabs";
import { ClaudePanel } from "@/features/claude/claude-panel";
import { EditorPane } from "@/features/editor/editor-pane";
import { FilePanel } from "@/features/files/file-panel";
import { CredentialDialog } from "@/features/git/credential-dialog";
import { GitPanel } from "@/features/git/git-panel";
import { PdfViewer } from "@/features/preview/pdf-viewer";
import { LogPanel } from "@/features/problems/log-panel";
import { ProblemsPanel } from "@/features/problems/problems-panel";
import { TerminalPanel } from "@/features/terminal/terminal-panel";
import { debounced, spawn } from "@/lib/async";
import type { ForwardHit } from "@/lib/tauri";

function useCompileOrchestration(): {
  readonly save: () => void;
  readonly compileNow: () => void;
  readonly forwardSearch: () => void;
  readonly forwardHits: readonly ForwardHit[] | undefined;
  readonly forwardNonce: number;
} {
  const { documents, backend, store } = useServices();
  const settings = useStore((state) => state.settings);
  const syncStatus = useStore((state) => state.syncStatus);
  const [forwardHits, setForwardHits] = useState<readonly ForwardHit[] | undefined>(undefined);
  const [forwardNonce, setForwardNonce] = useState(0);
  const autocompile = useRef(
    debounced(settings?.autocompileDebounceMs ?? 1200, () =>
      spawn(store.getState().compile("queue")),
    ),
  );

  useEffect(() => {
    autocompile.current.cancel();
    autocompile.current = debounced(settings?.autocompileDebounceMs ?? 1200, () =>
      spawn(store.getState().compile("queue")),
    );
  }, [settings?.autocompileDebounceMs, store]);

  useEffect(() => {
    const dirty = Object.values(syncStatus).some(
      (status) => status === "dirty" || status === "saving",
    );

    if (
      dirty &&
      settings?.autocompile === true &&
      store.getState().project?.record.mainFile !== null
    ) {
      autocompile.current.call();
    }
  }, [syncStatus, settings?.autocompile, store]);

  const flushAll = async (): Promise<void> => {
    await Promise.all(documents.all().map((document) => document.provider.flush()));
  };

  const save = (): void => {
    spawn(flushAll());
  };

  const compileNow = (): void => {
    autocompile.current.cancel();
    spawn(flushAll().then(() => store.getState().compile("preempt")));
  };

  const forwardSearch = (): void => {
    const state = store.getState();
    const file = state.activeTab;

    if (file === undefined) {
      return;
    }

    spawn(
      backend.texSynctexForward(file, state.cursor.line, state.cursor.column).then((result) => {
        if (!result.ok) {
          state.toast("warn", `SyncTeX: ${result.error.kind}`);

          return;
        }

        if (result.value.length === 0) {
          state.toast("warn", "SyncTeX found no PDF position for this line.");

          return;
        }

        state.setRightTab("pdf");
        setForwardHits(result.value);
        setForwardNonce((nonce) => nonce + 1);
      }),
    );
  };

  return { save, compileNow, forwardSearch, forwardHits, forwardNonce };
}

function compileDot(state: string): string {
  switch (state) {
    case "running":
      return "dot-info";
    case "success":
      return "dot-ok";
    case "errors":
      return "dot-warn";
    case "failed":
      return "dot-err";
    default:
      return "dot-idle";
  }
}

export function Workspace(): ReactNode {
  const { store } = useServices();
  const project = useStore((state) => state.project);
  const rightTab = useStore((state) => state.rightTab);
  const bottomTab = useStore((state) => state.bottomTab);
  const bottomOpen = useStore((state) => state.bottomOpen);
  const setRightTab = useStore((state) => state.setRightTab);
  const setBottomTab = useStore((state) => state.setBottomTab);
  const toggleBottom = useStore((state) => state.toggleBottom);
  const compileState = useStore((state) => state.compileState);
  const diagnostics = useStore((state) => state.diagnostics);
  const repo = useStore((state) => state.repo);
  const cursor = useStore((state) => state.cursor);
  const texProbe = useStore((state) => state.texProbe);
  const setSettingsOpen = useStore((state) => state.setSettingsOpen);
  const closeProject = useStore((state) => state.closeProject);
  const cancelCompile = useStore((state) => state.cancelCompile);
  const { save, compileNow, forwardSearch, forwardHits, forwardNonce } = useCompileOrchestration();

  useEffect(() => {
    const state = store.getState();
    spawn(
      state.loadLatestOutcome().then(() => {
        const current = store.getState();

        if (
          current.pdf === undefined &&
          current.project !== undefined &&
          current.project.record.mainFile !== null
        ) {
          return current.compile("queue");
        }

        return undefined;
      }),
    );
  }, [store]);

  useEffect(() => {
    const handler = (event: KeyboardEvent): void => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "b" && !event.shiftKey) {
        event.preventDefault();
        toggleBottom();
      }
    };
    window.addEventListener("keydown", handler);

    return () => window.removeEventListener("keydown", handler);
  }, [toggleBottom]);

  if (project === undefined) {
    return null;
  }

  const errors = diagnostics.filter((diagnostic) => diagnostic.severity === "error").length;
  const warnings = diagnostics.filter((diagnostic) => diagnostic.severity === "warning").length;
  const centre = (
    <Split
      id="centre"
      direction="vertical"
      panes={[
        {
          id: "editor",
          defaultSize: bottomOpen ? 70 : 100,
          content: (
            <EditorPane onSave={save} onCompile={compileNow} onForwardSearch={forwardSearch} />
          ),
        },
        ...(bottomOpen
          ? [
              {
                id: "bottom",
                defaultSize: 30,
                minSize: 12,
                content: (
                  <div className="panel">
                    <Tabs
                      items={[
                        {
                          id: "problems",
                          label: `Problems${errors + warnings > 0 ? ` (${errors + warnings})` : ""}`,
                        },
                        { id: "log", label: "Log" },
                        { id: "git", label: "Git" },
                        { id: "terminal", label: "Terminal" },
                      ]}
                      active={bottomTab}
                      onSelect={(id) => setBottomTab(id as typeof bottomTab)}
                    />
                    <div
                      className="panel-body"
                      style={{ display: bottomTab === "terminal" ? "none" : "block" }}
                    >
                      {bottomTab === "problems" ? (
                        <ProblemsPanel />
                      ) : bottomTab === "log" ? (
                        <LogPanel />
                      ) : bottomTab === "git" ? (
                        <GitPanel />
                      ) : null}
                    </div>
                    <div
                      className="panel-body"
                      style={{ display: bottomTab === "terminal" ? "block" : "none" }}
                    >
                      <TerminalPanel visible={bottomTab === "terminal"} />
                    </div>
                  </div>
                ),
              },
            ]
          : []),
      ]}
    />
  );
  const right = (
    <div className="panel">
      <Tabs
        items={[
          { id: "pdf", label: "PDF" },
          { id: "claude", label: "Claude" },
        ]}
        active={rightTab}
        onSelect={(id) => setRightTab(id as typeof rightTab)}
      />
      <div className="panel-body" style={{ display: rightTab === "pdf" ? "block" : "none" }}>
        <PdfViewer forwardHits={forwardHits} forwardNonce={forwardNonce} />
      </div>
      <div className="panel-body" style={{ display: rightTab === "claude" ? "block" : "none" }}>
        <ClaudePanel />
      </div>
    </div>
  );

  return (
    <div className="app">
      <div className="app-body">
        <Split
          id="workspace"
          direction="horizontal"
          panes={[
            { id: "sidebar", defaultSize: 18, minSize: 10, content: <FilePanel /> },
            { id: "centre", defaultSize: 44, minSize: 20, content: centre },
            { id: "right", defaultSize: 38, minSize: 15, content: right },
          ]}
        />
      </div>
      <StatusBar
        left={[
          {
            id: "project",
            content: <span>📁 {project.record.root.split("/").at(-1)}</span>,
            onClick: () => spawn(closeProject()),
            title: "Close project",
          },
          {
            id: "branch",
            content: (
              <span>
                ⎇ {repo === null || repo === undefined ? "no git" : (repo.branch ?? "detached")}
              </span>
            ),
            onClick: () => setBottomTab("git"),
          },
          {
            id: "compile",
            content: (
              <>
                <span className={`dot ${compileDot(compileState)}`} />
                <span>{compileState === "running" ? "compiling…" : compileState}</span>
              </>
            ),
            onClick: () => setBottomTab("problems"),
          },
          {
            id: "problems",
            content: (
              <span>
                ✗ {errors} ⚠ {warnings}
              </span>
            ),
            onClick: () => setBottomTab("problems"),
          },
        ]}
        right={[
          {
            id: "main",
            content: <span>main: {project.record.mainFile ?? "not set"}</span>,
            title: "Right-click a .tex file in the tree to set it as the main file",
          },
          {
            id: "cursor",
            content: (
              <span>
                Ln {cursor.line}, Col {cursor.column}
              </span>
            ),
          },
          {
            id: "tex",
            content: <span>{texProbe.state === "ok" ? "TeX ✓" : "TeX ✗"}</span>,
            onClick: () => setSettingsOpen(true),
            title: texProbe.state === "ok" ? texProbe.value.binDir : "TeX not found; open settings",
          },
          {
            id: "actions",
            content: (
              <>
                {compileState === "running" ? (
                  <Button small variant="ghost" onClick={() => spawn(cancelCompile())}>
                    Cancel
                  </Button>
                ) : (
                  <Button
                    small
                    variant="primary"
                    onClick={compileNow}
                    title="Save and compile (Ctrl+Enter)"
                  >
                    Compile
                  </Button>
                )}
                <Button
                  small
                  variant="ghost"
                  onClick={() => setSettingsOpen(true)}
                  title="Settings"
                >
                  ⚙
                </Button>
              </>
            ),
          },
        ]}
      />
      <CredentialDialog />
    </div>
  );
}
