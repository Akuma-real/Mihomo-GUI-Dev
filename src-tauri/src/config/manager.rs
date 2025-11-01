use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default)]
pub struct ConfigManager {
  pub config_dir: PathBuf,
  pub current_config: Option<PathBuf>,
}

impl ConfigManager {
  pub fn new() -> io::Result<Self> {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("mihomo-gui").join("configs");
    fs::create_dir_all(&dir)?;
    Ok(Self { config_dir: dir, current_config: None })
  }

  pub fn load_all_configs(&self) -> io::Result<Vec<ConfigInfo>> {
    let mut result = Vec::new();
    if !self.config_dir.exists() {
      return Ok(result);
    }
    for entry in fs::read_dir(&self.config_dir)? {
      let entry = entry?;
      let path = entry.path();
      if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if matches!(ext, "yaml" | "yml") {
          let meta = entry.metadata()?;
          let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
          result.push(ConfigInfo::from_path_and_meta(&path, meta.len(), modified));
        }
      }
    }
    Ok(result)
  }

  pub fn validate(&self, config_path: &Path) -> io::Result<ValidationResult> {
    let text = fs::read_to_string(config_path)?;
    // 尝试用 serde_yaml 解析，不要求完整 schema
    let yaml: serde_yaml::Value = match serde_yaml::from_str(&text) {
      Ok(v) => v,
      Err(_) => {
        return Ok(ValidationResult {
          is_valid: false,
          warnings: vec!["YAML 解析失败".to_string()],
          needs_privilege: false,
        })
      }
    };

    let mut warnings = Vec::new();
    let mut needs_privilege = false;

    // mixed-port
    if yaml.get("mixed-port").is_none() && yaml.get("mixed_port").is_none() {
      warnings.push("缺少 mixed-port".to_string());
    }
    // external-controller
    if yaml.get("external-controller").is_none() && yaml.get("external_controller").is_none() {
      warnings.push("缺少 external-controller".to_string());
    }
    // tun.enabled
    if let Some(tun) = yaml.get("tun") {
      if tun.get("enable").and_then(|v| v.as_bool()).unwrap_or(false)
        || tun.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false)
        || tun.get("enable").is_some() && tun.get("enable").unwrap() == &serde_yaml::Value::Bool(true)
      {
        needs_privilege = true;
      }
    }

    Ok(ValidationResult { is_valid: true, warnings, needs_privilege })
  }

  pub fn import_config(&self, source_path: &Path) -> io::Result<PathBuf> {
    let file_name = source_path
      .file_name()
      .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file path"))?;
    let target = self.config_dir.join(file_name);
    fs::create_dir_all(&self.config_dir)?;
    fs::copy(source_path, &target)?;
    Ok(target)
  }

  pub fn export_config(&self, config_path: &Path, target_path: &Path) -> io::Result<()> {
    fs::copy(config_path, target_path)?;
    Ok(())
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigInfo {
  pub name: String,
  pub path: String,
  pub size: u64,
  pub modified: String,
}

impl ConfigInfo {
  fn from_path_and_meta(path: &Path, size: u64, modified: SystemTime) -> Self {
    let ts = modified.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let name = path
      .file_stem()
      .map(|s| s.to_string_lossy().to_string())
      .unwrap_or_else(|| "unknown".into());
    Self {
      name,
      path: path.to_string_lossy().to_string(),
      size,
      modified: format!("{}", ts),
    }
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
  pub is_valid: bool,
  pub warnings: Vec<String>,
  pub needs_privilege: bool,
}

