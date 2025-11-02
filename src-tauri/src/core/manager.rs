use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::io::{BufRead, BufReader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreStatus {
  Running,
  Stopped,
  Error,
}

impl Default for CoreStatus {
  fn default() -> Self {
    CoreStatus::Stopped
  }
}

#[derive(Debug, Default)]
pub struct CoreManager {
  pub status: CoreStatus,
  pub current_config: Option<PathBuf>,
  pub core_path: Option<PathBuf>,
  pub child: Option<Child>,
}

impl CoreManager {
  pub fn set_core_path(&mut self, path: PathBuf) {
    self.core_path = Some(path);
  }

  pub fn start(&mut self, config_path: PathBuf) -> Result<(), String> {
    self.current_config = Some(config_path.clone());
    let core = self
      .core_path
      .as_ref()
      .ok_or_else(|| "尚未设置内核可执行文件路径".to_string())?;

    if !core.exists() {
      return Err("内核可执行文件不存在".into());
    }
    if !config_path.exists() {
      return Err("配置文件不存在".into());
    }

    // 以常见方式启动：mihomo -f <config>
    let mut cmd = Command::new(core);
    cmd.arg("-f").arg(&config_path);
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());

    match cmd.spawn() {
      Ok(mut child) => {
        // 将子进程的 stdout/stderr 转发到日志，便于在 dev 终端观察运行日志
        if let Some(stdout) = child.stdout.take() {
          std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
              match line {
                Ok(l) => log::info!(target: "mihomo-core", "[stdout] {}", l),
                Err(e) => {
                  log::error!(target: "mihomo-core", "读取 stdout 失败: {}", e);
                  break;
                }
              }
            }
          });
        }
        if let Some(stderr) = child.stderr.take() {
          std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
              match line {
                Ok(l) => log::warn!(target: "mihomo-core", "[stderr] {}", l),
                Err(e) => {
                  log::error!(target: "mihomo-core", "读取 stderr 失败: {}", e);
                  break;
                }
              }
            }
          });
        }
        self.child = Some(child);
        self.status = CoreStatus::Running;
        Ok(())
      }
      Err(e) => {
        self.status = CoreStatus::Error;
        Err(format!("启动失败: {}", e))
      }
    }
  }

  pub fn stop(&mut self) -> Result<(), String> {
    if let Some(child) = self.child.as_mut() {
      if let Err(e) = child.kill() {
        return Err(format!("停止进程失败: {}", e));
      }
      let _ = child.wait();
    }
    self.child = None;
    self.status = CoreStatus::Stopped;
    Ok(())
  }

  pub fn get_status(&mut self) -> CoreStatus {
    if let Some(child) = self.child.as_mut() {
      match child.try_wait() {
        Ok(Some(_)) => CoreStatus::Stopped,
        Ok(None) => CoreStatus::Running,
        Err(_) => CoreStatus::Error,
      }
    } else {
      self.status
    }
  }

  pub fn restart(&mut self) -> Result<(), String> {
    let cfg = self
      .current_config
      .clone()
      .ok_or_else(|| "尚未指定配置文件".to_string())?;
    self.stop().ok();
    self.start(cfg)
  }
}
