mod ai;
mod dictionary;
mod selection;
mod settings;
mod translator;

use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_log::{Target, TargetKind};

#[tauri::command]
async fn capture_selection_and_lookup(
    app: tauri::AppHandle,
) -> Result<translator::LookupResult, String> {
    log::debug!("capture_selection_and_lookup command started");
    let text = selection::capture_selected_text().await.map_err(|error| {
        log::warn!("failed to capture selected text: {error}");
        error.to_string()
    })?;
    log::debug!(
        "captured selected text; char_count={}",
        text.chars().count()
    );

    let result = translator::lookup_text(&app, text).await.map_err(|error| {
        log::warn!("lookup after selection capture failed: {error}");
        error.to_string()
    })?;
    let _ = app.emit("lookup-result", &result);
    log::debug!("lookup-result event emitted");
    Ok(result)
}

#[tauri::command]
async fn lookup_text(
    app: tauri::AppHandle,
    text: String,
) -> Result<translator::LookupResult, String> {
    log::debug!(
        "lookup_text command started; char_count={}",
        text.chars().count()
    );
    translator::lookup_text(&app, text).await.map_err(|error| {
        log::warn!("lookup_text command failed: {error}");
        error.to_string()
    })
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<settings::AppSettings, String> {
    log::debug!("get_settings command started");
    settings::load_settings(&app).map_err(|error| {
        log::warn!("get_settings command failed: {error}");
        error.to_string()
    })
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: settings::AppSettings) -> Result<(), String> {
    log::debug!(
        "save_settings command started; base_url_present={} model_present={} target_language={}",
        !settings.base_url.trim().is_empty(),
        !settings.model.trim().is_empty(),
        settings.target_language
    );
    settings::save_settings(&app, &settings).map_err(|error| {
        log::warn!("save_settings command failed: {error}");
        error.to_string()
    })
}

#[tauri::command]
async fn test_ai_connection(settings: settings::AppSettings) -> Result<(), String> {
    log::debug!(
        "test_ai_connection command started; base_url_present={} model_present={} target_language={}",
        !settings.base_url.trim().is_empty(),
        !settings.model.trim().is_empty(),
        settings.target_language
    );
    ai::test_connection(&settings).await.map_err(|error| {
        log::warn!("test_ai_connection command failed: {error}");
        error.to_string()
    })
}

#[tauri::command]
async fn import_mdict(
    app: tauri::AppHandle,
    mdx_path: String,
) -> Result<dictionary::ImportSummary, String> {
    log::debug!("import_mdict command started; mdx_path={mdx_path}");
    tauri::async_runtime::spawn_blocking(move || {
        dictionary::import_mdict(&app, &mdx_path).map_err(|error| {
            log::warn!("import_mdict command failed: {error}");
            error.to_string()
        })
    })
    .await
    .map_err(|error| {
        log::warn!("import_mdict background task failed: {error}");
        error.to_string()
    })?
}

#[tauri::command]
async fn reindex_builtin_dictionary(
    app: tauri::AppHandle,
) -> Result<dictionary::ImportSummary, String> {
    log::debug!("reindex_builtin_dictionary command started");
    tauri::async_runtime::spawn_blocking(move || {
        dictionary::reindex_builtin_dictionary(&app).map_err(|error| {
            log::warn!("reindex_builtin_dictionary command failed: {error}");
            error.to_string()
        })
    })
    .await
    .map_err(|error| {
        log::warn!("reindex_builtin_dictionary background task failed: {error}");
        error.to_string()
    })?
}

#[tauri::command]
fn get_dictionary_index_progress() -> Option<dictionary::DictionaryIndexProgress> {
    dictionary::current_index_progress()
}

#[tauri::command]
fn list_dictionaries(app: tauri::AppHandle) -> Result<Vec<dictionary::DictionaryMetadata>, String> {
    log::debug!("list_dictionaries command started");
    dictionary::list_dictionaries(&app).map_err(|error| {
        log::warn!("list_dictionaries command failed: {error}");
        error.to_string()
    })
}

#[tauri::command]
fn delete_dictionary(app: tauri::AppHandle, dictionary_id: i64) -> Result<(), String> {
    log::debug!("delete_dictionary command started; dictionary_id={dictionary_id}");
    dictionary::delete_dictionary(&app, dictionary_id).map_err(|error| {
        log::warn!("delete_dictionary command failed: {error}");
        error.to_string()
    })
}

fn register_global_shortcut(app: &tauri::App) {
    if let Err(error) = app.global_shortcut().register("CommandOrControl+Shift+E") {
        log::error!("failed to register global shortcut: {error}");
    } else {
        log::info!("registered global shortcut CommandOrControl+Shift+E");
    }
}

fn log_plugin() -> tauri_plugin_log::Builder {
    let level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    tauri_plugin_log::Builder::new()
        .level(level)
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir {
                file_name: Some("selection-translator".to_string()),
            }),
        ])
        .max_file_size(256_000)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(log_plugin().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        log::debug!("global shortcut pressed");
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            match capture_selection_and_lookup(app.clone()).await {
                                Ok(_) => {
                                    log::debug!("shortcut lookup completed");
                                    if let Some(window) = app.get_webview_window("main") {
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                    } else {
                                        log::warn!("main window not found after shortcut lookup");
                                    }
                                }
                                Err(error) => {
                                    log::warn!("shortcut lookup failed: {error}");
                                    let _ = app.emit("lookup-error", error);
                                }
                            }
                        });
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            capture_selection_and_lookup,
            lookup_text,
            get_settings,
            save_settings,
            test_ai_connection,
            import_mdict,
            reindex_builtin_dictionary,
            get_dictionary_index_progress,
            list_dictionaries,
            delete_dictionary
        ])
        .setup(|app| {
            log::info!("selection translator app setup started");
            register_global_shortcut(app);
            log::info!("selection translator app setup completed");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running selection translator");
}
