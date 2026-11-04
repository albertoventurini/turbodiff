//! The `#[tauri::command]` surface of the IPC API (TECH.md §5).

use specta_typescript::Typescript;
use tauri_specta::{Builder, collect_commands};

/// Where the generated TypeScript bindings live.
pub const TYPES_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/types.ts");

/// Returns the app version from `Cargo.toml`.
#[tauri::command]
#[specta::specta]
pub async fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![app_version])
}

pub fn export_types(
    builder: &Builder<tauri::Wry>,
    path: &str,
) -> Result<(), specta_typescript::Error> {
    builder.export(Typescript::default(), path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_version_matches_cargo_toml() {
        let version = tauri::async_runtime::block_on(app_version());
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
    }

    /// Fails when `src/lib/types.ts` doesn't match the Rust types, and rewrites it
    /// so that running the tests again (and committing the file) fixes it.
    #[test]
    fn types_ts_is_up_to_date() {
        let fresh = std::env::temp_dir().join(format!("turbodiff-types-{}.ts", std::process::id()));
        export_types(&specta_builder(), fresh.to_str().unwrap()).unwrap();
        let expected = std::fs::read_to_string(&fresh).unwrap();
        std::fs::remove_file(&fresh).unwrap();

        let current = std::fs::read_to_string(TYPES_PATH).unwrap_or_default();
        if current != expected {
            std::fs::write(TYPES_PATH, &expected).unwrap();
            panic!(
                "src/lib/types.ts was out of date and has been regenerated. Review and commit it."
            );
        }
    }
}
