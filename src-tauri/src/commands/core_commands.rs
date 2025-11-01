use std::path::PathBuf;

use tauri::State;

use crate::core::manager::{CoreManager, CoreStatus};
use crate::core::version::{ReleaseChannel, VersionManager};

type Shared<T> = tauri::async_runtime::Mutex<T>;

#[tauri::command]
pub async fn start_core(
  core_manager: State<'_, Shared<CoreManager>>,
  config_path: String,
  _need_privilege: Option<bool>,
) -> Result<(), String> {
  let mut mgr = core_manager.lock().await;
  let path = PathBuf::from(config_path);
  mgr.start(path)
}

#[tauri::command]
pub async fn stop_core(core_manager: State<'_, Shared<CoreManager>>) -> Result<(), String> {
  let mut mgr = core_manager.lock().await;
  mgr.stop()
}

#[tauri::command]
pub async fn get_core_status(core_manager: State<'_, Shared<CoreManager>>) -> Result<String, String> {
  let mut mgr = core_manager.lock().await;
  let status = match mgr.get_status() {
    CoreStatus::Running => "running",
    CoreStatus::Stopped => "stopped",
    CoreStatus::Error => "error",
  };
  Ok(status.to_string())
}

#[tauri::command]
pub async fn set_core_path(
  core_manager: State<'_, Shared<CoreManager>>,
  core_path: String,
) -> Result<(), String> {
  let mut mgr = core_manager.lock().await;
  mgr.set_core_path(PathBuf::from(core_path));
  Ok(())
}

#[tauri::command]
pub async fn get_core_path(core_manager: State<'_, Shared<CoreManager>>) -> Result<Option<String>, String> {
  let mgr = core_manager.lock().await;
  Ok(mgr.core_path.as_ref().map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn restart_core(core_manager: State<'_, Shared<CoreManager>>) -> Result<(), String> {
  let mut mgr = core_manager.lock().await;
  mgr.restart()
}

#[tauri::command]
pub async fn fetch_latest_version(
  version_manager: State<'_, Shared<VersionManager>>,
  channel: String,
) -> Result<String, String> {
  let ch = match channel.as_str() {
    "stable" => ReleaseChannel::Stable,
    "dev" => ReleaseChannel::Dev,
    _ => return Err("无效的渠道".into()),
  };
  let mgr = version_manager.lock().await;
  mgr
    .fetch_latest(ch)
    .await
    .map(|info| info.version)
    .map_err(|e| e.to_string())
}
