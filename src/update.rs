use std::fs::File;
use std::path::{Path, PathBuf};
use std::thread;

use anyhow::{anyhow, Context, Result};
use crossbeam_channel::Sender;
use semver::Version;
use self_update::backends::github::ReleaseList;
use self_update::update::Release;
use self_update::Download;

const REPO_OWNER: &str = "Nitmi";
const REPO_NAME: &str = "serial-scope";
#[cfg(any(target_os = "linux", target_os = "macos"))]
const APP_NAME: &str = "serial-scope";
#[cfg(target_os = "macos")]
const APP_DISPLAY_NAME: &str = "Serial Scope";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateState {
    Idle,
    Checking,
    Available { version: String, notes: Option<String> },
    UpToDate,
    Downloading { version: String },
    ReadyToRestart { version: String },
    Error(String),
}

#[derive(Debug, Clone)]
pub enum UpdateEvent {
    CheckCompleted(Result<UpdateCheckResult, String>),
    InstallCompleted(Result<String, String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckResult {
    Available { version: String, notes: Option<String> },
    UpToDate,
}

pub fn spawn_check(sender: Sender<UpdateEvent>) {
    thread::spawn(move || {
        let result = check_for_update()
            .map_err(|err| format!("检查更新失败: {err}"));
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
    let asset = release
        .release
        .assets
        .iter()
        .find(|asset| asset.name == target_asset_name())
        .cloned()
        .ok_or_else(|| anyhow!("未找到当前平台对应的更新包"))?;

    let tmp_dir = self_update::TempDir::new().context("创建临时目录失败")?;
    let download_path = tmp_dir.path().join(&asset.name);
    let mut download_file = File::create(&download_path).context("创建临时更新文件失败")?;

    Download::from_url(&asset.download_url)
        .show_progress(false)
        .download_to(&mut download_file)
        .context("下载更新包失败")?;

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
        let extracted_name = PathBuf::from(format!("{APP_DISPLAY_NAME}.app/Contents/MacOS/{APP_NAME}"));
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
    let mut releases = fetch_releases()?;
    releases.sort_by(|left, right| right.version.cmp(&left.version));
    releases
        .into_iter()
        .find(|release| {
            release
                .release
                .assets
                .iter()
                .any(|asset| asset.name == target_asset_name())
        })
        .ok_or_else(|| anyhow!("没有找到适用于当前平台的稳定版本"))
}

fn release_for_version(version: &str) -> Result<ResolvedRelease> {
    let target_version = Version::parse(version).context("解析目标版本号失败")?;
    fetch_releases()?
        .into_iter()
        .find(|release| {
            release.version == target_version
                && release
                    .release
                    .assets
                    .iter()
                    .any(|asset| asset.name == target_asset_name())
        })
        .ok_or_else(|| anyhow!("没有找到版本 {version} 的当前平台更新包"))
}

fn fetch_releases() -> Result<Vec<ResolvedRelease>> {
    let releases = ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .context("初始化 GitHub Release 检查器失败")?
        .fetch()
        .context("获取 GitHub Release 列表失败")?;

    let mut resolved = Vec::new();
    for release in releases {
        let Some(version) = parse_release_version(&release) else {
            continue;
        };
        if !version.pre.is_empty() {
            continue;
        }
        resolved.push(ResolvedRelease {
            version,
            body: release.body.clone().filter(|body| !body.trim().is_empty()),
            release,
        });
    }
    Ok(resolved)
}

fn parse_release_version(release: &Release) -> Option<Version> {
    let raw = release.version.trim_start_matches('v');
    Version::parse(raw).ok()
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

#[derive(Debug, Clone)]
struct ResolvedRelease {
    version: Version,
    body: Option<String>,
    release: Release,
}

#[cfg(test)]
mod tests {
    use super::target_asset_name;

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
}
