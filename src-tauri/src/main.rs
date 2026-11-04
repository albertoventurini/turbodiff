mod commands;

fn main() {
    let builder = commands::specta_builder();

    // Debug builds regenerate the TypeScript bindings on startup.
    #[cfg(debug_assertions)]
    commands::export_types(&builder, commands::TYPES_PATH)
        .expect("failed to export TypeScript types");

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running TurboDiff");
}
