# TurboDiff — Tasks

The implementation plan for v1, organised into milestones. Each milestone leaves the app working, or the backend testable, so progress can be checked along the way. References point to [PRD.md](PRD.md) and [TECH.md](TECH.md).

Tick a task (`[x]`) when its code, tests and any document updates are done.

---

## M0 — Project setup

- [x] Set up the Tauri 2 + Svelte 5 + TypeScript project with Vite and pnpm, using the layout in TECH §3.
- [x] Set up linting and formatting:
  - ESLint, Prettier and `svelte-check`, behind `pnpm lint`,
  - `rustfmt` and `clippy`.
- [x] Add `clippy.toml` with `disallowed-methods` that block gitoxide's write and lock APIs (TECH §8).
- [x] Set up Vitest and `@testing-library/svelte` (`pnpm test`).
- [x] Set up `tauri-specta` to generate `src/lib/types.ts`, with a sample command to prove it works.
- [x] Lock down Tauri capabilities so the webview can only call our own commands.
- [x] Add a CI workflow (`ci.yml`) that runs lint and tests on Ubuntu and macOS (TECH §11).
- [x] Fill in the Commands section of CLAUDE.md with the real commands.

## M1 — Repository core (Rust, no UI)

Everything here lives in the `turbodiff-repo` crate (`crates/turbodiff-repo/`) and is tested with `cargo test`.

- [x] Fixture scripts in `tests/fixtures/` covering every case listed in TECH §10.1.
- [x] Open a repository with `gix::discover`. Return a typed `NotARepository` error when there isn't one (TECH §4.1).
- [x] List branches, with the current one marked, local before remote, sorted by most recent commit.
- [x] List tags, newest first.
- [x] Commits:
  - paginated list from `HEAD`,
  - `search_commits` by message,
  - `resolve_commit_prefix` for SHAs (TECH §4.2).
- [x] Resolve a comparison:
  - work tree or commit on the compare side,
  - two-dot or three-dot, using the merge base,
  - fall back when there's no merge base,
  - empty tree when there are no commits yet (TECH §4.3).
- [x] Changed files, commit vs commit: tree diff with rename tracking, statuses, mode changes and submodule changes (TECH §4.4).
- [x] Changed files, tree vs work tree:
  - combine status results, untracked files and `.gitignore`,
  - compare each blob directly with the file on disk,
  - apply `.gitattributes` filters,
  - detect renames (TECH §4.4).
- [x] `+/−` counts for every file, using the active algorithm and whitespace setting.
- [x] Single-file diff:
  - binary and large-file checks,
  - UTF-8 decoding,
  - missing final newline,
  - Histogram or Myers,
  - hunks with configurable context (TECH §4.5).
- [x] Hide whitespace: lines are compared with whitespace removed, the original text is kept for display, and whitespace-only files are reported.
- [x] Word-level ranges: pair deleted and inserted lines, tokenise, diff the tokens, skip pairs that are too different, return UTF-16 offsets.
- [x] `get_lines` for expanding context, backed by an LRU cache of file contents.
- [x] Comparison tests against `git diff --name-status -M` and `--numstat --diff-algorithm=histogram` for every fixture (TECH §10.1).

## M2 — IPC and app services

- [x] Tauri commands and the `AppError` enum, matching the API in TECH §5. Run repository work on a blocking thread pool.
- [x] A session per window: one repository and one comparison per window, a `generation` counter, and cancellation when the comparison changes.
- [x] A settings store: load, save, apply defaults, and broadcast `settings-changed` (PRD §6.8).
- [x] Recent repositories: at most 10, deduplicated, with entries whose path is gone marked as unavailable (PRD §6.1).
- [x] One window per repository. Opening a repository that's already open focuses its window.
- [x] CLI `turbodiff [PATH] [--base] [--compare]`, plus single-instance forwarding of a second launch to the running app (TECH §4.7).
- [x] Native menus: File › Open, Open Recent, Close; app menu › Settings, Quit. Shortcuts only as in PRD §6.10.
- [x] Opening a folder by dragging it onto a window.
- [x] Restore window size and position, and the sidebar width.

## M3 — UI shell and theming

- [x] A build step that turns Primer tokens into CSS custom properties for GitHub light and dark (TECH §6).
- [x] Mode switching: System, Light, Dark. Follow OS changes live, without the wrong theme flashing at startup.
- [x] App layout: toolbar, resizable and collapsible sidebar, main pane (PRD §6.4).
- [x] Welcome screen: Open button, recent repositories, drop hint (PRD §6.1).
- [x] Empty and error states: no changes, not a repository, invalid ref, no merge base (PRD §7).
- [x] Wire `api.ts` and the session, view and settings stores to the backend.

## M4 — Sidebar and ref picker

- [ ] Summary line: number of files, total additions and deletions.
- [ ] `FileTree`: tree view with compacted single-child folders, flat view, a toggle between them, status badges, `+/−` counts (PRD §6.5).
- [ ] File selection and deselection, including `Esc`, and a "current file" marker in the all-files view.
- [ ] `RefPicker` (PRD §6.3):
  - search box, with Working tree pinned in the compare picker,
  - tabs Branches | Tags | Commits,
  - commits load more as you scroll,
  - pasting a SHA finds the commit,
  - opens on the tab of the current selection.
- [ ] Swap button and the three-dot / two-dot toggle, with its explanatory tooltip.
- [ ] Component tests for `FileTree` and `RefPicker`.

## M5 — Diff view

- [ ] A row model that turns a `FileDiff` into rows: file header, hunk header, lines, expand controls.
- [ ] A virtual list with fixed row heights, and measured heights when wrapping is on (TECH §7.3).
- [ ] Sticky file headers, with a collapse chevron, path, rename arrow, status badge, `+/−` counts and "copy path".
- [ ] Inline (unified) view, with old and new line numbers.
- [ ] Side-by-side view: changed blocks aligned with filler cells.
- [ ] Word-level highlighting in both views.
- [ ] Expand context: up, down and all, using `get_lines`.
- [ ] Load diffs on demand near the viewport, with placeholders of estimated height.
- [ ] Single-file focus, restoring the all-files scroll position when you go back (PRD §6.6).
- [ ] Special files: too large (with "Load diff"), binary, renames, mode changes, submodules, deleted, whitespace-only.
- [ ] Toolbar toggles for diff style and hide whitespace.
- [ ] Copying text without line numbers or `+`/`−` markers; soft wrap or horizontal scroll, depending on the setting.
- [ ] Component tests for building rows (inline, split, filler cells, word-level ranges).

## M6 — Syntax highlighting

- [ ] A Shiki Web Worker using the JavaScript regex engine, with GitHub light and dark themes and language grammars loaded on demand (TECH §7.4).
- [ ] Highlight complete files and combine the tokens with word-level ranges per line.
- [ ] Skip highlighting for large files and unknown languages. Fall back to plain text if the worker fails. Cache tokens.

## M7 — Live refresh

- [ ] File watcher with `notify` and a debouncer, filtered against `.gitignore`, watching `.git` refs and the index (TECH §4.6).
- [ ] Emit `repo-changed`. When the compare side is a commit, refresh only on changes in `.git`.
- [ ] Frontend: apply a refresh while keeping scroll position, selection, collapsed files and expanded context (PRD §6.9).
- [ ] `comparison-invalid`: fall back to work tree vs `HEAD` and show a notice.

## M8 — Settings window

- [ ] Settings window: appearance, theme, default diff style, font, size, tab width, wrapping, context lines, diff algorithm (PRD §6.8).
- [ ] Changes apply right away in every window. Add "Reset to defaults".

## M9 — Release

- [ ] Accessibility pass: status shown by letters and markers as well as colour, AA contrast in both modes, respect for OS font scaling and reduced motion (PRD §7).
- [ ] Performance pass on a large repository, on Linux (WebKitGTK) first, then macOS (TECH §9).
- [ ] App icon and branding.
- [ ] `release.yml`: `tauri-action` builds the AppImage, .deb and an unsigned universal .dmg, and uploads them to a draft release (TECH §11).
- [ ] A script that updates the version in `package.json`, `Cargo.toml` and `tauri.conf.json` together.
- [ ] `RELEASING.md`: the manual pre-release checklist (read-only check, performance check, installing and launching each build).
- [ ] Release v0.1.0.

---

## Open questions to settle along the way

- Large-diff thresholds (PRD §11.1): tune during M5 using real repositories.
- Commits tab, `HEAD` only or all branches (PRD §11.2): decide during M4. The default is `HEAD`.
