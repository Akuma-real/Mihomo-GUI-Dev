// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "linux")]
fn apply_linux_compat_env() {
  use std::env;
  // 兼容优先：默认禁用 WebKit 的 DMABUF 渲染，避免 Wayland+NVIDIA/驱动不兼容
  if env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
    env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
  }

  // 若在 Wayland 会话且用户未显式指定后端，则强制走 X11（XWayland）提高兼容性
  if env::var("GDK_BACKEND").is_err() {
    if env::var("XDG_SESSION_TYPE")
      .map(|v| v.eq_ignore_ascii_case("wayland"))
      .unwrap_or(false)
    {
      env::set_var("GDK_BACKEND", "x11");
    }
  }

  // 如需完全兜底（软件渲染），请在外部设置：LIBGL_ALWAYS_SOFTWARE=1
}

#[tauri::command]
fn greet() -> String {
  let now = SystemTime::now();
  let epoch_ms = now.duration_since(UNIX_EPOCH).unwrap().as_millis();
  format!("Hello world from Rust! Current epoch: {epoch_ms}")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  #[cfg(target_os = "linux")]
  apply_linux_compat_env();
  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .invoke_handler(tauri::generate_handler![greet])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
