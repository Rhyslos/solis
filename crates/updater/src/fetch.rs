//! Talking to GitHub: asking for the latest release and downloading its
//! files, over HTTPS checked by the system's own certificates.

use crate::release::{self, Asset, Release};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long a request may take to start answering.
const CONNECT_LIMIT: Duration = Duration::from_secs(20);
/// How long a whole download may take.
const DOWNLOAD_LIMIT: Duration = Duration::from_secs(30 * 60);

/// Where to ask, and who is asking.
#[derive(Debug, Clone)]
pub struct Source {
    /// The repository, as `owner/name`.
    pub repo: String,
    /// The installer's file name in each release.
    pub installer_name: String,
    /// Sent as the User-Agent, as GitHub asks.
    pub user_agent: String,
}

fn agent(total: Duration) -> ureq::Agent {
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
    let tls = TlsConfig::builder()
        .provider(TlsProvider::Rustls)
        .root_certs(RootCerts::PlatformVerifier)
        .build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .https_only(true)
        .timeout_connect(Some(CONNECT_LIMIT))
        .timeout_global(Some(total))
        .max_redirects(5)
        .build()
        .new_agent()
}

/// Asks GitHub for `source`'s latest release.
pub(crate) fn latest(source: &Source) -> Result<Release, String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        source.repo
    );
    let mut answer = agent(CONNECT_LIMIT * 3)
        .get(&url)
        .header("User-Agent", &source.user_agent)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| plain_error(&e))?;
    let text = answer
        .body_mut()
        .with_config()
        .limit(release::MAX_RELEASE_TEXT)
        .read_to_string()
        .map_err(|e| plain_error(&e))?;
    release::read_release(&text, &source.repo, &source.installer_name)
}

/// Downloads `release`'s installer into `folder`, checks it against its
/// published SHA-256, and gives its path. Anything already in `folder` is
/// removed first; a download that doesn't match is removed too.
pub(crate) fn download(
    source: &Source,
    release: &Release,
    folder: &Path,
) -> Result<PathBuf, String> {
    let checksum_text = fetch_text(source, &release.checksum)?;
    let expected = release::read_checksum(&checksum_text, &release.installer.name)?;

    clear_folder(folder)?;
    std::fs::create_dir_all(folder)
        .map_err(|e| format!("couldn't make {}: {e}", folder.display()))?;
    let partial = folder.join(format!("{}.part", release.installer.name));
    let digest = fetch_file(source, &release.installer, &partial)?;
    if digest != expected {
        let _ = std::fs::remove_file(&partial);
        return Err(format!(
            "build {}'s download didn't match its checksum, so it was thrown away",
            release.build
        ));
    }
    let path = folder.join(&release.installer.name);
    std::fs::rename(&partial, &path).map_err(|e| format!("couldn't keep the download: {e}"))?;
    Ok(path)
}

fn fetch_text(source: &Source, asset: &Asset) -> Result<String, String> {
    let mut answer = agent(CONNECT_LIMIT * 3)
        .get(&asset.url)
        .header("User-Agent", &source.user_agent)
        .call()
        .map_err(|e| plain_error(&e))?;
    answer
        .body_mut()
        .with_config()
        .limit(release::MAX_CHECKSUM_TEXT)
        .read_to_string()
        .map_err(|e| plain_error(&e))
}

/// Streams `asset` into `path`, at most its listed size (and never past
/// the installer limit), and gives the SHA-256 of what arrived.
fn fetch_file(source: &Source, asset: &Asset, path: &Path) -> Result<[u8; 32], String> {
    use sha2::Digest;
    let mut answer = agent(DOWNLOAD_LIMIT)
        .get(&asset.url)
        .header("User-Agent", &source.user_agent)
        .call()
        .map_err(|e| plain_error(&e))?;
    let limit = asset.size.min(release::MAX_INSTALLER_SIZE);
    let mut reader = answer.body_mut().with_config().limit(limit).reader();
    let mut file =
        std::fs::File::create(path).map_err(|e| format!("couldn't save the download: {e}"))?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| format!("the download stopped: {e}"))?;
        if read == 0 {
            break;
        }
        let chunk = buffer.get(..read).unwrap_or_default();
        hasher.update(chunk);
        file.write_all(chunk)
            .map_err(|e| format!("couldn't save the download: {e}"))?;
        total += read as u64;
    }
    file.sync_all()
        .map_err(|e| format!("couldn't save the download: {e}"))?;
    if total != asset.size {
        let _ = std::fs::remove_file(path);
        return Err("the download was cut short".into());
    }
    Ok(hasher.finalize().into())
}

/// Removes the files directly inside `folder` (earlier downloads).
pub fn clear_folder(folder: &Path) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("couldn't remove {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

/// A network error in plain words.
fn plain_error(error: &ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(404) => "GitHub has no builds to update to yet".into(),
        ureq::Error::StatusCode(403 | 429) => "GitHub asked to wait before checking again".into(),
        ureq::Error::StatusCode(code) => format!("GitHub answered with error {code}"),
        ureq::Error::Timeout(_) => "GitHub took too long to answer".into(),
        ureq::Error::BodyExceedsLimit(_) => "GitHub sent more than expected".into(),
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => {
            "couldn't reach GitHub (is this computer online?)".into()
        }
        other => format!("couldn't reach GitHub: {other}"),
    }
}
