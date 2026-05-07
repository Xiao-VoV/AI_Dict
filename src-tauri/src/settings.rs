use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
    pub target_language: String,
    #[serde(default = "default_ui_language")]
    pub ui_language: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
            temperature: 0.2,
            target_language: "简体中文".to_string(),
            ui_language: default_ui_language(),
        }
    }
}

fn default_ui_language() -> String {
    "zh-CN".to_string()
}

pub fn load_settings(app: &AppHandle) -> Result<AppSettings, SettingsError> {
    let path = settings_path(app)?;
    if !path.exists() {
        log::debug!("settings file not found; using defaults");
        return Ok(AppSettings::default());
    }
    log::debug!("loading settings; path={}", path.display());
    let content = fs::read_to_string(path)?;
    let settings = serde_json::from_str(&content)?;
    log::debug!("settings loaded");
    Ok(settings)
}

pub fn save_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), SettingsError> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    log::debug!("saving settings; path={}", path.display());
    let content = serde_json::to_string_pretty(settings)?;
    fs::write(path, content)?;
    log::info!("settings saved");
    Ok(())
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, SettingsError> {
    let dir = app.path().app_config_dir()?;
    Ok(dir.join("settings.json"))
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("无法定位配置目录：{0}")]
    Path(#[from] tauri::Error),
    #[error("无法读写配置文件：{0}")]
    Io(#[from] std::io::Error),
    #[error("配置文件格式错误：{0}")]
    Json(#[from] serde_json::Error),
}
