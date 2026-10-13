import type { DocumentId, OpenDocument, SyncProviderFactory } from "./document";
import { createDoc, createUndo } from "./document";

export type DocumentStore = {
  readonly open: (id: DocumentId, path: string) => Promise<OpenDocument>;
  readonly get: (id: DocumentId) => OpenDocument | undefined;
  readonly byPath: (path: string) => OpenDocument | undefined;
  readonly close: (id: DocumentId) => Promise<void>;
  readonly closeAll: () => Promise<void>;
  readonly all: () => readonly OpenDocument[];
};

type Entry = { readonly document: OpenDocument; refs: number };

export function createDocumentStore(factory: SyncProviderFactory): DocumentStore {
  const entries = new Map<DocumentId, Entry>();

  const close = async (id: DocumentId): Promise<void> => {
    const entry = entries.get(id);

    if (entry === undefined) {
      return;
    }

    entry.refs -= 1;

    if (entry.refs > 0) {
      return;
    }

    entries.delete(id);
    await entry.document.provider.disconnect();
    entry.document.undo.destroy();
    entry.document.doc.destroy();
  };

  return {
    open: async (id, path) => {
      const existing = entries.get(id);

      if (existing !== undefined) {
        existing.refs += 1;

        return existing.document;
      }

      const { doc, text } = createDoc();
      const provider = factory.create(id, path, doc);
      await provider.connect();
      const document: OpenDocument = { id, path, doc, text, undo: createUndo(text), provider };
      entries.set(id, { document, refs: 1 });

      return document;
    },
    get: (id) => entries.get(id)?.document,
    byPath: (path) => {
      for (const entry of entries.values()) {
        if (entry.document.path === path) {
          return entry.document;
        }
      }

      return undefined;
    },
    close,
    closeAll: async () => {
      const ids = [...entries.keys()];

      for (const id of ids) {
        const entry = entries.get(id);

        if (entry !== undefined) {
          entry.refs = 1;
        }

        await close(id);
      }
    },
    all: () => [...entries.values()].map((entry) => entry.document),
  };
}
