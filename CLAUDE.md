# CLAUDE.md

TurboDiff is a small, read-only desktop app for viewing git diffs, styled after GitHub's "Files changed" view. It runs on Linux and macOS.

## Documents

- [PRD.md](PRD.md): what the app does. This is the source of truth for features and scope.
- [TECH.md](TECH.md): how the app is built (stack, architecture, IPC API, testing, CI).
- [TASKS.md](TASKS.md): the implementation plan. Tick tasks off as they're finished.

If an implementation decision contradicts these documents, update the document in the same change. Don't let code and documents drift apart.

## Scope

Keep the app small. Before adding anything, check PRD §4 (non-goals). In particular:

- no git write operations,
- no history or log view,
- no editing or comments,
- no extra themes,
- only the keyboard shortcuts listed in PRD §6.10.

If a feature isn't in the PRD, ask before building it.

## Stack

Tauri 2 · Rust · gitoxide (`gix`) + `imara-diff` · Svelte 5 (runes) + TypeScript · Vite · pnpm · Shiki (in a Web Worker) · `@primer/primitives`.

## Commands

Needs Rust stable, Node 26 (LTS) and pnpm 12 (pinned by `packageManager` in `package.json`).

```bash
pnpm install          # frontend deps
pnpm tauri dev        # run the app (debug builds also regenerate src/lib/types.ts)
pnpm test             # Vitest
pnpm lint             # ESLint + Prettier check + svelte-check
pnpm format           # apply Prettier
cargo test --workspace        # also fails, and regenerates src/lib/types.ts, if it's stale
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Cargo commands run from the repository root, which holds the workspace `Cargo.toml`. If clippy prints "does not refer to a reachable function", a `clippy.toml` path is stale. That's only a warning, so check for it (CI fails on it).

## Architecture rules

- **Read-only, always.** Only the `turbodiff-repo` crate (`crates/turbodiff-repo/`) may use `gix`. Never call gitoxide APIs that write, lock or refresh the index on disk. `clippy.toml` `disallowed-methods` enforces this, so don't silence it.
- **`turbodiff-repo` has no Tauri dependency.** It's a plain Rust library, tested with `cargo test`. Tauri-specific code belongs in `src-tauri` (`commands.rs`, `watch/` and `app/`), which uses `turbodiff-repo` and doesn't depend on `gix` itself.
- **Rust computes, Svelte renders.** Diffing, hunks, word-level ranges and line counts are done in Rust. The frontend never diffs text.
- **The TypeScript types for the IPC API are generated** by `tauri-specta` into `src/lib/types.ts`. Don't edit that file by hand. Change the Rust types and regenerate (run `cargo test`, then commit the file).
- **Every new command needs a permission.** Add it to the app manifest in `src-tauri/build.rs` and its `allow-*` permission to `src-tauri/capabilities/default.json`. Don't grant core or plugin permissions the webview doesn't need.
- **Every response that depends on the repository carries a `generation`.** The frontend discards responses older than the latest one it has.
- **Colours come only from Primer tokens** (CSS custom properties). Never put raw colour values in components.
- **The diff list is virtualised by row.** Don't render whole files into the DOM.

## Testing

- Every Rust change in `turbodiff-repo` comes with unit tests or fixture tests. Fixture repositories are built by scripts in `tests/fixtures/`. They need `git`, but only to run the tests.
- Tests that compare with `git diff` check paths, statuses, renames and `+/−` counts. They do not compare hunk boundaries (TECH.md §1).
- Svelte components get Vitest + Testing Library tests, with `api.ts` mocked.
- Run lint and all tests before calling a task done.
- For UI changes, also check the result in the real app, in light and dark mode. [docs/checking-the-ui.md](docs/checking-the-ui.md) explains how.

## Working on a milestone

A request like "Let's work on M6" means: implement every task of that milestone in TASKS.md.

- Before writing code, read PRD.md, TECH.md and TASKS.md, then the code and `git log` from earlier milestones.
- Work on a new branch named after the milestone (for example `m6-syntax-highlighting`), cut from `main`.
- Keep going until the milestone is done. Ask only for decisions that the documents and sensible defaults can't settle: scope, UX the PRD leaves open, or documents that contradict each other.
- Work in this session only. Don't start other sessions.
- Never merge into `main`, push, tag or publish anything without asking first. When the milestone is done, report what was built, the decisions recorded in TECH.md and anything you couldn't verify, then ask whether to merge.

## Conventions

- Commit messages are in the imperative mood ("Add file tree component"), with a short subject and a body that explains why when it isn't obvious.
- Rust follows `rustfmt` and `clippy` defaults; TypeScript and Svelte follow Prettier and ESLint.
- Match the style of the surrounding code: naming, comment density, idioms.
- Use current stable versions of dependencies. pnpm 12 refuses releases that are too new. When it does, pick an older version. Never add `minimumReleaseAgeExclude` to get around it, and remove it if pnpm adds it by itself.
