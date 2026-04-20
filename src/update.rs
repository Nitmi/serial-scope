use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;

use anyhow::{anyhow, Context, Result};
use crossbeam_channel::Sender;
use self_update::Download;
use semver::Version;
use serde::Deserialize;

#[cfg(any(target_os = "linux", target_os = "macos"))]
const APP_NAME: &str = "serial-scope";
#[cfg(target_os = "macos")]
const APP_DISPLAY_NAME: &str = "Serial Scope";
const LATEST_JSON_URLS: &[&str] = &[
    "https://gh.123778.xyz/serial-scope/releases/latest/download/latest.json",
    "https://github.com/Nitmi/serial-scope/releases/latest/download/latest.json",
    "https://gh-proxy.org/https://github.com/Nitmi/serial-scope/releases/latest/download/latest.json",
    "https://hk.gh-proxy.org/https://github.com/Nitmi/serial-scope/releases/latest/download/latest.json",
    "https://edgeone.gh-proxy.org/https://github.com/Nitmi/serial-scope/releases/latest/download/latest.json",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateState {
    Idle,
    Checking,
    Available {
        version: String,
        notes: Option<String>,
    },
    UpToDate,
    Downloading {
        version: String,
    },
    ReadyToRestart {
        version: String,
    },
    Error(String),
}

#[derive(Debug, Clone)]
pub enum UpdateEvent {
    CheckCompleted(Result<UpdateCheckResult, String>),
    InstallCompleted(Result<String, String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckResult {
    Available {
        version: String,
        notes: Option<String>,
    },
    UpToDate,
}

pub fn spawn_check(sender: Sender<UpdateEvent>) {
    thread::spawn(move || {
        let result = check_for_update().map_err(|err| format!("检查更新失败: {err}"));
        let _ = sender.send(UpdateEvent::CheckCompleted(result));
    });
}

pub fn spawn_install(version: String, sender: Sender<UpdateEvent>) {
    thread::spawn(move || {
        let result = install_update(&version)
            .map(|_| version.clone())
            .map_err(|err| format!("更新失败: {err}"));
        let _ = sender.send(UpdateEvent::InstallCompleted(result));
    });
}

fn check_for_update() -> Result<UpdateCheckResult> {
    let release = latest_stable_release()?;
    let current = current_version()?;
    if release.version <= current {
        return Ok(UpdateCheckResult::UpToDate);
    }

    Ok(UpdateCheckResult::Available {
        version: release.version.to_string(),
        notes: release.body,
    })
}

fn install_update(version: &str) -> Result<()> {
    let release = release_for_version(version)?;
    let tmp_dir = self_update::TempDir::new().context("创建临时目录失败")?;
    let download_path = tmp_dir.path().join(&release.asset.name);
    download_asset_with_fallback(&release.asset.download_urls, &download_path)?;

    let new_exe = extract_executable(&download_path, tmp_dir.path())?;
    self_update::self_replace::self_replace(new_exe).context("替换当前程序失败")?;
    Ok(())
}

fn extract_executable(download_path: &Path, tmp_dir: &Path) -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let _ = tmp_dir;
        return Ok(download_path.to_path_buf());
    }

    #[cfg(target_os = "linux")]
    {
        let extracted_name = PathBuf::from(APP_NAME);
        self_update::Extract::from_source(download_path)
            .archive(self_update::ArchiveKind::Tar(Some(
                self_update::Compression::Gz,
            )))
            .extract_file(tmp_dir, &extracted_name)
            .context("解压 Linux 更新包失败")?;
        return Ok(tmp_dir.join(extracted_name));
    }

    #[cfg(target_os = "macos")]
    {
        let extracted_name =
            PathBuf::from(format!("{APP_DISPLAY_NAME}.app/Contents/MacOS/{APP_NAME}"));
        self_update::Extract::from_source(download_path)
            .archive(self_update::ArchiveKind::Zip)
            .extract_file(tmp_dir, &extracted_name)
            .context("解压 macOS 更新包失败")?;
        return Ok(tmp_dir.join(extracted_name));
    }

    #[allow(unreachable_code)]
    Err(anyhow!("当前平台暂不支持自更新"))
}

fn current_version() -> Result<Version> {
    Version::parse(env!("CARGO_PKG_VERSION")).context("解析当前版本号失败")
}

fn latest_stable_release() -> Result<ResolvedRelease> {
    resolve_release(fetch_latest_manifest()?)
}

fn release_for_version(version: &str) -> Result<ResolvedRelease> {
    let target_version = Version::parse(version).context("解析目标版本号失败")?;
    let release = resolve_release(fetch_latest_manifest()?)?;
    if release.version != target_version {
        return Err(anyhow!("可更新版本已变化，请等待自动重新检查后再更新"));
    }
    Ok(release)
}

fn fetch_latest_manifest() -> Result<LatestManifest> {
    let mut errors = Vec::new();
    for url in LATEST_JSON_URLS {
        match fetch_manifest_from_url(url) {
            Ok(manifest) => return Ok(manifest),
            Err(err) => errors.push(format!("{url}: {err}")),
        }
    }

    Err(anyhow!("所有更新源均不可用：{}", errors.join(" | ")))
}

fn fetch_manifest_from_url(url: &str) -> Result<LatestManifest> {
    let mut body = Vec::new();
    Download::from_url(url)
        .show_progress(false)
        .download_to(&mut body)
        .with_context(|| format!("下载更新清单失败"))?;

    serde_json::from_slice::<LatestManifest>(&body).with_context(|| "解析更新清单失败")
}

fn resolve_release(manifest: LatestManifest) -> Result<ResolvedRelease> {
    let version =
        Version::parse(manifest.version.trim_start_matches('v')).context("解析远端版本号失败")?;
    if !version.pre.is_empty() {
        return Err(anyhow!("更新清单中的最新版本为预发布版本"));
    }

    let asset = manifest
        .assets
        .get(current_target_asset_key())
        .cloned()
        .ok_or_else(|| anyhow!("更新清单中缺少当前平台的安装包"))?;

    if asset.name != target_asset_name() {
        return Err(anyhow!("更新清单中的平台安装包名称不匹配"));
    }

    if asset.download_urls.is_empty() {
        return Err(anyhow!("更新清单中的安装包下载地址为空"));
    }

    Ok(ResolvedRelease {
        version,
        body: manifest.notes.filter(|body| !body.trim().is_empty()),
        asset,
    })
}

fn download_asset_with_fallback(urls: &[String], download_path: &Path) -> Result<()> {
    let mut errors = Vec::new();
    for url in urls {
        match download_asset(url, download_path) {
            Ok(()) => return Ok(()),
            Err(err) => errors.push(format!("{url}: {err}")),
        }
    }

    Err(anyhow!("下载更新包失败：{}", errors.join(" | ")))
}

fn download_asset(url: &str, download_path: &Path) -> Result<()> {
    let mut download_file = File::create(download_path).context("创建临时更新文件失败")?;
    Download::from_url(url)
        .show_progress(false)
        .download_to(&mut download_file)
        .with_context(|| "下载更新包失败")?;
    download_file.flush().ok();
    Ok(())
}

fn target_asset_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "serial-scope-windows-x86_64.exe"
    }
    #[cfg(target_os = "linux")]
    {
        "serial-scope-linux-x86_64.tar.gz"
    }
    #[cfg(target_os = "macos")]
    {
        "serial-scope-macos.app.zip"
    }
}

fn current_target_asset_key() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows-x86_64"
    }
    #[cfg(target_os = "linux")]
    {
        "linux-x86_64"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
}

#[derive(Debug, Clone, Deserialize)]
struct LatestManifest {
    version: String,
    #[serde(default)]
    notes: Option<String>,
    assets: std::collections::BTreeMap<String, ManifestAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestAsset {
    name: String,
    download_urls: Vec<String>,
}

#[derive(Debug, Clone)]
struct ResolvedRelease {
    version: Version,
    body: Option<String>,
    asset: ManifestAsset,
}

#[cfg(test)]
mod tests {
    use super::{current_target_asset_key, resolve_release, target_asset_name, LATEST_JSON_URLS};

    #[test]
    fn target_asset_name_matches_release_packaging() {
        let asset_name = target_asset_name();
        #[cfg(target_os = "windows")]
        assert_eq!(asset_name, "serial-scope-windows-x86_64.exe");
        #[cfg(target_os = "linux")]
        assert_eq!(asset_name, "serial-scope-linux-x86_64.tar.gz");
        #[cfg(target_os = "macos")]
        assert_eq!(asset_name, "serial-scope-macos.app.zip");
    }

    #[test]
    fn latest_json_urls_keep_primary_proxy_first() {
        assert_eq!(
            LATEST_JSON_URLS[0],
            "https://gh.123778.xyz/serial-scope/releases/latest/download/latest.json"
        );
    }

    #[test]
    fn manifest_resolves_current_platform_asset() {
        let manifest = serde_json::from_str::<super::LatestManifest>(&format!(
            r#"{{
                "version": "0.2.0",
                "notes": "test",
                "assets": {{
                    "windows-x86_64": {{
                        "name": "serial-scope-windows-x86_64.exe",
                        "download_urls": ["https://example.com/windows.exe"]
                    }},
                    "linux-x86_64": {{
                        "name": "serial-scope-linux-x86_64.tar.gz",
                        "download_urls": ["https://example.com/linux.tar.gz"]
                    }},
                    "macos": {{
                        "name": "serial-scope-macos.app.zip",
                        "download_urls": ["https://example.com/macos.zip"]
                    }}
                }}
            }}"#
        ))
        .unwrap();

        let release = resolve_release(manifest).unwrap();
        assert_eq!(release.asset.name, target_asset_name());
        assert!(!current_target_asset_key().is_empty());
    }
}
