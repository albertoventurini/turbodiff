# TurboDiff

TurboDiff is a small, read-only desktop app for reading git diffs, laid out like GitHub's "Files changed" view: a file tree on the left and the diff on the right. Point it at a repository and it shows everything that changed since the last commit, untracked files included, or the difference between any two branches, tags or commits. It never writes to the repository, and it doesn't need `git` to be installed.

It runs on Linux and macOS.

## Install

Download a build from the [latest release](https://github.com/albertoventurini/turbodiff/releases/latest).

### Linux

- **.deb** (Debian, Ubuntu and derivatives):

  ```bash
  sudo apt install ./TurboDiff_*_amd64.deb
  ```

  This also puts the `turbodiff` command on your `PATH`.

- **AppImage** (other distributions): make it executable and run it.

  ```bash
  chmod +x TurboDiff_*_amd64.AppImage
  ./TurboDiff_*_amd64.AppImage
  ```

  To use it from the terminal, link it as `turbodiff` somewhere on your `PATH`, for example `ln -s "$PWD/TurboDiff_0.1.0_amd64.AppImage" ~/.local/bin/turbodiff`.

### macOS

Open the `.dmg` (a universal build, for Apple Silicon and Intel) and drag TurboDiff to Applications.

The build isn't notarised yet, so macOS refuses to open it at first. Remove the quarantine flag once:

```bash
xattr -dr com.apple.quarantine /Applications/TurboDiff.app
```

To use it from the terminal:

```bash
sudo ln -s /Applications/TurboDiff.app/Contents/MacOS/turbodiff /usr/local/bin/turbodiff
```

## Usage

```bash
turbodiff [PATH] [--base REF] [--compare REF]
```

`turbodiff .` shows the working tree against `HEAD`. `turbodiff . --base main --compare my-branch` shows what `my-branch` adds to `main`. Without a path, TurboDiff opens its welcome screen, where you can pick a folder or a recent repository.

## Development

See [CLAUDE.md](CLAUDE.md) for the commands, [PRD.md](PRD.md) for what the app does, [TECH.md](TECH.md) for how it's built and [RELEASING.md](RELEASING.md) for making a release.
