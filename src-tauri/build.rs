fn main() {
    // Listing our commands in the app manifest means each one needs an explicit
    // `allow-*` permission in `capabilities/`, instead of being allowed by default.
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&["app_version"])),
    )
    .expect("failed to run tauri-build");
}
