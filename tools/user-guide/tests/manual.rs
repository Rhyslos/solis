//! The guide is the manual, built (decision 0112): these fail when the
//! manual and the guide kept in the repository disagree, when a page of
//! the manual is missing from its contents, or when a link in the manual
//! goes nowhere. Each failure says what to do.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[allow(
    clippy::panic,
    reason = "a test's helper: stopping loudly is the point"
)]
fn manual() -> user_guide::Manual {
    user_guide::read(&root()).unwrap_or_else(|why| panic!("{why}"))
}

#[test]
fn every_page_of_the_manual_is_in_its_contents() {
    let missing = user_guide::unlisted(&manual()).unwrap_or_else(|p| panic!("{p:?}"));
    assert!(
        missing.is_empty(),
        "These pages of docs/manual aren't in docs/manual/SUMMARY.md, so the guide has no \
         chapter for them: add each to a group there.\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn the_manual_builds_with_every_link_going_somewhere() {
    if let Err(problems) = user_guide::build(&manual()) {
        panic!(
            "The user guide can't be built from the manual:\n  {}",
            problems.join("\n  ")
        );
    }
}

#[test]
fn the_kept_guide_is_the_manual_built() {
    let Ok(built) = user_guide::build(&manual()) else {
        return; // The test above says why.
    };
    let kept_path = root().join(user_guide::KEPT_COPY);
    let kept = std::fs::read_to_string(&kept_path).unwrap_or_default();
    // Git may give Windows its own line ends.
    let same = kept.replace("\r\n", "\n") == built;
    assert!(
        same,
        "{} isn't the manual as it is now. Build it again and commit it with the manual's \
         change:\n  cargo run -p user-guide",
        user_guide::KEPT_COPY
    );
}

#[test]
fn the_guide_needs_no_network() {
    let Ok(built) = user_guide::build(&manual()) else {
        return;
    };
    for loads in ["<link ", "src=\"http", "url(\"http", "url(http", "@import"] {
        assert!(
            !built.contains(loads),
            "the guide loads something from outside: {loads}"
        );
    }
}
