use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::{env, fs, io, path::PathBuf};

#[derive(Debug, Clone, Serialize)]
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
    let base = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("mihomo-gui").join("cores");
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
    // 仅使用 MetaCubeX/mihomo
    const REPO: &str = "MetaCubeX/mihomo";

    let client = build_gh_client()?;

    // 1) 获取目标 release：Stable => /latest；Dev => releases 列表中首个 prerelease
    let rel = match channel {
      ReleaseChannel::Stable => fetch_latest_release(&client, REPO).await?,
      ReleaseChannel::Dev => fetch_dev_release(&client, REPO).await?,
    };

    // 2) 在 release 资产中查找 version.txt 并读取其文本，作为最终版本号
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

    Ok(VersionInfo {
      version,
      release_date: rel.published_at,
      download_url: rel.assets.get(0).map(|a| a.browser_download_url.clone()),
      checksum: None,
      channel,
    })
  }
}

fn build_gh_client() -> Result<reqwest::Client> {
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

async fn fetch_text(client: &reqwest::Client, url: &str, what: &str) -> Result<String> {
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
