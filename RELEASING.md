# Releasing TurboDiff

The steps to publish a release. CI builds the bundles into a draft release; nothing is built locally, and there are no manual checks (TECH.md §10.3).

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

## 2. Tag and build

Tag the release commit and push the tag. `release.yml` checks that the tag matches the version, builds the AppImage, .deb and universal .dmg, and uploads them to a **draft** release, which only maintainers can see:

```bash
git tag -a v0.2.0 -m "TurboDiff v0.2.0"
git push origin v0.2.0
```

When the workflow has finished, open the draft on GitHub's Releases page and check that it has the three bundles.

## 3. Publish

Write the release notes (what changed since the last release), then publish the draft.

If something is wrong with the draft, delete the draft and the tag, on GitHub and locally, fix the problem on `main`, and start again from §1 (the version can stay the same):

```bash
gh release delete v0.2.0 --cleanup-tag --yes
git tag -d v0.2.0
```
