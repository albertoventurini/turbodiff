fn main() {
    // Listing our commands in the app manifest means each one needs an explicit
    // `allow-*` permission in `capabilities/`, instead of being allowed by default.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "app_version",
            "get_window_state",
            "open_repo",
            "show_open_dialog",
            "list_branches",
            "list_tags",
            "list_commits",
            "search_commits",
            "resolve_commit_prefix",
            "set_comparison",
            "get_file_diff",
            "get_lines",
            "get_settings",
            "set_settings",
            "reset_settings",
            "list_monospace_fonts",
            "list_recent",
            "remove_recent",
        ]),
    ))
    .expect("failed to run tauri-build");
}
