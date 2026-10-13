import { Awareness } from "y-protocols/awareness";
import * as Y from "yjs";

/** Origin for programmatic loads and reloads so that undo never reverts a file load. */
export const LOCAL_ORIGIN = Symbol("gihmd-local");

export type DocumentId = string;

export type SyncStatus = "loading" | "synced" | "saving" | "dirty" | "error";

export type SyncProvider = {
  readonly kind: "local" | "remote";
  readonly awareness: Awareness;
  readonly connect: () => Promise<void>;
  readonly disconnect: () => Promise<void>;
  readonly flush: () => Promise<void>;
  readonly onStatus: (callback: (status: SyncStatus) => void) => () => void;
  readonly status: () => SyncStatus;
};

export type SyncProviderFactory = {
  readonly create: (id: DocumentId, path: string, doc: Y.Doc) => SyncProvider;
};

export type OpenDocument = {
  readonly id: DocumentId;
  readonly path: string;
  readonly doc: Y.Doc;
  readonly text: Y.Text;
  readonly undo: Y.UndoManager;
  readonly provider: SyncProvider;
};

export type UserPresence = { readonly name: string; readonly colour: string };

export function documentId(projectId: string, path: string): DocumentId {
  return `${projectId}/${path}`;
}

export function createDoc(): { readonly doc: Y.Doc; readonly text: Y.Text } {
  const doc = new Y.Doc();

  return { doc, text: doc.getText("content") };
}

export function createUndo(text: Y.Text): Y.UndoManager {
  // NOTE: Only user edits (origin null) and the collaboration binding are undoable.
  return new Y.UndoManager(text, { trackedOrigins: new Set([null]), captureTimeout: 400 });
}

export function createAwareness(doc: Y.Doc, user: UserPresence): Awareness {
  const awareness = new Awareness(doc);
  awareness.setLocalStateField("user", {
    name: user.name,
    color: user.colour,
    colorLight: `${user.colour}33`,
  });

  return awareness;
}

export function replaceContent(doc: Y.Doc, text: Y.Text, content: string): void {
  doc.transact(() => {
    text.delete(0, text.length);
    text.insert(0, content);
  }, LOCAL_ORIGIN);
}

export function isLocalOrigin(transaction: Y.Transaction): boolean {
  return transaction.origin === LOCAL_ORIGIN;
}

export function observeText(
  text: Y.Text,
  callback: (transaction: Y.Transaction) => void,
): () => void {
  const handler = (_event: Y.YTextEvent, transaction: Y.Transaction): void => callback(transaction);
  text.observe(handler);

  return () => text.unobserve(handler);
}
