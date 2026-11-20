//! The check's `--smoke` (Fixes 1, T2): a project that plays cleanly
//! passes, and one whose screen stops while playing, or can't start at
//! all, fails with where and when.

use crate::check_project::check_with;

const MADE_BY: &str = "Test 1.0";

/// The UI sample in a folder of its own, with `change` made to its HUD.
fn ui_sample(name: &str, change: impl Fn(&str) -> String) -> reflect::test_support::TempFolder {
    let temp = reflect::test_support::TempFolder::new(&format!("smoke-{name}")).unwrap();
    let root = temp.join("UI");
    crate::ui_sample::make(&root).unwrap();
    let hud = root.join("ui/hud.lst");
    let text = std::fs::read_to_string(&hud).unwrap();
    std::fs::write(&hud, change(&text)).unwrap();
    temp
}

#[test]
fn a_project_that_plays_cleanly_passes() {
    let temp = ui_sample("fine", str::to_string);
    let report = check_with(&temp.join("UI"), MADE_BY, Some(30));
    let said = report.text();
    assert_eq!(report.failed, 0, "{said}");
    assert!(
        said.contains("ui.sollevel plays for 30 ticks with nothing wrong in the Console"),
        "{said}"
    );
    // Without --smoke, nothing is played.
    let report = check_with(&temp.join("UI"), MADE_BY, None);
    assert!(!report.text().contains("Playing"));
}

#[test]
fn a_screen_that_stops_while_playing_fails_with_its_tick() {
    let temp = ui_sample("stops", |hud| {
        hud.replace(
            "        int most = turret.maxAmmo;",
            "        int most = turret.maxAmmo;\n        colour tint = Colour:hex(\"#\" + shade());",
        )
        .replace(
            "\nassembly Hud",
            "\nmethod string shade() { return \"zz\"; }\n\nassembly Hud",
        )
    });
    let report = check_with(&temp.join("UI"), MADE_BY, Some(10));
    let said = report.text();
    assert!(report.failed >= 1, "{said}");
    let line = said
        .lines()
        .find(|l| l.contains("FAIL") && l.contains("Hud"))
        .unwrap_or_else(|| panic!("{said}"));
    assert!(line.contains("tick "), "{line}");
    assert!(line.contains("ui/hud.lst"), "{line}");
    // The code itself checks out: only playing finds it.
    assert!(said.contains("the game's code checks"), "{said}");
}

#[test]
fn screens_that_can_t_start_fail_the_check_without_playing() {
    let temp = ui_sample("no-start", |hud| {
        hud.replace(
            "assembly Hud {\n",
            "method string shade() { return \"zz\"; }\n\nassembly Hud {\n    has {\n        colour \
             tint = Colour:hex(\"#\" + shade());\n    }\n",
        )
    });
    let report = check_with(&temp.join("UI"), MADE_BY, None);
    let said = report.text();
    assert_eq!(report.failed, 1, "{said}");
    assert!(said.contains("the game's screens couldn't start"), "{said}");
    assert!(said.contains("ui/hud.lst"), "{said}");
}

#[test]
fn a_project_without_levels_says_so() {
    let temp = reflect::test_support::TempFolder::new("smoke-empty").unwrap();
    let root = temp.join("Empty");
    asset_db::Project::create(&root, "Empty").unwrap();
    let report = check_with(&root, MADE_BY, Some(5));
    let said = report.text();
    assert_eq!(report.failed, 0, "{said}");
    assert!(said.contains("no level to play"), "{said}");
}
