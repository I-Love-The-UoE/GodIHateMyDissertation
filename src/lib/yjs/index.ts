export type {
  DocumentId,
  OpenDocument,
  SyncProvider,
  SyncProviderFactory,
  SyncStatus,
  UserPresence,
} from "./document";
export { documentId } from "./document";
export type { ExternalChange, LocalProvider, LocalProviderOptions } from "./local-provider";
export { createLocalProvider, createLocalProviderFactory, isLocalProvider } from "./local-provider";
export type { DocumentStore } from "./store";
export { createDocumentStore } from "./store";
