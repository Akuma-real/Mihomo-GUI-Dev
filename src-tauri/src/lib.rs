pub mod core;
pub mod config;
pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  use tauri::Manager;
  use crate::commands::config_commands::{export_config, import_config, load_all_configs, validate_config};
  use crate::commands::core_commands::{download_install_latest, fetch_latest_version, get_core_install_dir, get_core_status, get_default_core_path, restart_core, start_core, stop_core};
  use crate::commands::system_commands::{check_tun_hint, install_systemd_service, uninstall_systemd_service, systemd_service_status};
  use crate::core::manager::CoreManager;
  use crate::core::version::VersionManager;
  use crate::config::manager::ConfigManager;

  let builder = tauri::Builder::default()
    .manage(tauri::async_runtime::Mutex::new(CoreManager::default()))
    .manage(tauri::async_runtime::Mutex::new(VersionManager::new().expect("init version manager")))
    .manage(tauri::async_runtime::Mutex::new(ConfigManager::new().expect("init config manager")))
    .setup(|app| {
      if cfg!(debug_assertions) {
        app
          .handle()
          .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Debug).build())?;
      }
      // 启用系统对话框插件（供前端 @tauri-apps/plugin-dialog 使用）
      app.handle().plugin(tauri_plugin_dialog::init())?;
      // 自动使用默认 core 路径（<data_dir>/mihomo-gui/cores/current/mihomo）
      let core_state = app.state::<tauri::async_runtime::Mutex<CoreManager>>();
      let vm_state = app.state::<tauri::async_runtime::Mutex<VersionManager>>();
      let vm = tauri::async_runtime::block_on(vm_state.lock());
      let bin = if cfg!(target_os = "windows") { "mihomo.exe" } else { "mihomo" };
      let p = vm.cores_dir.join("current").join(bin);
      if p.exists() {
        let mut cm = tauri::async_runtime::block_on(core_state.lock());
        cm.set_core_path(p);
      }
      Ok(())
    });

  builder
    .invoke_handler(tauri::generate_handler![
      // core
      start_core,
      stop_core,
      restart_core,
      get_core_status,
      fetch_latest_version,
      download_install_latest,
      get_core_install_dir,
      get_default_core_path,
      // system
      check_tun_hint,
      install_systemd_service,
      uninstall_systemd_service,
      systemd_service_status,
      // config
      load_all_configs,
      validate_config,
      import_config,
      export_config,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
