use crate::release::{self, build_of_tag, is_release_download, read_checksum, read_release};
use crate::*;
use std::cell::Cell;
use std::path::{Path, PathBuf};

const REPO: &str = "Rhyslos/solis";
const SETUP: &str = "solis-setup.exe";

fn url(tag: &str, name: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/{tag}/{name}")
}

fn release_text(tag: &str) -> String {
    format!(
        r#"{{"tag_name":"{tag}","draft":false,"prerelease":false,"assets":[
            {{"name":"solis.exe","browser_download_url":"{}","size":10}},
            {{"name":"{SETUP}","browser_download_url":"{}","size":1234}},
            {{"name":"{SETUP}.sha256","browser_download_url":"{}","size":82}}
        ]}}"#,
        url(tag, "solis.exe"),
        url(tag, SETUP),
        url(tag, &format!("{SETUP}.sha256")),
    )
}

// --- Reading releases ---

#[test]
fn a_release_gives_its_build_installer_and_checksum() {
    let release = read_release(&release_text("build-57"), REPO, SETUP).unwrap();
    assert_eq!(release.build, 57);
    assert_eq!(release.installer.name, SETUP);
    assert_eq!(release.installer.size, 1234);
    assert_eq!(release.installer.url, url("build-57", SETUP));
    assert_eq!(release.checksum.name, format!("{SETUP}.sha256"));
}

#[test]
fn github_s_other_fields_are_ignored() {
    let text = release_text("build-3").replacen(
        "\"draft\"",
        "\"html_url\":\"x\",\"author\":{\"login\":\"a\"},\"draft\"",
        1,
    );
    assert_eq!(read_release(&text, REPO, SETUP).unwrap().build, 3);
}

#[test]
fn tags_must_be_builds() {
    assert_eq!(build_of_tag("build-1"), Some(1));
    assert_eq!(build_of_tag("build-000120"), Some(120));
    for bad in [
        "",
        "build-",
        "v1.0",
        "build-1a",
        "build--1",
        "build-+1",
        "Build-1",
        " build-1",
        "build-99999999999999999999",
    ] {
        assert_eq!(build_of_tag(bad), None, "{bad:?}");
    }
    let err = read_release(&release_text("v0.1"), REPO, SETUP).unwrap_err();
    assert!(err.contains("isn't a build"), "{err}");
}

#[test]
fn drafts_and_pre_releases_are_refused() {
    let draft = release_text("build-2").replace("\"draft\":false", "\"draft\":true");
    assert!(read_release(&draft, REPO, SETUP).is_err());
    let pre = release_text("build-2").replace("\"prerelease\":false", "\"prerelease\":true");
    assert!(read_release(&pre, REPO, SETUP).is_err());
}

#[test]
fn missing_files_are_named() {
    let text = release_text("build-9").replace(&format!("\"{SETUP}.sha256\""), "\"other\"");
    let err = read_release(&text, REPO, SETUP).unwrap_err();
    assert!(
        err.contains("build 9 has no solis-setup.exe.sha256"),
        "{err}"
    );
    let err = read_release(&release_text("build-9"), REPO, "other.exe").unwrap_err();
    assert!(err.contains("has no other.exe"), "{err}");
}

#[test]
fn downloads_from_anywhere_else_are_refused() {
    let elsewhere = release_text("build-4").replace(
        &url("build-4", SETUP),
        "https://example.com/solis-setup.exe",
    );
    let err = read_release(&elsewhere, REPO, SETUP).unwrap_err();
    assert!(err.contains("isn't on Rhyslos/solis's releases"), "{err}");
    // Another repository's releases aren't ours.
    let other = release_text("build-4").replace("Rhyslos/solis", "Someone/solis");
    assert!(read_release(&other, REPO, SETUP).is_err());
}

#[test]
fn only_plain_release_addresses_count() {
    assert!(is_release_download(&url("build-1", SETUP), REPO));
    for bad in [
        format!("http://github.com/{REPO}/releases/download/build-1/{SETUP}"),
        format!("https://github.com/{REPO}/releases/download/"),
        format!("https://github.com/{REPO}/releases/download/build-1/"),
        format!("https://github.com/{REPO}/releases/download/build-1/a/b.exe"),
        format!("https://github.com/{REPO}/releases/download/../x/{SETUP}"),
        format!("https://github.com/{REPO}/releases/download/build-1/{SETUP}?x=1"),
        format!("https://github.com/{REPO}/releases/download/build-1/{SETUP}#x"),
        format!("https://github.com/{REPO}/releases/download/build-1\\{SETUP}"),
        format!("https://github.com.evil.com/{REPO}/releases/download/build-1/{SETUP}"),
        format!("https://user@github.com/{REPO}/releases/download/build-1/{SETUP}"),
    ] {
        assert!(!is_release_download(&bad, REPO), "{bad}");
    }
}

#[test]
fn oversized_answers_and_installers_are_refused() {
    let huge = " ".repeat(release::MAX_RELEASE_TEXT as usize + 1);
    assert!(
        read_release(&huge, REPO, SETUP)
            .unwrap_err()
            .contains("too long")
    );
    let big = release_text("build-5").replace("\"size\":1234", "\"size\":999999999999");
    assert!(
        read_release(&big, REPO, SETUP)
            .unwrap_err()
            .contains("too big")
    );
    let many: Vec<String> = (0..=release::MAX_ASSETS)
        .map(|i| format!(r#"{{"name":"f{i}","browser_download_url":"x"}}"#))
        .collect();
    let text = format!(r#"{{"tag_name":"build-1","assets":[{}]}}"#, many.join(","));
    assert!(
        read_release(&text, REPO, SETUP)
            .unwrap_err()
            .contains("too many")
    );
}

#[test]
fn unreadable_answers_say_so() {
    for bad in ["", "{", "null", "[]", "{\"tag_name\":5}", "{\"assets\":[]}"] {
        let err = read_release(bad, REPO, SETUP).unwrap_err();
        assert!(!err.is_empty(), "{bad}");
    }
}

// --- Checksums ---

#[test]
fn checksums_read_as_sha256sum_writes_them() {
    let digest = release::sha256(b"hello");
    let hex = release::hex(&digest);
    assert_eq!(
        hex,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
    for text in [
        hex.clone(),
        format!("{hex}\n"),
        format!("{hex}  {SETUP}\n"),
        format!("{hex} *{SETUP}"),
        format!("\u{feff}{}  {SETUP}\r\n", hex.to_uppercase()),
    ] {
        assert_eq!(read_checksum(&text, SETUP), Ok(digest), "{text:?}");
    }
}

#[test]
fn bad_checksums_are_refused() {
    let hex = release::hex(&release::sha256(b"x"));
    for bad in [
        String::new(),
        hex[..63].to_string(),
        format!("{hex}0"),
        format!("{}g", &hex[..63]),
        format!("{hex}  other.exe"),
        "é".repeat(32),
        "x".repeat(release::MAX_CHECKSUM_TEXT as usize + 1),
    ] {
        assert!(read_checksum(&bad, SETUP).is_err(), "{bad:?}");
    }
}

// --- This copy ---

#[test]
fn build_numbers_are_plain_positive_numbers() {
    assert_eq!(parse_build("57"), Some(57));
    assert_eq!(parse_build(" 8\n"), Some(8));
    for bad in ["", "0", "-1", "1.0", "abc", "99999999999999999999"] {
        assert_eq!(parse_build(bad), None, "{bad:?}");
    }
}

#[test]
fn a_copy_built_here_has_no_build_number() {
    // The checks don't set BUILD_NUMBER; only the Windows EXE build does.
    if std::env::var_os("BUILD_NUMBER").is_none() {
        assert_eq!(this_build(), None);
    }
}

#[test]
fn installed_copies_have_the_marker_beside_them() {
    let folder = temp_folder("installed");
    let exe = folder.join("solis.exe");
    assert!(!is_installed(&exe));
    std::fs::write(folder.join(INSTALLED_MARKER), "").unwrap();
    assert!(is_installed(&exe));
    // A folder of that name doesn't count.
    let other = temp_folder("installed-dir");
    std::fs::create_dir(other.join(INSTALLED_MARKER)).unwrap();
    assert!(!is_installed(&other.join("solis.exe")));
    assert!(!is_installed(Path::new("solis.exe")));
}

#[test]
fn updates_go_in_a_folder_of_their_own() {
    let folder = updates_folder("solis");
    assert!(folder.ends_with("solis/updates") || folder.ends_with("solis\\updates"));
    assert!(folder.is_absolute());
}

#[test]
fn the_installer_runs_quietly_and_relaunches_only_when_asked() {
    let plain = installer_arguments(false);
    assert!(plain.contains(&"/VERYSILENT"));
    assert!(plain.contains(&"/CLOSEAPPLICATIONS"));
    assert!(!plain.contains(&"/RELAUNCH=1"));
    assert!(installer_arguments(true).contains(&"/RELAUNCH=1"));
}

#[test]
fn a_missing_installer_isn_t_started() {
    let err = start_installer(Path::new("/no/such/setup.exe"), false).unwrap_err();
    assert!(err.contains("is gone"), "{err}");
}

#[test]
fn clearing_removes_old_downloads_only() {
    let folder = temp_folder("clear");
    std::fs::write(folder.join("old.exe"), "x").unwrap();
    std::fs::write(folder.join("old.exe.part"), "x").unwrap();
    std::fs::create_dir(folder.join("keep")).unwrap();
    clear_folder(&folder).unwrap();
    let left: Vec<_> = std::fs::read_dir(&folder).unwrap().flatten().collect();
    assert_eq!(left.len(), 1);
    assert!(clear_folder(&folder.join("missing")).is_ok());
}

// --- Checks ---

/// A stand-in for GitHub.
struct Fake {
    latest: Result<u64, String>,
    download: Result<(), String>,
    downloads: Cell<u32>,
}

impl Fake {
    fn new(latest: Result<u64, String>) -> Self {
        Fake {
            latest,
            download: Ok(()),
            downloads: Cell::new(0),
        }
    }
}

impl Remote for Fake {
    fn latest(&self) -> Result<Release, String> {
        let build = self.latest.clone()?;
        let tag = format!("build-{build}");
        Ok(read_release(&release_text(&tag), REPO, SETUP).unwrap())
    }
    fn download(&self, release: &Release, folder: &Path) -> Result<PathBuf, String> {
        self.downloads.set(self.downloads.get() + 1);
        self.download.clone()?;
        Ok(folder.join(&release.installer.name))
    }
}

fn copy(build: Option<u64>, installed: bool) -> Copy {
    Copy {
        build,
        installed,
        folder: PathBuf::from("updates"),
    }
}

fn check(remote: &Fake, copy: &Copy) -> (Vec<Status>, Status) {
    let mut steps = Vec::new();
    let last = run(remote, copy, &mut |s| steps.push(s));
    (steps, last)
}

#[test]
fn a_newer_build_is_downloaded_for_an_installed_copy() {
    let fake = Fake::new(Ok(12));
    let (steps, last) = check(&fake, &copy(Some(10), true));
    assert_eq!(steps, vec![Status::Checking, Status::Downloading(12)]);
    assert_eq!(
        last,
        Status::Ready {
            build: 12,
            installer: PathBuf::from("updates").join(SETUP)
        }
    );
    assert!(last.is_final());
}

#[test]
fn the_same_or_an_older_build_is_up_to_date() {
    for latest in [10, 9, 1] {
        let fake = Fake::new(Ok(latest));
        let (_, last) = check(&fake, &copy(Some(10), true));
        assert_eq!(last, Status::UpToDate(10));
        assert_eq!(fake.downloads.get(), 0);
    }
}

#[test]
fn copies_not_installed_are_told_but_not_downloaded() {
    let fake = Fake::new(Ok(12));
    let (_, last) = check(&fake, &copy(Some(10), false));
    assert_eq!(last, Status::NotInstalled(12));
    assert_eq!(fake.downloads.get(), 0);
}

#[test]
fn copies_built_from_source_never_ask() {
    let fake = Fake::new(Err("asked".into()));
    let (steps, last) = check(&fake, &copy(None, true));
    assert!(steps.is_empty());
    assert_eq!(last, Status::FromSource);
}

#[test]
fn failures_are_reported_in_words() {
    let fake = Fake::new(Err("couldn't reach GitHub".into()));
    let (_, last) = check(&fake, &copy(Some(1), true));
    assert_eq!(last, Status::Failed("couldn't reach GitHub".into()));
    let mut fake = Fake::new(Ok(2));
    fake.download = Err("didn't match its checksum".into());
    let (_, last) = check(&fake, &copy(Some(1), true));
    assert_eq!(last, Status::Failed("didn't match its checksum".into()));
}

/// A thread-safe stand-in, for [`start`].
struct Sendable(u64);

impl Remote for Sendable {
    fn latest(&self) -> Result<Release, String> {
        Fake::new(Ok(self.0)).latest()
    }
    fn download(&self, release: &Release, folder: &Path) -> Result<PathBuf, String> {
        Ok(folder.join(&release.installer.name))
    }
}

#[test]
fn a_started_check_reports_every_step_and_wakes_the_window() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    let wakes = Arc::new(AtomicU32::new(0));
    let counter = wakes.clone();
    let receiver = start(Sendable(5), copy(Some(4), true), move || {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    let steps: Vec<Status> = receiver.iter().collect();
    assert_eq!(steps.len(), 3, "{steps:?}");
    assert!(matches!(steps.last(), Some(Status::Ready { build: 5, .. })));
    assert_eq!(wakes.load(Ordering::SeqCst), 3);
}

fn temp_folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir()
        .join("solis-updater-tests")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

// --- Damaged answers never crash ---

mod damaged {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn any_answer_is_read_or_refused(text in ".{0,400}") {
            let _ = read_release(&text, REPO, SETUP);
            let _ = read_checksum(&text, SETUP);
            let _ = build_of_tag(&text);
            let _ = is_release_download(&text, REPO);
        }

        #[test]
        fn damaged_releases_are_read_or_refused(cut in 0usize..600, byte in any::<u8>()) {
            let mut bytes = release_text("build-7").into_bytes();
            if let Some(b) = bytes.get_mut(cut) {
                *b = byte;
            }
            let text = String::from_utf8_lossy(&bytes);
            if let Ok(release) = read_release(&text, REPO, SETUP) {
                prop_assert!(is_release_download(&release.installer.url, REPO));
            }
        }
    }
}

/// Asks the real GitHub (needs the internet, so only when asked:
/// `cargo test -p updater -- --ignored`). Before the first build is
/// published, GitHub has none to give.
#[test]
#[ignore = "needs the internet"]
fn github_answers() {
    let source = Source {
        repo: REPO.into(),
        installer_name: SETUP.into(),
        user_agent: "solis-updater-test".into(),
    };
    match source.latest() {
        Ok(release) => assert!(release.build > 0),
        Err(why) => assert!(why.contains("no builds to update to yet"), "{why}"),
    }
}
