# TypeScript Code Style

This document is **binding** for every agent and contributor writing
TypeScript in this project. It describes the general style, structure, and
separation rules for the React frontend of the Tauri application.
Library-specific guidance lives in separate documents that build on this one
and must never contradict it. The Rust backend is described in
[`rust.md`](rust.md).

The key words MUST, MUST NOT, SHOULD, and MAY are to be read as in RFC 2119.
When existing code conflicts with this document, **this document wins**: bring
the code you touch into line rather than copying the old pattern.

---

## 1. Runtime and tooling

### 1.1 TypeScript, React, and the WebView

- The language is **TypeScript 5.9 or newer**, compiled by Vite for the
  Tauri WebView (WebKitGTK on Linux, WebView2 on Windows, WKWebView on
  macOS). There is no Node runtime at run time and no browser other than
  the one Tauri ships; nothing is written for "a browser".
- The UI library is **React 19** with the **React Compiler** enabled in
  `vite.config.ts`. Nothing older is supported.
- `tsconfig.app.json` MUST contain exactly these checking options; a
  project that relaxes any of them is non-compliant:

  ```jsonc
  {
    "compilerOptions": {
      "target": "ES2023",
      "lib": ["ES2023", "DOM", "DOM.Iterable"],
      "module": "ESNext",
      "moduleResolution": "bundler",
      "jsx": "react-jsx",
      "types": ["vite/client"],
      "paths": { "@/*": ["./src/*"] },

      "strict": true,
      "noUncheckedIndexedAccess": true,
      "exactOptionalPropertyTypes": true,
      "noImplicitOverride": true,
      "noFallthroughCasesInSwitch": true,
      "noPropertyAccessFromIndexSignature": true,
      "noUncheckedSideEffectImports": true,

      "verbatimModuleSyntax": true,
      "erasableSyntaxOnly": true,
      "isolatedModules": true,
      "moduleDetection": "force",
      "noEmit": true,
      "skipLibCheck": true
    },
    "include": ["src"]
  }
  ```

  `tsconfig.json` holds only `references` to `tsconfig.app.json` and
  `tsconfig.node.json` (the latter for `vite.config.ts`); type checking
  is `tsc -b`.
- `erasableSyntaxOnly` means `enum`, `namespace`, parameter properties,
  and `import x = require()` are compile errors. `verbatimModuleSyntax`
  means every type-only import is written `import type`.

### 1.2 Project management with pnpm

Every JavaScript dependency is managed by **pnpm**. There is no npm, no
yarn, no Bun, no `npx`, and no global installs.

| Task | Command |
|------|---------|
| Install everything | `pnpm install --frozen-lockfile` |
| Add a runtime dependency | `pnpm add <package>` |
| Add a development dependency | `pnpm add -D <package>` |
| Run a tool from the project | `pnpm exec <tool>` or a `scripts` entry |
| Run the application in development | `pnpm tauri dev` |
| Type-check | `pnpm exec tsc -b` |
| Format and lint | `pnpm exec biome check --write .` |
| CI gate | `pnpm exec biome ci .` |

- `package.json` is the single source of truth for dependencies, scripts,
  and the `packageManager` field, which pins the exact pnpm version
  (`"packageManager": "pnpm@<exact version>"`); pnpm 10 self-selects that
  version. `pnpm-lock.yaml` MUST be committed. `.npmrc` contains
  `engine-strict=true` and nothing that changes resolution.
- Development-only tools (`@biomejs/biome`, `typescript`, `vite`,
  `@tauri-apps/cli`, type packages) go in `devDependencies`, never in
  `dependencies`.
- Scripts, Makefiles, and CI MUST invoke tools through pnpm rather than a
  bare binary on `PATH`.

### 1.3 No fallbacks, no compatibility shims

The runtime is a single known WebView with a single known set of
dependencies. Code that hedges against a different environment is wrong, not
defensive. The following are **forbidden**:

- `typeof window !== "undefined"`, `typeof document`, `typeof process`,
  `"navigator" in globalThis`, or any check used to decide whether a
  browser API exists. It exists.
- `if ("__TAURI_INTERNALS__" in window)` or any check for "running inside
  Tauri". The app runs only inside Tauri; the wrapper in `lib/tauri/`
  assumes it.
- Polyfills, `core-js`, `regenerator-runtime`, `browserslist` targets
  older than the shipped WebView, and `@babel/preset-env`.
- Dynamic `import()` guarded by `try/catch` to pick between two
  implementations. Optional peer dependencies with a "works without it"
  branch. If something is needed it is a hard dependency; if it is not
  needed it is not installed.
- `// @ts-ignore` (lint error). `// @ts-expect-error <reason>` is permitted
  only with a reason on the same line and only where a vendor type is
  wrong; it is a defect to be removed when the vendor fixes it.
- `any`, explicit or through `as any`, `Function`, `Object`, or `{}` as a
  type (lint error).
- `?? default`, `|| default`, or `?.` used to **mask a missing or
  malformed configuration value** (§2.3). A required value that is absent
  fails the build.
- Pinning an older version of a library to keep a deprecated API alive, or
  calling a deprecated export when a current replacement exists.

If a dependency does not support TypeScript 5.9, React 19, or ESM, replace
the dependency, patch it upstream, or fork it. Do **not** write around the
problem.

### 1.4 Prefer the newest syntax

Always use the most modern construct the language and libraries provide.

| Instead of | Use |
|------------|-----|
| `import { type X, y }` mixed imports | `import type { X }` on its own line, `import { y }` on another |
| `enum E { A, B }` | `const E = { A: "a", B: "b" } as const` plus `type E = (typeof E)[keyof typeof E]`, or a plain literal union |
| `namespace`, `module` blocks | ES modules |
| `class Foo { constructor(private x: number) {} }` | A `type` and a function; parameter properties are not erasable |
| `React.FC<Props>`, `React.FunctionComponent` | `function Name(props: NameProps): ReactNode` |
| `forwardRef` | `ref` as an ordinary prop (React 19) |
| `useContext(Ctx)` | `use(Ctx)` |
| `<Ctx.Provider value={v}>` | `<Ctx value={v}>` (React 19) |
| `useMemo`, `useCallback`, `memo` by hand | Nothing; the React Compiler memoises |
| `useEffect` to derive state from props or store | Derive during render, or a store selector |
| `array[i]` then a null check | `array.at(i)` with `let ... = ...; if (x === undefined) return ...` |
| `Object.assign({}, a, b)` | Spread `{ ...a, ...b }` |
| `arr.indexOf(x) !== -1` | `arr.includes(x)` |
| `Promise.all` where a failure of one should not abort the rest | `Promise.allSettled` |
| `.then().catch()` chains | `async`/`await` |
| `x != null` | `x !== null && x !== undefined`, or narrow the type so one case cannot occur |
| `interface` for data shapes | `type` (interface only when a library requires declaration merging) |
| `Array<T>` | `T[]`; `readonly T[]` for inputs |
| Hand-written `toSorted`-style copies | `toSorted`, `toReversed`, `with` (ES2023) |
| `structuredClone` polyfills | `structuredClone` |

### 1.5 Biome

Formatting and linting are **Biome**; there is no ESLint, no Prettier, no
stylelint. `biome.json` at the repository root:

```jsonc
{
  "$schema": "./node_modules/@biomejs/biome/configuration_schema.json",
  "vcs": { "enabled": true, "clientKind": "git", "useIgnoreFile": true },
  "files": { "includes": ["**", "!**/dist", "!**/src-tauri", "!**/node_modules"] },
  "formatter": {
    "enabled": true,
    "indentStyle": "space",
    "indentWidth": 2,
    "lineWidth": 100,
    "lineEnding": "lf"
  },
  "javascript": {
    "formatter": {
      "quoteStyle": "double",
      "semicolons": "always",
      "trailingCommas": "all",
      "arrowParentheses": "always"
    }
  },
  "assist": { "actions": { "source": { "organizeImports": "on" } } },
  "linter": {
    "enabled": true,
    "domains": { "react": "recommended", "project": "recommended", "types": "recommended" },
    "rules": {
      "recommended": true,
      "correctness": {
        "noUnusedImports": "error",
        "noUnusedVariables": "error",
        "noUndeclaredDependencies": "error",
        "useExhaustiveDependencies": "error"
      },
      "suspicious": {
        "noExplicitAny": "error",
        "noConsole": "error",
        "noTsIgnore": "error",
        "noEmptyBlockStatements": "error"
      },
      "style": {
        "useImportType": "error",
        "useExportType": "error",
        "noDefaultExport": "error",
        "noNonNullAssertion": "error",
        "noEnum": "error",
        "noNamespace": "error",
        "noParameterAssign": "error",
        "useConst": "error",
        "noRestrictedImports": {
          "level": "error",
          "options": {
            "paths": {
              "@tauri-apps/api": "Import from @/lib/tauri instead.",
              "@tauri-apps/api/core": "Import from @/lib/tauri instead.",
              "@tauri-apps/api/event": "Import from @/lib/tauri instead.",
              "@codemirror/state": "Import from @/lib/codemirror instead.",
              "@codemirror/view": "Import from @/lib/codemirror instead.",
              "pdfjs-dist": "Import from @/lib/pdf instead.",
              "yjs": "Import from @/lib/yjs instead."
            }
          }
        }
      },
      "performance": { "noBarrelFile": "error", "noReExportAll": "error" },
      "complexity": { "noExcessiveCognitiveComplexity": "error" },
      "nursery": {
        "noFloatingPromises": "error",
        "noMisusedPromises": "error"
      }
    }
  },
  "overrides": [
    {
      "includes": ["src/lib/tauri/**", "src/lib/codemirror/**", "src/lib/pdf/**", "src/lib/yjs/**"],
      "linter": { "rules": { "style": { "noRestrictedImports": "off" } } }
    },
    { "includes": ["src/lib/log.ts"], "linter": { "rules": { "suspicious": { "noConsole": "off" } } } },
    { "includes": ["**/*.lazy.tsx", "vite.config.ts"], "linter": { "rules": { "style": { "noDefaultExport": "off" } } } }
  ]
}
```

- `noFloatingPromises` and `noMisusedPromises` are type-aware and need the
  `types` domain, which turns on Biome's project scanner. They are still
  in `nursery`; when they graduate, move the keys, do not drop them.
- A lint is suppressed only with `// biome-ignore lint/<group>/<rule>: <reason>`
  on the line above; Biome rejects a suppression without a reason. Do not
  add rules to `overrides` to silence a single file; fix the file.

---

## 2. Separation and structure

### 2.1 One-way dependencies

Code under `src/` is organised into directories with a **single direction
of dependency**, leaves first:

1. `src/lib/`: leaves. `result.ts`, `log.ts`, `config.ts`, `async.ts`,
   pure utilities, and the vendor wrappers (`lib/tauri/`,
   `lib/codemirror/`, `lib/pdf/`, `lib/yjs/`). Imports nothing from the
   project outside `lib/`. Contains no React.
2. `src/state/`: zustand store slices and their selectors. Imports `lib/`
   only. Contains no JSX.
3. `src/components/`: presentational components. Props in, callbacks out.
   Imports `lib/` only; never the store, never a wrapper's side-effecting
   functions.
4. `src/features/<feature>/`: hooks and feature components that compose
   `state/`, `components/`, and `lib/`. A feature never imports another
   feature; shared pieces move down to `components/` or `lib/`.
5. `src/app/`: `main.tsx`, the root component, providers, routing. Imports
   everything; nothing imports it.

- Draw the arrow before writing the module; if two directories would need
  each other, the boundary is wrong. Import cycles are a design error, not
  something to paper over with lazy imports.
- The direction is checked in review and by the `project` domain's import
  analysis; `noRestrictedImports` enforces the vendor boundary.

### 2.2 Boundaries around third parties

- Every external system or heavyweight library (`@tauri-apps/api` and
  every `@tauri-apps/plugin-*`, CodeMirror, pdf.js, Yjs) is wrapped in
  **one** directory under `src/lib/` that owns the vendor import. The rest
  of the codebase types against the wrapper's own exported types and
  functions, never against the vendor's.
- `lib/tauri/` owns `invoke` and `listen`. It exports one typed function
  per backend command (`openDocument(path): Promise<Result<Document, DocumentError>>`)
  and one typed subscription per event. It is the **only** place a promise
  rejection is converted into a `Result` (§7).
- `lib/codemirror/` owns `EditorState`, `EditorView`, extensions, and
  theme. `lib/pdf/` owns the pdf.js worker setup and page rendering.
  `lib/yjs/` owns `Y.Doc`, providers, and awareness. Each exports a small
  interface in project vocabulary and a `create<Thing>()` factory.
- Vendor objects never cross into `state/`, `components/`, or `features/`.
  If a component needs to host a CodeMirror view, it receives a handle type
  defined by `lib/codemirror/`.

### 2.3 Configuration

- Build-time configuration is a flat module `src/lib/config.ts` exporting
  typed constants read from `import.meta.env`. `src/vite-env.d.ts`
  declares every required variable on `ImportMetaEnv` as `string` (not
  `string | undefined`).
- The build fails fast: `vite.config.ts` reads the same variables with
  `loadEnv` and refuses to build when one is missing. That is the one
  place a throw is written for configuration, and it is build tooling, not
  application code (§7). No `?? "default"` in `config.ts`.
- Run-time configuration (paths, user settings) comes from the backend
  through one command in `lib/tauri/` at start-up and is held in the
  store. Nothing else reads `import.meta.env`, `localStorage`, or the URL.

### 2.4 Dependency injection over globals

- Logic in `state/` and `features/` never reaches for a module-level
  singleton client. Whatever it needs is passed in, normally as the
  wrapper's interface type: a store slice's actions receive a `Backend`
  object (the `lib/tauri/` interface) at store creation, a test creates
  the store with an in-memory fake.
- The only permitted module-level state is the logger, configuration
  constants, immutable lookup tables, and the store instance created in
  `src/app/`.
- React context is used for things that are genuinely per-tree (theme,
  the store instance), not as a service locator.

### 2.5 Results and errors as values

Expected failures are **return values**, not thrown exceptions.

- `src/lib/result.ts` is the whole of the error machinery:

  ```ts
  export type Ok<T> = { readonly ok: true; readonly value: T };
  export type Err<E> = { readonly ok: false; readonly error: E };
  export type Result<T, E> = Ok<T> | Err<E>;

  export function ok<T>(value: T): Ok<T> {
    return { ok: true, value };
  }

  export function err<E>(error: E): Err<E> {
    return { ok: false, error };
  }

  export function isOk<T, E>(result: Result<T, E>): result is Ok<T> {
    return result.ok;
  }

  export function isErr<T, E>(result: Result<T, E>): result is Err<E> {
    return !result.ok;
  }
  ```

  No `map`, `andThen`, `unwrap`, or class wrapper is added; the union is
  narrowed with `if (isErr(result)) return result;` and read directly.
- A function that can fail in an expected way declares an error type
  `<Domain>Error` as a discriminated union on `kind`, whose members match
  the backend's `snake_case` variant names where the error originates in
  Rust:

  ```ts
  export type DocumentError =
    | { readonly kind: "not_found"; readonly path: string }
    | { readonly kind: "locked" }
    | { readonly kind: "repository"; readonly cause: RepositoryError };

  export async function openDocument(
    backend: Backend,
    path: string,
  ): Promise<Result<Document, DocumentError>> {
    const result = await backend.openDocument(path);

    if (isErr(result)) {
      return result;
    }

    return ok(toDocument(result.value));
  }
  ```

  The caller sees every failure mode in the signature. An error is never
  a `string`, never an `Error` instance, never a class.
- Callers narrow with `isOk` / `isErr` and handle each `kind` with a
  `switch` that is exhaustive (the `default` branch is
  `error satisfies never`). The feature layer is the only place that
  converts an error value into UI (a toast, an inline message).
- "Not found", "already exists", "not permitted", "invalid input" are all
  error values. `throw` is not written in application code (§7).

### 2.6 State

- The store is **zustand**, built from slices: one file per slice under
  `src/state/`, each exporting `create<Name>Slice` typed as
  `StateCreator<Store, [], [], NameSlice>`, combined once in
  `src/state/store.ts`. Slices hold data and the actions that change it;
  actions call the injected `Backend` and store `Result` outcomes as
  state, never throw.
- Components select the narrowest value: `useStore((s) => s.document.title)`;
  object selections use `useShallow`. Subscribing to the whole store is
  forbidden. The store object is never passed as a prop ("no prop
  drilling of the store"); a child that needs state calls the hook.
- Derived values are computed in selectors or during render, not stored.
- Persistence is a slice that calls the backend; no `persist` middleware
  writing to `localStorage` (§2.3).

### 2.7 Index files

There are no `index.ts` barrel files (`noBarrelFile`). A module is imported
by its own path: `@/lib/tauri/commands`, `@/features/editor/use-editor`.
The one exception is a vendor wrapper directory, whose `index.ts` MAY
re-export the public surface **by name** and nothing else:

```ts
export type { Backend, Document } from "./types";
export { createBackend } from "./backend";
```

No `export * from`, no logic, no side effects.

---

## 3. Modules and imports

### 3.1 Module layout

Top to bottom, separated by blank lines:

1. `import type` lines.
2. Third-party value imports.
3. First-party (`@/...`) value imports.
4. Same-directory (`./...`) imports.
5. Exported types.
6. Module constants (`UPPER_SNAKE`).
7. Private helpers.
8. Exported functions, hooks, components.
9. `create<Thing>()` factory (vendor wrapper modules only), last.

Biome's import organiser sorts within groups; the group order above is what
it produces with the default configuration.

### 3.2 Import style

- **Named imports only.** `export default` is forbidden except in
  `*.lazy.tsx` files consumed by `React.lazy` and in `vite.config.ts`
  (lint override, §1.5).
- Several names from one module MAY share one `import { a, b } from`
  line; Biome formats and sorts them. One module per import statement;
  never import the same module twice.
- Types are imported with `import type { X }` on a separate statement
  from values; inline `type` modifiers are not used.
- **Absolute imports** via the `@/` alias everywhere except siblings in
  the same directory, which use `./name`. `../` is forbidden.
- No file extensions in import specifiers (`bundler` resolution).
- Import **modules as namespaces** only for vendor wrappers that read as a
  namespace: `import * as log from "@/lib/log"` then `log.info(...)`. All
  other imports are by name.
- No `import x as y` renames unless resolving a genuine clash.
- Side-effect imports (`import "./styles.css"`) appear only in
  `src/app/main.tsx`.
- Unused imports are removed (lint error), not suppressed.

---

## 4. Naming

| Thing | Convention | Example |
|-------|------------|---------|
| Files and directories | `kebab-case`, singular for a thing, plural for a collection | `use-editor.ts`, `components/` |
| Component files | `kebab-case.tsx`, exporting one PascalCase component of the same name | `document-pane.tsx` exports `DocumentPane` |
| Functions, variables, properties | `camelCase`, verbs for actions | `openDocument`, `findById` |
| Components | `PascalCase` | `DocumentPane` |
| Hooks | `use<Thing>` | `useDocument` |
| Types, type aliases | `PascalCase`; no `I` prefix, no `T` suffix | `Document`, `DocumentError` |
| Props types | `<Component>Props` | `DocumentPaneProps` |
| Module constants | `UPPER_SNAKE` | `const QUEUE_SIZE = 100;` |
| Discriminant values | `snake_case` string literals | `kind: "not_found"` |
| Error types | `<Domain>Error` | `DocumentError` |
| Store slices | `<Name>Slice`, `create<Name>Slice` | `DocumentSlice` |
| Wrapper interfaces | A plain noun naming the capability | `Backend`, `Editor` |
| Tauri commands (wire) | `snake_case`, identical to the Rust function | `"open_document"` |
| Tests | `describe("<Subject>")`, `it("<observable behaviour>")` | `it("returns not_found for a missing path")` |

Additional rules:

- **British spelling** in identifiers, JSDoc, and comments: `initialise`,
  `serialise`, `finalise`, `behaviour`, `colour`. Vendor API names and
  DOM/CSS property names are used as spelled by the vendor (`color` in a
  style object).
- Module privacy is "not exported"; there is no `_` prefix. A leading `_`
  marks a deliberately unused parameter and nothing else.
- Booleans read as predicates: `isInitialised`, `hasRemotes`, `empty`.
  Props that are booleans are bare adjectives (`disabled`, `open`).
- No Hungarian notation, no type suffixes (`documentArray`), no
  abbreviations that are not universal (`id`, `ctx` are fine; `doc`,
  `cfg`, `mgr`, `btn` are not).
- Factory functions that build the production instance are
  `create<Thing>`; functions that mutate an existing object in place are
  `initialise<Aspect>`.
- Event handler props are `on<Event>`; the handler implementation is
  `handle<Event>`.

---

## 5. Typing

- **Every** exported function, hook, and component has explicit parameter
  types and an explicit return type (`ReactNode` for components,
  `Promise<Result<...>>` for async). Local helpers MAY rely on inference
  for the return type when it is a single expression.
- Prefer the narrowest type that is true. `unknown` is the type of a
  value you have not yet inspected; `any` is forbidden. `object` is
  preferred over `Record<string, unknown>` when the value is merely
  passed through.
- Use `T | undefined` for absent values and check with `=== undefined`.
  `null` appears only where a vendor or the DOM produces it, and is
  converted to `undefined` at the wrapper.
- `readonly` on every property of data types and `readonly T[]` for array
  inputs; outputs are plain `T[]`.
- `type` for everything; `interface` only when a library's contract
  requires declaration merging (`ImportMetaEnv`).
- Discriminated unions over optional properties: a value that can be in
  one of several shapes has a `kind` literal, not a bag of optionals.
- Generic functions use inline type parameters; no `T = any` defaults.
- Type guards return `value is X`; they are the only place a type
  assertion (`as`) over `unknown` is written, and they check every field
  they assert.
- `satisfies` is used to check a literal against a type without widening;
  `as const` for literal tables.
- `!` (non-null assertion) is forbidden (lint). Narrow instead.
- `// @ts-expect-error <reason>` MUST carry a reason on the same line and
  an adjacent comment naming the vendor defect. Do not use `as` to
  silence a design problem; fix the types.

---

## 6. Functions and components

### 6.1 Functions

- Prefer module-level functions over classes. `class` is written only to
  satisfy a vendor contract that requires `extends` (a CodeMirror
  `ViewPlugin`, a `Y.AbstractType`), inside that vendor's wrapper.
- Parameters that configure behaviour rather than supply data are
  collected in a trailing `options` object with a named `<Name>Options`
  type; data comes first, positionally.

  ```ts
  export function render(page: Page, options: RenderOptions): Result<Canvas, RenderError> { ... }
  ```

- Default values are immutable. Never mutate a parameter (lint).
- Return early on guard conditions. Avoid deep nesting; the happy path is
  at the lowest indentation.
- One responsibility per function. If a function needs a section comment
  to separate phases, split it.
- Functions that perform I/O are `async`. Functions that do not are not.
- Dispatch on a discriminant with `switch` on `kind`, exhaustive, with
  `default: return error satisfies never;` (or equivalent) so a new
  member is a compile error.
- Arrow functions are used for callbacks passed inline; named
  declarations (`function name()`) for everything exported or hoisted.

### 6.2 Components and hooks

- Function components only; no class components, no `React.FC`.
  Signature: `export function Name(props: NameProps): ReactNode`.
  Destructure `props` in the first line of the body, not in the parameter
  list, so the props type stays visible.
- One component per file; small private subcomponents MAY live in the same
  file if they are not exported.
- Hooks begin with `use`, obey the rules of hooks (lint), and live in
  `features/<feature>/use-<thing>.ts`. A hook that talks to the backend
  takes the `Backend` from context, not from a module import.
- `useEffect` is only for synchronising with an external system (a
  CodeMirror view, a Tauri event subscription, a Yjs provider). It
  returns a cleanup that unsubscribes. It is never used to derive state,
  fetch on mount without a subscription, or run business logic.
- No manual memoisation (`useMemo`, `useCallback`, `memo`); the React
  Compiler does it. `useRef` holds vendor handles, not state.
- Presentational components (`components/`) receive data and callbacks as
  props and have no knowledge of the store or the backend.
- Props objects are typed `readonly`; children are typed `ReactNode`.
- Lazy routes/panels are `*.lazy.tsx` with a default export, loaded via
  `React.lazy` inside `src/app/`.

---

## 7. Errors and exceptions

**Do not throw.** Every failure the code can anticipate is returned as a
value (§2.5); every failure it cannot anticipate is a bug that surfaces on
its own. Writing `throw` or `class SomethingError extends Error` is
therefore almost always wrong.

- MUST NOT define `Error` subclasses.
- MUST NOT `throw` to signal a business outcome, a validation failure, a
  missing record, an invalid state, or a misuse of an API. Return an error
  value, or make the state impossible through types.
- MUST NOT re-wrap a library's thrown error in your own `Error`.
- The **only** permitted `throw` is one a third-party library's contract
  requires (for example, a React `use()` suspense boundary or a vendor
  callback whose protocol is "throw to reject"). Such a throw is confined
  to the callback the library invokes, uses the type the library
  documents, and carries a comment naming the library contract.
- A bare `throw error;` of the caught value, unchanged, is permitted inside
  a boundary `catch` (§7, below) when the caught value is not the
  expected rejection shape. It creates nothing.

Handling exceptions is equally restrained:

- `try/catch` is written **only** in a vendor wrapper under `src/lib/`,
  at the exact call that the vendor documents as throwing or rejecting.
  `lib/tauri/` is the canonical example: `invoke` rejects with the
  serialised Rust error, and the wrapper converts it:

  ```ts
  async function call<T, E extends WireError>(
    command: string,
    args: Record<string, unknown>,
  ): Promise<Result<T, E>> {
    try {
      return ok(await invoke<T>(command, args));
    } catch (rejection: unknown) {
      if (!isWireError(rejection)) {
        throw rejection; // NOTE: Not a backend error value; this is a bug and must surface.
      }

      return err(rejection as E);
    }
  }
  ```

- The `catch` parameter is `unknown` and is narrowed with a type guard
  before use. `catch {}` with an empty body is a lint error.
- Do not catch to convert into a value unless the library offers no
  non-throwing API for that outcome. Prefer the non-throwing API
  (`map.get` over `map.get()!`, `Number.parseInt` plus `Number.isNaN`
  over `JSON.parse` of untrusted text, a `safeParse` over a `parse`).
- There is no global `window.onerror` or `unhandledrejection` handler
  that swallows; one is registered in `src/app/main.tsx` solely to log
  (§9) and re-throw.
- Fail fast at start-up. A missing build-time value fails the build
  (§2.3); a failed start-up command renders the error state, it does not
  retry silently.

---

## 8. Async

- All I/O is `async`. Backend calls, file dialogs, event subscriptions,
  and Yjs provider connections return promises; nothing blocks the render
  thread with synchronous loops over large data.
- **No floating promises** (`noFloatingPromises`, type-aware). Every
  promise is `await`ed, returned, or handed to `spawn` from
  `src/lib/async.ts`:

  ```ts
  export function spawn(task: Promise<void>): void {
    task.catch((rejection: unknown) => {
      log.error("A spawned task rejected.", { rejection });
    });
  }
  ```

  Because project functions return `Result` and never reject, a rejection
  reaching `spawn` is a bug and is logged at `error`. `void promise` is
  not a handling and is forbidden.
- Event handlers that start async work are `async` arrow functions whose
  returned promise React discards; inside them every call is awaited.
  Effects are never `async`; they call `spawn` or subscribe and return the
  unsubscribe.
- Every Tauri event subscription stores its `UnlistenFn` and calls it in
  the effect cleanup; every long-running operation accepts an
  `AbortSignal` and the effect aborts it on cleanup.
- Concurrency uses `Promise.all` only when one failure should abort the
  group; otherwise `Promise.allSettled` with each outcome inspected.
- Promises are never created with `new Promise` outside a vendor wrapper
  that adapts a callback API.
- Timers (`setTimeout`, `setInterval`) are written only inside `lib/`,
  wrapped in a function that returns a cancel handle.

---

## 9. Logging

- Every module that logs imports the project's own logger,
  `import * as log from "@/lib/log"`, never `console` directly
  (`noConsole` is an error everywhere except `lib/log.ts`). `lib/log.ts`
  MAY forward to `@tauri-apps/plugin-log` so entries reach the same sink
  as the backend's `tracing` output.
- Log messages are **constant strings** ending with a full stop. Variable
  data goes in the structured second argument, never in the message:

  ```ts
  log.info("Opened the document.");
  log.warn("Ignored an unknown event.", { name: event.name });
  log.error("Failed to save.", { error });
  ```

  Template literals in log messages are forbidden.
- Levels: `debug` for wiring and lifecycle detail, `info` for state
  changes a user or operator cares about, `warn` for recoverable
  anomalies, `error` for failures that reach the user or abort an
  operation.
- Scoped context (a document id, a session id) is added through
  `log.withContext({ documentId })`, which returns a child logger, not by
  repeating the field on every call.
- Secrets never appear in fields.

---

## 10. Formatting

Formatting is mechanical and enforced by `biome check` in pre-commit and
`biome ci` in CI. Run it; do not argue with it. The rules it encodes:

- Line width 100. Two-space indentation. Double quotes. Semicolons always.
  Trailing commas everywhere. Parentheses around every arrow parameter.
  Files end with a single newline. No trailing whitespace. Unix line
  endings.
- When a call, signature, or literal does not fit on one line, Biome
  breaks it with one element per line, a hanging indent, and a **trailing
  comma** after the last element. A multi-line construct is also
  acceptable when it *would* fit on one line if a trailing-comma object or
  array literal is already split, which Biome preserves.
- JSX: one attribute per line when the opening tag does not fit; the
  closing `>` on its own line; a multi-line JSX return is wrapped in
  parentheses.
- Imports are sorted by Biome's organiser (§3.1), groups separated by a
  blank line.
- Blank lines inside a function body follow a fixed rule, not taste.
  Biome does not enforce it, so it is checked in review:

  - Consecutive **single-line simple statements** (`const` bindings,
    calls, assignments) are written contiguously with no blank line
    between them.
  - Every **compound statement** (`if`, `for`, `while`, `switch`, `try`,
    a nested `function`, a multi-line arrow) and every **statement that
    spans more than one line** (a multi-line call, literal, destructuring,
    or JSX expression) is isolated by one blank line before it and one
    after it.
  - A `return`, `break`, or `continue` is preceded by one blank line
    unless it is the only statement in the block.
  - The first statement of a block is never preceded by a blank line, the
    last is never followed by one, and `else`, `else if`, `catch`,
    `finally`, and `case` clauses are never separated from the clause above
    them.
  - Never more than one consecutive blank line inside a function. A
    comment belongs to the statement below it and sits inside that
    statement's blank-line envelope.
  - Hooks at the top of a component are consecutive single-line statements
    and sit together; the first non-hook statement starts a new group only
    if it is compound or multi-line.

  ```ts
  export async function openDocument(
    backend: Backend,
    path: string,
  ): Promise<Result<Document, DocumentError>> {
    const trimmed = path.trim();

    if (trimmed === "") {
      return err({ kind: "invalid_path", path });
    }

    const result = await backend.openDocument(trimmed);

    if (isErr(result)) {
      return result;
    }

    const document = toDocument(result.value);
    log.debug("Opened a document.", { documentId: document.id });

    return ok(document);
  }
  ```

---

## 11. Comments and JSDoc

**Less is the default.** Names and types carry the meaning; comments and
JSDoc exist only for what those cannot express. Agents tend to
over-document; resist it. A file with a JSDoc block on every function and a
comment on every JSX element is wrong, not thorough.

- JSDoc is **not** required on exported functions, hooks, components, or
  types. Being exported is not a reason to document something.
- Write a JSDoc block only to state **non-obvious behaviour**: an ordering
  constraint ("MUST be called after the store is created"), a side effect,
  a surprising return, a performance characteristic, a contract the
  signature cannot express. If the name and types say everything, there is
  no JSDoc.
- Never restate the signature; `@param`, `@returns`, `@type`, and
  `@template` are forbidden. The types already say that.
- No file-header comments or module docstrings in application code.
- JSDoc opens with `/**`, has a one-line summary ending with a full stop,
  continues directly without a blank line, and closes on its own line.
- Comments explain **why**, never **what**. A comment that paraphrases the
  next line, labels an obvious phase (`// Render the list`), or narrates
  ordinary control flow is deleted. JSX comments (`{/* */}`) follow the
  same rule and are rare.
- Comments are full sentences, capitalised, ending with a full stop.
- Use `// NOTE:` for non-obvious behaviour worth flagging, `// TODO:` for
  known follow-ups. Both are complete sentences after the marker.
- Any suppression (`// biome-ignore lint/<group>/<rule>: <reason>`,
  `// @ts-expect-error <reason>`) carries its rule and a reason.
- Banner comments (`// ====`) are reserved for warnings that would
  otherwise be silently violated by a well-meaning edit.

---

## 12. Tests

**Tests are optional.** This is a deliberate project decision. Agents MUST NOT
add tests unless explicitly asked to, MUST NOT block or delay a change because
it lacks tests, and MUST NOT mention missing tests as a defect. When a task
does include tests, the rules below apply.

- The runner is **vitest** (`pnpm add -D vitest`; `@testing-library/react`
  and `jsdom` only if component tests are requested). It is configured in
  `vite.config.ts` under `test`, not in a separate file.
- Tests live in `tests/`, mirroring the source tree:
  `tests/unit/<path under src>/<module>.test.ts`; end-to-end tests (if
  any) in a separate top-level directory.
- Tests are grouped with `describe("<Subject>")`; each case is
  `it("<observable behaviour>")`; the name is the documentation. Add a
  comment only when the behaviour under test is not clear from the name
  (§11 applies to tests too).
- Bodies follow **arrange / act / assert**, separated by blank lines. One
  behaviour per test; multiple `expect`s are fine when they describe one
  outcome.
- Shared fakes are real objects implementing the wrapper interface
  (`Backend`, `Editor`), living in `tests/fakes/`, not `vi.fn()` bags.
  Tests must run without Tauri, a network, or a real WebView.
- `vi.mock` is used only when the fake cannot be injected, at the
  narrowest module, with a comment saying why.
- Async tests are plain `async` functions; no manual promise plumbing.
- Test code obeys every rule in this document, including full types, the
  import style, and the no-throw rule (use `expect(result).toEqual(err({...}))`).

---

## 13. Pre-commit checklist for agents

Before considering any change complete:

1. `pnpm` was used for every command; nothing was installed with npm,
   yarn, or a global install; `pnpm-lock.yaml` is updated if
   `package.json` changed.
2. `pnpm exec tsc -b` passes with no new `@ts-expect-error`; every one
   that was added has a reason and a comment naming the vendor defect.
3. `pnpm exec biome ci .` passes with no new `biome-ignore`.
4. No `typeof window`, no Tauri-presence check, no polyfill, no
   `?? default` on a required value, no `any`, no `!`.
5. Dependencies point one way: `lib/` imports nothing above it, nothing
   below `features/` imports the store, no vendor package is imported
   outside its wrapper directory.
6. No `throw` and no `Error` subclass, except inside a `lib/` wrapper at
   the documented vendor boundary (or a bare re-throw) with its comment.
7. Every fallible function returns `Promise<Result<T, <Domain>Error>>`
   or `Result<T, <Domain>Error>`; every `kind` switch is exhaustive.
8. Every promise is awaited, returned, or passed to `spawn`; every
   subscription and effect returns its cleanup.
9. No `useMemo`/`useCallback`/`memo`, no `useEffect` that derives state,
   no whole-store subscription, no store passed as a prop.
10. Log calls have constant messages ending with a full stop and a
    structured fields object; no `console`.
11. If tests were requested, they sit in the mirrored location and follow
    §12.
12. No JSDoc or comment that restates what a name, type, or the next line
    already says.
13. The pre-commit hooks pass on all files.
