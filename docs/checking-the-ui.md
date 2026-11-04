# Checking the UI

How to look at the real app from an agent session, on the Linux development machine (GNOME on Wayland). Tests cover the logic; this checks what the window actually shows.

## Why X11

Native Wayland windows can't be captured from the command line on GNOME. Running the app with `GDK_BACKEND=x11` puts its window under XWayland, where the standard X tools can find and capture it.

## Steps

`$SCRATCH` below stands for the session's scratchpad directory. Keep screenshots and logs there, not in the repository.

1. Start the app in the background, writing its output to a log:

   ```bash
   GDK_BACKEND=x11 pnpm tauri dev > "$SCRATCH/dev.log" 2>&1
   ```

   To open a repository with a specific comparison, pass app arguments after `-- --`:

   ```bash
   GDK_BACKEND=x11 pnpm tauri dev -- -- /path/to/repo --base <ref> --compare <ref>
   ```

2. Wait for the window. The first build can take a few minutes. Poll until it appears:

   ```bash
   xwininfo -root -tree | grep '("turbodiff"' | grep -v 10x10
   ```

   The first column is the window id. The `10x10` window is a hidden helper window; ignore it.

3. Give the page a few seconds to render, then capture the window into the scratchpad:

   ```bash
   import -window <id> "$SCRATCH/shot.png"
   ```

   `import` comes from ImageMagick. Use `convert shot.png -resize 1200x small.png` or `-crop` to make the image easier to read.

4. Check what matters for the change: light and dark mode, empty and error states, and a real repository with a real diff. Where the app shows numbers, compare them with `git diff --shortstat` or `--numstat` for the same comparison.

5. Stop the app and the Vite dev server when done. Kill them by process id:

   ```bash
   kill $(pgrep -f 'debug/turbodiff') $(pgrep -f 'tauri.js dev') $(pgrep -f 'node_modules/.bin/vite')
   ```

   Don't use `pkill -f` with a pattern that also appears in your own command line: it kills the shell running it.

## Limits

- This only captures images. It can't click, type, scroll or copy, so interactions such as switching views, expanding context or copying text still need checking by hand. Say so in the report.
- A change to `src-tauri/capabilities/`, `build.rs` or `tauri.conf.json` needs a rebuild. `tauri dev` rebuilds and restarts the app when those files change.
- macOS isn't covered here. CI builds and tests on macOS, but nothing checks its UI automatically.
