mod app;
mod commands;
mod error;
mod events;
mod session;
#[cfg(test)]
mod test_support;

fn main() {
    // Parse first, so `--help`, `--version` and usage errors print and exit
    // before a second launch is forwarded to the running app.
    let cwd = std::env::current_dir().unwrap_or_default();
    let launch = app::cli::parse(std::env::args_os(), &cwd).unwrap_or_else(|err| err.exit());

    let builder = commands::specta_builder();

    // Debug builds regenerate the TypeScript bindings on startup.
    #[cfg(debug_assertions)]
    commands::export_types(&builder, commands::TYPES_PATH)
        .expect("failed to export TypeScript types");

    tauri::Builder::default()
        // Must come first, so a second launch exits before anything else runs.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            app::windows::second_launch(app, argv, cwd)
        }))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            app::setup(app.handle(), launch)?;
            Ok(())
        })
        .on_menu_event(app::menu::on_menu_event)
        .on_window_event(app::windows::on_window_event)
        .run(tauri::generate_context!())
        .expect("error while running TurboDiff");
}
