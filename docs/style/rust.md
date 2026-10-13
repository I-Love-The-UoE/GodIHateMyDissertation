# Rust Code Style

This document is **binding** for every agent and contributor writing Rust in
this project. It describes the general style, structure, and separation rules
for the Tauri backend and every workspace crate. Library-specific guidance
lives in separate documents that build on this one and must never contradict
it. The TypeScript frontend is described in [`typescript.md`](typescript.md).

The key words MUST, MUST NOT, SHOULD, and MAY are to be read as in RFC 2119.
When existing code conflicts with this document, **this document wins**: bring
the code you touch into line rather than copying the old pattern.

---

## 1. Runtime and tooling

### 1.1 Rust version and edition

- The toolchain is **stable Rust 1.98** on **edition 2024**. Nothing older is
  supported, and no effort is spent keeping code compilable on older
  toolchains or editions. Nightly is never used, not even for tooling.
- The repository MUST contain a `rust-toolchain.toml` pinning the exact
  stable release:

  ```toml
  [toolchain]
  channel = "1.98.1"
  components = ["rustfmt", "clippy"]
  profile = "minimal"
  ```

- `[workspace.package]` MUST declare `edition = "2024"` and
  `rust-version = "1.98"`; every crate inherits both with
  `edition.workspace = true` and `rust-version.workspace = true`.
- CI and release builds use the same pinned toolchain; the file is the single
  place a version is written.

### 1.2 Cargo workspace

The repository is **one Cargo workspace** rooted at the repository root. The
Tauri application crate (`src-tauri/`) and every domain crate (`crates/*`) are
members. There is no per-crate dependency version, no vendored copy, no
`[patch]` section, and no `path = "../other-project"` reference.

| Task | Command |
|------|---------|
| Build everything | `cargo build --workspace` |
| Add a dependency to the workspace table | `cargo add --workspace <crate>`, then reference it from the crate with `<crate>.workspace = true` |
| Run the application in development | `pnpm tauri dev` (wraps `cargo run` in `src-tauri`) |
| Format | `cargo fmt --all` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Refresh the lockfile after editing a manifest | `cargo update -w` (or let `cargo build` do it) |
| Reproducible build (CI, release) | `cargo build --workspace --locked --release` |

- `[workspace.dependencies]` is the **single source of truth** for every
  external crate and its version and features. A member crate MUST NOT write
  a version number; it writes `serde.workspace = true` and nothing else. If a
  crate needs an additional feature, the feature is added in the workspace
  table, not in the member.
- `Cargo.lock` MUST be committed.
- Development-only crates go under `[dev-dependencies]` of the crate that
  uses them (still via `workspace = true`), never in the main list.
- Lints are declared **once** under `[workspace.lints]` (§1.5); every member
  has `[lints] workspace = true`.

### 1.3 No fallbacks, no compatibility shims

The build is a single known toolchain producing a single known binary for a
small set of desktop targets. Code that hedges against a different build is
wrong, not defensive. The following are **forbidden**:

- A `[features]` table in any project crate. No optional dependencies
  (`optional = true`), no feature that selects between two implementations,
  no `#[cfg(feature = "...")]`. If something is needed it is a hard
  dependency; if it is not needed it is not in the manifest.
  `default-features = false` on a *third-party* crate is permitted to trim
  what it pulls in.
- `#[cfg(target_os = "...")]` or `#[cfg(windows)]`/`#[cfg(unix)]` anywhere
  except inside the **one wrapper module** that owns the OS API which
  genuinely differs (§2.2). Business logic never knows which OS it is on.
- `#[cfg(debug_assertions)]` to change behaviour between profiles.
- `rustversion`, `cfg_if`, `autocfg`, `build.rs` probing for compiler
  capabilities, and version-gated code of any kind.
- Pinning an older version of a crate to keep a deprecated API alive, or
  calling a `#[deprecated]` item when a current replacement exists.
- Hand-rolled polyfills of `std` items that are stable in 1.98
  (`once_cell`, `lazy_static`, `async-trait`, `thiserror`'s predecessors).
- `unwrap_or_default()`, `unwrap_or(...)`, `unwrap_or_else(...)`, or
  `.ok()` used to **mask a missing or malformed configuration value**. A
  required value that is absent is a startup error (§2.3), never silently
  replaced.

If a crate does not build on 1.98 / edition 2024, replace the crate, patch it
upstream, or fork it. Do **not** downgrade the toolchain or write around it.

Configuration follows the same principle: every required value is read once
at startup into a typed `Config` struct, and a missing or unparsable value
returns `Err(ConfigError::...)` from `Config::load`, which aborts start-up.
A default is only acceptable when the empty value is a genuine product
decision (for example, "no remotes configured" meaning "sync off"), and that
meaning MUST be stated in a comment next to the default.

### 1.4 Prefer the newest syntax

Always use the most modern construct the language provides. Everything in the
right-hand column is stable on 1.98 / edition 2024.

| Instead of | Use |
|------------|-----|
| `#[allow(lint)]` | `#[expect(lint, reason = "...")]` |
| `lazy_static!`, `once_cell::sync::Lazy` | `std::sync::LazyLock` (or `OnceLock` when initialised late) |
| `match x { Some(v) => v, None => return Err(..) }` | `let Some(v) = x else { return Err(..) };` |
| Nested `if let` / `if let ... { if cond ... }` | Let chains: `if let Some(v) = x && v.is_ready() { ... }` |
| `#[async_trait]`, `Pin<Box<dyn Future>>` in trait returns | Native `async fn` in traits, or `fn f(&self) -> impl Future<Output = T> + Send` when callers need `Send` |
| `impl Trait + 'a` to restrict a return's captures | `impl Trait + use<'a>` (edition 2024 captures every in-scope lifetime unless told otherwise) |
| `\|\| async { ... }` returning a future | `async \|\| { ... }` with `AsyncFn` / `AsyncFnMut` bounds |
| `opt.map_or(true, f)` / `opt.map_or(false, f)` | `opt.is_none_or(f)` / `opt.is_some_and(f)` |
| `if let Some(v) = opt { if p(v) { opt.take() } }` | `opt.take_if(p)` |
| `n as u32` for narrowing | `u32::try_from(n)` and handle the error |
| `format!("{}", x)` | Inline arguments: `format!("{x}")` |
| `Box<dyn Any>` downcasting from a subtrait | Trait upcasting coercion (`&dyn Sub` to `&dyn Super`) |
| `mem::replace(&mut x, Default::default())` | `mem::take(&mut x)` |
| `retain` with an inverted predicate to pull items out | `Vec::extract_if` |
| `Box<dyn std::error::Error>`, `anyhow` | A `#[derive(thiserror::Error)]` enum per domain (§2.5, §7) |
| `dyn Trait` for dependency injection | Generics over traits (`fn f<C: Context>(ctx: &C)`) |
| `.collect::<Vec<_>>()` only to iterate again | Iterate the original iterator |

`impl Trait` in associated-type position is **not** stable and MUST NOT be
written; use a named type or a generic parameter.

### 1.5 Lints

Every warning is an error: CI runs
`cargo clippy --workspace --all-targets -- -D warnings`. The lint set is
declared once at the workspace root:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
missing_debug_implementations = "warn"
unreachable_pub = "warn"
unused_qualifications = "warn"
unused_import_braces = "warn"
trivial_casts = "warn"
trivial_numeric_casts = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }

# Pedantic lints that fight this document (§11: comments are minimal).
missing_errors_doc = "allow"
missing_panics_doc = "allow"
must_use_candidate = "allow"
module_name_repetitions = "allow"

# Restriction lints that encode §1.3 and §7.
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
todo = "warn"
unimplemented = "warn"
unreachable = "warn"
unwrap_in_result = "warn"
get_unwrap = "warn"
map_err_ignore = "warn"
exit = "warn"
dbg_macro = "warn"
print_stdout = "warn"
print_stderr = "warn"
allow_attributes = "warn"
allow_attributes_without_reason = "warn"
cfg_not_test = "warn"
str_to_string = "warn"
string_to_string = "warn"
mem_forget = "warn"
infinite_loop = "warn"
```

`clippy.toml` at the root relaxes the panic lints inside test code only:

```toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-dbg-in-tests = true
allow-print-in-tests = true
```

- A lint is never silenced with `#[allow]`. `#[expect(lint, reason = "...")]`
  is the only suppression; it fails the build when the lint stops firing, so
  stale suppressions cannot accumulate.
- Do not add lints to a single crate. If a lint is wrong for the project it
  changes at the workspace root with a comment saying why.

### 1.6 rustfmt

`rustfmt.toml` at the root, stable options only:

```toml
edition = "2024"
style_edition = "2024"
max_width = 100
newline_style = "Unix"
use_field_init_shorthand = true
use_try_shorthand = true
```

`imports_granularity` and `group_imports` are nightly-only and MUST NOT be
set; the import rules in §3 are written so that stable rustfmt never rewrites
them, and they are checked in review.

---

## 2. Separation and structure

These rules apply to every crate in the workspace regardless of what it does.

### 2.1 One-way dependencies

- Code is organised into crates and modules with a **single direction of
  dependency**. Draw the arrow before writing the crate; if two crates would
  need each other, the boundary is wrong. Cargo rejects crate cycles; module
  cycles inside a crate are a design error even though the compiler accepts
  them.
- The layering, leaves first:

  1. `crates/<app>-core`: the shared error base, `Context` trait, domain
     models, pure logic. Depends on `std`, `serde`, `thiserror`, `tracing`.
  2. `crates/<app>-<vendor>` wrapper crates (`-git`, `-secrets`,
     `-process`, ...): each owns exactly one external system (§2.2) and
     depends on `-core` for the trait it implements.
  3. `src-tauri`: the application crate. Owns `tauri`, builds the concrete
     context, registers commands, and is the **only** crate that depends on
     every other one.

- Cross-cutting helpers (logging setup, small pure utilities) and
  configuration are **leaves**: everything may depend on them, they depend
  on nothing else from the project.
- Business logic never depends on `src-tauri`; `tauri::AppHandle`,
  `tauri::State`, `tauri::Window`, and `tauri::Emitter` never cross into a
  domain crate.

### 2.2 Boundaries around third parties

- Every external system (`git2`, `keyring`, `tokio::process`, the
  filesystem beyond `std::fs` basics, any network client) is wrapped in
  **one** crate whose `[dependencies]` is the only place the vendor crate is
  listed. Because the vendor crate appears in exactly one member manifest,
  Cargo itself enforces that nothing else can import it.
- The wrapper exposes a **trait** declared in `-core` (for example
  `Repository`, `Secrets`, `Processes`) and a concrete implementation named
  `<Vendor><Trait>` (`Git2Repository`, `KeyringSecrets`,
  `TokioProcesses`). The rest of the codebase types against the trait,
  never against vendor types. Vendor types do not appear in the trait's
  signatures.
- Trait methods that perform I/O are declared as
  `fn name(&self, ...) -> impl Future<Output = Result<T, E>> + Send` so
  that Tauri commands and spawned tasks can call them; methods that are
  only ever awaited on the current task MAY use `async fn`.
- Where an OS API genuinely differs, the `#[cfg(target_os)]` branches live
  inside the wrapper crate, in one module, behind a single
  platform-neutral function. Nothing outside that module sees the `cfg`.
- The wrapper exposes a `default()` function that builds the production
  instance from `Config`, so construction is in one place.

### 2.3 Configuration

- Configuration is one struct, `Config`, in the `config` module of the
  application crate, with public fields of already-parsed types
  (`PathBuf`, `u16`, `Duration`, `Vec<String>`). No getters, no builder, no
  lazy loading, no global.
- `Config::load(resolver: &tauri::path::PathResolver) -> Result<Config, ConfigError>`
  is called **once** from `run()` before anything else is constructed. Every
  failure (`MissingValue { key }`, `Invalid { key }`, `Unreadable { path }`)
  aborts start-up. Parsing happens in the same module so consumers see typed
  values only.
- Nothing else in the project reads environment variables, the
  configuration file, or the path resolver directly. `Config` is stored
  on the context (§2.4) and reached from there.

### 2.4 Dependency injection over globals

- Business logic never reaches for a global client or static resource.
  Whatever it needs is passed in, bundled in a **context** whose trait
  declares the available dependencies as associated types with accessor
  methods:

  ```rust
  pub trait Context: Send + Sync {
      type Repository: Repository;
      type Secrets: Secrets;

      fn config(&self) -> &Config;
      fn repository(&self) -> &Self::Repository;
      fn secrets(&self) -> &Self::Secrets;
  }
  ```

- Logic is generic over the context: `fn open<C: Context>(ctx: &C, ...)`.
  Callers construct the concrete context: `run()` builds `AppContext` from
  `Config` and the wrappers' `default()` functions and hands it to
  `tauri::Builder::manage`; a test builds one from in-memory fakes. Logic is
  therefore exercisable without I/O and without global mutable state.
- Dynamic dispatch (`Box<dyn Trait>`, `Arc<dyn Trait>`) is not used for
  injection. It is permitted only where a collection must hold
  heterogeneous implementations.
- The only permitted module-level state is `const` items, immutable
  `static` lookup tables, and `LazyLock` values that are pure functions of
  constants (a compiled regex, a lookup table). Never a `LazyLock<Config>`,
  never a `static` client, never `OnceLock` holding a handle.

### 2.5 Results and errors as values

Expected failures are **return values**, not panics.

- A function that can fail in an expected way returns
  `Result<T, <Domain>Error>`, where `<Domain>Error` is an enum declared in
  the domain's crate:

  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize)]
  #[serde(tag = "kind", rename_all = "snake_case")]
  pub enum DocumentError {
      #[error("The document does not exist.")]
      NotFound { path: PathBuf },
      #[error("The document is locked by another process.")]
      Locked,
      #[error("The repository could not be read.")]
      Repository(#[from] RepositoryError),
  }
  ```

  The caller sees every failure mode in the signature. Variants carry only
  the data a caller can act on; a `Message(String)` or `Other(String)`
  variant is forbidden because it hides the failure mode.
- Domain errors derive `Serialize` with `#[serde(tag = "kind")]` so a Tauri
  command can return them unchanged and the frontend receives a
  discriminated union whose `kind` values match the variant names in
  `snake_case`. The command layer is the only place that converts an error
  value into a wire response, and that conversion is serialisation, not
  rewrapping.
- `?` is used only when the error type is identical or a `From` impl exists
  that was written deliberately in the receiving domain (`#[from]` on a
  variant counts). A `From` impl for a third-party error type exists only
  inside that vendor's wrapper crate. `map_err(|_| ...)` that discards the
  cause is forbidden (`clippy::map_err_ignore`); carry it as `#[source]`.
- `anyhow`, `eyre`, `Box<dyn std::error::Error>`, and `impl std::error::Error`
  returns are forbidden in **every** crate, including the application
  crate. `main` returns `Result<(), StartupError>`; `StartupError` is an
  ordinary domain enum wrapping `ConfigError` and `tauri::Error`.
- "Not found", "already exists", "not permitted", "invalid input" are all
  error values. Panicking is reserved for bugs (§7).

### 2.6 Data access

- Each stored resource has exactly one type responsible for reading and
  writing it, named `<Resource>Store` (`DocumentStore`, `SessionStore`).
  It takes the wrapper trait as a generic parameter and holds it in a
  private field.
- Store methods are small and map one operation to one typed model or a
  collection of models. They return models, never vendor objects or raw
  bytes, and contain no business rules.
- Paths, query strings, and serialisation formats live only inside stores.

### 2.7 Crate roots and `mod.rs`

A crate's `lib.rs` and every `mod.rs`-equivalent (a `foo.rs` with a `foo/`
directory) declare submodules and re-export the public surface, and nothing
else:

```rust
mod error;
mod store;

pub use error::DocumentError;
pub use store::DocumentStore;
```

- `mod.rs` files are not used; the parent module is `foo.rs` beside
  `foo/`.
- No logic, no `static` items, no `impl` blocks in a crate root or parent
  module. The one exception is `src-tauri/src/lib.rs`, which contains
  `run()` and nothing else.
- Everything that is not re-exported is `pub(crate)` at most. `pub` on an
  item that is not reachable from the crate root is caught by
  `unreachable_pub`.

---

## 3. Modules and imports

### 3.1 Module layout

Top to bottom, separated by blank lines:

1. Inner attributes (`#![expect(...)]`), rare.
2. `use` items from `std` / `core`.
3. `use` items from external crates.
4. `use` items from the workspace (`crate::`, other `<app>-*` crates).
5. Type aliases (`type X = ...;`).
6. Constants and statics (`const`, `static`, `LazyLock`).
7. Private helpers.
8. Types (`struct`, `enum`, `trait`) each immediately followed by its `impl`
   blocks.
9. Public functions.
10. `default()` factory (wrapper crates only), last before tests.
11. `#[cfg(test)] mod tests`, last.

### 3.2 Import style

- **One path per `use` item.** Never `use std::{fmt, io};`, never
  `use crate::document::{Document, DocumentError};`. Each line imports one
  thing; stable rustfmt keeps the lines sorted and never merges them.
- **Absolute paths** everywhere: `crate::` inside a crate, the crate name
  across crates. `super::` is permitted only as `use super::*;` in a
  `#[cfg(test)] mod tests`. `self::` is never written.
- Import **modules** and call through them when the module is a coherent
  namespace: `use crate::document;` then `document::open(ctx, path)`.
  Import **names** when they are types or traits used as such:
  `use <app>_core::Context;`.
- No glob imports except `use super::*;` in test modules. No `as` renames
  unless resolving a genuine clash, and then the rename is `_`-prefixed
  only if the item is private.
- Fully qualified paths inline (`std::fs::read_to_string(...)`) are
  permitted for a single use; a second use promotes it to a `use` item.
- Local (function-scope) `use` is permitted only for enum variants inside a
  `match` on that enum (`use Status::*;` directly above the `match`) or
  for a trait needed by one expression. Nothing else.
- Unused imports are removed, not `#[expect]`ed.
- Macros are invoked by path (`tracing::info!`, `serde_json::json!`), never
  imported with `use`, so the originating crate is visible at the call
  site.

---

## 4. Naming

| Thing | Convention | Example |
|-------|------------|---------|
| Crates | `kebab-case`, prefixed with the application name | `<app>-core`, `<app>-git` |
| Modules, files | `snake_case`, singular nouns for a thing, plural for a collection of things | `document.rs`, `stores/` |
| Functions, methods, variables | `snake_case`, verbs for actions | `open_document`, `find_by_id` |
| Types, traits, enum variants | `PascalCase` | `Git2Repository`, `NotFound` |
| Type aliases | `PascalCase` | `type CommitId = String;` |
| Constants, statics | `UPPER_SNAKE` | `const QUEUE_SIZE: usize = 100;` |
| Lifetimes | short lowercase, descriptive when more than one | `'a`, `'doc` |
| Generic parameters | Single capital letter, or `PascalCase` noun when more than one | `T`, `C: Context` |
| Boundary traits | A plain noun naming the capability | `Repository`, `Secrets`, `Processes` |
| Vendor implementations | `<Vendor><Trait>` | `KeyringSecrets` |
| Error enums | `<Domain>Error` | `DocumentError` |
| Stores | `<Resource>Store` | `DocumentStore` |
| Tauri commands | `snake_case` verb phrases; the Rust name is the wire name | `open_document` |
| Test functions | `<observable_behaviour>` inside `mod tests` | `returns_not_found_for_missing_path` |

Additional rules:

- **British spelling** in identifiers, doc comments, and comments:
  `initialise`, `serialise`, `finalise`, `behaviour`, `colour`. Vendor
  API names are used as spelled by the vendor.
- Visibility is the privacy marker; there is no `_` prefix for private
  items. A leading `_` means "deliberately unused" and nothing else.
- Booleans read as predicates: `is_initialised`, `has_remotes`, `empty`.
  Methods returning `bool` are `is_*`/`has_*`; fields are bare adjectives.
- No Hungarian notation, no type suffixes (`document_vec`), no
  abbreviations that are not universal (`ctx`, `id`, `db` are fine; `doc`,
  `cfg`, `mgr`, `repo` are not).
- Factory functions that build the production instance are called
  `default` (a free function, not `Default::default`, which is reserved
  for value types).
- Functions that build a composed value are `create_<thing>` or `new`;
  functions that mutate in place are `initialise_<aspect>`. Conversions
  follow `std`: `as_*` (cheap borrow), `to_*` (expensive copy), `into_*`
  (consume).

---

## 5. Typing

- Every function signature is fully explicit; return types are never
  omitted except for `()` on functions that return nothing. Closures are
  annotated when the inferred type is not obvious at the call site.
- Prefer the narrowest type that is true. Identifiers are newtypes
  (`struct DocumentId(Uuid);`), not bare `String` or `u64`. Quantities with
  units use `Duration`, `PathBuf`, `NonZeroU32`, not raw integers.
- Parameters borrow (`&str`, `&Path`, `&[T]`, `impl AsRef<Path>` only when
  the function is called with both `&str` and `&Path` in practice); return
  values own (`String`, `PathBuf`, `Vec<T>`).
- `Option<T>` is unwrapped with `let ... else`, `if let`, `match`, or the
  combinators (`map`, `and_then`, `is_some_and`, `is_none_or`,
  `ok_or`). Never `.unwrap()`, `.expect()`, or indexing that can panic.
- Generics over traits for abstraction (§2.4); `dyn Trait` only for
  heterogeneous collections. Trait objects that must be shared are
  `Arc<dyn Trait + Send + Sync>`.
- `Send + Sync` bounds are declared on the boundary traits themselves, not
  repeated at every use site.
- Numeric conversions use `From`/`TryFrom`; `as` is permitted only between
  types where it is lossless (`u8 as u32`) and the pedantic cast lints are
  silent.
- Structs that cross the IPC boundary derive
  `serde::Serialize`/`serde::Deserialize` with
  `#[serde(rename_all = "camelCase")]` so that the frontend sees its own
  naming convention; enums use `#[serde(tag = "kind", rename_all = "snake_case")]`.
- Every public type derives `Debug`; value types additionally derive
  `Clone, PartialEq, Eq` (and `Hash`, `Copy` where cheap and meaningful).
- `#[must_use]` is written on functions whose result is the whole point
  (builders, pure computations); `Result` is already `must_use`.
- Any `#[expect]` on a typing lint carries a `reason` that explains why the
  checker is wrong. Do not use `transmute`, `unsafe` (forbidden), or
  `Any` downcasting to silence a design problem; fix the types.

---

## 6. Functions and types

### 6.1 Functions

- Prefer free functions over types with a single method or types that only
  hold associated functions. A struct exists to hold state.
- Parameters that configure behaviour rather than supply data are
  collected into a small `Options` struct with `Default` rather than
  trailing booleans; data comes first, positionally.
- Return early on guard conditions with `let ... else` and `?`. Avoid deep
  nesting; the happy path is at the lowest indentation.
- One responsibility per function. If a function needs a section comment
  to separate phases, split it. `clippy::too_many_lines` is the backstop,
  not the target.
- Functions that perform I/O are `async` and take the context. Functions
  that do not are not `async` and do not take it.
- Dispatch on a discriminant with `match`; `match` is exhaustive, so a
  wildcard arm is written only when the enum is `#[non_exhaustive]` or
  belongs to a vendor, and then it returns an explicit error value.
- Iterator chains are preferred over index loops; a chain longer than four
  adaptors is broken into named intermediate bindings.

### 6.2 Types

- Fields are private unless the type is a plain data carrier. Read-only
  state is exposed through `fn name(&self) -> &T` accessors, not `pub`.
- Data-carrying types are plain structs with `pub` fields and derives, no
  constructor beyond struct literal syntax, and no behaviour beyond derived
  accessors.
- Types with invariants have a `new` that returns `Result<Self, E>` or an
  `Option<Self>`; there is no way to construct an invalid instance.
- Prefer composition over inheritance-by-trait. Implement a vendor trait
  only when extending the vendor's machinery is the whole point, and keep
  the impl thin.
- Builder patterns are used only for types with more than four optional
  fields; otherwise an `Options` struct with `..Default::default()`.
- `impl` blocks are ordered: inherent `impl` first (constructors, then
  accessors, then methods), then `std` trait impls (`Default`, `Display`,
  `From`), then project trait impls.
- Resources that must be released implement `Drop` only when releasing
  cannot fail; otherwise expose an explicit `async fn close(self)`.
- Trait method bodies in trait definitions are omitted (no default
  implementations) unless the default is the only sensible one for every
  implementor.

---

## 7. Errors and panics

**Do not panic.** Every failure the code can anticipate is returned as a
value (§2.5); every failure it cannot anticipate is a bug that surfaces on
its own. The lints in §1.5 make `unwrap`, `expect`, `panic!`, `todo!`,
`unimplemented!`, and `unreachable!` build errors outside tests.

- MUST NOT call `.unwrap()`, `.expect()`, `panic!`, `todo!`,
  `unimplemented!`, `unreachable!`, `assert!`, or index with `[]` where the
  index is not provably in bounds, in any non-test code, including the
  application crate and `main`.
- MUST NOT signal a business outcome, a validation failure, a missing
  record, an invalid state, or a misuse of an API by panicking. Return an
  error value, or make the state impossible through types and construction.
- MUST NOT re-wrap a vendor error with a message string. Carry it as a
  `#[source]` or `#[from]` field inside the wrapper crate's error enum so
  the chain is preserved.
- MUST NOT use `anyhow`, `eyre`, `Box<dyn Error>`, or `String` as an error
  type anywhere (§2.5).
- The **only** permitted panic is one a third-party library's contract
  requires (for example, a `Drop` impl that discovers an invariant
  violation, or a `tauri::Builder` callback whose signature returns `()`
  and has no error channel). Such a panic is confined to the callback the
  library invokes, carries an `#[expect(clippy::panic, reason = "...")]`
  naming the contract, and is logged at `error` first.

Handling failures is equally restrained:

- Never `catch_unwind` to continue after a panic. The process aborts on
  panic in release (`panic = "abort"` in `[profile.release]`).
- Do not convert a vendor `Result` into a value you then ignore. If an
  outcome does not matter, the call site says so with `let _ = ...;` and a
  comment stating why the outcome is irrelevant.
- Prefer the non-panicking API when one exists: `get` over `[]`,
  `checked_add` over `+` on untrusted values, `try_from` over `as`,
  `Path::try_exists` over `Path::exists`.
- Fail fast at start-up. `Config::load` returns the first error it meets;
  `run()` propagates it; `main` returns it. Do not add a nicer message, a
  retry, or a default.

---

## 8. Async

- The runtime is **Tokio**, provided by Tauri. All I/O is `async`. Blocking
  calls (`std::fs` on large files, `std::process::Command::output`,
  `std::thread::sleep`, `git2` operations) are forbidden on the runtime
  thread; a wrapper crate that must call a blocking vendor API does so via
  `tokio::task::spawn_blocking` inside the wrapper, and exposes an `async`
  method.
- Hold a reference to every task created with `tokio::spawn`. Store the
  `JoinHandle` on the owning struct or in a `JoinSet` so that it can be
  awaited or aborted; a dropped handle is a lint error in review.
- Long-running background loops wrap each unit of work in a function that
  returns `Result`, log the error with the structured cause, and continue.
  A loop never exits because of one failed iteration.
- Use `tokio::sync::Mutex` only when the guard is held across an `.await`;
  otherwise `std::sync::Mutex`. Never hold a `std` guard across an await.
- Cancellation is cooperative: every background task takes a
  `tokio_util::sync::CancellationToken` (owned by the `-core` crate's
  `Processes` abstraction) and checks it in `tokio::select!`.
- `async fn` in traits and `impl Future + Send` returns are used as
  described in §2.2; `#[async_trait]` is forbidden (§1.3).
- Never build a runtime (`tokio::runtime::Builder`, `#[tokio::main]`) in a
  library crate. Tauri owns the runtime; tests use `#[tokio::test]`.
- Streams of events to the frontend are emitted through one function in
  the command layer; domain crates return values or `impl Stream`, never
  call `emit`.

---

## 9. Logging

- Logging is `tracing`. The subscriber is initialised once in `run()` and
  nowhere else. `println!`, `eprintln!`, `dbg!`, and the `log` crate are
  forbidden (lints in §1.5).
- Log messages are **constant strings** ending with a full stop. Variable
  data goes in structured fields, never in the message:

  ```rust
  tracing::info!("Opened the repository.");
  tracing::warn!(path = %path.display(), "Ignored an untracked file.");
  tracing::error!(error = %error, "Failed to commit.");
  ```

  `format!` or `{}` placeholders in a log message are forbidden.
- Macros are called by path (`tracing::info!`), not imported.
- Levels: `trace` for per-item detail inside loops, `debug` for wiring and
  lifecycle detail, `info` for state changes an operator cares about,
  `warn` for recoverable anomalies, `error` for failures that reach the
  user or abort an operation.
- Scoped context (a document id, a sync job id) is attached with a span
  (`#[tracing::instrument(skip_all, fields(document_id = %id))]` or
  `tracing::info_span!`), not by threading identifiers through every call
  or repeating them on every event.
- Secrets never appear in fields; the `Secrets` trait's types do not
  implement `Display` or `Debug` for their inner values.

---

## 10. Formatting

Formatting is mechanical and enforced by `cargo fmt --all --check` in
pre-commit and CI. Run it; do not argue with it. The rules it encodes:

- Line length 100. Four-space indentation. Files end with a single newline.
  No trailing whitespace. Unix line endings.
- One blank line between items; no blank line after an opening brace or
  before a closing brace.
- When a call, signature, or literal does not fit on one line, rustfmt
  breaks it with one element per line, a hanging indent, and a **trailing
  comma** after the last element. A multi-line construct with a trailing
  comma is also acceptable when it *would* fit on one line, if it improves
  readability or future diffs; rustfmt preserves it when the struct literal
  or array is already multi-line.
- `use` items are one path per line (§3.2), grouped `std` / external /
  workspace, each group sorted alphabetically (rustfmt sorts within a
  group), groups separated by a blank line.
- `match` arms are one per line; an arm whose body is a single expression
  stays on the arm's line, otherwise the body is a block.
- Blank lines inside a function body follow a fixed rule, not taste.
  rustfmt does not enforce it, so it is checked in review:

  - Consecutive **single-line simple statements** (`let` bindings, calls,
    assignments) are written contiguously with no blank line between them.
  - Every **compound statement** (`if`, `match`, `for`, `while`, `loop`,
    a block, a nested `fn`, a closure spanning multiple lines) and every
    **statement that spans more than one line** (a multi-line call,
    literal, or `let`) is isolated by one blank line before it and one
    after it.
  - A `return`, `break`, `continue`, a `?`-bearing early exit that is the
    whole statement, and the function's tail expression are preceded by one
    blank line, unless they are the only statement in the block.
  - The first statement of a block is never preceded by a blank line, the
    last is never followed by one, and `else`, `else if`, and `match` arms
    are never separated from the line above them.
  - Never more than one consecutive blank line inside a function. A comment
    belongs to the statement below it and sits inside that statement's
    blank-line envelope.

  ```rust
  pub async fn open<C: Context>(ctx: &C, path: &Path) -> Result<Document, DocumentError> {
      let Some(root) = ctx.config().workspace_root() else {
          return Err(DocumentError::NoWorkspace);
      };

      let absolute = root.join(path);
      let bytes = ctx.repository().read(&absolute).await?;
      let text = String::from_utf8(bytes).map_err(|error| DocumentError::NotUtf8 {
          path: absolute.clone(),
          valid_up_to: error.utf8_error().valid_up_to(),
      })?;

      if text.is_empty() && !ctx.config().allow_empty_documents {
          return Err(DocumentError::Empty { path: absolute });
      }

      let id = DocumentId::from_path(&absolute);
      tracing::debug!(document_id = %id, "Opened a document.");

      Ok(Document { id, path: absolute, text })
  }
  ```

---

## 11. Comments and doc comments

**Less is the default.** Names and types carry the meaning; comments and doc
comments exist only for what those cannot express. Agents tend to
over-document; resist it. A file with `///` on every item and a comment on
every block is wrong, not thorough.

- Doc comments (`///`) are **not** required on public items. Being `pub` is
  not a reason to document something; `missing_docs` is deliberately off.
- Write a `///` only to state **non-obvious behaviour**: an ordering
  constraint ("MUST be called before `run`"), a side effect, a surprising
  return, a performance characteristic, a contract the signature cannot
  express. If the name and types say everything, there is no doc comment.
- Never restate the signature, list parameters, or write `# Errors` /
  `# Panics` sections (the lints that demand them are off). Never write
  `# Examples` in application code.
- No crate-level (`//!`) docs in application code.
- Doc comments open with a one-line summary sentence ending with a full
  stop; further lines continue directly without a blank line.
- Comments explain **why**, never **what**. A comment that paraphrases the
  next line, labels an obvious phase (`// Loop over documents`), or
  narrates ordinary control flow is deleted.
- Comments are full sentences, capitalised, ending with a full stop.
- Use `// NOTE:` for non-obvious behaviour worth flagging, `// TODO:` for
  known follow-ups. Both are complete sentences after the marker.
- Any suppression (`#[expect(lint, reason = "...")]`) carries the lint
  name and a reason. `#[allow]` is a build error.
- Banner comments (`// ====`) are reserved for warnings that would otherwise
  be silently violated by a well-meaning edit.

---

## 12. Tests

**Tests are optional.** This is a deliberate project decision. Agents MUST NOT
add tests unless explicitly asked to, MUST NOT block or delay a change because
it lacks tests, and MUST NOT mention missing tests as a defect. When a task
does include tests, the rules below apply.

- Unit tests live in a `#[cfg(test)] mod tests` at the bottom of the module
  they test, opening with `use super::*;`. Integration tests live in the
  crate's `tests/` directory, one file per subject.
- Each test function is named `<observable_behaviour>` (no `test_` prefix;
  the attribute already says it). Add a comment only when the behaviour
  under test is not clear from the name (§11 applies to tests too).
- Bodies follow **arrange / act / assert**, separated by blank lines. One
  behaviour per test; multiple asserts are fine when they describe one
  outcome.
- Shared fakes are real types implementing the boundary trait or the
  `Context` trait, living in `<app>-core`'s `testing` module (compiled only
  under `cfg(test)` or as a dev-dependency). No mocking crates
  (`mockall`), no monkeypatching. Tests must run without any external
  service, filesystem path outside `tempfile`, or network.
- Async tests use `#[tokio::test]`. Do not build runtimes by hand.
- `unwrap`, `expect`, `panic!`, and `assert!` are permitted in test code
  (`clippy.toml`, §1.5) and nowhere else.
- Test code obeys every other rule in this document, including the import
  style and the blank-line rule.

---

## 13. Pre-commit checklist for agents

Before considering any change complete:

1. `cargo fmt --all --check` passes.
2. `cargo clippy --workspace --all-targets -- -D warnings` passes with no
   new `#[expect]`; every `#[expect]` that was added has a `reason`.
3. No `[features]`, no `optional = true`, no `#[cfg(target_os)]` outside
   the owning wrapper module, no version numbers outside
   `[workspace.dependencies]`.
4. Dependencies point one way; no vendor crate appears in more than one
   manifest; no `tauri::` type outside `src-tauri`.
5. No `.unwrap()`, `.expect()`, `panic!`, `todo!`, `unimplemented!`,
   `anyhow`, `Box<dyn Error>`, or `String` error, outside tests and the one
   library-contract exception, with its comment.
6. Every fallible function returns `Result<T, <Domain>Error>`; every `?`
   crosses an identical error type or a deliberate `From`.
7. `tracing` calls have constant messages ending with a full stop and
   structured fields; no `println!`/`dbg!`.
8. `use` items are one path per line, absolute, grouped, and no glob
   outside `mod tests`.
9. If tests were requested, they sit in the right place and follow §12.
10. No doc comment or comment that restates what a name, type, or the next
    line already says.
11. The pre-commit hooks pass on all files.
