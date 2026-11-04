# TurboDiff — Technical Design

| | |
|---|---|
| **Status** | Draft |
| **Companion to** | [PRD.md](PRD.md) |
| **Last updated** | 2026-11-04 |

This document explains *how* TurboDiff is built. It covers the stack, the architecture, the data model, the backend and frontend APIs, and the approach to testing and releases. The PRD is the source of truth for *what* the app does. Section references like "PRD §6.3" point there.

---

## 1. Decisions at a glance

| Area | Choice | Why |
|---|---|---|
| App shell | **Tauri 2** | Small binaries, low memory use, Rust backend, web UI that makes the GitHub look easy to reproduce. |
| Frontend | **Svelte 5** (runes) + **TypeScript 6.0** | Little boilerplate. Fine-grained reactivity suits a large, frequently refreshed diff list. TypeScript 7 isn't used yet because `typescript-eslint` and `svelte-check` don't support it. |
| Build tooling | **Vite** + **pnpm**, on Node LTS (26) | Standard Tauri and Svelte setup. The pnpm version is pinned by `packageManager` in `package.json`. |
| Git access | **gitoxide (`gix`)** | Pure Rust, no dependency on an installed `git`, structured data, direct access to file contents. |
| Diff engine | **`imara-diff`** (what gitoxide uses), called directly | Histogram and Myers algorithms. Full control over hunks, context and whitespace handling. |
| Default algorithm | **Histogram**, with Myers available in Settings | Usually produces more readable diffs. |
| Diff computation | **In Rust.** The frontend only renders | Fast. Keeps the UI simple. |
| Syntax highlighting | **Shiki** in a **Web Worker** | Official GitHub Light and Dark themes and VS Code grammars, without blocking the UI. |
| Colours | **`@primer/primitives`** (GitHub Primer tokens) as CSS custom properties | Matches GitHub exactly. |
| File watching | **`notify`**, with our own debounce | Works across platforms. `notify-debouncer-full` was the first plan, but it debounces each path separately, so a burst like a checkout would arrive as several batches (§4.6). |
| Tests | Rust unit and fixture tests; **Vitest** + Testing Library for Svelte | See §10. |
| CI / release | **GitHub Actions** + **`tauri-action`** | Lint and tests on Linux and macOS; release builds when a version tag is pushed. |
| macOS signing | **Deferred** | Unsigned builds until there's an Apple Developer account. |

**Accepted trade-off of using gitoxide.** TurboDiff shows the same files, statuses, renames and changes as `git diff`. Hunk boundaries may occasionally differ. Rename detection reimplements git's own similarity estimate and pairing (§4.4), so it agrees with `git diff -M` even near the 50% threshold. Function names after `@@` hunk headers are not shown in v1.

---

## 2. Architecture

```
┌──────────────────────────── Tauri process ────────────────────────────┐
│                                                                        │
│  Rust backend (src-tauri)                 Webview (src)                │
│  ┌────────────────────────┐   commands    ┌──────────────────────────┐ │
│  │ commands.rs  (IPC API) │◀─────────────│ api.ts  (typed wrappers)  │ │
│  ├────────────────────────┤               ├──────────────────────────┤ │
│  │ repo/   open, refs,    │   events      │ stores  (Svelte runes)   │ │
│  │         compare,       │─────────────▶│ components:              │ │
│  │         changes,       │ repo-changed  │  Toolbar, RefPicker,     │ │
│  │         filediff       │               │  FileTree, DiffList,     │ │
│  ├────────────────────────┤               │  FileDiff, Settings      │ │
│  │ watch/  notify,        │               ├──────────────────────────┤ │
│  │         debounce       │               │ highlight.worker.ts      │ │
│  ├────────────────────────┤               │  (Shiki)                 │ │
│  │ app/    windows, CLI,  │               └──────────────────────────┘ │
│  │         settings,      │                                            │
│  │         recent repos   │                                            │
│  └────────────────────────┘                                            │
└────────────────────────────────────────────────────────────────────────┘
```

- **One Tauri process, one webview window per repository** (PRD §6.1). Each window has its own repository session in Rust, keyed by window label.
- **Rust holds the authoritative state:** the open `gix::Repository`, the current comparison, and the cached list of changed files. The frontend asks for data and renders it.
- **Data loads lazily.** Opening a comparison returns only the **list of changed files** with their counts. The diff for each file is fetched only when it's about to scroll into view (§7.3).

---

## 3. Repository layout

```
turbodiff/
├── PRD.md
├── TECH.md
├── package.json            # pnpm workspace root (frontend)
├── vite.config.ts
├── src/                    # Svelte frontend
│   ├── main.ts
│   ├── App.svelte
│   ├── lib/
│   │   ├── api.ts          # typed invoke() wrappers + event listeners
│   │   ├── types.ts        # mirrors Rust DTOs (generated, see §5)
│   │   ├── stores/         # session, settings, selection (runes)
│   │   ├── components/     # Toolbar, RefPicker, FileTree, DiffList, …
│   │   ├── diff/           # row model, split/inline layout, virtual list
│   │   ├── highlight/      # Shiki worker, client, languages
│   │   └── theme/          # Primer token mapping, mode switching
│   └── styles/             # global.css, themes.css (generated, §6)
├── scripts/                # build-tokens.js (§6)
├── Cargo.toml              # Cargo workspace: src-tauri, crates/turbodiff-repo
├── clippy.toml             # disallowed gix write APIs, for the whole workspace (§8)
├── crates/
│   └── turbodiff-repo/     # gitoxide + imara-diff logic (no Tauri types)
│       ├── src/
│       └── tests/          # fixture tests (§10.1)
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/       # Tauri permission sets
│   └── src/
│       ├── main.rs
│       ├── commands.rs     # #[tauri::command] surface
│       ├── events.rs       # events sent to the webview
│       ├── session.rs      # per-window session, generation, cancellation
│       ├── error.rs        # AppError
│       ├── watch/          # file watcher (no Tauri types)
│       └── app/            # windows, CLI, settings, recent repos, menus
├── tests/fixtures/         # scripts that build fixture repositories
└── .github/workflows/
```

Repository access is a separate library crate, `turbodiff-repo`, in a Cargo workspace whose root is the repository root. It does not depend on Tauri, so it can be tested as an ordinary Rust library, and its tests build without building Tauri. `src-tauri` uses it through a path dependency and doesn't depend on `gix` itself, so Cargo enforces both rules. The shared target directory is `target/` at the root.

---

## 4. Backend (Rust)

### 4.1 Opening a repository

- `gix::discover(path)` finds the enclosing repository (PRD §6.1). A bare repository, or a path outside any repository, returns a typed error (`NotARepository`).
- The repository is opened with default trust settings and is **only ever read** (§8).
- **Windows:** opening a repository whose work-tree path is already open in a window just focuses that window.

### 4.2 Refs (for the picker)

- **Branches:** the current branch first, then local branches, then remote-tracking branches, each group sorted by most recent commit. Symbolic refs such as `origin/HEAD` are left out. Each `RefItem` has the short and full name, the peeled commit and a time.
- **Tags:** sorted newest first, using the tagger date, or the commit date for lightweight tags (git's `creatordate`). Tags that don't point to a commit are left out.
- **Commits:** a paginated walk from `HEAD` by commit time, newest first (`offset`, `limit` = 50), as `git log` orders them. Each entry has the full and short SHA (7 characters), subject, author name and commit time. Only `HEAD` is walked, not every branch (PRD open question 2, settled in M4): it's what `git log` shows, and a commit on another branch can be picked through that branch or by pasting its SHA.
- **SHA lookup:** if the search text is 4–40 hex characters (either case), `resolve_commit_prefix` looks it up in the object database. An ambiguous prefix returns all matching commits, newest first. Other text returns nothing.
- **Search** for branches and tags runs in the frontend over the loaded lists. Search over commit messages runs in the backend (`search_commits(query, limit)`), matching the whole message without regard to case, and stopping after `SEARCH_COMMITS_SCAN_LIMIT` (10,000) commits.
- Times are seconds since the epoch. They're `i64` in Rust and exported to TypeScript as `number`.

### 4.3 Comparison model

```rust
enum CompareSide { WorkTree, Commit { spec: String } }
enum Mode { ThreeDot, TwoDot }
struct Comparison { base: String, compare: CompareSide, mode: Mode, ignore_whitespace: bool }
```

`base` and `spec` are full ref names (`refs/heads/main`) or SHAs from the picker, or any revision from the CLI. They're resolved with gitoxide's `rev_parse` and peeled to a commit. One that doesn't resolve is `RefNotFound`. `Comparison::default()` is work tree vs `HEAD`, three-dot.

The *left* side is always a tree:

| compare | mode | left tree |
|---|---|---|
| commit C | TwoDot | tree of base |
| commit C | ThreeDot | tree of `merge_base(base, C)` |
| work tree | TwoDot | tree of base |
| work tree | ThreeDot | tree of `merge_base(base, HEAD)` |

- When there's no merge base (unrelated histories), three-dot falls back to two-dot, and the result has `no_merge_base` set so the UI shows a notice.
- When `HEAD` has no commits yet, `HEAD` resolves to the empty tree (on either side), and there's no merge base to look for.

### 4.4 Listing changed files

**Commit vs commit:** gitoxide's tree-to-tree diff, with its own rename tracking off. Renames are found afterwards, as for the work tree. Each change gets a status: `Added`, `Modified`, `Deleted`, `Renamed{from, similarity}`, `TypeChanged` (between file, symlink and submodule), plus a mode change if there is one.

**Tree vs work tree** (the default view):
1. Use `gix` status to collect the paths that differ between the tree and the index, and between the index and the work tree. Add **untracked files** from the directory walk, which respects `.gitignore` and `info/exclude`.
2. For the union of these paths, compare the **left tree's blob** with the **current file on disk** directly. This merges staged and unstaged changes into a single result, which is what `git diff HEAD` does.
3. Run rename detection on the resulting adds and deletes, with untracked files counting as adds.

Status runs with its own rename tracking off and lists untracked files one by one, not as collapsed folders. A submodule's new side is the commit its checkout's `HEAD` points to. One that isn't checked out counts as unchanged. With `core.fileMode` off, the executable bit comes from the index.

Work-tree content goes through the same **filters** git would apply (line-ending conversion and `ident` from `.gitattributes`), using gitoxide's filter pipeline. That way, a file that is only "changed" because of line endings doesn't show up. **External filter drivers** (`filter.<name>.clean`, `.process`, such as Git LFS) **never run**: they're removed from the in-memory configuration when the repository is opened, because a driver can write into `.git` (LFS stores objects there). Files that use one are compared as they are on disk, so LFS files show as changed pointer text.

**Renames** follow git's algorithm (`diffcore-rename.c`, `diffcore-delta.c`), not gitoxide's rename tracker, so results agree with `git diff -M`:
1. identical content first, preferring a source with the same file name,
2. then files whose name is unique among the remaining sources and destinations, if at least 75% similar,
3. then every remaining pair, best score first, each source used once, at 50% or more.

Similarity is git's estimate: content is cut into spans at each newline or every 64 bytes (a CR before a LF is skipped in text), the bytes of matching spans are counted, and the count is divided by the larger size. Only regular files take part in inexact matching. Above 1,000 × 1,000 candidates, only identical and same-name renames are found, as with git's default `diff.renameLimit`.

**Line counts** (`+N −M`) are computed for every file while building the list, with `imara-diff`, the active algorithm and whitespace setting, so the sidebar totals are always accurate. They match `git diff --numstat` with the same algorithm. Binary files have no counts. A submodule counts as git shows it, one `Subproject commit` line per side. With hide whitespace on, files whose only changes are whitespace stay in the list with `+0 −0` (git leaves them out), so the UI can show "Whitespace-only changes". Files above the large-file threshold (§4.5) get their counts but not their diff, until the user clicks "Load diff".

### 4.5 Diffing one file

`get_file_diff(path, opts)` returns a `FileDiff`. In `turbodiff-repo` this is `Repo::file_diff(file, options)`, where `file` is a `ChangedFile` from the cached list (it holds both versions) and `DiffOptions` has `algorithm`, `ignore_whitespace`, `context` and `force`. Steps:

1. **Load both sides:** the tree blob and/or the filtered work-tree content.
2. **Binary check**, following git's rule (a NUL byte in the first 8,000 bytes) or a `-diff`/`binary` attribute in `.gitattributes`. Binary files return `Binary { old_size, new_size }`.
3. **Large-file check:** if the file is over 500 KB or has more than 1,000 changed lines and `force` isn't set, return `TooLarge { additions, deletions }`. The thresholds are the provisional constants `LARGE_FILE_BYTES` and `LARGE_DIFF_LINES` (PRD open question 1). The size check comes before diffing, using the counts from the file list.
4. **Decode** as UTF-8. Content that is not valid UTF-8 is treated as binary (PRD §8).
5. **Split into lines,** recording any missing final newline on either side.
6. **Prepare for hide whitespace:** when it's on, lines are compared using a key with all whitespace removed (as `git diff -w` does), while the original text is kept for display.
7. **Diff** with `imara-diff` (`Histogram` or `Myers`) to get a list of changed line ranges. Above 20,000 lines, both sides together (`HISTOGRAM_MAX_LINES`), Myers is used whatever the setting. imara-diff's Histogram is quadratic at worst: in the M9 performance pass, a 100,000-line lock file with every other line changed took 9 s to count (Myers: 2 ms), and held up the whole file list. git's own histogram falls back to Myers in a similar way. The same applies to the file list's `+/−` counts.
8. **Build hunks** with `context` lines (from Settings), merging hunks whose context would touch or overlap (at most 2 × `context` unchanged lines between changes), as git does. As in git's headers, a side with no lines starts at the line before the change (`@@ -0,0 +1,3 @@`). `Line.text` has no `\n`. With hide whitespace on, unchanged lines show the new text. If nothing is left of the diff but the text differs, the result is `WhitespaceOnly`.
9. **Word-level ranges:** inside each changed block, delete lines are paired with insert lines in order, the first deletion with the first insertion and so on. Each pair is split into tokens (words, runs of whitespace, punctuation) and diffed with `imara-diff`. The changed character ranges are returned as `[start, end)` UTF-16 offsets, ready to use in JavaScript. Pairs that are too different (less than 30% of their bytes in common) get no word-level ranges, as on GitHub. Words are runs of letters, digits and `_`. Every other non-space character is its own token. With hide whitespace on, whitespace tokens compare equal and are never marked.

**Expand context:** the response includes `old_line_count` and `new_line_count`. The frontend requests hidden lines with `get_lines(path, side, start, end)`, where `start` and `end` are 1-based and inclusive, clamped to the file. The backend keeps the most recently loaded file contents in an LRU cache of up to 64 MB (`CACHE_BYTES`), keyed by object id, so expanding doesn't re-read files. A work-tree file's id is the hash of its filtered content, so an edited file never hits a stale entry. Content that changed on disk after the list was built isn't cached under the old id.

### 4.6 Watching for changes

- **One watcher per window session** (`src-tauri/src/watch/`, plain Rust with no Tauri types). It lives in the window's `WindowSlot`, starts when a session is placed there, and stops when the slot is replaced or the window closes. It's started on the blocking pool, since listing the folders to watch can take a while.
- **What's watched:**
  - the git dir, non-recursively, for `HEAD` and `index`;
  - the common dir's `packed-refs` and `refs/` (recursively). The git dir and common dir differ in a linked worktree (`Repo::git_dir`, `Repo::common_dir`).
  - **The work tree.** On Linux, inotify needs a watch per folder, so only folders git doesn't ignore are watched, one by one (`Repo::unignored_dirs`), and folders created later get a watch when their creation is seen. `target/`, `node_modules/` and other ignored folders are never watched. On macOS, one recursive FSEvents watch on the root covers the work tree and a `.git` inside it.
- **Filtering:** events go through a pure function, `classify`, which sorts them into a `Trigger { git, work_tree }`.
  - Reads (`Access` events, ours included) are dropped.
  - In `.git`, only `HEAD`, `index`, `packed-refs` and `refs/**` count; `*.lock`, `objects/`, `logs/` and the like don't.
  - Work-tree paths that git ignores are dropped (`Repo::ignored`, which reads `.gitignore` files, `info/exclude` and `core.excludesFile` again on every batch, and never ignores a tracked file).
- **Debounce:** our own trailing debounce. A change is reported once the repository has been quiet for 300 ms (PRD §6.9), or after 2 s of changes that never stop (`MAX_DELAY`). Filtering comes first, so ignored files never delay or cause a refresh.
- **When compare is a commit** (not the work tree), only a `git` trigger refreshes (`should_refresh` in `app/refresh.rs`). A work-tree comparison refreshes on either, since a moved `HEAD` moves its base.
- **A refresh** (`Session::begin_refresh`) starts a new generation for the same comparison and cancels the listing in progress. It keeps the cached `Changes` until the new list is done, so `get_file_diff` and `get_lines` keep working, tagged with the generation of the list they came from. The new list is compared with the old one (`Changes::paths_changed_since`: added, removed, or a different entry or content on either side), and the window gets `repo-changed { comparison, changes: ChangeSet, filesChanged, head }`. A newer refresh or `set_comparison` cancels one still running.
- **A ref that no longer resolves** (`RefNotFound` while refreshing) sends `comparison-invalid { name }`. The frontend falls back to work tree vs `HEAD` and shows a notice naming the ref (PRD §6.9). Other refresh errors, such as a file read halfway through a write, are logged, and the next change tries again.

### 4.7 App services

- **Windows and sessions** (`src-tauri/src/app/windows.rs`, `session.rs`): every repository window is created from Rust with the label `repo-N`, the lowest number free. Each one has a slot in `AppState`, holding its `Session` or nothing while it shows the welcome screen. Opening a folder (from `open_repo`, the menu, a drop or the command line) opens the repository on the blocking pool, then decides under one lock where it goes (`app/placement.rs`): the window that already shows that work-tree root is focused; otherwise the window the request came from is used if it's empty; otherwise a new window opens. A window that's already loaded gets `repo-opened`; a new window asks with `get_window_state` when its page loads. Errors from requests with no caller waiting (menu, drop, second launch) go to the window as `open-failed`. If the first launch's path isn't a repository, the welcome window opens and `get_window_state` returns the error once. Closing a window drops its session and cancels any listing in progress.
- **Generation and cancellation:** a session's `generation` starts at 0 and goes up each time `set_comparison` is called, and on each refresh (§4.6). Starting a generation cancels the previous one's listing through a `turbodiff_repo::Cancel` flag, which `changes` checks between files, and a listing that finishes for an older generation is dropped (`Cancelled`) instead of replacing the cached `Changes`. `get_file_diff` and `get_lines` look files up in the cached list by path.
- **CLI:** `turbodiff [PATH] [--base REF] [--compare REF]`, parsed with `clap` (`app/cli.rs`) rather than `tauri-plugin-cli`, because the same parser has to handle the arguments a second launch forwards, and it can be unit-tested. Without `PATH` the welcome screen is shown, and `--base`/`--compare` need a `PATH`. A relative `PATH` is resolved against the launch's working directory. `tauri-plugin-single-instance` sends the arguments of a second launch to the running process, which opens or focuses the right window (or focuses an empty window, or opens one, for a launch without `PATH`). Arguments are parsed before the plugin runs, so `--help`, `--version` and usage errors print in the terminal. The CLI accepts any revision expression and resolves it with gitoxide's `rev_parse`. When the repository is already open, a launch that names refs sends that window `repo-opened` with the new comparison.
- **Settings:** JSON stored in `settings.json` in the platform config directory, which Tauri names after the app identifier `com.albertoventurini.turbodiff` (`~/.config/com.albertoventurini.turbodiff/` or `~/Library/Application Support/com.albertoventurini.turbodiff/`), through `tauri-plugin-store`, used only from Rust. Stored values are read field by field, so a missing or invalid value gets its default without resetting the others. `set_settings` takes a patch: numbers are clamped to their ranges (PRD §6.8) and a tab width other than 2, 4 or 8 is ignored. An empty `codeFont` means the platform's default monospace font. Changes are sent to all windows with a `settings-changed` event. Settings that change the file list (the diff algorithm) take effect when the frontend calls `set_comparison` again, which each repository window does as soon as the algorithm changes. `reset_settings` restores the defaults of everything the Settings window shows; the sidebar width isn't shown there, so it stays.
- **Monospace fonts** for the code font setting come from `list_monospace_fonts` (`app/fonts.rs`), which reads the installed fonts with the `fontdb` crate (pure Rust, on Linux and macOS) and returns the family names of the faces marked monospaced, without duplicates and sorted ignoring case. Fonts are read once, the first time they're asked for, on the blocking pool. The list is the same as `fc-list :spacing=mono`, so it includes the odd font that declares itself monospaced, such as Noto Color Emoji.
- **Recent repositories:** in the same store, at most 10 work-tree roots, most recent first, deduplicated. Whether each one still exists is checked when the list is read (`available`). Changes are sent as `recent-changed` and rebuild the Open Recent menu.
- **Window size, position and sidebar width:** `tauri-plugin-window-state` saves each window's geometry by label when the app exits and restores it when a window with that label is created, so reusing the lowest free `repo-N` label puts a new window where that window was last time. The sidebar width is the `sidebarWidth` setting.
- **Native menus** (`app/menu.rs`), built with Tauri's menu API. macOS: app menu › Settings…, Quit; File › Open…, Open Recent, Close Window; Edit › Copy, Select All (the system items, which the webview needs for the clipboard). Linux has no app menu, so File holds Open…, Open Recent, Settings…, Close Window and Quit. The only accelerator is Quit's `Mod+Q` (PRD §6.10). Open uses `tauri-plugin-dialog`'s folder picker from Rust; the webview has no dialog permission and asks for the picker with `show_open_dialog`. Settings… opens the `settings` window (`index.html?window=settings`).
- **Drag and drop:** the window's drag-drop event, handled in Rust. The first dropped path that is a folder gets opened, targeting the window it was dropped on.

---

## 5. IPC API

All commands are `async`. Repository work runs on Tauri's blocking thread pool. Commands that can fail return `Result<T, AppError>`. `AppError` is an enum tagged by `kind` (`notARepository`, `refNotFound`, `noRepository`, `fileNotFound`, `cancelled`, `io`, `internal`), so the UI can show the matching empty or error state (PRD §7). `cancelled` means a newer comparison replaced the request, and `fileNotFound` that the path isn't in the current file list; the UI ignores both. Commands act on the calling window's session.

| Command | Returns |
|---|---|
| `get_window_state()` | `WindowState { repo: RepoOpened \| null, error: AppError \| null }`, what the window shows when its page loads |
| `open_repo(path)` | nothing; the window that shows the repository gets `repo-opened` (§4.7) |
| `show_open_dialog()` | nothing; the folder picker, then as `open_repo`, with errors as `open-failed` |
| `list_branches()` / `list_tags()` | `Versioned<RefItem[]>` |
| `list_commits(offset, limit)` / `search_commits(query, limit)` | `Versioned<CommitItem[]>` |
| `resolve_commit_prefix(prefix)` | `Versioned<CommitItem[]>` |
| `set_comparison(Comparison)` | `ChangeSet { generation, files: FileEntry[], totals, noMergeBase }` |
| `get_file_diff(path, force)` | `Versioned<FileDiff>` |
| `get_lines(path, side, start, end)` | `Versioned<string[]>` |
| `get_settings()` / `set_settings(patch)` / `reset_settings()` | `Settings` |
| `list_monospace_fonts()` | `string[]` |
| `list_recent()` / `remove_recent(path)` | `RecentRepo[]` |

`Versioned<T>` is `{ generation, value: T }`. `RepoOpened` is `{ repo: RepoInfo, comparison: Comparison }`, where `RepoInfo` is `{ root, name, head: HeadInfo }`.

**Events:** `settings-changed` (`Settings`, to every window), `recent-changed` (`RecentRepo[]`, to every window), `repo-opened` and `open-failed { error }` (to one window, §4.7), and `repo-changed { comparison, changes, filesChanged, head }` and `comparison-invalid { name }` (to one window, §4.6). `repo-changed` carries the whole `ChangeSet`, so the frontend needs no second request; a window that has asked for another comparison since ignores it.

**Shared types** (Rust structs that are serialised to JSON) get TypeScript definitions generated with **`specta`/`tauri-specta`**, so `src/lib/types.ts` never falls out of sync with the Rust code.

- The data types returned by `turbodiff-repo` derive `serde::Serialize` and `specta::Type` (`specta` isn't a Tauri crate), so commands can return them directly. `serde` and `specta` versions are shared through `[workspace.dependencies]`.
- Tauri 2 support in `tauri-specta` has only been released as release candidates, so `specta`, `tauri-specta` and `specta-typescript` are pinned to exact versions (`=`) in `Cargo.toml`, as the crate recommends.
- **One TypeScript type per Rust type.** By default `tauri-specta` splits any type with an `Option` field into `X_Serialize` and `X_Deserialize` (`oldPath: string | null` versus `oldPath?: string | null`) and exports `X` as their union. The builder turns this off (`disable_serde_phases`). Results from Rust have their optional fields as `T | null`, which is what serde sends. The one input with optional fields, `SettingsPatch`, marks them `#[specta(optional)]`, so the frontend can leave them out.
- 64-bit integers (times, file sizes) are exported as `number` (`dangerously_cast_bigints_to_number`), since none comes near 2^53.
- Debug builds rewrite `types.ts` on startup. A `cargo test` test compares the committed file with freshly generated output. If they differ, it rewrites the file and fails, so CI catches a stale `types.ts`.

```ts
type FileEntry = {
  path: string; oldPath: string | null; status: 'A'|'M'|'D'|'R'|'T';
  similarity: number | null; additions: number; deletions: number;
  binary: boolean; modeChange: [string, string] | null; submodule: [string, string] | null;
};
type FileDiff =
  | { kind: 'text'; hunks: Hunk[]; oldLineCount: number; newLineCount: number;
      noNewlineAtEof: { old: boolean; new: boolean } }
  | { kind: 'binary'; oldSize: number | null; newSize: number | null }
  | { kind: 'tooLarge'; additions: number; deletions: number }
  | { kind: 'whitespaceOnly' };
type Hunk = { oldStart: number; oldLines: number; newStart: number; newLines: number; lines: Line[] };
type Line = { kind: 'ctx'|'add'|'del'; oldNo: number | null; newNo: number | null; text: string; words: [number, number][] | null };
```

Every response that depends on the repository includes the `generation` it was computed for (`ChangeSet.generation`, or `Versioned`). For ref and commit lists it's the generation when the request started. The frontend discards a response whose `generation` is older than the one it already has.

---

## 6. Theming

- **Tokens:** `scripts/build-tokens.js` (`pnpm build:tokens`) reads the resolved colour values from `@primer/primitives` (`dist/docs/functional/themes/light.json` and `dark.json`, which include the diff-specific `diffBlob-*` tokens) and writes them as CSS custom properties to `src/styles/themes.css`, one block per `[data-theme="github"][data-mode="light|dark"]`, sorted by name, with `color-scheme` set for the mode. **The output is committed**, not built on install or dev: it only changes when the package does, so a Primer upgrade shows up as a reviewable diff, and the app, lint and tests don't need an extra build step. A Vitest test fails when the committed file doesn't match what the script generates from the installed package. Mode-independent Primer tokens (sizes, spacing, radii, typography) are `:root` stylesheets that `src/styles/global.css` imports from the package directly.
- **Components use only these variables,** never raw colour values. A Vitest test checks that no `.svelte` or `.css` file outside `themes.css` contains a colour literal, and that every `var(--…)` they use is defined.
- **Contrast** (PRD §7): `src/lib/contrast.test.ts` checks WCAG AA (4.5:1) in both modes for every text colour the UI puts on a background: body and muted text, counts, status badges, notices, line numbers, hunk headers, added, deleted and word-highlighted lines, find matches and controls. Many dark-mode tokens are translucent, so each background is composited over the ones beneath it, down to `bgColor-default`. Syntax highlighting colours aren't checked; they're GitHub's own pairings with these backgrounds.
- **A theme** is a named set of tokens plus a Shiki theme name. v1 has one: `github` (`github-light` / `github-dark` in Shiki). Adding a theme means adding one entry to `THEMES` in `scripts/build-tokens.js` and one in `src/lib/theme/theme.svelte.ts`.
- **Mode:** `System` follows `prefers-color-scheme` and Tauri's window theme-change event (whichever reports a change), and switches live. `Light` and `Dark` override it. On Linux, tao reads the colour scheme from the XDG desktop portal and sets GTK's dark preference, which WebKitGTK's `prefers-color-scheme` follows.
- **No flash at startup:** every window is created from Rust with an initialization script (`src-tauri/src/app/boot.js`, filled in by `app/boot.rs`) that carries the current settings. It runs before the page's own scripts and sets `data-theme` and `data-mode` on `<html>`, and leaves the settings in `window.__TURBODIFF_SETTINGS__` so the settings store starts from them (the sidebar width is right on the first render, too). `index.html` links `global.css` as a blocking stylesheet, so the first paint already has the tokens, in dev as well. After that, the frontend keeps the attributes in step with the settings and the OS. Neither needs a window or plugin permission: the theme event only needs `core:event:allow-listen`.
- The native window decorations and menu bar follow the OS, not a `Light` or `Dark` override.

---

## 7. Frontend (Svelte)

### 7.1 State

These are Svelte rune stores, one set per window:

- `session`: repository info, the comparison, the `ChangeSet`, `generation`, loading and error state, a notice for failures that leave the window as it was (`open-failed`), and `fallback`, the ref that disappeared after `comparison-invalid`. A `repo-changed` that arrives while a `set_comparison` is pending replaced that listing in the backend, so it counts as its result. Hide whitespace lives here, as part of the `Comparison`.
- `view`: selected file, the file at the top of the all-files view (`current`, which the sidebar marks; M5's diff list sets it), tree or flat file list, diff style (inline or split), collapsed files, whether the sidebar is collapsed and whether the find bar is open (§7.5). The diff style starts at the default from Settings, and every window switches to a new default when it changes (PRD §6.8); other settings changes leave the toolbar's choice alone. The tree or flat choice isn't saved (PRD §6.11).
- `diffs` (`DiffStore`, owned by `DiffList` rather than shared through context): each file's loaded diff and the unchanged lines revealed by expanding context, keyed by path. It's emptied when a new list replaces the old one (`session.listing`: a `set_comparison` result) or the context-lines or algorithm settings change, since every diff depends on them. A refresh (`session.refresh`, from `repo-changed`) only touches what changed (§7.3).
- `highlights` (`HighlightStore`, also owned by `DiffList`): syntax highlighting tokens by path and Shiki theme (§7.4), emptied when `session.listing` changes.
- `system` (`SystemMode`): whether the OS is in dark mode, shared through context so `DiffList` can pick the Shiki theme for the current mode.
- `settings`: shared with every window through the `settings-changed` event.
- `recent`: the recent repositories for the welcome screen, following `recent-changed`.

The stores are classes in `src/lib/stores/`, created by `App.svelte` and handed to components through Svelte context, so component tests can pass in their own.

`api.ts` wraps the generated bindings: commands that can fail throw an `ApiError` holding the `AppError`. **Stale responses:** the session store keeps the latest `generation` it has seen and drops any response computed for an older one (`accept`, `current`). Errors carry no generation, so an error is shown only if it answers the latest `set_comparison` request, and `cancelled` is never shown. A newly opened repository counts generations from zero again, since its backend session is new.

### 7.2 Components

`Toolbar` · `RepoView` (toolbar, sidebar with a draggable divider whose width is saved as `sidebarWidth` when the drag ends, main pane) · `Notice` · `RefPicker` (search, Working tree pinned, tabs Branches | Tags | Commits, infinite scroll, SHA lookup) · `ModeToggle` · `SegmentedControl` · `Sidebar` (summary line, tree / flat toggle) · `FileTree` (tree or flat; compacted single-child folders) · `StatusBadge` · `DiffList` (virtual list, sticky header, horizontal scrolling) · `FindBar` · `DiffRow` (one row of any kind, inline or split) · `FileHeader` · `Welcome` · `SettingsWindow` · `EmptyState`.

**Sidebar** (PRD §6.5). The tree lists folders before files, each in path order (numbers in natural order, case ignored), as GitHub does; the flat list is in path order. `orderedFiles` in `src/lib/fileTree.ts` gives the same order, for the diff list to follow (PRD §6.6). A renamed file sits at its new path, with the old one in its tooltip. Clicking a file sets `view.selected`, and clicking it again or pressing `Esc` clears it. A selected file that leaves the list is deselected, and a collapsed one is forgotten. Folders the user collapsed stay collapsed while the list refreshes. The tree isn't virtualised: it's one small row per file, unlike the diff list.

**Settings window** (PRD §6.8). `SettingsWindow` is what `index.html?window=settings` shows: no repository session, only the settings store. Three groups (Colours, Diff, Code), each setting a row with a label, a short hint and a control: segmented controls for the choices, number inputs for font size and context lines, and selects for the theme (GitHub only, disabled) and the code font ("Default monospace", then the installed fonts; a saved font that isn't installed stays in the list, marked "not installed"). Every change is saved at once with `set_settings`, which sends `settings-changed` to every window. Number inputs save when they change, clamped to their range, and text that isn't a number puts the saved value back. **Reset to defaults** calls `reset_settings`. The Linux File menu stays in this window too, since `Mod+Q` is attached to it.

**Ref picker** (PRD §6.3). Each opening reloads the branch and tag lists, so they're never stale, and keeps the previous ones until the new ones arrive, which lets the picker open on the right tab at once. For a ref from the command line, which may be a short name or a revision such as `main~3`, the tab is found from the loaded lists: a tag wins over a branch of the same name, as in git, and anything else is a commit. The Commits tab loads 50 commits at a time, and the next page when the list is scrolled to within 200 px of its end. Typing searches the backend after a 150 ms pause: commit messages on the Commits tab, and, when the text looks like a SHA, `resolve_commit_prefix` on every tab, so a pasted SHA is offered under "Commits with this SHA" even on Branches or Tags. `Esc` closes the picker without also deselecting a file. The swap button is disabled while compare is the working tree, which can't be a base. The three-dot / two-dot toggle explains itself in a native tooltip.

**Diff view** (PRD §6.6). `buildRows` in `src/lib/diffRows.ts` turns the ordered files and their loaded diffs into the row list; it's a pure function, unit-tested apart from the list.
- **Rows:** a file header; for each hidden run of unchanged lines a hunk-style row with its expand buttons in the gutter, carrying the `@@ … @@` header when the run ends right above a hunk (a hunk starting at line 1 gets a header row with nothing to expand); the lines (inline) or old/new cell pairs (split); messages for special files; a placeholder until the diff loads; and the card's bottom edge. Once the gap above a hunk is fully revealed, its header goes, since the lines run on. Row keys are built from the path and line numbers, so they survive loading, expanding and refreshes.
- **Expand context:** *expand up* and *expand down* reveal 20 lines next to the change (`EXPAND_STEP`); a run of 20 or fewer gets a single *expand all*. A run at the top of the file can only expand up and one at the end only down. Lines come from `get_lines` on the new side (unchanged lines are the same on both; with hide whitespace on, unchanged lines show the new text anyway), and their old numbers follow from the hunk boundaries around the gap.
- **Special files:** binary (sizes when known), too large (with "Load diff", which asks again with `force`), whitespace-only, a mode change (a message above the content diff, or alone), "renamed without changes" and "empty file" for diffs without hunks, and a failed load. A submodule's message comes straight from `FileEntry.submodule`, without loading a diff. Renames show `old → new` and the similarity in the header. "No newline at end of file" is a marker icon at the end of the line, on the side it applies to.
- **Copying:** line numbers and `+`/`−` markers are CSS generated content (`attr()`), and headers, hunk rows and messages aren't selectable, so a copy holds only code. In split view, a selection stays on the side where it started.
- **Horizontal scrolling,** while wrapping is off: every line shares one offset, applied as a transform inside a clipping box, so file headers, line numbers and markers stay in place, and both split columns move together. The offset comes from a scrollbar under the list (with a track tall enough for overlay scrollbars) and from sideways or Shift wheel scrolling. Its range is the longest line among the loaded diffs, estimated in characters and corrected by the rendered lines' measured widths.
- **Sizes:** the line height is the font size × 20/13 (20 px at 13 px, as on GitHub). Line-number gutters are as wide as the largest line number among the loaded diffs.
- **Reduced motion:** nothing in the UI moves: scrolling is never animated and there are no transitions. The only animations are the delayed "Loading…" texts, which appear at once rather than fade, and `global.css` turns animations off altogether under `prefers-reduced-motion`.
- **OS font scaling:** fixed row heights are in CSS px, while the UI text is in rem. GTK's text scaling (`text-scaling-factor`, or `GDK_DPI_SCALE` for one process) scales CSS px and rem alike in WebKitGTK, so rows keep fitting their content; checked at 1.5× in M9. On macOS, display scaling is uniform in the same way.

### 7.3 Rendering large diffs

- **`DiffList` is virtualised** at the level of individual **rows**: file headers, hunk headers, lines and expand controls all go into one flat row list. Rows have fixed heights while wrapping is off. With wrapping on, row heights are measured and cached, starting from an estimate.
- **Sticky file headers** come from the virtual list: whichever file owns the first visible row has its header pinned at the top.
- **Diffs load on demand:** a file's diff is fetched when it comes within about two screen-heights of the viewport. Until then it's shown as a placeholder of the estimated height, based on its line counts.
- **Keeping the place:** the list remembers the row at the top and the scroll position relative to it (and relative to its file's header, for a placeholder that gets replaced). Whenever row offsets change (a diff loads, wrapped rows are measured, context is expanded or a file collapses), the scroll position is set so that row stays where it was. Collapsing the file whose header is pinned brings that header to the top.
- **Single-file focus** (PRD §6.6) gives the same component a row list for one file. The scroll position of the all-files view is saved and restored when you go back.
- **Live refresh** (PRD §6.9): `DiffStore.refresh` forgets files that left the list and keeps unchanged files as they are, expanded context included. A changed file keeps showing its old diff, marked stale, so its rows keep their keys and the anchor holds. It loads again when it's near the viewport. Its new diff then reveals the same new-side line numbers that were expanded before, where they're still hidden unchanged lines (`hiddenRuns`). If lines moved, those numbers may now show different lines; that's accepted. A stale diff can't be expanded and isn't highlighted.
- **Split view** is built from the same hunks: within each changed block, deletions go to the left column and additions to the right, aligned row by row, with blank filler cells on the shorter side.

### 7.4 Syntax highlighting

- Shiki runs in a **Web Worker** (`src/lib/highlight/highlight.worker.ts`) using its JavaScript regex engine, so no WASM is needed. Only `shiki/core` and the engine are bundled with the worker. The worker is built as an ES module (`worker.format: 'es'` in `vite.config.ts`), so it can load grammars as separate chunks; the CSP's `default-src 'self'` covers it.
- **Languages** are a curated list of about 50 common ones in `src/lib/highlight/languages.ts`. `languageFor(path)` picks one from a few whole file names (`Dockerfile`, `Makefile`, `CMakeLists.txt`, `Cargo.lock`, …) or the extension, ignoring case. Each grammar is its own chunk, loaded the first time a file in that language is highlighted. Files in other languages are plain text.
- Shiki highlights **complete old and new file contents**, rather than isolated lines, so multi-line constructs like block comments and strings are right. The text comes from `get_lines(path, side, 1, lineCount)`, which the backend serves from its content cache: the old side only when the file has deletions, the new side whenever it has lines. Deleted lines take their tokens from the old side; added and unchanged lines from the new side (with hide whitespace on, unchanged lines show the new text). In split view, an unchanged line's left cell uses the right cell's line number for this.
- The worker returns each line as `[end, style]` runs with a per-file table of inline styles (colour, italic, bold). Runs in the theme's default foreground get no style, so the diff's own Primer colours apply to them. `spans` in `diffRows.ts` combines the runs with the word-level ranges; highlighting that runs past the end of a line is dropped for that line. Syntax colours come from Shiki's GitHub themes as data and are applied as inline styles, so no colour literals appear in the source.
- Highlighting is **skipped** for files with more than `LARGE_DIFF_LINES` changed lines, for a side longer than `LARGE_FILE_BYTES` (500,000 UTF-16 units, which also catches files shown with "Load diff"), and for unknown languages. Lines over 1,000 characters stay plain, and Shiki gives up on a line after 500 ms. If the worker can't start or fails, it's stopped and everything is plain text from then on; a file that fails to highlight on its own stays plain. Rows render plain text first and pick up colours when they arrive, without changing height, so highlighting never blocks the first view of a diff.
- **Cache:** `HighlightStore` (`src/lib/stores/highlights.svelte.ts`, owned by `DiffList` beside `DiffStore`) keeps tokens by path and Shiki theme, and is cleared whenever a new list replaces the old one. A file whose diff changed in a refresh drops its tokens when the new diff arrives, and is highlighted again. Switching between light and dark keeps both, so switching back is instant. Files are highlighted when their diff is loaded and they're within the load-ahead window (§7.3).

### 7.5 Find in page

Neither WebKitGTK nor WKWebView has a find bar of its own, and the webview could only search the rows the virtual list has rendered anyway, so `Mod+F` (PRD §6.10) opens TurboDiff's own find bar (`FindBar.svelte`, inside `DiffList`):

- **What's searched:** the text of the lines in the row list (`findMatches` in `src/lib/find.ts`): case-insensitive, non-overlapping, in row order, the left side of a split row before the right. File paths, hunk headers, messages and the bodies of collapsed files aren't searched, since they aren't lines of the diff. Searching text isn't diffing, so it doesn't break "Rust computes, Svelte renders".
- **Loading:** while there's a query, `DiffStore.loadAll` loads the diffs of every searched file, four at a time. The count shows `+` until they've all arrived. Files over the large-diff thresholds aren't searched until "Load diff" is clicked, the same as what the page shows.
- **Current match:** a new query takes the first match at or below the top of the viewport, and keeps following it as more diffs load, until the user steps with Enter, Shift+Enter or the arrow buttons. Stepping scrolls the match into view, a third of the way down, and scrolls sideways when lines don't wrap.
- **Marks:** `spans()` cuts each line at match boundaries as well as word ranges and highlighting tokens, and the row wraps matches in `<mark>`: outlined attention-muted for every match, solid attention-emphasis for the current one.
- `Esc` in the find bar closes it (and doesn't go back to all files). The query is kept for the next `Mod+F`, which selects it.

---

## 8. Read-only guarantee

The PRD requires that TurboDiff never modifies the repository (PRD §8, §10). This is guaranteed by how the code is built:

- Only the `turbodiff-repo` crate imports `gix`. `src-tauri` has no `gix` dependency.
- External filter drivers never run (§4.4).
- `gix` is built with **default features off**, enabling only what reading needs (`revision`, `status` and what they pull in). Write-capable modules such as checkout (`worktree-mutation`), `merge`, `notes` and `credentials` aren't compiled in.
- `clippy.toml`, at the workspace root so it applies to every crate, uses **`disallowed-methods`** to block gitoxide's write APIs: index writes, ref edits, object writes (including `filter::Pipeline::worktree_file_to_object`, which writes a blob as a side effect), config writes and lock acquisition. The paths come from the `gix` version in `Cargo.lock` and must be checked again when `gix` is upgraded. CI runs clippy with warnings treated as errors, so calling one fails the build. A configured path that no longer resolves only makes clippy print a warning ("does not refer to a reachable function"), which `-D warnings` doesn't turn into an error, so CI also fails when that text appears in clippy's output.
- Status is computed **without refreshing the index on disk.** File timestamp information stays in memory and is never written back.
- Tauri **capabilities** give the webview no filesystem access. It can only call the commands listed in §5. Our commands are declared in the app manifest (`build.rs`), so each one needs its `allow-*` permission in `capabilities/default.json`, and no core or plugin permissions are granted beyond what the frontend actually uses: `core:event:allow-listen` and `allow-unlisten` for the events. The store, dialog and window-state plugins are used only from Rust and grant the webview nothing. The CSP allows only the app's own resources.

A `turbodiff-repo` test (`tests/read_only.rs`) runs every repository operation on the fixtures and checks that no file in `.git` or the work tree changed, timestamps included. The running app has no automated before-and-after check, by choice. Before each release, the PRD's read-only criterion is checked manually: run the app against a repository, then compare `git status`, the reflog and the checksum of `.git/index` before and after.

---

## 9. Performance approach

These are qualitative goals (PRD §8), met by design rather than by benchmark gates:

- Opening a comparison sends **counts only**. Diffs are fetched per file, only for files near the viewport.
- Work runs **off the UI thread:** Tauri's async commands use a blocking thread pool for git work, and highlighting runs in a Web Worker.
- **Refreshes are cheap:** after a debounced change, only the file list is rebuilt. Diffs of files whose content IDs haven't changed are reused from the cache.
- Responses that are no longer needed (an older `generation`) are discarded. Listing changed files checks a cancellation flag (`turbodiff_repo::Cancel`) between files, so changing the comparison stops the previous listing (§4.7).
- **Watch out for:** WebKitGTK on Linux handles very large DOMs less well than Chromium. Row-level virtualisation is what makes this manageable, so performance should be checked on Linux first.

**M9 performance pass** (Linux, WebKitGTK, release build), on `scripts/make-large-repo.sh`: 3,271 changed files, about 320,000 changed lines and a 100,000-line lock file, against `HEAD`.

- The window appears in 0.3 s. The file list first took 10–20 s, almost all of it counting the lock file with Histogram; with Myers above 20,000 lines (§4.5) the backend lists it in 0.5 s and the diff shows 2–4 s after launch. The rest is mostly the sidebar's file tree, which isn't virtualised (§7.2); fine at this size, and the first thing to look at if a bigger change set is slow. For comparison, `git diff HEAD -M --shortstat` takes 0.3 s.
- Find in page over every file (a word that matches 500,000 times) first applied each loaded diff on its own, and each rebuilt the row list (300 ms with everything loaded). Applying them in batches (§7.5) brings it to under 15 s for all 3,271 files, with the window drawing in between.
- Not checked here: scrolling smoothness, which needs a person at the window, and macOS. Both are in `RELEASING.md`.

---

## 10. Testing

### 10.1 Rust: unit and fixture tests (`cargo test`)

- **Unit tests:** hunk building and merging, context handling, whitespace normalisation, word-level tokenising and pairing, binary detection, UTF-16 offset conversion, missing final newline.
- **Fixture repositories:** shell scripts in `tests/fixtures/` create small repositories with `git` (needed only to *run the tests*, not in the app). Each test builds its fixture in a temporary directory. The scripts share `lib.sh`, which fixes author and committer names, emails and dates (one hour apart per commit) and ignores the user's git config, so results are reproducible. They use only portable `sed` and `awk`, so they run on macOS too. Cases covered:
  - staged, unstaged and untracked changes, and ignored files,
  - a repository with no commits yet,
  - renames, with and without content changes, and near the similarity threshold,
  - binary files, mode changes, submodule updates, deleted files,
  - line-ending conversion via `.gitattributes`,
  - three-dot vs two-dot where base has moved on (the `billing.py` example in the PRD),
  - unrelated histories (no merge base),
  - branches (local and remote-tracking), lightweight and annotated tags, and a long history for paging and searching commits.
- **Comparison with git:** for each fixture and comparison, TurboDiff's file list is checked against `git diff --name-status -M` and `git diff --numstat --diff-algorithm=histogram`. Paths, statuses, rename pairs and similarities, and `+/−` counts must match. Hunk boundaries are not compared, as accepted in §1. Work-tree comparisons run git on a copy of the fixture with everything staged (`git add -A`, then `git diff --cached`), since that's what TurboDiff's work-tree view shows. Myers counts and hide whitespace (`git diff -w`) are compared too. Known differences are listed in `ALLOWED_DIFFERENCES` in `tests/changes.rs`, and the test checks that each one still happens. There's one: a file that isn't valid UTF-8 is binary for TurboDiff (PRD §8), while git counts its lines.

### 10.2 Frontend: component tests (Vitest + `@testing-library/svelte`)

- `FileTree`: compacting single-child folders, switching between tree and flat, selecting and deselecting files.
- `RefPicker`: switching tabs, filtering, pinned Working tree, SHA lookup results.
- Building rows for diffs: inline and split alignment, filler cells, word-level ranges, expand controls.
- Syntax highlighting: picking languages, tokenizing with the real Shiki (multi-line comments, both themes, long lines), combining tokens with word-level ranges, the highlight store (which sides it fetches, what it skips, stale generations, the cache) and the worker client's failure handling. The worker itself isn't started in tests: jsdom has none, and the diff list tests replace the client.
- The Settings window: showing the saved values, saving each change, clamping numbers, a saved font that isn't installed, a font list that fails to load, and resetting. Open windows taking up a new default diff style (and keeping the toolbar's choice otherwise), and listing files again when the algorithm changes.
- Theme and mode switching, the session, settings and recent stores (including discarding stale generations), the welcome screen, the layout and the empty and error states.
- Find in page: finding matches in inline and split rows (and not in paths, hunk headers or collapsed files), marking them in `spans()`, `Mod+F`, stepping with Enter and Shift+Enter, loading every diff to search it, and closing with `Esc`.
- Contrast of every text and background pair in both modes (§6), and the version script's file rewrites (`scripts/set-version.test.ts`).
- `api.ts` is mocked with fixed responses.

### 10.3 Not automated in v1

End-to-end tests of the whole app, the read-only before-and-after check, and performance benchmarks. These are covered by a short **manual pre-release checklist** in `RELEASING.md`, which will be written in milestone M9 of [TASKS.md](TASKS.md).

---

## 11. CI and release

**GitHub Actions**

- **`ci.yml`** runs on every push and PR, on `ubuntu-latest` and `macos-latest`:
  - Node 26, with pnpm set up from `packageManager` in `package.json`,
  - `pnpm install --frozen-lockfile`, `pnpm lint` (ESLint, Prettier check, `svelte-check`), `pnpm test`,
  - `cargo fmt --check`, `cargo clippy -D warnings` (plus the stale-path check in §8), `cargo test`, for the whole Cargo workspace from the repository root,
  - Linux system packages: `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`.
- **`release.yml`** runs when a `v*` tag is pushed and uses `tauri-action` to build:
  - Linux, on `ubuntu-22.04` (the oldest runner, so the bundles work with an older glibc): **AppImage** and **.deb** (`--bundles deb,appimage`),
  - macOS: a **universal `.dmg`** (Apple Silicon and Intel; `--target universal-apple-darwin`, with both Rust targets installed), **unsigned** for now: it's ad-hoc signed (`signingIdentity: "-"`), which Apple Silicon needs to launch it at all, and users remove the quarantine flag once (README),

  then uploads them to a draft GitHub Release named `TurboDiff vX.Y.Z`. A first step fails the run if the tag doesn't match the version in `package.json`. A pull request that changes the workflow, `tauri.conf.json` or the icons runs the same builds without a release and keeps the bundles as workflow artifacts. The .deb package is called `turbo-diff` (Tauri derives it from the product name) and installs `/usr/bin/turbodiff`.

**Versioning:** SemVer. The version is kept the same in `package.json`, the workspace `Cargo.toml` (both crates inherit it with `version.workspace = true`) and `tauri.conf.json`. `pnpm set-version X.Y.Z` (`scripts/set-version.js`) updates all three and `Cargo.lock`. `RELEASING.md` has the rest of the steps.

**Icon:** `src-tauri/icons/icon.svg` is the source, a ± in GitHub's diff green and red on the macOS icon grid; `pnpm tauri icon src-tauri/icons/icon.svg` regenerates the sizes, of which only those listed in `tauri.conf.json` are kept.

**Signing later:** adding macOS signing and notarisation means adding Apple certificates as CI secrets and the matching `tauri-action` settings. No code changes are needed.

---

## 12. Risks and open technical questions

| Risk / question | Mitigation |
|---|---|
| gitoxide's status or filter handling misses an unusual repository setup (LFS, textconv, sparse checkout, submodule edge cases) | Fixture tests for the common cases. Show a clear "unsupported" notice instead of a wrong diff. Report problems upstream. |
| Rename detection differs from git in borderline cases | Reimplements git's similarity estimate and pairing (§4.4). Fixtures just above and below 50% are compared with git. |
| WebKitGTK performance with huge diffs | Row-level virtualisation. Test on Linux first. |
| Size of Shiki grammars | A curated list of about 50 common languages (§7.4), each a chunk loaded only when needed: about 3 MB before Tauri compresses the assets, C++ the largest at 0.8 MB. Other languages are plain text. |
| Hide-whitespace semantics vs `git diff -w` | Same rule (ignore all whitespace when comparing lines). Counts are compared with `git diff -w --numstat` on a fixture. |
| Find in page (`Mod+F`, PRD §6.10) can't use the webview's find: the virtual list renders only rows near the viewport, and neither webview has a find bar | Settled in M9: a find bar of our own that searches the rows and loads every diff while it has a query (§7.5). |
| Tools that rewrite the index often (an editor running `git status` every few seconds) make TurboDiff list the changes again each time | Seen as an index change, so a refresh. Cheap for normal repositories, and an unchanged result changes nothing on screen. Not seen in the M9 performance pass; a refresh of the large repository costs the same as listing it (about half a second in the backend). |
| Large-diff thresholds | PRD open question 1. Defined as constants. Checked in M5 on this repository (lock files and the generated `themes.css` start collapsed, hand-written files don't), and again in M9 on the large repository, with no reason found to change them. |
