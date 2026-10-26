//! Every way starting can go: checking off, up to date, offline, slow, a
//! newer build (updated, skipped or not), and a download that fails.

use super::*;
use std::sync::mpsc;
use updater::{Asset, Release};

const MS: Duration = Duration::from_millis(1);

fn release(build: u64) -> Release {
    let asset = |name: &str| Asset {
        name: name.into(),
        url: format!("https://github.com/Rhyslos/solis/releases/download/build-{build}/{name}"),
        size: 10,
    };
    Release {
        build,
        installer: asset("solis-setup.exe"),
        checksum: asset("solis-setup.exe.sha256"),
    }
}

/// A check that will end with `last` (sent at once).
fn answered(last: Status) -> Receiver<Status> {
    let (sender, receiver) = mpsc::channel();
    sender.send(Status::Checking).unwrap();
    sender.send(last).unwrap();
    receiver
}

fn starting(last: Status) -> Startup {
    Startup::new(Some(answered(last)), None, true, Some(10))
}

#[test]
fn the_splash_shows_for_at_least_its_time_even_when_up_to_date() {
    let mut s = starting(Status::UpToDate(10));
    assert_eq!(s.step(100 * MS), Next::Wait, "answered, but too soon");
    assert_eq!(s.step(SPLASH_AT_LEAST), Next::Open(None));
}

#[test]
fn with_checking_off_nothing_is_asked_and_it_opens_after_the_splash() {
    let mut s = Startup::new(None, None, true, Some(10));
    assert_eq!(s.doing(), "Starting…");
    assert_eq!(s.step(MS), Next::Wait);
    assert_eq!(s.step(SPLASH_AT_LEAST), Next::Open(None));
}

#[test]
fn offline_it_opens_and_says_why_quietly() {
    let mut s = starting(Status::Failed("couldn't reach GitHub".into()));
    let Next::Open(Some((level, text))) = s.step(SPLASH_AT_LEAST) else {
        panic!()
    };
    assert_eq!(level, LogLevel::Info);
    assert!(text.contains("couldn't reach GitHub"), "{text}");
}

#[test]
fn a_slow_github_doesn_t_hold_the_splash_and_the_check_goes_on() {
    let (_sender, receiver) = mpsc::channel();
    let mut s = Startup::new(Some(receiver), None, true, Some(10));
    assert_eq!(s.doing(), "Checking for updates…");
    assert_eq!(s.step(CHECK_WAIT - MS), Next::Wait);
    assert_eq!(s.step(CHECK_WAIT), Next::Open(None));
    assert!(s.take_check().is_some(), "handed on, to show under Help");
}

#[test]
fn a_newer_build_is_asked_about_and_waits_for_the_answer() {
    let mut s = starting(Status::Available(release(12)));
    assert_eq!(s.step(10 * Duration::from_secs(1)), Next::Wait);
    assert_eq!(
        s.offer(),
        Some(UpdateOffer {
            latest: 12,
            current: Some(10),
            installed: true
        })
    );
    // Still asking, however long it takes.
    assert_eq!(s.step(Duration::from_secs(600)), Next::Wait);
}

#[test]
fn update_now_downloads_on_the_splash_then_installs() {
    let mut s = starting(Status::Available(release(12)));
    s.step(SPLASH_AT_LEAST);
    let (sender, receiver) = mpsc::channel();
    let mut asked_for = None;
    s.answer(UpdateAnswer::UpdateNow, |r| {
        asked_for = Some(r.build);
        receiver
    });
    assert_eq!(asked_for, Some(12));
    assert_eq!(s.offer(), None, "the question goes");
    assert_eq!(s.doing(), "Downloading build 12…");
    sender.send(Status::Downloading(12)).unwrap();
    assert_eq!(s.step(SPLASH_AT_LEAST), Next::Wait);
    sender
        .send(Status::Ready {
            build: 12,
            installer: PathBuf::from("setup.exe"),
        })
        .unwrap();
    assert_eq!(
        s.step(SPLASH_AT_LEAST),
        Next::Install {
            installer: PathBuf::from("setup.exe")
        }
    );
    assert_eq!(s.skipped(), None);
}

#[test]
fn a_download_that_fails_opens_this_build_and_says_so() {
    let mut s = starting(Status::Available(release(12)));
    s.step(SPLASH_AT_LEAST);
    s.answer(UpdateAnswer::UpdateNow, |_| {
        answered(Status::Failed("didn't match its checksum".into()))
    });
    let Next::Open(Some((level, text))) = s.step(SPLASH_AT_LEAST) else {
        panic!()
    };
    assert_eq!(level, LogLevel::Warning);
    assert!(
        text.contains("build 12") && text.contains("checksum"),
        "{text}"
    );
}

#[test]
fn not_now_opens_and_skip_remembers_the_build() {
    let mut s = starting(Status::Available(release(12)));
    s.step(SPLASH_AT_LEAST);
    s.answer(UpdateAnswer::NotNow { skip: false }, |_| unreachable!());
    assert_eq!(s.step(SPLASH_AT_LEAST), Next::Open(None));
    assert_eq!(s.skipped(), None);

    let mut s = starting(Status::Available(release(12)));
    s.step(SPLASH_AT_LEAST);
    s.answer(UpdateAnswer::NotNow { skip: true }, |_| unreachable!());
    assert_eq!(s.step(SPLASH_AT_LEAST), Next::Open(None));
    assert_eq!(s.skipped(), Some(12));
}

#[test]
fn a_skipped_build_isn_t_asked_about_but_a_newer_one_is() {
    let mut s = Startup::new(
        Some(answered(Status::Available(release(12)))),
        Some(12),
        true,
        Some(10),
    );
    let Next::Open(Some((_, text))) = s.step(SPLASH_AT_LEAST) else {
        panic!("not asked")
    };
    assert!(text.contains("skip"), "{text}");
    let mut s = Startup::new(
        Some(answered(Status::Available(release(13)))),
        Some(12),
        true,
        Some(10),
    );
    s.step(SPLASH_AT_LEAST);
    assert_eq!(s.offer().map(|o| o.latest), Some(13));
}

#[test]
fn a_copy_not_installed_is_asked_too_and_can_open_the_page() {
    let mut s = Startup::new(
        Some(answered(Status::Available(release(12)))),
        None,
        false,
        Some(10),
    );
    s.step(SPLASH_AT_LEAST);
    assert_eq!(s.offer().map(|o| o.installed), Some(false));
    s.answer(UpdateAnswer::OpenDownloadPage, |_| unreachable!());
    assert_eq!(s.step(SPLASH_AT_LEAST), Next::Open(None));
}

#[test]
fn a_copy_built_from_source_just_opens() {
    let mut s = Startup::new(Some(answered(Status::FromSource)), None, false, None);
    assert_eq!(s.step(SPLASH_AT_LEAST), Next::Open(None));
}
