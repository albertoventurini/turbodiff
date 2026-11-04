# TurboDiff — Product Requirements Document

| | |
|---|---|
| **Status** | Draft |
| **Version** | v1 |
| **Last updated** | 2026-11-04 |
| **Platforms** | Linux, macOS |

---

## 1. Summary

TurboDiff is a small, good-looking desktop app for **reading git diffs**. You point it at a folder that contains a git repository and it shows what changed, in a layout modelled on GitHub's "Files changed" view: a file tree on the left and the diff on the right.

Out of the box it shows the same thing as `git diff HEAD`, plus untracked files: everything that has changed in the working tree since the last commit. You can also compare any two refs (branches, tags or commits).

TurboDiff is **read-only and single-purpose**. It does not stage, commit, check out or change the repository in any way. It is a diff viewer and nothing else.

## 2. Problem and motivation

- Terminal `git diff` output is hard to scan for anything beyond a few lines: no file overview, no side-by-side view, limited highlighting.
- GitHub's diff UI is pleasant to read, but it only works for pushed branches and pull requests, not for local, uncommitted work.
- Full git GUIs (and IDEs) can show diffs, but they are heavy and crowded with features that get in the way when all you want is to *read* a change.

TurboDiff gives the GitHub reading experience for any local repository, with no setup and nothing else to learn.

## 3. Goals

1. Show a local repository's diff, GitHub-style, within moments of opening it.
2. Default to the most useful view: the current uncommitted changes against `HEAD`.
3. Make comparing any two refs as simple as choosing them from two dropdowns.
4. Be pleasant to look at: GitHub's visual language, light and dark modes, careful typography.
5. Stay small. Every feature must serve *reading a diff*.

## 4. Non-goals (v1)

- **No git write operations.** No staging, committing, checkout, stash, reset, merge or anything else that changes the repository.
- **No history or log browser.** No commit graph or log view. Commits can be picked only as refs to compare.
- **No editing or review.** No editing files, no inline comments, no GitHub, GitLab or other remote integration.
- **No Windows build.** Linux and macOS only.
- **No themes beyond GitHub.** The theme system exists internally, but only GitHub Light and GitHub Dark ship.
- **No advanced keyboard navigation.** No vim-style keys and no command palette (see §6.10).
- **No image previews.** Binary files, images included, show a placeholder.

## 5. Target user

A developer who uses git from the terminal or an editor and wants a clean, focused way to review changes: before committing, before opening a PR, or when comparing two branches locally.

---

## 6. Features

### 6.1 Opening a repository

A repository can be opened in four ways:

| Method | Behaviour |
|---|---|
| **Open-folder dialog** | *File → Open…* shows the native directory picker. |
| **Command line** | `turbodiff [PATH]` opens the repository at `PATH` (default: current directory). Optional `--base <ref>` and `--compare <ref>` preselect the comparison. |
| **Recent repositories** | The welcome screen and *File → Open Recent* list recently opened repositories (most recent first, about 10 entries). Entries whose path no longer exists are shown as unavailable and can be removed. |
| **Drag and drop** | Dropping a folder onto any TurboDiff window opens it. |

**Rules**

- If the chosen folder is inside a repository (not necessarily its root), TurboDiff opens the enclosing repository.
- If the folder is not in a git repository, a clear error is shown ("This folder is not a git repository") and the user stays where they were.
- **One repository per window.** Opening a repository while one is already shown opens it in a new window. Opening a repository that is already open focuses that window.
- **Launching with no argument** shows the welcome screen: the TurboDiff logo, an "Open Repository…" button, the recent repositories list and a hint that folders can be dropped onto the window.

### 6.2 The default comparison: working tree vs HEAD

When a repository is opened, TurboDiff always starts with **Base: `HEAD`, Compare: Working tree**.

This covers:

- staged changes,
- unstaged changes,
- **untracked files**, shown as added files. Files matched by `.gitignore` are excluded.

In git terms this is `git diff HEAD` with untracked files added.

Edge cases:

- **Repository with no commits:** every file in the working tree is shown as added.
- **Clean working tree:** an empty state says "No changes. Working tree matches HEAD."

### 6.3 Choosing refs to compare

A **comparison bar** at the top of the window, styled like GitHub's compare view:

```
base: [ main ▾ ]  ←  compare: [ Working tree ▾ ]     ⇄     [ ••• | •• ]
```

There are two dropdowns, *base* and *compare*, both built like GitHub's branch picker, so the user is never shown one long mixed list:

```
┌─────────────────────────────────┐
│ 🔍 Find a branch, tag or commit │
├─────────────────────────────────┤
│ ◉ Working tree                  │  ← compare picker only, pinned
├─────────────────────────────────┤
│ Branches │ Tags │ Commits       │
├─────────────────────────────────┤
│ ✓ main                (current) │
│   feature/login                 │
│   origin/main                   │
│   …                             │
└─────────────────────────────────┘
```

- **One search box** at the top filters whichever tab is active.
- **Tabs** show one kind of ref at a time:
  - **Branches:** the current branch (marked) first, then local branches, then remote-tracking branches.
  - **Tags:** newest first.
  - **Commits:** recent commits, each shown as short SHA, subject, author and relative date. The list loads more as you scroll. The search matches commit messages and SHAs.
- **Working tree** appears only in the *compare* picker. It is pinned above the tabs, so it is always one click away.
- **Pasting a SHA:** if the search text looks like a commit SHA (full or short), the matching commit is offered even when it isn't in the loaded list. Revision expressions such as `HEAD~3` or `v1.2.0^` are **not** supported in the UI. Every ref is picked from the lists. (The `--base` / `--compare` command-line options still accept any ref git understands.)
- **The picker opens on the tab of the current selection.** For example, if base is a tag, the base picker opens on the Tags tab.
- **Swap button (⇄):** swaps base and compare. It is disabled when compare is the working tree.
- Changing either ref updates the diff straight away.

#### Comparison mode: three-dot (default) vs two-dot

A small segmented toggle chooses how *base* and *compare* are compared:

- **Three-dot (default): "Changes on compare."** The *compare* side is used as it is now. The *base* side is not base as it is now, but the commit where *compare* split off from it (their merge base). The result is only what was changed on *compare*. This is what GitHub's compare view and pull request "Files changed" tab show.
- **Two-dot: "Direct comparison."** Both sides are used as they are now and compared directly.

**The two modes only differ when *base* has gained commits since *compare* split off from it.** Otherwise they show the same result.

*Example:* you create `feature` from `main`, and later a colleague adds `billing.py` to `main`.
- Three-dot (`main...feature`) doesn't show `billing.py`, because it isn't a change made on `feature`.
- Two-dot (`main..feature`) shows `billing.py` as **deleted**, because today's `main` has it and `feature` doesn't.
- Once `main` is merged into `feature`, the two modes agree again.

A tooltip on the toggle gives this explanation in short form. When compare is the working tree, three-dot compares it against the merge base of *base* and `HEAD`, and two-dot compares it directly against *base*. With the default comparison (base `HEAD`), both modes give the same result.

The comparison chosen is **not** saved between launches. Each time a repository is opened it starts at working tree vs `HEAD`.

### 6.4 Layout

```
┌──────────────────────────────────────────────────────────────────────┐
│  repo-name  ·  base: [HEAD ▾] ← compare: [Working tree ▾]  [•••|••] │
│                                   [Inline | Split]  [⚙︎ whitespace]  │
├──────────────────┬───────────────────────────────────────────────────┤
│ 12 files  +340 −87│ ▾ src/app/main.rs                    +12 −4  ⋯ │
│ [tree | flat]     │ ┌──────────────────────────────────────────────┐│
│                   │ │ @@ -10,7 +10,9 @@ fn main()                  ││
│ ▾ src/            │ │  10   fn main() {                            ││
│   ▾ app/          │ │  11 -     let x = 1;                         ││
│     M main.rs +12 │ │  11 +     let x = compute();                 ││
│     A util.rs +40 │ │ ...                                          ││
│ ▾ docs/           │ └──────────────────────────────────────────────┘│
│     D old.md  −30 │ ▸ src/app/util.rs                    +40      ⋯ │
│                   │   ...                                           │
└──────────────────┴───────────────────────────────────────────────────┘
```

- **Header / toolbar:** repository name, comparison bar (§6.3), diff-style toggle (§6.6), hide-whitespace toggle (§6.7).
- **Left sidebar:** summary and file tree (§6.5). It is resizable by dragging the divider and can be collapsed.
- **Main pane:** the diff (§6.6).

### 6.5 File tree (left sidebar)

- **Summary line** at the top: number of files changed, total additions (green) and deletions (red).
- **Tree / flat toggle:**
  - *Tree* (default): files grouped by folder. Chains of single-child folders are compacted (`src/app/core/`), and folders can be expanded or collapsed.
  - *Flat*: one alphabetical list of full paths.
- **Per-file information:**
  - a **status badge**: Added (A, green), Modified (M, amber), Deleted (D, red), Renamed (R, blue). Untracked files are shown as Added.
  - **line counts**: `+N −M` in muted green and red. Binary files show "bin".
- **Selection** (see §6.6, "single-file focus"):
  - Clicking a file **selects** it, and the main pane shows only that file's diff.
  - Clicking the selected file again, or pressing `Esc`, **deselects** it and returns to the all-files view.
  - The selected file is highlighted. In all-files view, the file currently at the top of the main pane is shown with a subtle "current" marker.

### 6.6 Diff view (main pane)

#### All-files view (default)

- Every changed file is stacked in one scrolling list, in the same order as the file tree.
- Each file has a **sticky header** showing the collapse chevron, the path (renames shown as `old/path → new/path`), the status badge, `+N −M`, and a "copy path" action.
- Each file can be **collapsed or expanded** individually.

#### Single-file focus

- Selecting a file in the tree shows **only that file's** diff in the main pane.
- Deselecting returns to the all-files view at the scroll position the user had before.

#### Diff styles

- **Inline (unified):** one column; deletions above additions; old and new line-number gutters.
- **Side-by-side (split):** old version on the left, new on the right, lines aligned, each side with its own gutter.
- A toolbar toggle switches styles instantly. The default style is set in Settings (§6.8).

#### Rendering

- **Syntax highlighting** chosen by file extension or name, using GitHub's syntax colours for the active mode.
- **Word-level highlighting:** within a changed line, the specific changed characters or words get a stronger background, as on GitHub.
- **Hunk headers** (`@@ -a,b +c,d @@`) styled as on GitHub. v1 does not show the enclosing function name after the header.
- **Expand context:** between hunks, and at the start and end of a file, controls reveal hidden unchanged lines: *expand up*, *expand down* (about 20 lines per click) and *expand all* when the gap is small.
- **"No newline at end of file"** markers are shown.
- Long lines **soft-wrap** or scroll horizontally, depending on the setting.
- Text in the diff can be selected and copied. Copying does not include line numbers or `+`/`−` markers.

#### Special files

| Case | Behaviour |
|---|---|
| **Large diff** (over a threshold, e.g. 1,000 changed lines or 500 KB) | Starts collapsed: "Large diff not shown. [Load diff]". |
| **Binary file** | Placeholder: "Binary file changed", with old and new sizes when known. |
| **Rename / copy** | Header shows `old → new` and a similarity %. If the content also changed, the content diff appears below. Renames are detected automatically. |
| **Mode change only** | "File mode changed 100644 → 100755". |
| **Submodule** | Shows the old and new commit SHAs of the submodule pointer. |
| **Deleted file** | Full content shown as removed, collapsed by default when large. |

### 6.7 Hide whitespace

A toolbar toggle hides whitespace-only changes, like GitHub's "Hide whitespace". When it is on, lines that differ only in whitespace are treated as unchanged, and files whose only changes are whitespace show "Whitespace-only changes" instead of a diff. The toggle lasts for the current window session and resets to off.

### 6.8 Settings

A **Settings** window, opened from the app menu. Changes apply immediately to all open windows and are saved.

| Setting | Options | Default |
|---|---|---|
| **Appearance** | System · Light · Dark | System |
| **Theme** | GitHub (only option in v1; the selector is shown so more themes can be added later) | GitHub |
| **Default diff style** | Inline · Side-by-side | Inline |
| **Code font** | Any installed monospace font | Platform default monospace (e.g. SF Mono / DejaVu Sans Mono) |
| **Font size** | 10–24 px | 13 px |
| **Tab width** | 2 · 4 · 8 | 4 |
| **Line wrapping** | On · Off | Off |
| **Context lines** | 0–20 | 3 |
| **Diff algorithm** | Histogram · Myers | Histogram |

- **System** appearance follows the OS light or dark preference live, without a restart.
- A **Reset to defaults** button restores every setting.

### 6.9 Live refresh

- TurboDiff **watches the repository** and refreshes automatically when files in the working tree, the index or refs change (for example after editing a file, `git add`, committing or switching branches).
- Refreshes are **debounced** (around 300 ms) so bursts of changes cause a single update.
- A refresh **keeps the user's place**: scroll position, the selected file, collapsed and expanded files, and expanded context are kept wherever the files still exist.
- When compare is a fixed ref rather than the working tree, refresh only happens when refs move (for example a branch gaining a commit).
- If a selected ref disappears (for example a branch is deleted), a non-blocking notice appears and the view falls back to working tree vs `HEAD`.
- Files ignored by `.gitignore`, and the `.git` internals that don't affect the diff, are not watched, so busy build folders don't cause constant refreshes.

### 6.10 Keyboard shortcuts (minimal)

Only three shortcuts. `Mod` means `Cmd` on macOS and `Ctrl` on Linux. Everything else is done with the mouse through the menus and toolbar.

| Action | Shortcut |
|---|---|
| Quit | `Mod+Q` |
| Deselect file (back to all files) | `Esc` |
| Find in page | `Mod+F` |

### 6.11 Persistence

TurboDiff remembers, across launches:

- application settings (§6.8),
- the recent repositories list,
- window size and position, and sidebar width.

It does **not** remember the last comparison per repository, the open repositories, or per-file collapsed state. Each repository opens fresh at working tree vs `HEAD`.

---

## 7. Visual design

- **Visual language:** GitHub's "Files changed" page, using **GitHub Primer** colour tokens for light and dark: canvas, borders, diff addition and deletion backgrounds, word-highlight backgrounds, hunk-header blue, muted line numbers.
- **Typography:** the system UI font for interface text; the configured monospace font for code.
- **Density and spacing:** comfortable rather than cramped. Clear 1 px borders and rounded file cards, as on GitHub.
- **Native feel:** a native menu bar on macOS and standard window decorations on Linux. Light and dark follow the OS unless overridden.
- **Empty and error states** are designed as well: welcome screen, "No changes", "Not a git repository", "Invalid ref", "Large diff not shown".
- **Accessibility:** status is never shown by colour alone (badges carry letters; diff lines carry `+`/`−` markers); text meets WCAG AA contrast in both modes; the UI respects the OS font scaling and reduced-motion settings.

## 8. Non-functional requirements

| Area | Requirement |
|---|---|
| **Startup** | Launching the app feels instant. The welcome screen appears without a noticeable wait. |
| **Open repo** | For a typical change set, the diff appears as soon as the repository opens, with no loading screen to sit through. |
| **Large change sets** | Very large diffs (thousands of files, tens of thousands of changed lines) don't freeze the app. Scrolling stays smooth and the UI keeps responding while content loads. |
| **Refresh** | After a file is saved, the change shows up in the app before you've switched back to the window. |
| **Safety** | Never writes to the repository: nothing that changes the index, refs, working tree or config, and no lock files. |
| **Dependencies** | Self-contained. Does not require `git` to be installed. |
| **Encoding** | UTF-8 by default. Files that cannot be decoded are shown as binary. |
| **Distribution** | Linux: AppImage and/or .deb. macOS: signed `.app` in a `.dmg` (Apple Silicon and Intel). The CLI launcher `turbodiff` is installable on `PATH`. |

## 9. User stories

1. *As a developer about to commit,* I run `turbodiff .` and see every change I've made, untracked files included, so I can review it before committing.
2. *As a developer preparing a PR,* I set base to `main` and compare to my feature branch and see exactly what the PR will contain (three-dot).
3. *As a developer checking what changed between releases,* I compare `v1.3.0` to `v1.4.0` directly (two-dot).
4. *As a reviewer of a large change,* I click one file in the tree to focus on it, then press `Esc` to return to the full list.
5. *As a developer editing code in another window,* I leave TurboDiff open and it updates on every save without losing my place.
6. *As someone who prefers side-by-side diffs,* I set "Side-by-side" as the default in Settings once and never think about it again.
7. *As someone working at night,* the app follows my OS dark mode automatically.

## 10. Success criteria

- The default view shows the same files, statuses (including renames) and changes as `git diff HEAD` plus untracked files, for the same repository state.
- Ref-to-ref comparisons show the same files and changes as `git diff A...B` (three-dot) and `git diff A B` (two-dot).
- Hunk boundaries may occasionally differ slightly from `git diff`, because TurboDiff uses its own diff engine (see TECH.md).
- Visual review: someone familiar with GitHub recognises the diff layout at once and finds it as easy to read.
- The app never changes the repository (checked by running it against a repo and confirming `git status` and the reflog are unchanged).
- The app feels fast in everyday use on a mid-range laptop, as described in §8.

## 11. Open questions

The tech stack and how the app reads git repositories have been decided. See [TECH.md](TECH.md).

1. **Large-diff thresholds:** confirm the exact line and size limits after testing on real repositories.
2. **Commits tab in the ref picker:** list commits reachable from `HEAD` only, or from all branches?
3. **App icon and branding:** to be designed.

## 12. Possible future work (out of scope for v1)

Listed only to show where TurboDiff could go; none of it is committed to.

- More themes and user-defined theme files.
- Image before/after previews.
- "Viewed" checkboxes per file.
- Filtering the file tree by name.
- Richer keyboard navigation (next/previous file and hunk).
- Function names in hunk headers (`@@ … @@ fn main()`).
- Windows support.
