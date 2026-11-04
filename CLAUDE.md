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

The project hasn't been set up yet. Update this section once it has.

```bash
pnpm install          # frontend deps
pnpm tauri dev        # run the app
pnpm test             # Vitest
pnpm lint             # ESLint + Prettier check + svelte-check
cd src-tauri && cargo test
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo fmt --check
```

## Architecture rules

- **Read-only, always.** Only `src-tauri/src/repo/` may use `gix`. Never call gitoxide APIs that write, lock or refresh the index on disk. `clippy.toml` `disallowed-methods` enforces this, so don't silence it.
- **`repo/` has no Tauri dependency.** It's a plain Rust library, tested with `cargo test`. Tauri-specific code belongs in `commands.rs`, `watch/` and `app/`.
- **Rust computes, Svelte renders.** Diffing, hunks, word-level ranges and line counts are done in Rust. The frontend never diffs text.
- **The TypeScript types for the IPC API are generated** by `tauri-specta` into `src/lib/types.ts`. Don't edit that file by hand. Change the Rust types and regenerate.
- **Every response that depends on the repository carries a `generation`.** The frontend discards responses older than the latest one it has.
- **Colours come only from Primer tokens** (CSS custom properties). Never put raw colour values in components.
- **The diff list is virtualised by row.** Don't render whole files into the DOM.

## Testing

- Every Rust change in `repo/` comes with unit tests or fixture tests. Fixture repositories are built by scripts in `tests/fixtures/`. They need `git`, but only to run the tests.
- Tests that compare with `git diff` check paths, statuses, renames and `+/−` counts. They do not compare hunk boundaries (TECH.md §1).
- Svelte components get Vitest + Testing Library tests, with `api.ts` mocked.
- Run lint and all tests before calling a task done.

## Conventions

- Commit messages are in the imperative mood ("Add file tree component"), with a short subject and a body that explains why when it isn't obvious.
- Rust follows `rustfmt` and `clippy` defaults; TypeScript and Svelte follow Prettier and ESLint.
- Match the style of the surrounding code: naming, comment density, idioms.
