import type * as Y from "yjs";
import { debounced, spawn } from "@/lib/async";
import { errorLog, infoLog } from "@/lib/log";
import { isErr } from "@/lib/result";
import type { Backend } from "@/lib/tauri";
import { describeError } from "@/lib/tauri";
import type { SyncProvider, SyncProviderFactory, SyncStatus, UserPresence } from "./document";
import { createAwareness, isLocalOrigin, observeText, replaceContent } from "./document";

const SAVE_DEBOUNCE_MS = 600;

export type ExternalChange = "reloaded" | "kept";

export type LocalProviderOptions = {
  readonly backend: Backend;
  readonly user: UserPresence;
  readonly onExternalChange?: (path: string, change: ExternalChange) => void;
};

export type LocalProvider = SyncProvider & {
  /**
   * Called when the watcher reports the file. A clean document reloads; a document with
   * unsaved edits keeps them and overwrites the disk on its next save. `discardEdits` forces
   * the disk version in.
   */
  readonly reloadFromDisk: (discardEdits: boolean) => Promise<void>;
};

type Listener = (status: SyncStatus) => void;

/**
 * Keeps a Yjs document in sync with one file on disk. Editor content wins whenever the document
 * has unsaved edits; the disk wins only while the document is clean.
 */
export function createLocalProvider(
  path: string,
  doc: Y.Doc,
  options: LocalProviderOptions,
): LocalProvider {
  const listeners = new Set<Listener>();
  let status: SyncStatus = "loading";
  const text = doc.getText("content");
  const awareness = createAwareness(doc, options.user);
  let mtimeMs: number | null = null;
  let unobserve: (() => void) | undefined;
  // NOTE: Bumped on every user edit so a save that raced with typing leaves the document dirty.
  let generation = 0;

  const setStatus = (next: SyncStatus): void => {
    status = next;

    for (const listener of listeners) {
      listener(next);
    }
  };

  const write = async (expectedMtimeMs: number | null): Promise<boolean> => {
    const result = await options.backend.fileWrite(path, text.toString(), expectedMtimeMs);

    if (isErr(result)) {
      if (result.error.kind === "stale_write" && expectedMtimeMs !== null) {
        infoLog(
          "The file changed on disk while unsaved edits existed; the editor version was kept.",
          { path },
        );
        options.onExternalChange?.(path, "kept");

        return write(null);
      }

      errorLog("Saving the document failed.", { path, error: describeError(result.error) });
      setStatus("error");

      return false;
    }

    mtimeMs = result.value.mtimeMs;

    return true;
  };

  const save = async (): Promise<void> => {
    if (status !== "dirty" && status !== "error") {
      return;
    }

    const saved = generation;
    setStatus("saving");
    const written = await write(mtimeMs);

    if (!written) {
      return;
    }

    if (generation === saved) {
      setStatus("synced");
    } else {
      setStatus("dirty");
      debouncedSave.call();
    }
  };

  const debouncedSave = debounced(SAVE_DEBOUNCE_MS, () => spawn(save()));

  const load = async (): Promise<boolean> => {
    const result = await options.backend.fileRead(path);

    if (isErr(result)) {
      errorLog("Loading the document failed.", { path, error: describeError(result.error) });
      setStatus("error");

      return false;
    }

    mtimeMs = result.value.stat.mtimeMs;

    if (result.value.text !== text.toString()) {
      replaceContent(doc, text, result.value.text);
    }

    setStatus("synced");

    return true;
  };

  return {
    kind: "local",
    awareness,
    connect: async () => {
      await load();
      unobserve = observeText(text, (transaction) => {
        if (isLocalOrigin(transaction)) {
          return;
        }

        generation += 1;
        setStatus("dirty");
        debouncedSave.call();
      });
    },
    disconnect: async () => {
      debouncedSave.cancel();
      unobserve?.();
      unobserve = undefined;

      if (status === "dirty" || status === "error") {
        await save();
      }

      awareness.destroy();
    },
    flush: async () => {
      debouncedSave.cancel();
      await save();
    },
    onStatus: (callback) => {
      listeners.add(callback);

      return () => listeners.delete(callback);
    },
    status: () => status,
    reloadFromDisk: async (discardEdits) => {
      if (discardEdits) {
        debouncedSave.cancel();
        generation += 1;
        const loaded = await load();

        if (loaded) {
          options.onExternalChange?.(path, "reloaded");
        }

        return;
      }

      const result = await options.backend.fileRead(path);

      if (isErr(result)) {
        return;
      }

      const unchanged =
        result.value.stat.mtimeMs === mtimeMs || result.value.text === text.toString();

      if (unchanged) {
        mtimeMs = result.value.stat.mtimeMs;

        return;
      }

      if (status === "dirty" || status === "saving") {
        options.onExternalChange?.(path, "kept");
        debouncedSave.call();

        return;
      }

      mtimeMs = result.value.stat.mtimeMs;
      replaceContent(doc, text, result.value.text);
      setStatus("synced");
      options.onExternalChange?.(path, "reloaded");
    },
  };
}

export function createLocalProviderFactory(options: LocalProviderOptions): SyncProviderFactory {
  return { create: (_id, path, doc) => createLocalProvider(path, doc, options) };
}

export function isLocalProvider(provider: SyncProvider): provider is LocalProvider {
  return provider.kind === "local" && "reloadFromDisk" in provider;
}
