import { cpSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import type { Plugin } from "vite";
import { defineConfig } from "vite";

const root = dirname(fileURLToPath(import.meta.url));
const host = process.env["TAURI_DEV_HOST"];

// NOTE: pdf.js loads its wasm decoders, CMaps and standard fonts by URL at run time, so they are
// copied next to the page rather than bundled.
function copyPdfjsAssets(): Plugin {
  return {
    name: "gihmd-copy-pdfjs-assets",
    buildStart(): void {
      const source = resolve(root, "node_modules/pdfjs-dist");
      const target = resolve(root, "public/pdfjs");
      mkdirSync(target, { recursive: true });

      for (const folder of ["wasm", "cmaps", "standard_fonts"]) {
        cpSync(resolve(source, folder), resolve(target, folder), { recursive: true });
      }
    },
  };
}

export default defineConfig({
  plugins: [react({ compiler: true }), copyPdfjsAssets()],
  resolve: { alias: { "@": resolve(root, "src") } },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host === undefined ? false : host,
    hmr: host === undefined ? undefined : { protocol: "ws", host, port: 5174 },
    watch: { ignored: ["**/src-tauri/**", "**/target/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: "es2023",
    minify: process.env["TAURI_ENV_DEBUG"] === undefined ? "oxc" : false,
    sourcemap: process.env["TAURI_ENV_DEBUG"] !== undefined,
  },
});
