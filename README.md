# GodIHateMyDissertation

A desktop LaTeX editor that looks and feels like Overleaf, but runs on your own machine. Open a
folder, write, and watch the PDF update as you type. Git and Claude Code are built in, so you can
version your thesis and ask an AI to fix that compile error without leaving the window.

Works on Windows, macOS and Linux.

## What you get

- Live PDF preview that recompiles a moment after you stop typing.
- LaTeX syntax highlighting, bracket matching, and autocomplete for commands, environments,
  `\ref` labels and `\cite` keys taken from your own files.
- A Problems panel that lists compile errors and warnings; click one to jump to the line.
- Click a line in the PDF to jump to the source, or press Ctrl+Alt+J to jump from source to PDF.
- Git inside the app: see changes, stage, commit, switch branches, and push or pull over SSH or
  HTTPS. Passwords and tokens go in your system keychain.
- A Claude Code panel that can read and edit your project, with Allow/Deny buttons for every
  action it wants to take.
- A terminal, already in your project folder.

## Before you start

You need two things installed:

1. **A TeX distribution** with `latexmk`: [TeX Live](https://tug.org/texlive/) (Windows, macOS
   via MacTeX, Linux) or [MiKTeX](https://miktex.org/) (Windows).
2. **Claude Code** if you want the AI panel: follow the
   [official install guide](https://code.claude.com/docs/en/quickstart) and sign in once with
   `claude`.

If the app cannot find TeX or Claude, the settings dialog (gear icon, bottom right) lets you point
it at the right folder.

## First steps

1. Launch the app and click **Open folder** (or **New project** for a fresh `main.tex`, or
   **Clone repository** to pull one from GitHub or GitLab).
2. Pick your main `.tex` file if the app did not guess it: right-click the file in the tree and
   choose **Set as main file**.
3. Start writing. The PDF on the right updates on its own. Press **Ctrl+Enter** to compile right
   now, or **Ctrl+S** to save.
4. Use the tabs at the bottom for **Problems**, the compile **Log**, **Git** and the **Terminal**,
   and the tabs on the right to switch between the **PDF** and **Claude**.

## Keyboard shortcuts

| Keys | What it does |
|---|---|
| Ctrl+S (Cmd+S) | Save all open files |
| Ctrl+Enter (Cmd+Enter) | Save and compile now |
| Ctrl+Alt+J | Jump from the cursor to the matching spot in the PDF |
| Double-click in the PDF | Jump to the matching line in the source |
| Ctrl+B (Cmd+B) | Show or hide the bottom panel |
| Ctrl+F (Cmd+F) | Find and replace in the editor |

## Good to know

- Your files are the only thing that matters. Builds go to a cache folder, never into your project,
  so nothing litters your git repository.
- If a file changes on disk while you have unsaved edits, your edits win and the app tells you.
  If the file is clean, it reloads quietly (so edits made by Claude or `git checkout` show up).
- The Claude panel uses your existing Claude Code login and subscription.

## Building from source

```sh
pnpm install --frozen-lockfile
pnpm tauri dev      # run in development
pnpm tauri build    # produce installers in target/release/bundle
```

Requires Rust (see `rust-toolchain.toml`), Node 22+ and pnpm 10. Linux additionally needs the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
