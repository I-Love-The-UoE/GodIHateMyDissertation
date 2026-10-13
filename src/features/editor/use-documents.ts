import { useEffect, useRef, useState } from "react";
import { useServices, useStore } from "@/app/services";
import { spawn } from "@/lib/async";
import type { EditorCommands, EditorDiagnostic, EditorHandle, EditorState } from "@/lib/codemirror";
import { createDocumentState, createEditor } from "@/lib/codemirror";
import { errorLog } from "@/lib/log";
import type { OpenDocument } from "@/lib/yjs";
import { documentId } from "@/lib/yjs";

export type DocumentsHandle = {
  readonly editor: EditorHandle | undefined;
  readonly hostRef: (element: HTMLDivElement | null) => void;
};

/**
 * Owns one CodeMirror view and one EditorState per open tab; must be mounted once in the
 * workspace so that documents outlive tab switches.
 */
export function useDocuments(commands: EditorCommands): DocumentsHandle {
  const { documents, store } = useServices();
  const project = useStore((state) => state.project);
  const tabs = useStore((state) => state.tabs);
  const activeTab = useStore((state) => state.activeTab);
  const goTo = useStore((state) => state.goTo);
  const diagnostics = useStore((state) => state.diagnostics);
  const closeRequests = useStore((state) => state.tabCloseRequests);
  const [editor, setEditor] = useState<EditorHandle | undefined>(undefined);
  const [loadedVersion, setLoadedVersion] = useState(0);
  const states = useRef(new Map<string, EditorState>());
  const unsubscribes = useRef(new Map<string, () => void>());
  const commandsRef = useRef(commands);
  commandsRef.current = commands;

  const hostRef = (element: HTMLDivElement | null): void => {
    if (element === null || editor !== undefined) {
      return;
    }

    setEditor(createEditor(element));
  };

  useEffect(() => {
    if (project === undefined) {
      return;
    }

    const projectId = project.record.id;
    const openAll = async (): Promise<void> => {
      for (const path of tabs) {
        if (states.current.has(path)) {
          continue;
        }

        const document: OpenDocument = await documents.open(documentId(projectId, path), path);
        const state = createDocumentState({
          text: document.text,
          awareness: document.provider.awareness,
          undo: document.undo,
          index: () => store.getState().index,
          commands: {
            save: () => commandsRef.current.save(),
            compile: () => commandsRef.current.compile(),
            forwardSearch: () => commandsRef.current.forwardSearch(),
          },
        });
        states.current.set(path, state);
        store.getState().setSyncStatus(path, document.provider.status());
        unsubscribes.current.set(
          path,
          document.provider.onStatus((status) => store.getState().setSyncStatus(path, status)),
        );
        // NOTE: Bumping a version re-runs the swap effect below with the current editor, so a load
        // that finishes before the view exists is still applied once it does.
        setLoadedVersion((version) => version + 1);
      }
    };

    spawn(openAll());
  }, [project, tabs, documents, store]);

  useEffect(() => {
    if (editor === undefined || activeTab === undefined || loadedVersion < 0) {
      return;
    }

    const state = states.current.get(activeTab);

    if (state !== undefined && editor.view.state !== state) {
      editor.setDocument(state);
    }
  }, [editor, activeTab, loadedVersion]);

  useEffect(() => {
    if (closeRequests.length === 0 || project === undefined) {
      return;
    }

    const projectId = project.record.id;

    for (const path of closeRequests) {
      const current = editor?.view.state;
      const state = states.current.get(path);

      if (current !== undefined && state !== undefined && current === state) {
        states.current.set(path, current);
      }

      states.current.delete(path);
      unsubscribes.current.get(path)?.();
      unsubscribes.current.delete(path);
      spawn(
        documents.close(documentId(projectId, path)).catch((rejection: unknown) => {
          errorLog("Closing a document failed.", { path, rejection });
        }),
      );
      store.getState().tabClosed(path);
    }
  }, [closeRequests, project, documents, store, editor]);

  useEffect(() => {
    if (editor === undefined || activeTab === undefined) {
      return;
    }

    const mapped: EditorDiagnostic[] = [];

    for (const diagnostic of diagnostics) {
      if (
        diagnostic.line === null ||
        diagnostic.file === null ||
        !diagnostic.file.endsWith(`/${activeTab}`)
      ) {
        continue;
      }

      mapped.push({
        line: diagnostic.line,
        severity:
          diagnostic.severity === "error"
            ? "error"
            : diagnostic.severity === "warning"
              ? "warning"
              : "info",
        message: diagnostic.message,
      });
    }

    editor.setDiagnostics(mapped);
  }, [editor, activeTab, diagnostics]);

  useEffect(() => {
    if (editor === undefined || goTo === undefined || goTo.path !== activeTab) {
      return;
    }

    if (editor.view.state === states.current.get(activeTab)) {
      editor.goTo(goTo.line, goTo.column);
    }
  }, [editor, goTo, activeTab]);

  useEffect(() => {
    if (editor === undefined) {
      return;
    }

    const interval = setInterval(() => {
      const { line, column } = editor.cursor();
      const current = store.getState().cursor;

      if (current.line !== line || current.column !== column) {
        store.getState().setCursor(line, column);
      }
    }, 150);

    return () => clearInterval(interval);
  }, [editor, store]);

  useEffect(() => {
    return () => {
      editor?.destroy();
    };
  }, [editor]);

  return { editor, hostRef };
}
