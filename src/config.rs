use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub const APP_ID: &str = "com.cosmic.CosmicMatrix";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StoredSession {
    pub homeserver: String,
    pub user_id: String,
    pub access_token: String,
    pub device_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum SortMode {
    #[default]
    RecentActivity,
    Alphabetical,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub sort_mode: SortMode,
    /// Maps section key → collapsed state. Missing key = not collapsed.
    #[serde(default)]
    pub sections_collapsed: HashMap<String, bool>,
}

pub fn config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("cosmic-matrix")
}

pub fn data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("cosmic-matrix")
}

pub fn session_path() -> PathBuf {
    config_dir().join("session.json")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn save_session(session: &StoredSession) -> Result<(), String> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    std::fs::write(session_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_session() -> Option<StoredSession> {
    let path = session_path();
    if !path.exists() {
        return None;
    }
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn clear_session() {
    let _ = std::fs::remove_file(session_path());
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_settings() -> AppSettings {
    let path = settings_path();
    if !path.exists() {
        return AppSettings::default();
    }
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return AppSettings::default(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}
