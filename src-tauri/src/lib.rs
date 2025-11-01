pub mod core;
pub mod config;
pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  use crate::commands::config_commands::{export_config, import_config, load_all_configs, validate_config};
  use crate::commands::core_commands::{fetch_latest_version, get_core_path, get_core_status, restart_core, set_core_path, start_core, stop_core};
  use crate::core::manager::CoreManager;
  use crate::core::version::VersionManager;
  use crate::config::manager::ConfigManager;

  let builder = tauri::Builder::default();

  let builder = builder.setup(|app| {
    if cfg!(debug_assertions) {
      app
        .handle()
        .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())?;
    }
    Ok(())
  });

  let builder = builder
    .manage(tauri::async_runtime::Mutex::new(CoreManager::default()))
    .manage(tauri::async_runtime::Mutex::new(VersionManager::new().expect("init version manager")))
    .manage(tauri::async_runtime::Mutex::new(ConfigManager::new().expect("init config manager")));

  builder
    .invoke_handler(tauri::generate_handler![
      // core
      start_core,
      stop_core,
      restart_core,
      get_core_status,
      set_core_path,
      get_core_path,
      fetch_latest_version,
      // config
      load_all_configs,
      validate_config,
      import_config,
      export_config,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
