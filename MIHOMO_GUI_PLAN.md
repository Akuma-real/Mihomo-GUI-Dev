# Mihomo GUI 应用程序实施计划

## 1. 项目概述

### 1.1 功能需求
- 核心版本切换（稳定版/开发版）
- 内核下载、更新、启动、停止管理
- 强制配置文件选择才能启动
- Tun 模式检测与权限处理
- WebUI 自动检测与一键打开

### 1.2 技术栈
- **前端**: Next.js 16 + React 19 + TypeScript
- **UI 库**: Tailwind CSS + shadcn/ui 组件
- **后端**: Tauri 2.x + Rust
- **通信**: Tauri Command API + IPC + WebSocket
- **平台**: Windows/macOS/Linux

## 2. 项目结构设计

```
Mihomo-GUI-Dev/
├── src-tauri/
│   ├── src/
│   │   ├── core/               # 内核管理模块
│   │   │   ├── manager.rs      # 内核进程管理
│   │   │   ├── downloader.rs   # 内核下载器
│   │   │   ├── version.rs      # 版本管理
│   │   │   └── mod.rs
│   │   ├── config/             # 配置管理
│   │   │   ├── parser.rs       # 配置文件解析
│   │   │   ├── validator.rs    # 配置验证
│   │   │   ├── manager.rs      # 配置管理器
│   │   │   └── mod.rs
│   │   ├── api/                # API 客户端
│   │   │   ├── mihomo.rs       # Mihomo API 客户端
│   │   │   └── mod.rs
│   │   ├── system/             # 系统功能
│   │   │   ├── permission.rs   # 权限检测与提升
│   │   │   ├── tray.rs         # 托盘功能
│   │   │   └── mod.rs
│   │   ├── commands/           # Tauri 命令
│   │   │   ├── core_commands.rs
│   │   │   ├── config_commands.rs
│   │   │   ├── system_commands.rs
│   │   │   └── mod.rs
│   │   ├── utils/              # 工具函数
│   │   │   ├── logger.rs       # 日志管理
│   │   │   ├── file.rs         # 文件操作
│   │   │   └── mod.rs
│   │   ├── main.rs
│   │   └── lib.rs
│   ├── capabilities/
│   │   └── desktop.json        # Tauri 权限配置
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
├── app/
│   ├── (dashboard)/            # 主仪表板路由组
│   │   ├── layout.tsx          # 仪表板布局
│   │   └── page.tsx            # 主页仪表板
│   ├── core/                   # 核心管理页面
│   │   ├── page.tsx
│   │   └── components/
│   │       ├── VersionSwitcher.tsx
│   │       ├── CoreStatus.tsx
│   │       └── CoreControls.tsx
│   ├── config/                 # 配置管理页面
│   │   ├── page.tsx
│   │   └── components/
│   │       ├── ConfigSelector.tsx
│   │       ├── ConfigEditor.tsx
│   │       └── ConfigValidator.tsx
│   ├── proxies/                # 代理节点管理
│   │   ├── page.tsx
│   │   └── components/
│   │       ├── ProxyList.tsx
│   │       └── ProxyEditor.tsx
│   ├── logs/                   # 日志查看
│   │   ├── page.tsx
│   │   └── components/
│   │       └── LogViewer.tsx
│   ├── settings/               # 设置页面
│   │   ├── page.tsx
│   │   └── components/
│   │       ├── TunSettings.tsx
│   │       └── GeneralSettings.tsx
│   ├── components/             # 共享组件
│   │   ├── ui/                 # shadcn/ui 组件
│   │   ├── Layout.tsx          # 主布局
│   │   ├── Navbar.tsx          # 导航栏
│   │   └── StatusBar.tsx       # 状态栏
│   ├── hooks/                  # React Hooks
│   │   ├── useMihomo.ts        # Mihomo 状态管理
│   │   ├── useConfig.ts        # 配置管理
│   │   └── useWebSocket.ts     # WebSocket 连接
│   ├── lib/                    # 工具库
│   │   ├── api.ts              # 前端 API 客户端
│   │   ├── types.ts            # TypeScript 类型
│   │   └── utils.ts
│   ├── layout.tsx
│   ├── page.tsx
│   └── globals.css
├── public/
│   └── icons/                  # 应用图标
├── package.json
├── next.config.ts
├── tailwind.config.ts
└── tsconfig.json
```

## 3. 核心模块设计

### 3.1 内核管理模块 (core/manager.rs)

```rust
#[derive(Debug, Clone)]
pub struct CoreManager {
    // 当前运行的内核进程
    current_process: Option<Child>,
    // 内核版本信息
    version: Option<String>,
    // 内核路径
    core_path: PathBuf,
    // 配置文件路径
    config_path: Option<PathBuf>,
    // 运行时目录
    runtime_dir: PathBuf,
}

impl CoreManager {
    /// 启动内核
    pub async fn start(&mut self, config_path: &Path, need_privilege: bool) -> Result<()> {
        // 1. 验证配置
        let config = ConfigParser::parse(config_path).await?;

        // 2. 检测 Tun 模式
        let need_tun = config.tun_enabled();

        // 3. 检查权限
        if need_tun && need_privilege {
            self.request_privilege().await?;
        }

        // 4. 启动内核进程
        let mut cmd = Command::new(&self.core_path);
        cmd.arg("-f")
           .arg(config_path)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let child = cmd.spawn()?;
        self.current_process = Some(child);
        Ok(())
    }

    /// 停止内核
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.current_process.take() {
            child.kill().await?;
            child.wait().await?;
        }
        Ok(())
    }

    /// 重启内核
    pub async fn restart(&mut self) -> Result<()> {
        self.stop().await?;
        // 重新启动逻辑
        Ok(())
    }

    /// 获取内核状态
    pub fn get_status(&self) -> CoreStatus {
        match &self.current_process {
            Some(child) => {
                if child.is_finished() {
                    CoreStatus::Stopped
                } else {
                    CoreStatus::Running
                }
            }
            None => CoreStatus::Stopped,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CoreStatus {
    Running,
    Stopped,
    Error(String),
}
```

### 3.2 版本管理模块 (core/version.rs)

```rust
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: String,
    pub release_date: String,
    pub download_url: String,
    pub checksum: String,
    pub channel: ReleaseChannel, // Stable | Dev
}

#[derive(Debug, Clone)]
pub enum ReleaseChannel {
    Stable,
    Dev,
}

pub struct VersionManager {
    // 版本信息缓存
    versions: HashMap<ReleaseChannel, VersionInfo>,
    // 下载缓存目录
    cache_dir: PathBuf,
    // 自动更新设置
    auto_update: bool,
}

impl VersionManager {
    /// 获取最新版本信息
    pub async fn fetch_latest(&mut self, channel: ReleaseChannel) -> Result<VersionInfo> {
        let url = match channel {
            ReleaseChannel::Stable => {
                "https://api.github.com/repos/mihomo-org/mihomo/releases/latest"
            }
            ReleaseChannel::Dev => {
                "https://api.github.com/repos/mihomo-org/mihomo/releases"
            }
        };

        let response = reqwest::get(url).await?;
        let versions: Vec<VersionInfo> = response.json().await?;

        // 选择最新版本
        let latest = versions.into_iter().next()
            .ok_or_else(|| anyhow!("No versions found"))?;

        self.versions.insert(channel.clone(), latest.clone());
        Ok(latest)
    }

    /// 下载内核
    pub async fn download(&self, version: &VersionInfo) -> Result<PathBuf> {
        let target_file = self.cache_dir
            .join(format!("mihomo-{}-{}.tar.gz",
                version.version,
                self.get_target_arch()));

        if target_file.exists() {
            return Ok(target_file);
        }

        // 下载并解压
        let response = reqwest::get(&version.download_url).await?;
        let bytes = response.bytes().await?;

        // 验证校验和
        let computed_checksum = sha256::digest(&bytes);
        if computed_checksum != version.checksum {
            return Err(anyhow!("Checksum mismatch"));
        }

        // 写入文件
        tokio::fs::write(&target_file, &bytes).await?;

        // 解压到可执行目录
        let extracted_path = self.extract(&target_file).await?;

        Ok(extracted_path)
    }

    /// 获取目标架构
    fn get_target_arch(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "windows-amd64";

        #[cfg(target_os = "macos")]
        return match std::env::consts::ARCH {
            "aarch64" => "darwin-arm64",
            "x86_64" => "darwin-amd64",
            _ => unreachable!(),
        };

        #[cfg(target_os = "linux")]
        return "linux-amd64";
    }
}
```

实现对齐（当前代码行为）：
- 仓库固定为 `MetaCubeX/mihomo`。
- 稳定版：读取 latest release 的 assets 中的 `version.txt` 内容作为版本号。
- 开发版：读取 releases 列表中首个 `prerelease == true` 的 release 的 `version.txt` 内容。
- 不再使用 tag_name 作为兜底；若缺少 `version.txt`，直接报错。
- 支持通过 `GITHUB_TOKEN` 提升 GitHub API 配额；请求头包含 `Accept` 与 `User-Agent`。

### 3.3 配置管理模块 (config/manager.rs)

```rust
pub struct ConfigManager {
    // 配置文件目录
    config_dir: PathBuf,
    // 当前选中的配置
    current_config: Option<PathBuf>,
    // 配置监听器
    watchers: Vec<notify::RecommendedWatcher>,
}

impl ConfigManager {
    /// 初始化配置管理器
    pub async fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mihomo-gui")
            .join("configs");

        // 创建配置目录
        tokio::fs::create_dir_all(&config_dir).await?;

        Ok(ConfigManager {
            config_dir,
            current_config: None,
            watchers: Vec::new(),
        })
    }

    /// 加载所有配置
    pub async fn load_all_configs(&self) -> Result<Vec<ConfigInfo>> {
        let mut configs = Vec::new();

        let entries = tokio::fs::read_dir(&self.config_dir).await?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                let content = tokio::fs::read_to_string(&path).await?;
                let parsed = ConfigParser::parse_str(&content)?;

                configs.push(ConfigInfo {
                    name: path.file_stem().unwrap().to_string_lossy().to_string(),
                    path: path.clone(),
                    size: entry.metadata().await?.len(),
                    modified: entry.metadata().await?.modified()?,
                    parsed,
                });
            }
        }

        Ok(configs)
    }

    /// 验证配置文件
    pub async fn validate(&self, config_path: &Path) -> Result<ValidationResult> {
        let content = tokio::fs::read_to_string(config_path).await?;
        let config = ConfigParser::parse_str(&content)?;

        let issues = vec![
            (config.mixed_port.is_some(), "Mixed port not configured"),
            (config.external_controller.is_some(), "External controller not set"),
            (config.tun.is_some(), "TUN mode configured"),
        ];

        let mut warnings = Vec::new();
        let mut has_tun = false;

        for (check, message) in issues {
            if !check {
                warnings.push(message.to_string());
            }
            if message.contains("TUN") {
                has_tun = true;
            }
        }

        Ok(ValidationResult {
            is_valid: true, // 基础验证通过
            warnings,
            needs_privilege: has_tun,
            config,
        })
    }

    /// 监听配置文件变化
    pub async fn watch_config(&mut self, path: &Path) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);
        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(path, notify::RecursiveMode::NonRecursive)?;

        self.watchers.push(watcher);

        // 处理变化事件
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    Ok(event) => {
                        // 通知前端配置已更新
                        println!("Config file changed: {:?}", event);
                    }
                    Err(e) => {
                        eprintln!("Watch error: {:?}", e);
                    }
                }
            }
        });

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConfigInfo {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub parsed: ParsedConfig,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub warnings: Vec<String>,
    pub needs_privilege: bool,
    pub config: ParsedConfig,
}
```

### 3.4 权限处理模块 (system/permission.rs)

```rust
pub struct PermissionManager;

impl PermissionManager {
    /// 检查是否需要管理员权限
    pub fn check_tun_requirement(config: &ParsedConfig) -> bool {
        config.tun.is_some() || config.tun.enabled
    }

    /// 启动带权限的进程 (Windows)
    #[cfg(target_os = "windows")]
    pub async fn request_admin_privilege() -> Result<bool> {
        use std::process::Command;

        let current_exe = env::current_exe()?;
        let args = env::args().collect::<Vec<_>>();

        // 使用 ShellExecute 启动管理员进程
        let result = Command::new("powershell")
            .arg("-Command")
            .arg(&format!(
                "Start-Process -FilePath '{}' -ArgumentList '{}' -Verb RunAs",
                current_exe.display(),
                args.join("' '")
            ))
            .spawn()?;

        Ok(result.success())
    }

    /// 启动带权限的进程 (macOS)
    #[cfg(target_os = "macos")]
    pub async fn request_admin_privilege() -> Result<bool> {
        use std::process::Command;

        let current_exe = env::current_exe()?;
        let args = env::args().collect::<Vec<_>>();

        let result = Command::new("osascript")
            .arg("-e")
            .arg(&format!(
                "do shell script \"{} {}\" with administrator privileges",
                current_exe.display(),
                args.join(" ")
            ))
            .spawn()?;

        Ok(result.success())
    }

    /// 启动带权限的进程 (Linux)
    #[cfg(target_os = "linux")]
    pub async fn request_admin_privilege() -> Result<bool> {
        use std::process::Command;

        // 尝试使用 pkexec
        let current_exe = env::current_exe()?;
        let args = env::args().collect::<Vec<_>>();

        let result = Command::new("pkexec")
            .args(&args)
            .spawn()?;

        Ok(result.success())
    }

    /// 检查当前权限状态
    pub fn check_current_privileges() -> PrivilegeLevel {
        #[cfg(target_os = "windows")]
        {
            let is_admin = is_root();
            if is_admin {
                PrivilegeLevel::Admin
            } else {
                PrivilegeLevel::User
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if unsafe { libc::getuid() } == 0 {
                PrivilegeLevel::Root
            } else {
                PrivilegeLevel::User
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PrivilegeLevel {
    User,
    Admin,
    Root,
}
```

### 3.5 API 客户端模块 (api/mihomo.rs)

```rust
pub struct MihomoApiClient {
    base_url: String,
    secret: Option<String>,
    client: reqwest::Client,
}

impl MihomoApiClient {
    pub fn new(base_url: String, secret: Option<String>) -> Self {
        MihomoApiClient {
            base_url,
            secret,
            client: reqwest::Client::new(),
        }
    }

    /// 发起 API 请求
    async fn request<T>(&self, method: Method, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.request(method, url);

        if let Some(secret) = &self.secret {
            request = request.header("Authorization", format!("Bearer {}", secret));
        }

        let response = request.send().await?;
        let data = response.json().await?;
        Ok(data)
    }

    /// 获取配置
    pub async fn get_configs(&self) -> Result<Vec<ConfigInfo>> {
        self.request(Method::GET, "/configs").await
    }

    /// 更新配置
    pub async fn update_config(&self, config: &ConfigContent) -> Result<()> {
        self.request(Method::POST, "/configs").await
    }

    /// 获取代理列表
    pub async fn get_proxies(&self) -> Result<HashMap<String, ProxyInfo>> {
        self.request(Method::GET, "/proxies").await
    }

    /// 切换代理
    pub async fn select_proxy(&self, group: &str, proxy: &str) -> Result<()> {
        let path = format!("/proxies/{}/select", group);
        let body = json!({ "name": proxy });
        self.request(Method::POST, &path).await
    }

    /// 获取流量统计
    pub async fn get_traffic(&self) -> Result<TrafficInfo> {
        self.request(Method::GET, "/traffic").await
    }

    /// 获取日志
    pub async fn get_logs(&self, level: LogLevel) -> Result<String> {
        let path = format!("/logs?level={}", level.as_str());
        self.request(Method::GET, &path).await
    }

    /// 检测 WebUI 是否可用
    pub async fn check_webui(&self) -> Result<bool> {
        match self.get_configs().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ProxyInfo {
    pub name: String,
    pub type_: String,
    pub udp: bool,
    pub xudp: Option<bool>,
    pub history: Option<Vec<HistoryItem>>,
    pub all: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct TrafficInfo {
    pub up: u64,
    pub down: u64,
}

#[derive(Debug, Deserialize)]
pub struct HistoryItem {
    pub time: u64,
    pub value: String,
}
```

## 4. Tauri 命令定义

### 4.1 核心管理命令 (commands/core_commands.rs)

```rust
#[tauri::command]
pub async fn start_core(
    core_manager: State<'_, Mutex<CoreManager>>,
    config_path: String,
    need_privilege: bool,
) -> Result<(), String> {
    let mut manager = core_manager.lock().await;
    let path = PathBuf::from(config_path);

    manager.start(&path, need_privilege)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_core(
    core_manager: State<'_, Mutex<CoreManager>>,
) -> Result<(), String> {
    let mut manager = core_manager.lock().await;
    manager.stop().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restart_core(
    core_manager: State<'_, Mutex<CoreManager>>,
) -> Result<(), String> {
    let mut manager = core_manager.lock().await;
    manager.restart().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_core_status(
    core_manager: State<'_, Mutex<CoreManager>>,
) -> CoreStatus {
    let manager = core_manager.lock().await;
    manager.get_status()
}

#[tauri::command]
pub async fn download_core(
    version_manager: State<'_, Mutex<VersionManager>>,
    channel: String,
    version: Option<String>,
) -> Result<String, String> {
    let mut manager = version_manager.lock().await;
    let channel_enum = match channel.as_str() {
        "stable" => ReleaseChannel::Stable,
        "dev" => ReleaseChannel::Dev,
        _ => return Err("Invalid channel".to_string()),
    };

    // 获取或下载版本
    let version_info = if let Some(v) = version {
        // 指定版本下载
        // TODO: 实现版本详情获取
        todo!()
    } else {
        // 获取最新版本
        manager.fetch_latest(channel_enum)
            .await
            .map_err(|e| e.to_string())?
    };

    manager.download(&version_info)
        .await
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}
```

### 4.2 配置管理命令 (commands/config_commands.rs)

```rust
#[tauri::command]
pub async fn load_all_configs(
    config_manager: State<'_, Mutex<ConfigManager>>,
) -> Result<Vec<ConfigInfo>, String> {
    let manager = config_manager.lock().await;
    manager.load_all_configs()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_config(
    config_manager: State<'_, Mutex<ConfigManager>>,
    config_path: String,
) -> Result<ValidationResult, String> {
    let manager = config_manager.lock().await;
    let path = PathBuf::from(config_path);

    manager.validate(&path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_config(
    config_manager: State<'_, Mutex<ConfigManager>>,
    source_path: String,
) -> Result<String, String> {
    let mut manager = config_manager.lock().await;
    let source = PathBuf::from(source_path);

    // 生成目标路径
    let file_name = source.file_name()
        .ok_or("Invalid file path")?
        .to_string_lossy();

    let target = manager.config_dir.join(file_name.as_ref());

    // 复制文件
    tokio::fs::copy(&source, &target)
        .await
        .map_err(|e| e.to_string())?;

    Ok(target.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_config(
    config_manager: State<'_, Mutex<ConfigManager>>,
    config_path: String,
    target_path: String,
) -> Result<(), String> {
    let manager = config_manager.lock().await;
    let source = PathBuf::from(config_path);
    let target = PathBuf::from(target_path);

    tokio::fs::copy(&source, &target)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
```

### 4.3 系统功能命令 (commands/system_commands.rs)

```rust
#[tauri::command]
pub async fn check_tun_requirement(
    config_path: String,
) -> Result<bool, String> {
    let content = tokio::fs::read_to_string(&config_path)
        .await
        .map_err(|e| e.to_string())?;

    let config = ConfigParser::parse_str(&content)?;
    Ok(PermissionManager::check_tun_requirement(&config))
}

#[tauri::command]
pub async fn request_admin_privilege() -> Result<bool, String> {
    PermissionManager::request_admin_privilege()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_current_privileges() -> PrivilegeLevel {
    PermissionManager::check_current_privileges()
}

#[tauri::command]
pub async fn check_webui(
    base_url: String,
    secret: Option<String>,
) -> Result<bool, String> {
    let client = MihomoApiClient::new(base_url, secret);
    client.check_webui().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_webui(
    base_url: String,
    secret: Option<String>,
) -> Result<(), String> {
    // 打开默认浏览器
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .arg("/c")
            .arg("start")
            .arg(&base_url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&base_url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&base_url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
```

- 核心管理：`start_core`、`stop_core`、`restart_core`、`get_core_status`
- 版本管理：`fetch_latest_version`（从 release 资产 `version.txt` 读取版本号）
- 配置管理：`load_all_configs`、`validate_config`、`import_config`、`export_config`

## 5. 前端组件设计

### 5.1 核心状态管理 (hooks/useMihomo.ts)

```typescript
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

type CoreStatus = 'running' | 'stopped' | 'error';

export function useMihomo() {
  const [status, setStatus] = useState<CoreStatus>('stopped');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const start = async (configPath: string) => {
    setIsLoading(true);
    setError(null);

    try {
      // 检查是否需要权限
      const needsPrivilege = await invoke<boolean>('check_tun_requirement', {
        configPath,
      });

      if (needsPrivilege) {
        const granted = await invoke<boolean>('request_admin_privilege');
        if (!granted) {
          throw new Error('权限被拒绝，无法启动 TUN 模式');
        }
      }

      await invoke('start_core', {
        configPath,
        needPrivilege: needsPrivilege,
      });

      setStatus('running');
    } catch (err: any) {
      setError(err.message || '启动失败');
      setStatus('error');
    } finally {
      setIsLoading(false);
    }
  };

  const stop = async () => {
    setIsLoading(true);
    setError(null);

    try {
      await invoke('stop_core');
      setStatus('stopped');
    } catch (err: any) {
      setError(err.message || '停止失败');
    } finally {
      setIsLoading(false);
    }
  };

  // 定期检查状态
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const currentStatus = await invoke<CoreStatus>('get_core_status');
        setStatus(currentStatus);
      } catch (err) {
        console.error('Failed to get status:', err);
      }
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  return {
    status,
    isLoading,
    error,
    start,
    stop,
  };
}
```

### 5.2 核心控制组件 (core/components/CoreControls.tsx)

```typescript
'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { AlertCircle, Play, Square, RotateCcw } from 'lucide-react';
import { useMihomo } from '@/hooks/useMihomo';

export function CoreControls() {
  const { status, isLoading, error, start, stop } = useMihomo();

  const handleStart = async () => {
    // TODO: 显示文件选择对话框
    const configPath = await selectConfigFile();
    if (configPath) {
      await start(configPath);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>内核控制</CardTitle>
        <CardDescription>
          管理 Mihomo 内核的启动、停止和重启
        </CardDescription>
      </CardHeader>
      <CardContent>
        {error && (
          <div className="flex items-center gap-2 text-destructive mb-4">
            <AlertCircle className="h-4 w-4" />
            <span className="text-sm">{error}</span>
          </div>
        )}

        <div className="flex gap-2">
          {status === 'stopped' && (
            <Button onClick={handleStart} disabled={isLoading}>
              <Play className="h-4 w-4 mr-2" />
              启动
            </Button>
          )}

          {status === 'running' && (
            <>
              <Button onClick={stop} variant="destructive" disabled={isLoading}>
                <Square className="h-4 w-4 mr-2" />
                停止
              </Button>
              <Button variant="outline" disabled={isLoading}>
                <RotateCcw className="h-4 w-4 mr-2" />
                重启
              </Button>
            </>
          )}

          <div className="ml-auto flex items-center gap-2">
            <div
              className={`h-2 w-2 rounded-full ${
                status === 'running'
                  ? 'bg-green-500'
                  : status === 'error'
                  ? 'bg-red-500'
                  : 'bg-gray-300'
              }`}
            />
            <span className="text-sm text-muted-foreground">
              {status === 'running' && '运行中'}
              {status === 'stopped' && '已停止'}
              {status === 'error' && '错误'}
            </span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

// TODO: 实现文件选择对话框
async function selectConfigFile(): Promise<string | null> {
  // 使用 Tauri dialog API
  const { open } = await import('@tauri-apps/plugin-dialog');
  const file = await open({
    filters: [
      {
        name: 'Mihomo 配置',
        extensions: ['yaml', 'yml'],
      },
    ],
  });

  return file;
}
```

### 5.3 版本切换器 (core/components/VersionSwitcher.tsx)

```typescript
'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import { Download, Check } from 'lucide-react';
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type ReleaseChannel = 'stable' | 'dev';

interface Version {
  version: string;
  releaseDate: string;
  channel: ReleaseChannel;
  downloadUrl: string;
}

export function VersionSwitcher() {
  const [currentChannel, setCurrentChannel] = useState<ReleaseChannel>('stable');
  const [currentVersion, setCurrentVersion] = useState<string>('');
  const [availableVersions, setAvailableVersions] = useState<Version[]>([]);
  const [isDownloading, setIsDownloading] = useState(false);

  const handleChannelChange = async (channel: ReleaseChannel) => {
    setCurrentChannel(channel);
    // TODO: 获取该渠道的最新版本
    const versions = await invoke<Version[]>('get_versions', { channel });
    setAvailableVersions(versions);
  };

  const handleDownload = async (version: Version) => {
    setIsDownloading(true);
    try {
      await invoke('download_core', {
        channel: version.channel,
        version: version.version,
      });
      setCurrentVersion(version.version);
    } catch (err) {
      console.error('Download failed:', err);
    } finally {
      setIsDownloading(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>内核版本管理</CardTitle>
        <CardDescription>
          选择和下载 Mihomo 内核版本
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center gap-4">
          <div className="flex-1">
            <label className="text-sm font-medium mb-2 block">
              发布渠道
            </label>
            <Select value={currentChannel} onValueChange={handleChannelChange}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="stable">
                  <div className="flex items-center gap-2">
                    <Badge variant="secondary">稳定版</Badge>
                    <span>推荐用于生产环境</span>
                  </div>
                </SelectItem>
                <SelectItem value="dev">
                  <div className="flex items-center gap-2">
                    <Badge variant="outline">开发版</Badge>
                    <span>包含最新功能</span>
                  </div>
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          {currentVersion && (
            <div className="flex items-center gap-2">
              <Check className="h-4 w-4 text-green-500" />
              <span className="text-sm text-muted-foreground">
                当前版本: {currentVersion}
              </span>
            </div>
          )}
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">可用版本</label>
          {availableVersions.map((version) => (
            <div
              key={version.version}
              className="flex items-center justify-between p-3 border rounded-lg"
            >
              <div className="flex items-center gap-3">
                <div>
                  <p className="font-medium">{version.version}</p>
                  <p className="text-sm text-muted-foreground">
                    {version.releaseDate}
                  </p>
                </div>
                <Badge>{version.channel}</Badge>
              </div>

              <Button
                size="sm"
                onClick={() => handleDownload(version)}
                disabled={isDownloading || currentVersion === version.version}
              >
                <Download className="h-4 w-4 mr-2" />
                {currentVersion === version.version ? '已安装' : '下载'}
              </Button>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
```

### 5.4 配置选择器 (config/components/ConfigSelector.tsx)

```typescript
'use client';

import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { AlertCircle, Upload, FileText } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

interface ConfigInfo {
  name: string;
  path: string;
  size: number;
  modified: string;
  warnings: string[];
  needsPrivilege: boolean;
}

export function ConfigSelector() {
  const [configs, setConfigs] = useState<ConfigInfo[]>([]);
  const [selectedConfig, setSelectedConfig] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    loadConfigs();
  }, []);

  const loadConfigs = async () => {
    setIsLoading(true);
    try {
      const allConfigs = await invoke<ConfigInfo[]>('load_all_configs');
      setConfigs(allConfigs);
    } catch (err) {
      console.error('Failed to load configs:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleImport = async () => {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const file = await open({
      filters: [
        {
          name: 'Mihomo 配置',
          extensions: ['yaml', 'yml'],
        },
      ],
    });

    if (file) {
      await invoke('import_config', { sourcePath: file });
      await loadConfigs();
    }
  };

  const handleValidate = async (configPath: string) => {
    try {
      const result = await invoke<{
        warnings: string[];
        needsPrivilege: boolean;
      }>('validate_config', { configPath });

      setConfigs((prev) =>
        prev.map((c) =>
          c.path === configPath
            ? { ...c, warnings: result.warnings, needsPrivilege: result.needsPrivilege }
            : c
        )
      );
    } catch (err) {
      console.error('Validation failed:', err);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>配置文件</CardTitle>
        <CardDescription>
          选择或导入 Mihomo 配置文件
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {configs.map((config) => (
            <div
              key={config.path}
              className={`p-4 border rounded-lg cursor-pointer transition-colors ${
                selectedConfig === config.path
                  ? 'border-primary bg-primary/5'
                  : 'hover:border-primary/50'
              }`}
              onClick={() => {
                setSelectedConfig(config.path);
                handleValidate(config.path);
              }}
            >
              <div className="flex items-start justify-between">
                <div className="flex items-start gap-3">
                  <FileText className="h-5 w-5 text-muted-foreground mt-0.5" />
                  <div>
                    <p className="font-medium">{config.name}</p>
                    <div className="flex items-center gap-2 mt-1">
                      <span className="text-sm text-muted-foreground">
                        {(config.size / 1024).toFixed(2)} KB
                      </span>
                      <span className="text-sm text-muted-foreground">•</span>
                      <span className="text-sm text-muted-foreground">
                        {config.modified}
                      </span>
                      {config.needsPrivilege && (
                        <Badge variant="destructive">需管理员权限</Badge>
                      )}
                    </div>

                    {config.warnings.length > 0 && (
                      <div className="mt-2 space-y-1">
                        {config.warnings.map((warning, i) => (
                          <div key={i} className="flex items-center gap-2 text-sm text-amber-600">
                            <AlertCircle className="h-3 w-3" />
                            <span>{warning}</span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>

                <Button
                  variant="outline"
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    // TODO: 打开配置编辑器
                  }}
                >
                  编辑
                </Button>
              </div>
            </div>
          ))}

          <Button variant="outline" className="w-full" onClick={handleImport}>
            <Upload className="h-4 w-4 mr-2" />
            导入配置文件
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
```

### 5.5 WebUI 控制组件

```typescript
'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ExternalLink, Globe } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

export function WebUIControl() {
  const [isAvailable, setIsAvailable] = useState(false);
  const [baseUrl, setBaseUrl] = useState('http://127.0.0.1:9090');
  const [isChecking, setIsChecking] = useState(false);

  const checkWebUI = async () => {
    setIsChecking(true);
    try {
      const available = await invoke<boolean>('check_webui', {
        baseUrl,
        secret: null,
      });
      setIsAvailable(available);
    } catch (err) {
      console.error('Failed to check WebUI:', err);
      setIsAvailable(false);
    } finally {
      setIsChecking(false);
    }
  };

  const openWebUI = async () => {
    try {
      await invoke('open_webui', {
        baseUrl,
        secret: null,
      });
    } catch (err) {
      console.error('Failed to open WebUI:', err);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>WebUI 控制</CardTitle>
        <CardDescription>
          访问 Mihomo Web 管理界面
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center gap-4">
          <div className="flex-1">
            <label className="text-sm font-medium block mb-2">
              WebUI 地址
            </label>
            <input
              type="text"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              className="w-full px-3 py-2 border rounded-md"
              placeholder="http://127.0.0.1:9090"
            />
          </div>

          <Button variant="outline" onClick={checkWebUI} disabled={isChecking}>
            {isChecking ? '检测中...' : '检测'}
          </Button>
        </div>

        {isAvailable ? (
          <div className="flex items-center gap-2 text-green-600">
            <Globe className="h-4 w-4" />
            <span className="text-sm">WebUI 可用</span>
            <Badge variant="secondary">在线</Badge>
          </div>
        ) : (
          <div className="text-sm text-muted-foreground">
            请确保 Mihomo 已启动且配置了 external-controller
          </div>
        )}

        <Button className="w-full" onClick={openWebUI} disabled={!isAvailable}>
          <ExternalLink className="h-4 w-4 mr-2" />
          在浏览器中打开 WebUI
        </Button>
      </CardContent>
    </Card>
  );
}
```

### 5.6 路由与导航
- 根路由 `/` 已重定向到 `/core`，以便桌面端默认展示核心页面。

### 5.7 安装进度与错误提示
- 版本切换器组件监听 `version_install_progress` 事件，展示阶段与进度条。
- 阶段示例：开始下载(10%) → 完成(100%)；错误时展示详细信息。
- 后续可进一步细化为按字节下载进度（流式下载与解压阶段进度）。

### 5.8 核心路径策略
- 移除“手动设置内核路径”入口，统一采用固定路径：`…/mihomo-gui/cores/current/mihomo(.exe)`。
- “下载并安装”后自动更新为当前路径；启动时自动探测并使用该路径。
- UI 仅提供“刷新默认路径”用于重新读取并显示该路径（无需手动输入）。

## 6. Tauri 权限配置

### 6.1 capabilities/desktop.json

```json
{
  "identifier": "desktop",
  "description": "Desktop permissions",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:tray:default",
    "log:default"
  ]
}
```

## 7. 实施计划

### 第一阶段：项目初始化 (1周)
- [x] 配置 Tauri 后端基础结构
- [x] 创建 Rust 模块结构 (core, config, commands, version)（`api/system` 暂未启用）
- [x] 实现基础的 CoreManager 和 ConfigManager
- [x] 配置前端基础页面和路由（新增 `/core`、`/config`）
- [x] 建立前后端通信桥梁（Tauri `invoke`）

### 第二阶段：核心功能 (1-2周)
- [ ] 实现内核版本管理 (VersionManager)
  - 已完成：基于 `MetaCubeX/mihomo` 读取 `version.txt` 的最新版本查询（稳定版/开发版）
  - 已完成：下载、校验（sha256，存在校验文件则验证）、解压与安装，自动更新 `core_path`
- [x] 开发进程生命周期管理（启动/停止/重启、状态查询）
- [x] 实现配置文件导入/导出/验证（轻量 YAML 校验）
- [ ] 创建核心控制 UI 组件（目前提供最小启动/停止界面）
- [x] 实现版本切换器组件（渠道切换/查询最新、设置内核路径、下载并安装）
  - 新增：实时安装进度条与错误提示，固定安装目录展示。

### 第三阶段：系统集成 (1周)
- [ ] 实现权限检测与提升逻辑
- [ ] 开发配置选择器 UI
- [ ] 实现 WebUI 检测与启动
- [ ] 添加状态栏和托盘功能

### 第四阶段：用户体验优化 (1周)
- [ ] 实现实时状态监控
- [ ] 添加错误处理和用户提示
- [ ] 优化 UI/UX 体验
- [ ] 完善日志查看功能

### 第五阶段：测试与打包 (1周)
- [ ] 跨平台兼容性测试
- [ ] 构建和打包配置
- [ ] 创建安装程序
- [ ] 编写用户文档

## 8. 关键注意事项

### 8.1 权限处理
- **Windows**: 使用 ShellExecute 启动管理员进程
- **macOS**: 使用 AppleScript 请求管理员权限
- **Linux**: 使用 pkexec 或 PolicyKit
- **用户提示**: 清晰说明为什么需要权限

### 8.2 配置文件存储
- **默认位置**: `~/.config/mihomo-gui/configs/`
- **格式支持**: YAML (.yaml, .yml)
- **备份机制**: 自动备份重要配置
- **语法验证**: 实时检查配置文件语法

### 8.3 内核管理
- **版本存储**: `数据目录/mihomo-gui/cores/`（跨平台规范路径）
  - Linux: `~/.local/share/mihomo-gui/cores/`
  - macOS: `~/Library/Application Support/mihomo-gui/cores/`
  - Windows: `%APPDATA%\mihomo-gui\cores\`
- **自动下载**: 从官方 GitHub Releases 获取
- **校验和验证**: SHA256 校验下载文件
- **当前版本路径**: `cores/current/mihomo(.exe)` 固定为当前内核；安装完成后拷贝至此路径；应用启动时自动采用该路径
- **回滚机制**: 支持版本回退（后续：切换 `current` 指向或拷贝）

### 8.4 错误处理
- **进程崩溃**: 自动重启机制
- **网络错误**: 重试和降级策略
- **权限不足**: 友好的错误提示
- **配置错误**: 详细的位置和修复建议

## 9. 技术亮点

1. **跨平台兼容性**: 一套代码支持 Windows/macOS/Linux
2. **性能优化**: Rust 后端保证高性能和低资源占用
3. **用户体验**: 直观的 Web UI，无需学习成本
4. **安全性**: 严格的权限控制和文件校验
5. **可扩展性**: 模块化设计，易于添加新功能

---

本计划提供了完整的实施路径和详细的技术方案。根据实际开发进度，可适当调整阶段划分和优先级。
