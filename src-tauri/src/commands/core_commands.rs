use std::path::PathBuf;

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{State, Window, Emitter};

use crate::core::manager::{CoreManager, CoreStatus};
use crate::core::version::{ReleaseChannel, VersionManager, build_gh_client, fetch_text, verify_sha256};

type Shared<T> = tauri::async_runtime::Mutex<T>;

#[tauri::command]
pub async fn start_core(
  core_manager: State<'_, Shared<CoreManager>>,
  config_path: String,
  _need_privilege: Option<bool>,
) -> Result<(), String> {
  let mut mgr = core_manager.lock().await;
  let path = PathBuf::from(config_path);
  log::info!("start_core with config: {}", path.display());
  mgr.start(path)
}

#[tauri::command]
pub async fn stop_core(core_manager: State<'_, Shared<CoreManager>>) -> Result<(), String> {
  let mut mgr = core_manager.lock().await;
  log::info!("stop_core called");
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
  log::debug!("get_core_status => {}", status);
  Ok(status.to_string())
}

// 已移除手动设置/读取 core_path 的命令，统一采用固定路径。

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
  log::info!("fetch_latest_version channel={}", channel);
  mgr
    .fetch_latest(ch)
    .await
    .map(|info| info.version)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_install_latest(
  window: Window,
  core_manager: State<'_, Shared<CoreManager>>,
  version_manager: State<'_, Shared<VersionManager>>,
  channel: String,
) -> Result<String, String> {
  let ch = match channel.as_str() {
    "stable" => ReleaseChannel::Stable,
    "dev" => ReleaseChannel::Dev,
    _ => return Err("无效的渠道".into()),
  };

  // 进度事件负载
  #[derive(Serialize, Clone)]
  struct ProgressPayload<'a> { stage: &'a str, progress: u8, message: Option<String> }

  // 规划下载（资产与校验）
  let plan = {
    let vm = version_manager.lock().await;
    vm.plan_download(ch).await.map_err(|e| e.to_string())?
  };
  log::info!("download plan: version={}, asset={} url={} checksum={:?}", plan.version, plan.asset_name, plan.asset_url, plan.checksum_url);

  let _ = window.emit("version_install_progress", ProgressPayload { stage: "开始下载", progress: 0, message: None });

  // 构建客户端并下载（流式）
  let client = build_gh_client().map_err(|e| e.to_string())?;
  let resp = client.get(&plan.asset_url).send().await.map_err(|e| e.to_string())?;
  let total = resp.content_length();
  let mut stream = resp.bytes_stream();
  let mut received: u64 = 0;
  let mut buf: Vec<u8> = Vec::with_capacity(total.unwrap_or(0) as usize);
  while let Some(chunk) = stream.next().await {
    let chunk = chunk.map_err(|e| e.to_string())?;
    received += chunk.len() as u64;
    buf.extend_from_slice(&chunk);
    if let Some(t) = total {
      let pct = ((received as f64 / t as f64) * 90.0).min(90.0).max(1.0) as u8; // 下载占前90%
      if pct % 5 == 0 { log::debug!("downloading... {}%", pct); }
      let _ = window.emit("version_install_progress", ProgressPayload { stage: "下载中", progress: pct, message: None });
    } else {
      // 未知大小，伪进度
      let pct = ((received / (1024 * 1024)) % 90) as u8; // 每MB+1直到90
      let _ = window.emit("version_install_progress", ProgressPayload { stage: "下载中", progress: pct, message: None });
    }
  }

  // 校验
  if let Some(url) = plan.checksum_url.as_ref() {
    let _ = window.emit("version_install_progress", ProgressPayload { stage: "校验中", progress: 92, message: None });
    log::info!("verifying checksum from {}", url);
    let text = fetch_text(&client, url, "checksums").await.map_err(|e| e.to_string())?;
    verify_sha256(&buf, &text).map_err(|e| e.to_string())?;
  }

  // 安装
  let installed = {
    let mut vm = version_manager.lock().await;
    let _ = window.emit("version_install_progress", ProgressPayload { stage: "安装中", progress: 95, message: None });
    log::info!("installing version {}", plan.version);
    vm.install_from_bytes(&plan.version, &plan.asset_name, &buf).map_err(|e| e.to_string())?
  };

  // 自动更新 core_path
  let mut cm = core_manager.lock().await;
  cm.set_core_path(installed.clone());
  log::info!("installed at {} and set as current core", installed.display());

  let _ = window.emit(
    "version_install_progress",
    ProgressPayload { stage: "完成", progress: 100, message: None },
  );

  Ok(installed.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_core_install_dir(version_manager: State<'_, Shared<VersionManager>>) -> Result<String, String> {
  let vm = version_manager.lock().await;
  Ok(vm.cores_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_default_core_path(version_manager: State<'_, Shared<VersionManager>>) -> Result<Option<String>, String> {
  let vm = version_manager.lock().await;
  let bin = if cfg!(target_os = "windows") { "mihomo.exe" } else { "mihomo" };
  let p = vm.cores_dir.join("current").join(bin);
  if p.exists() { Ok(Some(p.to_string_lossy().to_string())) } else { Ok(None) }
}

// 已移除 use_default_core_path 命令，应用启动时自动采用固定路径。
