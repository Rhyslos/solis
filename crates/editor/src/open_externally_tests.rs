//! What each place in the editor asks the computer's own programs for
//! (`WorkbenchAction::OpenExternally`): only kinds of file whose program
//! just shows or edits them are asked to open; anything else is shown in
//! its folder, and nothing outside the project's assets is asked for at
//! all (the security review, SEC-03).

use crate::console::LogLevel;
use crate::library::{LibraryKind, LibraryRequest, ProjectInfo};
use crate::properties::PropertyRequest;
use crate::test_support::info;
use crate::workbench::{Workbench, WorkbenchAction};

const ASSETS: &str = "/projects/farm/assets";

fn with_a_project() -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: ASSETS.into(),
    }));
    w
}

/// What `actions` ask the computer's own programs for, as paths inside the
/// assets ("" for the assets folder itself).
fn asked(actions: &[WorkbenchAction]) -> Vec<String> {
    actions
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::OpenExternally(path) => Some(
                path.strip_prefix(ASSETS)
                    .unwrap_or_else(|_| panic!("outside the assets: {path:?}"))
                    .to_string_lossy()
                    .into_owned(),
            ),
            _ => None,
        })
        .collect()
}

/// What "Open <name>'s code" in Properties asks for, for a component
/// written in `file`.
fn open_code(file: &str) -> (Workbench, Vec<String>) {
    let mut w = with_a_project();
    w.apply_property(PropertyRequest::OpenCode(file.into()), 0);
    let asked = asked(&w.open.outbox);
    (w, asked)
}

#[test]
fn open_code_on_a_lua_component_shows_its_folder_and_never_opens_the_file() {
    // SEC-03: a downloaded project's Lua component, selected, then "Open
    // <name>'s code": the .lua file went to whatever program the computer
    // has for it, which with a Lua interpreter installed ran the project's
    // code outside the sandbox.
    let (w, asked) = open_code("ai/sparks.lua");
    assert_eq!(asked, ["ai"], "its folder, never the file");
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.level == LogLevel::Info && l.text.contains("Showing ai/sparks.lua")),
        "and the Console says why, as it does from the Library"
    );
    // A Lust component's file still opens in the editor for Lust.
    assert_eq!(open_code("crops/wheat.lst").1, ["crops/wheat.lst"]);
}

#[test]
fn open_code_asks_for_exactly_what_the_library_does_for_the_same_file() {
    for file in [
        "ai/sparks.lua",
        "ai/SPARKS.LUA",
        "main.lst",
        "MAIN.LST",
        "weapons/gun.rs",
        "props/brick.png.exe",
        "props/ship.glb",
        "run.bat",
        "main.lst.exe",
    ] {
        let mut w = with_a_project();
        let mut from_library = Vec::new();
        w.library_request(
            LibraryRequest::Open(LibraryKind::Code, file.into()),
            &mut from_library,
        );
        assert_eq!(open_code(file).1, asked(&from_library), "{file}");
        let opens = file.to_lowercase().ends_with(".lst");
        assert_eq!(
            asked(&from_library) == [file],
            opens,
            "{file}: only Lust opens in another program"
        );
    }
}

#[test]
fn open_code_never_asks_for_anything_outside_the_assets_folder() {
    // A component's file is named by the project's own code, so it could
    // name a network share, a drive or a folder above the project.
    for file in [
        "//attacker.example/share/a.lst",
        "\\\\attacker\\share\\a.lst",
        "C:/Windows/notepad.exe",
        "C:main.lst",
        "../outside.lst",
        "props/../../outside.lst",
        "/etc/passwd",
    ] {
        let (w, asked) = open_code(file);
        assert!(
            w.open.outbox.is_empty() && asked.is_empty(),
            "{file}: {:?}",
            w.open.outbox
        );
    }
}

#[test]
fn open_code_without_a_project_says_where_the_code_is_and_asks_for_nothing() {
    let mut w = Workbench::new(info(), None);
    w.apply_property(PropertyRequest::OpenCode("kit/mover.lst".into()), 0);
    assert!(w.open.outbox.is_empty());
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("kit/mover.lst is in the ready-made code"))
    );
}

#[test]
fn only_pictures_text_and_lust_may_open_in_another_program() {
    use LibraryKind as K;
    // Every kind, with a name of its own and names made to look like
    // another kind's.
    let kinds = [
        (K::Model, "ship.glb"),
        (K::Assembly, "tower.solassembly"),
        (K::Area, "farm.solarea"),
        (K::Texture, "brick.png"),
        (K::Code, "main.lst"),
        (K::Sound, "step.wav"),
        (K::SoundEvent, "steps.solsound"),
        (K::SpriteSheet, "hero.aseprite"),
        (K::SpriteClips, "hero.solframes"),
        (K::Animator, "hero.solanim"),
        (K::Tileset, "grass.tileset.lst"),
        (K::TileMap, "farm.tilemap.lst"),
        (K::Atlas, "icons.atlas.lst"),
        (K::Text, "plan.md"),
        (K::Other, "run.bat"),
    ];
    for (kind, name) in kinds {
        let opens = matches!(kind, K::Texture | K::Code | K::Text);
        assert_eq!(kind.opens_in_its_program(name), opens, "{kind:?} {name}");
        assert_eq!(
            kind.opens_in_its_program(&format!("deep/in/{name}")),
            opens,
            "{kind:?} in a folder"
        );
    }
    // Code is Lust, Lua or Rust: only Lust opens.
    for (name, opens) in [
        ("main.lst", true),
        ("MAIN.LST", true),
        ("ai/sparks.lua", false),
        ("ai/sparks.LUA", false),
        ("armour.rs", false),
        ("main.lst.lua", false),
        ("main.lst.exe", false),
        ("lst", false),
        ("", false),
    ] {
        assert_eq!(K::Code.opens_in_its_program(name), opens, "{name}");
    }
    // What isn't a picture, text or code never opens, whatever it's called.
    for name in ["brick.png", "plan.md", "main.lst", "brick.png.exe"] {
        assert!(!K::Other.opens_in_its_program(name), "{name}");
        assert!(!K::Model.opens_in_its_program(name), "{name}");
    }
}
