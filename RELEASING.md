# Releasing TurboDiff

The manual checks before a release (TECH.md §10.3), and the steps to publish it. CI builds the bundles; the checks below cover what it can't.

## 1. Prepare

1. On `main`, with a clean work tree, set the new version (SemVer) in `package.json`, `Cargo.toml` and `tauri.conf.json` at once:

   ```bash
   pnpm set-version 0.2.0
   ```

2. Run everything CI runs:

   ```bash
   pnpm lint && pnpm test
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

3. Commit ("Release v0.2.0") and push. Wait for CI to pass on Linux and macOS.

## 2. Check a release build

Build the bundles for your platform:

```bash
pnpm tauri build --bundles deb,appimage   # Linux
pnpm tauri build --bundles dmg            # macOS
```

### Read-only check (PRD §8)

Pick a real repository with uncommitted changes, staged and unstaged, and a few untracked files. Record its state:

```bash
cd /path/to/repo
git status --porcelain=v2 > /tmp/before-status
git reflog > /tmp/before-reflog
sha256sum .git/index > /tmp/before-index   # `shasum -a 256` on macOS
find .git -name '*.lock' > /tmp/before-locks
```

Open it in the release build and use it: switch comparisons, expand context, toggle whitespace, edit a file so the view refreshes, then quit. Record the same four things again (`/tmp/after-*`) and check that each pair is identical with `diff`.

### Performance check (TECH.md §9)

Build a large repository (about 3,000 changed files, over 250,000 changed lines and a 100,000-line lock file):

```bash
scripts/make-large-repo.sh /tmp/large
```

Open it with `turbodiff /tmp/large` and check, on Linux first (WebKitGTK), then macOS:

- the file list appears without a noticeable wait, and the window keeps responding while it does;
- scrolling the diff from top to bottom stays smooth, in inline and split view, with and without wrapping;
- `Mod+F` for a common word finds matches in every file, and stepping through them stays responsive;
- editing a file in `/tmp/large/src` shows the change before you've switched back to the window;
- `big.lock` starts collapsed as a large diff, and "Load diff" shows it.

### Install and launch each build

- **AppImage:** `chmod +x` it and run it. Open a repository from the welcome screen.
- **.deb:** install it with `sudo apt install ./TurboDiff_*.deb`. Check that TurboDiff is in the app launcher with its icon, and that `turbodiff .` in a repository opens it. Remove it afterwards with `sudo apt remove turbo-diff`.
- **.dmg (macOS):** drag TurboDiff to Applications, run `xattr -dr com.apple.quarantine /Applications/TurboDiff.app`, and open it from Launchpad. Check the icon in the Dock and the menu bar, then run `/Applications/TurboDiff.app/Contents/MacOS/turbodiff .` in a repository. Check both an Apple Silicon and an Intel Mac when you can.

In each build, check light and dark mode.

## 3. Publish

1. Tag the release commit and push the tag. `release.yml` checks that the tag matches the version, builds the AppImage, .deb and universal .dmg, and uploads them to a **draft** release:

   ```bash
   git tag -a v0.2.0 -m "TurboDiff v0.2.0"
   git push origin v0.2.0
   ```

2. When the workflow has finished, open the draft on GitHub's Releases page. Check that it has the three bundles, download them and repeat the launch checks above on the CI-built files.
3. Write the release notes (what changed since the last release), then publish the draft.
