import type { ReactNode } from "react";
import { createContext, use } from "react";
import type { Backend } from "@/lib/tauri";
import type { DocumentStore } from "@/lib/yjs";
import type { AppStore } from "@/state/store";
import type { Store } from "@/state/types";

export type Services = {
  readonly backend: Backend;
  readonly documents: DocumentStore;
  readonly store: AppStore;
};

const ServicesContext = createContext<Services | undefined>(undefined);

export function ServicesProvider(props: {
  readonly services: Services;
  readonly children: ReactNode;
}): ReactNode {
  const { services, children } = props;

  return <ServicesContext value={services}>{children}</ServicesContext>;
}

export function useServices(): Services {
  const services = use(ServicesContext);

  if (services === undefined) {
    throw new Error("ServicesProvider is missing."); // NOTE: React context contract; the provider is always mounted at the root.
  }

  return services;
}

export function useStore<T>(selector: (state: Store) => T): T {
  // NOTE: Bound to a `use`-prefixed name so the React Compiler treats the call as a hook.
  const useBoundStore = useServices().store;

  return useBoundStore(selector);
}
