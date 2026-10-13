# GodIHateMyDissertation

An Overleaf-like LaTeX editor for the desktop (Windows, Linux, macOS): project management, LaTeX
syntax highlighting, real-time PDF preview via `latexmk`, built-in git with remotes, and an
in-app Claude Code panel that drives the user's installed `claude` CLI.

## Layout

- `crates/gihmd-core`: models, boundary traits (`Repository`, `Secrets`, `Processes`, `Watcher`),
  the `Context` trait, and all pure logic (compile orchestration, log parsing, SyncTeX, project
  index, Claude stream-json protocol and session). No vendor crates.
- `crates/gihmd-git` (git2), `crates/gihmd-secrets` (keyring), `crates/gihmd-process`
  (tokio::process), `crates/gihmd-watch` (notify): one vendor each, implementing a core trait.
- `src-tauri`: the Tauri 2 app. `config.rs` loads once at start-up, `context.rs` builds the
  concrete `AppContext`, `state.rs` holds managed state, `commands/*` are thin async commands
  returning domain errors as values, `events.rs` names the global events.
- `src`: React 19 + TypeScript frontend. `lib/` (leaves and vendor wrappers: `tauri`, `codemirror`,
  `pdf`, `yjs`), `state/` (zustand slices), `components/` (presentational), `features/`
  (file tree, editor, preview, problems, git, claude, settings, workspace), `app/` (entry, services).

## Running

```sh
pnpm install --frozen-lockfile
pnpm tauri dev
```

Requires a system TeX distribution with `latexmk` and `synctex`, and the Claude Code CLI on PATH
(or set in Settings). Builds go to the app cache dir (`$APPCACHE/builds/<hash>/`), never into the
project.

## Multiplayer seam

Every open file is a Yjs `Y.Doc` bound to CodeMirror through `y-codemirror.next`. The only
provider today is `src/lib/yjs/local-provider.ts`: editor content wins whenever the document has
unsaved edits (stale writes are retried unconditionally and the user is told), and the disk wins only
while the document is clean, so Claude Code or git changing a file shows up in a clean buffer. A future
WebSocket/WebRTC provider implements the same `SyncProvider` interface and is selected by the
`SyncProviderFactory` in `src/app/main.tsx`; the editor does not change.

## Claude Code integration

`gihmd-core::claude` spawns `claude -p --input-format stream-json --output-format stream-json
--permission-prompt-tool stdio ...` per project, keeps stdin open, decodes every stdout line into
`Inbound` and answers `control_request/can_use_tool` with the user's Allow/Deny from the panel.
Subscription login is used as-is on the user's machine; distributing the app with subscription
auth requires Anthropic's approval.

## Code style (binding)

Every line of code in this repository is governed by a language style guide.
Read the relevant guide **before** writing or editing code; it wins over any
existing code that disagrees with it.

- Rust (`src-tauri/`, `crates/`): `docs/style/rust.md`
- TypeScript / React (`src/`): `docs/style/typescript.md`

Non-negotiables shared by both guides:

- **No fallbacks.** One toolchain, one runtime, one set of dependencies. No
  feature gates, environment probes, polyfills, optional dependencies, or
  defaults that mask a missing required value. Missing configuration fails
  at start-up (Rust) or at build time (TypeScript).
- **Errors are values.** `Result<T, DomainError>` on both sides of the IPC
  boundary; no `unwrap`/`expect`/`panic!`/`anyhow` in Rust, no `throw`/`any`
  in TypeScript, except at the single documented vendor boundary.
- **One wrapper per third party.** `git2`, `keyring`, `tokio::process`
  each live in one crate; `@tauri-apps/api`, CodeMirror, pdf.js, Yjs each
  live in one directory under `src/lib/`. Nothing else imports them.
- **Newest stable syntax**, British spelling in identifiers and comments,
  constant log messages with structured fields, minimal comments.
- **Tests are optional** and are not added unless asked.

Gates that must pass before a change is complete:

    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    pnpm exec tsc -b
    pnpm exec biome ci .

Each guide ends with a pre-commit checklist; run through it.
