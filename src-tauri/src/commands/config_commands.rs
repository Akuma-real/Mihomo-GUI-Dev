use std::path::PathBuf;
use tauri::State;

use crate::config::manager::{ConfigInfo, ConfigManager, ValidationResult};

type Shared<T> = tauri::async_runtime::Mutex<T>;

#[tauri::command]
pub async fn load_all_configs(config_manager: State<'_, Shared<ConfigManager>>) -> Result<Vec<ConfigInfo>, String> {
  let mgr = config_manager.lock().await;
  mgr
    .load_all_configs()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_config(
  config_manager: State<'_, Shared<ConfigManager>>,
  config_path: String,
) -> Result<ValidationResult, String> {
  let mgr = config_manager.lock().await;
  mgr.validate(&PathBuf::from(config_path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_config(
  config_manager: State<'_, Shared<ConfigManager>>,
  source_path: String,
) -> Result<String, String> {
  let mgr = config_manager.lock().await;
  mgr
    .import_config(&PathBuf::from(source_path))
    .map(|p| p.to_string_lossy().to_string())
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_config(
  config_manager: State<'_, Shared<ConfigManager>>,
  config_path: String,
  target_path: String,
) -> Result<(), String> {
  let mgr = config_manager.lock().await;
  mgr
    .export_config(&PathBuf::from(config_path), &PathBuf::from(target_path))
    .map_err(|e| e.to_string())
}

