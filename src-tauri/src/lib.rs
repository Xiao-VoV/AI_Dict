mod ai;
mod dictionary;
mod selection;
mod settings;
mod translator;

use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[tauri::command]
async fn capture_selection_and_lookup(
    app: tauri::AppHandle,
) -> Result<translator::LookupResult, String> {
    let text = selection::capture_selected_text()
        .await
        .map_err(|error| error.to_string())?;
    let result = translator::lookup_text(&app, text)
        .await
        .map_err(|error| error.to_string())?;
    let _ = app.emit("lookup-result", &result);
    Ok(result)
}

#[tauri::command]
async fn lookup_text(
    app: tauri::AppHandle,
    text: String,
) -> Result<translator::LookupResult, String> {
    translator::lookup_text(&app, text)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<settings::AppSettings, String> {
    settings::load_settings(&app).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: settings::AppSettings) -> Result<(), String> {
    settings::save_settings(&app, &settings).map_err(|error| error.to_string())
}

fn register_global_shortcut(app: &tauri::App) {
    if let Err(error) = app.global_shortcut().register("CommandOrControl+Shift+E") {
        eprintln!("failed to register global shortcut: {error}");
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            match capture_selection_and_lookup(app.clone()).await {
                                Ok(_) => {
                                    if let Some(window) = app.get_webview_window("main") {
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                    }
                                }
                                Err(error) => {
                                    let _ = app.emit("lookup-error", error);
                                }
                            }
                        });
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            capture_selection_and_lookup,
            lookup_text,
            get_settings,
            save_settings
        ])
        .setup(|app| {
            register_global_shortcut(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running selection translator");
}
