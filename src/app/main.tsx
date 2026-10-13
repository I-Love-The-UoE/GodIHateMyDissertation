import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { errorLog } from "@/lib/log";
import { createBackend } from "@/lib/tauri";
import { createDocumentStore, createLocalProviderFactory } from "@/lib/yjs";
import { createAppStore } from "@/state/store";
import { App } from "./app";
import { ServicesProvider } from "./services";
import "@xterm/xterm/css/xterm.css";
import "./theme.css";

const backend = createBackend();
const store = createAppStore(backend);
const documents = createDocumentStore(
  createLocalProviderFactory({
    backend,
    user: { name: "me", colour: "#3b82f6" },
    onExternalChange: (path, change) => store.getState().noteExternalChange(path, change),
  }),
);

window.addEventListener("unhandledrejection", (event) => {
  errorLog("An unhandled rejection reached the window.", { reason: event.reason });
});

window.addEventListener("error", (event) => {
  errorLog("An uncaught error reached the window.", { message: event.message });
});

const root = document.getElementById("root");

if (root !== null) {
  createRoot(root).render(
    <StrictMode>
      <ServicesProvider services={{ backend, documents, store }}>
        <App />
      </ServicesProvider>
    </StrictMode>,
  );
}
