//! The guide is the manual, built (decision 0112): these fail when the
//! manual and the guide kept in the repository disagree, when a page of
//! the manual is missing from its contents, or when a link in the manual
//! goes nowhere. Each failure says what to do.
//!
//! And its pictures and examples (decision 0113): a picture a page shows
//! is missing, a picture is shown by no page, or the pictures are older
//! than the editor's interface; a game-code example isn't in all three
//! languages (those stop the build, so the test above fails), its Rust's
//! brackets don't balance, or its Lua or Rust calls something game code
//! can't. Its Lust, and its Lua's syntax, are checked against the engine
//! by `crates/script/src/manual_examples_tests.rs`.

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

#[test]
fn the_guide_s_pictures_are_as_new_as_the_interface() {
    let older =
        user_guide::pictures::older_than_interface(&root()).unwrap_or_else(|why| panic!("{why}"));
    assert!(
        !older,
        "The guide's pictures are older than the interface: the editor's approved picture-test \
         images ({}) changed since they were taken. Take them again, which also builds the \
         guide, and commit both:\n  {}",
        user_guide::pictures::INTERFACE,
        user_guide::pictures::RETAKE
    );
}

#[allow(
    clippy::panic,
    reason = "a test's helper: stopping loudly is the point"
)]
fn examples() -> Vec<user_guide::examples::Example> {
    user_guide::examples(&manual()).unwrap_or_else(|p| panic!("{p:?}"))
}

#[test]
fn the_manual_has_game_code_examples_to_check() {
    assert!(examples().len() >= 20, "the examples weren't found");
}

#[test]
fn every_example_s_rust_has_its_brackets_balanced() {
    let broken: Vec<String> = examples()
        .iter()
        .filter_map(|e| {
            let why = user_guide::examples::brackets_balance(&e.code[1]).err()?;
            let start = e.code[0].lines().next().unwrap_or("");
            Some(format!(
                "{}.md, the example starting \"{start}\": its Rust's {why}",
                e.page
            ))
        })
        .collect();
    assert!(broken.is_empty(), "{}", broken.join("\n"));
}

#[test]
fn every_example_s_lua_and_rust_call_only_what_game_code_can() {
    let path = root().join(user_guide::examples::INTERFACE);
    let interface =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let wrong: Vec<String> = examples()
        .iter()
        .flat_map(|e| {
            let start = e.code[0].lines().next().unwrap_or("").to_string();
            user_guide::examples::unknown_calls(e, &interface)
                .into_iter()
                .map(move |call| format!("{}.md, the example starting \"{start}\": {call}", e.page))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "These examples call something game code can't (it isn't in {}):\n  {}",
        user_guide::examples::INTERFACE,
        wrong.join("\n  ")
    );
}
