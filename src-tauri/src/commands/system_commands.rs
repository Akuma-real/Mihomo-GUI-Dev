use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::State;

use crate::core::manager::CoreManager;

type Shared<T> = tauri::async_runtime::Mutex<T>;

#[derive(Debug, Serialize)]
pub struct TunHint {
  pub enabled: bool,
  pub has_permission: bool,
  pub platform: String,
  pub suggested_cmd: Option<String>,
  pub message: String,
}

#[tauri::command]
pub async fn check_tun_hint(
  core_manager: State<'_, Shared<CoreManager>>,
  config_path: String,
) -> Result<TunHint, String> {
  // 读取配置并判断是否启用 TUN
  let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
  let yaml: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;

  let mut enabled = false;
  if let Some(tun) = yaml.get("tun") {
    enabled = tun
      .get("enable")
      .and_then(|v| v.as_bool())
      .unwrap_or_else(|| tun.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false));
  }

  if !enabled {
    return Ok(TunHint {
      enabled: false,
      has_permission: true,
      platform: current_platform().into(),
      suggested_cmd: None,
      message: "未启用 TUN，无需额外权限".into(),
    });
  }

  // 需要 TUN：根据平台给出提示
  #[cfg(target_os = "linux")]
  {
    // 拿到已安装的内核路径
    let core_path: Option<PathBuf> = {
      let mgr = core_manager.lock().await;
      mgr.core_path.clone()
    };

    // root 判定
    let is_root = unsafe { libc::geteuid() } == 0;

    // 检测 setcap（cap_net_admin, cap_net_bind_service）
    let has_cap = core_path.as_ref().map(|p| has_required_caps(p)).unwrap_or(false);

    let has_permission = is_root || has_cap;

    let suggested_cmd = core_path.as_ref().map(|p| format!(
      "sudo setcap 'cap_net_admin,cap_net_bind_service=+eip' {}",
      p.display()
    ));

    let msg = if has_permission {
      "已满足 TUN 所需权限（root 或已设置 setcap）".to_string()
    } else if core_path.is_some() {
      "检测到启用 TUN，但当前权限不足；可执行下列命令赋权或以 root 运行，或关闭 TUN 后再启动".to_string()
    } else {
      "检测到启用 TUN；请先安装内核后再执行 setcap，或关闭 TUN 后再启动".to_string()
    };

    return Ok(TunHint {
      enabled: true,
      has_permission,
      platform: "linux".into(),
      suggested_cmd,
      message: msg,
    });
  }

  #[cfg(target_os = "macos")]
  {
    return Ok(TunHint {
      enabled: true,
      has_permission: false,
      platform: "macos".into(),
      suggested_cmd: None,
      message: "检测到启用 TUN；请使用管理员权限启动，或关闭 TUN 后再启动".into(),
    });
  }

  #[cfg(target_os = "windows")]
  {
    return Ok(TunHint {
      enabled: true,
      has_permission: false,
      platform: "windows".into(),
      suggested_cmd: None,
      message: "检测到启用 TUN；请以管理员权限运行，或关闭 TUN 后再启动".into(),
    });
  }
}

#[cfg(target_os = "linux")]
fn has_required_caps(path: &PathBuf) -> bool {
  // 尝试调用 getcap <path> 并解析是否包含 cap_net_admin
  // 解析真实路径（符号链接指向的目标）
  let real_path = std::fs::canonicalize(path).unwrap_or(path.clone());
  match Command::new("getcap").arg(&real_path).output() {
    Ok(out) if out.status.success() => {
      let s = String::from_utf8_lossy(&out.stdout).to_string();
      s.contains("cap_net_admin")
    }
    _ => false,
  }
}

fn current_platform() -> &'static str {
  if cfg!(target_os = "linux") {
    "linux"
  } else if cfg!(target_os = "windows") {
    "windows"
  } else if cfg!(target_os = "macos") {
    "macos"
  } else {
    "unknown"
  }
}

fn default_core_path_from_manager(mgr: &CoreManager) -> Option<PathBuf> {
  if let Some(p) = mgr.core_path.as_ref() { return Some(p.clone()); }
  let bin = if cfg!(target_os = "windows") { "mihomo.exe" } else { "mihomo" };
  // 与 lib.rs 中的逻辑保持一致
  let cores_dir = {
    use super::super::core::version::VersionManager;
    // 尝试与 VersionManager 相同的根目录
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("mihomo-gui").join("cores")
  };
  let p = cores_dir.join("current").join(bin);
  if p.exists() { Some(p) } else { None }
}

fn render_systemd_unit(binary: &str, config: &str) -> String {
  format!(r#"[Unit]
Description=Mihomo Core (managed by mihomo-gui)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={binary} -f "{config}"
Restart=on-failure
RestartSec=3
LimitNOFILE=1048576

[Install]
WantedBy=multi-user.target
"#, binary=binary, config=config)
}

#[tauri::command]
pub async fn install_systemd_service(
  core_manager: State<'_, Shared<CoreManager>>,
  config_path: String,
) -> Result<(), String> {
  #[cfg(not(target_os = "linux"))]
  {
    return Err("仅支持 Linux(systemd)".into());
  }

  #[cfg(target_os = "linux")]
  {
    let core_path = {
      let mgr = core_manager.lock().await;
      default_core_path_from_manager(&mgr).ok_or_else(|| "未找到内核路径，请先安装内核".to_string())?
    };
    // 复制 current 的二进制到 /usr/local/bin/mihomo（Fedora 推荐路径），并以此为 ExecStart
    let real = std::fs::canonicalize(&core_path).map_err(|e| e.to_string())?;
    let bin_target = "/usr/local/bin/mihomo";
    let etc_dir = "/etc/mihomo";
    let etc_cfg = "/etc/mihomo/config.yaml";
    let unit = render_systemd_unit(bin_target, etc_cfg);
    let sh = format!(
      concat!(
        "set -e; ",
        // 安装到 /usr/local/bin/mihomo，优先用 install，不存在时退回到 cp
        "install -Dm755 '{src}' '{dst}' 2>/dev/null || (cp -f '{src}' '{dst}' && chmod 755 '{dst}'); ",
        // SELinux 恢复上下文（Fedora）
        "command -v restorecon >/dev/null 2>&1 && restorecon -F '{dst}' || true; ",
        // 复制配置到 /etc/mihomo/config.yaml
        "install -d '{etc_dir}'; install -m644 '{cfg_src}' '{cfg_dst}'; ",
        "command -v restorecon >/dev/null 2>&1 && restorecon -F '{cfg_dst}' || true; ",
        // 写入 unit
        "cat > /etc/systemd/system/mihomo-gui.service <<'EOF'\n{unit}\nEOF\n",
        // 重新加载并启用启动
        "systemctl daemon-reload; systemctl enable --now mihomo-gui.service\n"
      ),
      src = real.to_string_lossy(),
      dst = bin_target,
      etc_dir = etc_dir,
      cfg_src = config_path,
      cfg_dst = etc_cfg,
      unit = unit
    );
    let status = Command::new("pkexec")
      .arg("/bin/sh")
      .arg("-lc")
      .arg(sh)
      .status()
      .map_err(|e| e.to_string())?;
    if !status.success() {
      return Err("安装 systemd 服务失败（被取消或执行错误）".into());
    }
    Ok(())
  }
}

#[tauri::command]
pub async fn uninstall_systemd_service(delete_binary: bool, delete_config: bool) -> Result<(), String> {
  #[cfg(not(target_os = "linux"))]
  {
    return Err("仅支持 Linux(systemd)".into());
  }
  #[cfg(target_os = "linux")]
  {
    let sh = format!(
      concat!(
        "set -e; ",
        "systemctl disable --now mihomo-gui.service >/dev/null 2>&1 || true; ",
        "rm -f /etc/systemd/system/mihomo-gui.service; ",
        "systemctl daemon-reload; ",
        "{rm_bin}",
        "{rm_cfg}"
      ),
      rm_bin = if delete_binary { "rm -f /usr/local/bin/mihomo; " } else { "" },
      rm_cfg = if delete_config { "rm -f /etc/mihomo/config.yaml; rmdir /etc/mihomo 2>/dev/null || true; " } else { "" }
    );
    let status = Command::new("pkexec")
      .arg("/bin/sh")
      .arg("-lc")
      .arg(sh)
      .status()
      .map_err(|e| e.to_string())?;
    if !status.success() {
      return Err("卸载 systemd 服务失败（被取消或执行错误）".into());
    }
    Ok(())
  }
}

#[tauri::command]
pub async fn systemd_service_status() -> Result<String, String> {
  #[cfg(not(target_os = "linux"))]
  {
    return Ok("unsupported".into());
  }
  #[cfg(target_os = "linux")]
  {
    let active = Command::new("systemctl").arg("is-active").arg("mihomo-gui.service").output().map_err(|e| e.to_string())?;
    let enabled = Command::new("systemctl").arg("is-enabled").arg("mihomo-gui.service").output().map_err(|e| e.to_string())?;
    let a = String::from_utf8_lossy(&active.stdout).trim().to_string();
    let e = String::from_utf8_lossy(&enabled.stdout).trim().to_string();
    Ok(format!("{}|{}", a, e))
  }
}
#[tauri::command]
pub async fn request_privilege() -> Result<bool, String> {
  #[cfg(target_os = "windows")]
  {
    let status = Command::new("powershell")
      .arg("-NoProfile")
      .arg("-Command")
      .arg("Start-Process -Verb RunAs powershell -ArgumentList '-NoProfile -Command Write-Output ok'")
      .status()
      .map_err(|e| e.to_string())?;
    return Ok(status.success());
  }

  #[cfg(target_os = "macos")]
  {
    let status = Command::new("osascript")
      .arg("-e")
      .arg("do shell script \"echo ok\" with administrator privileges")
      .status()
      .map_err(|e| e.to_string())?;
    return Ok(status.success());
  }

  #[cfg(target_os = "linux")]
  {
    let status = Command::new("pkexec")
      .arg("/bin/sh")
      .arg("-lc")
      .arg("echo ok")
      .status()
      .map_err(|e| e.to_string())?;
    return Ok(status.success());
  }

  #[allow(unreachable_code)]
  Ok(false)
}

#[tauri::command]
pub async fn grant_tun_cap(core_manager: State<'_, Shared<CoreManager>>) -> Result<bool, String> {
  #[cfg(target_os = "linux")]
  {
    let core_path = {
      let mgr = core_manager.lock().await;
      mgr.core_path.clone().ok_or_else(|| "尚未安装或设置内核路径".to_string())?
    };
    let cmd = format!(
      "set -e; command -v setcap >/dev/null 2>&1 || (echo 'setcap 未安装，请安装 libcap2-bin 或对应软件包' >&2; exit 1); setcap 'cap_net_admin,cap_net_bind_service=+eip' '{}'",
      core_path.display()
    );
    let status = Command::new("pkexec")
      .arg("/bin/sh")
      .arg("-lc")
      .arg(cmd)
      .status()
      .map_err(|e| e.to_string())?;
    return Ok(status.success());
  }

  #[cfg(any(target_os = "macos", target_os = "windows"))]
  {
    Err("当前平台不支持通过 setcap 赋权；请以管理员权限运行或关闭 TUN".into())
  }
}
