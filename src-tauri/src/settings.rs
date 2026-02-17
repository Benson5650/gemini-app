use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::AppHandle;
use tauri::Manager;

/// 應用程式設定結構
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    // 基本設定
    pub autostart: bool,
    pub default_window: String, // "normal", "panel", "none"

    // 行為設定
    pub normal_auto_load_conversation: bool,
    pub panel_auto_load_conversation: bool,
    pub tray_click_action: String, // "panel", "normal", "settings"

    // 外觀設定
    pub panel_width: i32,
    pub animation_speed: String, // "instant", "fast", "normal", "slow"

    // 快捷鍵設定
    pub hotkey_panel: String,
    pub hotkey_normal: String,
    pub enable_ctrl_n: bool,

    // 進階設定
    pub gemini_url: String,
    pub close_to_tray: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            autostart: true,
            default_window: "normal".to_string(),
            normal_auto_load_conversation: true,
            panel_auto_load_conversation: true,
            tray_click_action: "panel".to_string(),
            panel_width: 420,
            animation_speed: "fast".to_string(),
            hotkey_panel: "Ctrl+Alt+G".to_string(),
            hotkey_normal: "Ctrl+G".to_string(),
            enable_ctrl_n: true,
            gemini_url: "https://gemini.google.com".to_string(),
            close_to_tray: true,
        }
    }
}

/// 設定管理器
pub struct SettingsManager {
    settings: Mutex<AppSettings>,
    config_path: PathBuf,
}

impl SettingsManager {
    pub fn new(app: &AppHandle) -> Self {
        let config_dir = app.path().app_config_dir().expect("Failed to get config dir");
        fs::create_dir_all(&config_dir).ok();
        let config_path = config_dir.join("settings.json");

        let settings = if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => AppSettings::default(),
            }
        } else {
            AppSettings::default()
        };

        Self {
            settings: Mutex::new(settings),
            config_path,
        }
    }

    pub fn get(&self) -> AppSettings {
        self.settings.lock().unwrap().clone()
    }

    pub fn set(&self, settings: AppSettings) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        fs::write(&self.config_path, json)
            .map_err(|e| format!("Failed to write settings: {}", e))?;
        *self.settings.lock().unwrap() = settings;
        Ok(())
    }

    pub fn update<F>(&self, updater: F) -> Result<(), String>
    where
        F: FnOnce(&mut AppSettings),
    {
        let mut settings = self.settings.lock().unwrap();
        updater(&mut settings);
        let json = serde_json::to_string_pretty(&*settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        fs::write(&self.config_path, json)
            .map_err(|e| format!("Failed to write settings: {}", e))?;
        Ok(())
    }
}
