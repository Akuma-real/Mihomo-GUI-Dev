use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::{env, fs, io, path::{Path, PathBuf}};
use std::io::Cursor;

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
  Stable,
  Dev,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
  pub version: String,
  pub release_date: Option<String>,
  pub download_url: Option<String>,
  pub checksum: Option<String>,
  pub channel: ReleaseChannel,
}

#[derive(Debug, Clone)]
pub struct DownloadPlan {
  pub version: String,
  pub asset_name: String,
  pub asset_url: String,
  pub checksum_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct Asset {
  browser_download_url: String,
  name: String,
}

#[derive(Debug, Deserialize, Clone)]
struct Release {
  published_at: Option<String>,
  prerelease: bool,
  assets: Vec<Asset>,
}

#[derive(Debug)]
pub struct VersionManager {
  pub cores_dir: PathBuf,
  pub current_version: Option<String>,
  pub current_core_path: Option<PathBuf>,
}

impl VersionManager {
  pub fn new() -> io::Result<Self> {
    let dir = install_root();
    fs::create_dir_all(&dir)?;
    Ok(Self { cores_dir: dir, current_version: None, current_core_path: None })
  }

  pub fn latest_stub(channel: ReleaseChannel) -> VersionInfo {
    VersionInfo {
      version: match channel {
        ReleaseChannel::Stable => "v0.0.0".into(),
        ReleaseChannel::Dev => "v0.0.0-dev".into(),
      },
      release_date: None,
      download_url: None,
      checksum: None,
      channel,
    }
  }

  pub fn mark_installed(&mut self, version: &str, core_path: PathBuf) {
    self.current_version = Some(version.to_string());
    self.current_core_path = Some(core_path);
  }

  pub async fn fetch_latest(&self, channel: ReleaseChannel) -> Result<VersionInfo> {
    let (rel, version) = self.get_release_and_version(channel).await?;
    Ok(VersionInfo {
      version,
      release_date: rel.published_at,
      download_url: None,
      checksum: None,
      channel,
    })
  }

  pub async fn download_install_latest(&mut self, channel: ReleaseChannel) -> Result<PathBuf> {
    let (rel, version) = self.get_release_and_version(channel.clone()).await?;

    let client = build_gh_client()?;

    // 选择目标资产
    let asset = select_target_asset(&rel).context("未找到匹配当前平台与架构的资产")?;

    // 可选校验文件
    let checksum_asset = rel
      .assets
      .iter()
      .find(|a| a.name.to_ascii_lowercase().contains("sha256") || a.name.to_ascii_lowercase().contains("checksum"))
      .cloned();

    let bytes = fetch_bytes(&client, &asset.browser_download_url, &asset.name).await?;

    // 如存在校验文件，进行校验
    if let Some(sum) = checksum_asset {
      if let Ok(text) = fetch_text(&client, &sum.browser_download_url, &sum.name).await {
        verify_sha256(&bytes, &text).context("校验和不匹配或解析失败")?;
      }
    }

    // 安装目录：~/.cache/mihomo-gui/cores/<version>
    let install_dir = self.cores_dir.join(&version);
    fs::create_dir_all(&install_dir)?;

    let bin_name = target_bin_name();
    let installed_path = install_asset_bytes(&bytes, &asset.name, &install_dir, &bin_name)?;

    // 设置可执行权限（非 Windows）
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let mut perms = fs::metadata(&installed_path)?.permissions();
      perms.set_mode(0o755);
      fs::set_permissions(&installed_path, perms)?;
    }

    self.mark_installed(&version, installed_path.clone());
    Ok(installed_path)
  }

  async fn get_release_and_version(&self, channel: ReleaseChannel) -> Result<(Release, String)> {
    // 仅使用 MetaCubeX/mihomo
    const REPO: &str = "MetaCubeX/mihomo";
    let client = build_gh_client()?;

    let rel = match channel {
      ReleaseChannel::Stable => fetch_latest_release(&client, REPO).await?,
      ReleaseChannel::Dev => fetch_dev_release(&client, REPO).await?,
    };

    let version_txt_asset = rel
      .assets
      .iter()
      .find(|a| a.name.eq_ignore_ascii_case("version.txt"))
      .cloned()
      .ok_or_else(|| anyhow!("未找到 version.txt 资产"))?;

    let version = fetch_text(&client, &version_txt_asset.browser_download_url, "version.txt")
      .await?
      .trim()
      .to_string();

    Ok((rel, version))
  }

  pub async fn plan_download(&self, channel: ReleaseChannel) -> Result<DownloadPlan> {
    let (rel, version) = self.get_release_and_version(channel).await?;
    let asset = select_target_asset(&rel).context("未找到匹配当前平台与架构的资产")?;
    let checksum_asset = rel
      .assets
      .iter()
      .find(|a| a.name.to_ascii_lowercase().contains("sha256") || a.name.to_ascii_lowercase().contains("checksum"))
      .cloned();
    Ok(DownloadPlan {
      version,
      asset_name: asset.name.clone(),
      asset_url: asset.browser_download_url.clone(),
      checksum_url: checksum_asset.map(|a| a.browser_download_url),
    })
  }

  pub fn install_from_bytes(&mut self, version: &str, asset_name: &str, bytes: &[u8]) -> Result<PathBuf> {
    let install_dir = self.cores_dir.join(version);
    fs::create_dir_all(&install_dir)?;
    let bin_name = target_bin_name();
    let installed_path = install_asset_bytes(bytes, asset_name, &install_dir, bin_name)?;
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let mut perms = fs::metadata(&installed_path)?.permissions();
      perms.set_mode(0o755);
      fs::set_permissions(&installed_path, perms)?;
    }
    // 将 current 指向版本目录，避免复制导致 Linux 上的 setcap 能力丢失
    let current_dir = self.cores_dir.join("current");

    #[cfg(unix)]
    {
      use std::os::unix::fs::symlink as symlink_dir;
      // 删除现有的 current（可能是符号链接或目录）
      match fs::symlink_metadata(&current_dir) {
        Ok(meta) => {
          if meta.file_type().is_symlink() {
            let _ = fs::remove_file(&current_dir);
          } else if meta.is_dir() {
            let _ = fs::remove_dir_all(&current_dir);
          } else {
            let _ = fs::remove_file(&current_dir);
          }
        }
        Err(_) => {}
      }
      symlink_dir(&install_dir, &current_dir)?;
    }

    #[cfg(windows)]
    {
      // Windows 上创建目录符号链接需要管理员或开发者模式，失败则回退到复制
      #[allow(unused_imports)]
      use std::os::windows::fs::symlink_dir as win_symlink_dir;
      let mut linked = false;
      #[cfg(windows)]
      {
        if win_symlink_dir(&install_dir, &current_dir).is_ok() {
          linked = true;
        }
      }
      if !linked {
        fs::create_dir_all(&current_dir)?;
        let current_path = current_dir.join(bin_name);
        fs::copy(&installed_path, &current_path)?;
      }
    }

    let current_path = current_dir.join(bin_name);
    self.mark_installed(version, current_path.clone());
    Ok(current_path)
  }
}

pub(crate) fn build_gh_client() -> Result<reqwest::Client> {
  let mut headers = reqwest::header::HeaderMap::new();
  headers.insert(
    reqwest::header::ACCEPT,
    reqwest::header::HeaderValue::from_static("application/vnd.github+json"),
  );
  headers.insert(
    reqwest::header::USER_AGENT,
    reqwest::header::HeaderValue::from_static("mihomo-gui/0.1"),
  );

  if let Ok(token) = env::var("GITHUB_TOKEN") {
    let val = format!("Bearer {}", token);
    if let Ok(hv) = reqwest::header::HeaderValue::from_str(&val) {
      headers.insert(reqwest::header::AUTHORIZATION, hv);
    }
  }

  let client = reqwest::Client::builder()
    .default_headers(headers)
    .build()?;
  Ok(client)
}

async fn fetch_latest_release(client: &reqwest::Client, repo: &str) -> Result<Release> {
  let url = format!("https://api.github.com/repos/{repo}/releases/latest");
  let resp = client.get(url).send().await.context("请求 latest release 失败")?;
  let status = resp.status();
  if !status.is_success() {
    let body = resp.text().await.unwrap_or_default();
    return Err(anyhow!("latest release HTTP 状态错误: {} - {}", status, truncate(&body)));
  }
  Ok(resp.json::<Release>().await.context("解析 latest release JSON 失败")?)
}

async fn fetch_dev_release(client: &reqwest::Client, repo: &str) -> Result<Release> {
  let url = format!("https://api.github.com/repos/{repo}/releases?per_page=10");
  let resp = client.get(url).send().await.context("请求 releases 列表失败")?;
  let status = resp.status();
  if !status.is_success() {
    let body = resp.text().await.unwrap_or_default();
    return Err(anyhow!("releases 列表 HTTP 状态错误: {} - {}", status, truncate(&body)));
  }
  let list = resp
    .json::<Vec<Release>>()
    .await
    .context("解析 releases 列表 JSON 失败")?;
  if list.is_empty() {
    return Err(anyhow!("未找到任何 release"));
  }
  if let Some(pre) = list.iter().find(|r| r.prerelease) {
    Ok(pre.clone())
  } else {
    Ok(list.into_iter().next().unwrap())
  }
}

pub(crate) async fn fetch_text(client: &reqwest::Client, url: &str, what: &str) -> Result<String> {
  let resp = client.get(url).send().await.with_context(|| format!("下载 {what} 失败"))?;
  let status = resp.status();
  if !status.is_success() {
    let body = resp.text().await.unwrap_or_default();
    return Err(anyhow!("{what} HTTP 状态错误: {} - {}", status, truncate(&body)));
  }
  Ok(resp.text().await.with_context(|| format!("读取 {what} 文本失败"))?)
}

fn truncate(s: &str) -> String {
  const MAX: usize = 200;
  if s.len() <= MAX { s.to_string() } else { format!("{}...", &s[..MAX]) }
}

fn target_os_keyword() -> &'static str {
  #[cfg(target_os = "windows")] { return "windows"; }
  #[cfg(target_os = "macos")] { return "darwin"; }
  #[cfg(target_os = "linux")] { return "linux"; }
  #[allow(unreachable_code)]
  "unknown"
}

fn target_arch_keywords() -> Vec<&'static str> {
  match std::env::consts::ARCH {
    "x86_64" => vec!["amd64", "x86_64", "x64"],
    "aarch64" => vec!["arm64", "aarch64"],
    other if other.contains("arm") => vec!["arm"],
    other => vec![other],
  }
}

fn target_bin_name() -> &'static str {
  #[cfg(target_os = "windows")] { return "mihomo.exe"; }
  #[cfg(not(target_os = "windows"))] { return "mihomo"; }
}

fn select_target_asset(rel: &Release) -> Option<Asset> {
  let os = target_os_keyword();
  let arch_keys = target_arch_keywords();

  // 允许的二进制归档后缀（过滤掉打包格式：deb/rpm/pkg.tar.zst）
  fn ext_rank(name: &str) -> i32 {
    let n = name.to_ascii_lowercase();
    if n.ends_with(".deb") || n.ends_with(".rpm") || n.ends_with(".pkg.tar.zst") { return 99; }
    #[cfg(target_os = "windows")]
    {
      if n.ends_with(".zip") { return 0; }
      if n.ends_with(".gz") { return 2; }
      if n.ends_with(".tar.gz") || n.ends_with(".tgz") { return 3; }
      return 50;
    }
    #[cfg(not(target_os = "windows"))]
    {
      if n.ends_with(".gz") { return 0; }
      if n.ends_with(".tar.gz") || n.ends_with(".tgz") { return 1; }
      if n.ends_with(".zip") { return 3; }
      return 50;
    }
  }

  // 期望的 x86-64 变体优先级（自动检测 CPU 能力，提升 v2/v3 优先级；失败则走保守顺序）
  fn desired_variants() -> Vec<&'static str> {
    #[cfg(all(target_arch = "x86_64"))]
    {
      // Rust 标准库特性探测
      let v3 = std::arch::is_x86_feature_detected!("avx2")
        && std::arch::is_x86_feature_detected!("bmi2")
        && std::arch::is_x86_feature_detected!("fma");
      let v2 = std::arch::is_x86_feature_detected!("sse4.2");
      if v3 {
        return vec!["-v3", "-v2", "-v1", "compatible"];
      } else if v2 {
        return vec!["-v2", "-v1", "compatible", "-v3"];
      } else {
        return vec!["-v1", "compatible", "-v2", "-v3"];
      }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
      return vec!["", "compatible"]; // 其他架构通常没有 v1/v2/v3
    }
  }

  let variants = desired_variants();

  // 评分函数：越小越优
  fn score(name: &str, variants: &[&str]) -> i32 {
    let n = name.to_ascii_lowercase();
    let mut s = 0i32;
    // 1) 变体优先级
    let mut vscore = 10;
    for (i, tag) in variants.iter().enumerate() {
      if !tag.is_empty() && n.contains(tag) { vscore = i as i32; break; }
      if tag.is_empty() { vscore = i as i32; }
    }
    s += vscore * 10;
    // 2) 避免 go 特定版本
    if n.contains("-go") { s += 5; }
    // 3) 扩展名优先
    s += ext_rank(&n);
    s
  }

  rel
    .assets
    .iter()
    .filter(|a| {
      let name = a.name.to_ascii_lowercase();
      name.contains(os) && arch_keys.iter().any(|k| name.contains(k))
    })
    .min_by_key(|a| score(&a.name, &variants))
    .cloned()
}

async fn fetch_bytes(client: &reqwest::Client, url: &str, what: &str) -> Result<Vec<u8>> {
  let resp = client.get(url).send().await.with_context(|| format!("下载 {what} 失败"))?;
  let status = resp.status();
  if !status.is_success() {
    let body = resp.text().await.unwrap_or_default();
    return Err(anyhow!("{what} HTTP 状态错误: {} - {}", status, truncate(&body)));
  }
  Ok(resp.bytes().await.with_context(|| format!("读取 {what} 二进制失败"))?.to_vec())
}

pub(crate) fn verify_sha256(bytes: &[u8], checksum_list: &str) -> Result<()> {
  use sha2::{Digest, Sha256};
  let mut hasher = Sha256::new();
  hasher.update(bytes);
  let out = hasher.finalize();
  let hex = out.iter().map(|b| format!("{:02x}", b)).collect::<String>();

  // 常见格式："<sha256>  <filename>" 每行一条
  for line in checksum_list.lines() {
    let t = line.trim();
    if t.is_empty() { continue; }
    let parts: Vec<&str> = t.split_whitespace().collect();
    if parts.get(0).map(|s| s.to_ascii_lowercase()) == Some(hex.clone()) { return Ok(()); }
    if let Some(first) = parts.first() {
      if first.to_ascii_lowercase() == hex { return Ok(()); }
    }
  }
  Err(anyhow!("SHA256 未在校验列表中匹配"))
}

fn install_asset_bytes(bytes: &[u8], asset_name: &str, install_dir: &Path, bin_name: &str) -> Result<PathBuf> {
  let lname = asset_name.to_ascii_lowercase();
  if lname.ends_with(".zip") {
    extract_zip_and_find(bytes, install_dir, bin_name)
  } else if lname.ends_with(".tar.gz") || lname.ends_with(".tgz") {
    extract_targz_and_find(bytes, install_dir, bin_name)
  } else if lname.ends_with(".gz") {
    // 可能是单文件 gzip
    let path = install_dir.join(bin_name);
    extract_gz_to(bytes, &path)?;
    Ok(path)
  } else {
    // 认为是未压缩的可执行文件
    let path = install_dir.join(bin_name);
    fs::write(&path, bytes)?;
    Ok(path)
  }
}

fn extract_zip_and_find(bytes: &[u8], install_dir: &Path, bin_name: &str) -> Result<PathBuf> {
  let reader = Cursor::new(bytes);
  let mut zip = zip::ZipArchive::new(reader).context("解析 zip 失败")?;
  let mut found: Option<PathBuf> = None;
  for i in 0..zip.len() {
    let mut file = zip.by_index(i).context("读取 zip 条目失败")?;
    let outpath = install_dir.join(file.name());
    if file.name().ends_with('/') {
      fs::create_dir_all(&outpath)?;
      continue;
    }
    if let Some(parent) = outpath.parent() { fs::create_dir_all(parent)?; }
    let mut out = fs::File::create(&outpath)?;
    std::io::copy(&mut file, &mut out)?;
    if outpath.file_name().map(|n| n == bin_name).unwrap_or(false) {
      found = Some(outpath.clone());
    }
  }
  found.or_else(|| find_bin_recursive(install_dir, bin_name)).ok_or_else(|| anyhow!("未在 zip 中找到可执行文件"))
}

fn extract_targz_and_find(bytes: &[u8], install_dir: &Path, bin_name: &str) -> Result<PathBuf> {
  let gz = flate2::read::GzDecoder::new(Cursor::new(bytes));
  let mut tar = tar::Archive::new(gz);
  tar.unpack(install_dir).context("解包 tar.gz 失败")?;
  find_bin_recursive(install_dir, bin_name).ok_or_else(|| anyhow!("未在 tar.gz 中找到可执行文件"))
}

fn extract_gz_to(bytes: &[u8], path: &Path) -> Result<()> {
  let mut gz = flate2::read::GzDecoder::new(Cursor::new(bytes));
  let mut out = fs::File::create(path)?;
  std::io::copy(&mut gz, &mut out)?;
  Ok(())
}

fn find_bin_recursive(root: &Path, bin_name: &str) -> Option<PathBuf> {
  fn visit(dir: &Path, target: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
      let p = e.path();
      if p.is_dir() {
        if let Some(found) = visit(&p, target) { return Some(found); }
      } else if p.file_name().map(|n| n == target).unwrap_or(false) {
        return Some(p.clone());
      }
    }
    None
  }
  visit(root, bin_name)
}

fn install_root() -> PathBuf {
  let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
  base.join("mihomo-gui").join("cores")
}
