//! What GitHub says about the newest build, read with limits, and the
//! checksum file published beside it.

use serde::Deserialize;

/// The most text read from GitHub's answer about a release.
pub const MAX_RELEASE_TEXT: u64 = 1024 * 1024;
/// The most text read from a checksum file.
pub const MAX_CHECKSUM_TEXT: u64 = 4 * 1024;
/// The most files a release may list.
pub const MAX_ASSETS: usize = 64;
/// The biggest installer downloaded.
pub const MAX_INSTALLER_SIZE: u64 = 512 * 1024 * 1024;

/// The tag each build's release has: `build-` and its number.
pub const TAG_PREFIX: &str = "build-";

/// One build published on GitHub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// Its build number (from the tag `build-57`).
    pub build: u64,
    /// The installer.
    pub installer: Asset,
    /// The text file holding the installer's SHA-256.
    pub checksum: Asset,
}

/// One file of a release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    /// Its file name.
    pub name: String,
    /// Where to download it.
    pub url: String,
    /// Its size in bytes, as GitHub gives it.
    pub size: u64,
}

#[derive(Deserialize)]
struct RawRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<RawAsset>,
}

#[derive(Deserialize)]
struct RawAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

/// The build number in a release's tag (`build-57` gives 57).
pub fn build_of_tag(tag: &str) -> Option<u64> {
    let digits = tag.strip_prefix(TAG_PREFIX)?;
    if digits.is_empty() || digits.len() > 18 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Reads GitHub's answer about the latest release of `repo` (`owner/name`),
/// and finds the installer called `installer_name` and its checksum file
/// (the same name with `.sha256` added).
///
/// Refuses drafts, pre-releases, too many files, and download addresses
/// outside the repository's own releases on github.com.
pub fn read_release(text: &str, repo: &str, installer_name: &str) -> Result<Release, String> {
    if text.len() as u64 > MAX_RELEASE_TEXT {
        return Err("GitHub's answer was too long".into());
    }
    let raw: RawRelease =
        serde_json::from_str(text).map_err(|e| format!("GitHub's answer wasn't readable: {e}"))?;
    if raw.draft || raw.prerelease {
        return Err("the newest release isn't finished".into());
    }
    let build = build_of_tag(&raw.tag_name).ok_or_else(|| {
        format!(
            "the newest release ({}) isn't a build",
            short(&raw.tag_name)
        )
    })?;
    if raw.assets.len() > MAX_ASSETS {
        return Err("the newest release lists too many files".into());
    }
    let checksum_name = format!("{installer_name}.sha256");
    let find = |name: &str| -> Result<Asset, String> {
        let raw = raw
            .assets
            .iter()
            .find(|a| a.name == name)
            .ok_or_else(|| format!("build {build} has no {name}"))?;
        if !is_release_download(&raw.browser_download_url, repo) {
            return Err(format!("build {build}'s {name} isn't on {repo}'s releases"));
        }
        Ok(Asset {
            name: raw.name.clone(),
            url: raw.browser_download_url.clone(),
            size: raw.size,
        })
    };
    let installer = find(installer_name)?;
    if installer.size > MAX_INSTALLER_SIZE {
        return Err(format!("build {build}'s installer is too big"));
    }
    let checksum = find(&checksum_name)?;
    Ok(Release {
        build,
        installer,
        checksum,
    })
}

/// Whether `url` is a file of one of `repo`'s releases on github.com.
pub fn is_release_download(url: &str, repo: &str) -> bool {
    let prefix = format!("https://github.com/{repo}/releases/download/");
    match url.strip_prefix(&prefix) {
        // A tag, a slash, a file name; nothing that climbs out.
        Some(rest) => {
            !rest.is_empty()
                && !rest.contains("..")
                && !rest.contains(['?', '#', '\\', '@'])
                && rest.split('/').count() == 2
                && rest.split('/').all(|part| !part.is_empty())
        }
        None => false,
    }
}

/// Reads a checksum file: 64 hex digits, then optionally spaces and the
/// file's name (as `sha256sum` writes it). When a name is given, it must
/// be `file_name`.
pub fn read_checksum(text: &str, file_name: &str) -> Result<[u8; 32], String> {
    if text.len() as u64 > MAX_CHECKSUM_TEXT {
        return Err("the checksum file was too long".into());
    }
    let line = text
        .trim_start_matches('\u{feff}')
        .lines()
        .next()
        .unwrap_or("")
        .trim();
    let (hex, rest) = line.split_at(line.find(char::is_whitespace).unwrap_or(line.len()));
    let named = rest.trim().trim_start_matches('*');
    if !named.is_empty() && named != file_name {
        return Err(format!(
            "the checksum file is for {}, not {file_name}",
            short(named)
        ));
    }
    parse_hex(hex).ok_or_else(|| "the checksum file doesn't hold a SHA-256".into())
}

fn parse_hex(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, pair) in hex.as_bytes().chunks(2).enumerate() {
        let text = std::str::from_utf8(pair).ok()?;
        out[i] = u8::from_str_radix(text, 16).ok()?;
    }
    Some(out)
}

/// The SHA-256 of `bytes`.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).into()
}

/// A SHA-256 as 64 lowercase hex digits.
pub fn hex(digest: &[u8; 32]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// At most 60 characters of `text`, for messages.
fn short(text: &str) -> String {
    let mut out: String = text.chars().take(60).collect();
    if out.len() < text.len() {
        out.push('…');
    }
    out
}
