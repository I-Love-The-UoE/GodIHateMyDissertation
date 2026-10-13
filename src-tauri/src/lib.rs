#![expect(
    unreachable_pub,
    clippy::unreachable,
    clippy::exit,
    reason = "`#[tauri::command]` and `generate_context!` expand to `pub` items, `unreachable!` arms and a `process::exit`."
)]

mod commands;
mod config;
mod context;
mod error;
mod events;
mod state;

pub use error::StartupError;

pub fn run() -> Result<(), StartupError> {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            // NOTE: tauri's setup callback requires `Box<dyn Error>`; the `?` conversion from
            // `StartupError` is the only place the boxed form appears.
            state::initialise(app)?;

            Ok(())
        })
        .invoke_handler(commands::handler())
        .run(tauri::generate_context!())?;

    Ok(())
}
