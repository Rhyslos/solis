//! Nothing became unreachable when the menus went: every item the old
//! menu bar had can still be done, by name, from the ribbon, a File page
//! or the search (the plan's rule 1, `docs/plans/shared-1-ui-remake.md`).
//!
//! [`MOVED`] is the old menu bar, item by item, as the plan's inventory
//! lists it (part a.2): where each was, what it is called now, and where
//! it is drawn. The tests walk it.

use crate::shell::{RibbonTab, ShellCommand};
use crate::test_support::{do_command, harness, info};
use crate::workbench::{Workbench, WorkbenchAction};
use egui_kittest::kittest::Queryable;

/// Where an old menu item is drawn now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// On this tab's ribbon, as a control with the command's name.
    Ribbon(RibbonTab),
    /// In a list opened from this control of this tab's ribbon, under
    /// these words.
    RibbonList(RibbonTab, &'static str, &'static str),
    /// In the File page's column, under these words.
    File(&'static str),
    /// In the search alone (and by its key, where it has one).
    Search,
}

/// The old menu bar: each item, its command's name now, and its place.
const MOVED: [(&str, &str, Place); 43] = [
    (
        "File > New project…",
        "New project…",
        Place::File("New project"),
    ),
    (
        "File > Open project…",
        "Open project…",
        Place::File("Open…"),
    ),
    (
        "File > Projects…",
        "Recent projects",
        Place::File("Recent projects"),
    ),
    ("File > New area", "New area", Place::Search),
    ("File > Open area…", "Open area…", Place::Search),
    (
        "File > Save area",
        "Save area",
        Place::Ribbon(RibbonTab::Home),
    ),
    ("File > Save area as…", "Save area as…", Place::Search),
    (
        "File > Import model…",
        "Import model…",
        Place::Ribbon(RibbonTab::Library),
    ),
    (
        "File > Export for playtesting…",
        "Export for playtesting…",
        Place::Ribbon(RibbonTab::Build),
    ),
    ("File > Quit", "Exit", Place::File("Exit")),
    (
        "Edit > Undo",
        "Undo Add Point light 1",
        Place::Ribbon(RibbonTab::Home),
    ),
    (
        "Edit > Redo",
        "Redo Add Point light 1",
        Place::Ribbon(RibbonTab::Home),
    ),
    ("Edit > Duplicate", "Duplicate", Place::Search),
    ("Edit > Delete", "Delete", Place::Search),
    ("Edit > Rename", "Rename", Place::Search),
    ("Edit > Select all", "Select all", Place::Search),
    ("Edit > Select nothing", "Select nothing", Place::Search),
    // On Project settings' Input sub-tab, which the search opens too
    // (the File column's own Input item went with the window, M9).
    ("Edit > Input actions…", "Input actions…", Place::Search),
    (
        "Edit > Settings…",
        "Preferences",
        Place::File("Preferences"),
    ),
    (
        "Edit > Settings… > 2D",
        "Project settings",
        Place::File("Project settings"),
    ),
    ("View > Viewport tools", "Hide the toolbox", Place::Search),
    (
        "View > Grid, angle and camera speed",
        "Hide the viewport's axis locks and camera speed",
        Place::Search,
    ),
    (
        "View > Frame rate and entity count",
        "Hide the viewport's frame time and frame rate",
        Place::Search,
    ),
    (
        "Entity > Add entity",
        "Add entity",
        Place::Ribbon(RibbonTab::Home),
    ),
    (
        "Entity > Add point light",
        "Add point light",
        Place::Ribbon(RibbonTab::Home),
    ),
    (
        "Entity > Add spotlight",
        "Add spotlight",
        Place::RibbonList(RibbonTab::Home, "Light: more kinds", "Spotlight"),
    ),
    (
        "Entity > Create assembly from selection",
        "Create assembly from selection",
        Place::RibbonList(RibbonTab::Home, "More", "Assembly from selection"),
    ),
    (
        "Entity > Add camera",
        "Add camera",
        Place::Ribbon(RibbonTab::Home),
    ),
    ("Play > Play", "Play", Place::Ribbon(RibbonTab::Home)),
    ("Play > Pause", "Pause", Place::Ribbon(RibbonTab::Home)),
    (
        "Play > Step one tick",
        "Step one tick",
        Place::Ribbon(RibbonTab::Home),
    ),
    ("Play > Stop", "Stop", Place::Ribbon(RibbonTab::Home)),
    (
        "Play > Load a save…",
        "Load a save…",
        Place::Ribbon(RibbonTab::Debug),
    ),
    (
        "Play > Play and record inputs",
        "Play and record inputs",
        Place::Ribbon(RibbonTab::Debug),
    ),
    (
        "Play > Replay a recording…",
        "Replay a recording…",
        Place::Ribbon(RibbonTab::Debug),
    ),
    ("Window > Reset layout", "Reset layout", Place::Search),
    (
        "Help > Open the manual ↗",
        "Open the manual ↗",
        Place::Search,
    ),
    (
        "Help > Add the sample area",
        "Add the sample area",
        Place::Search,
    ),
    (
        "Help > Make the sample game…",
        "Make the sample game…",
        Place::Search,
    ),
    (
        "Help > Make the UI sample…",
        "Make the UI sample…",
        Place::Search,
    ),
    (
        "Help > Make the benchmark…",
        "Make the benchmark…",
        Place::Search,
    ),
    (
        "Help > Make the playground…",
        "Make the playground…",
        Place::Search,
    ),
    (
        "Help > Check for updates",
        "Check for updates",
        Place::Search,
    ),
];

/// An editor in which every old item can be done at some moment: a
/// project open, something selected, a step to undo, a manual to open.
fn ready() -> Workbench {
    let mut w = Workbench::new(info());
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    w.set_manual_url("https://example.org/manual".into());
    w.add_point_light();
    w
}

/// The editor at a moment when the command called `name` can be done.
fn at_the_moment_for(name: &str) -> Workbench {
    let mut w = ready();
    match name {
        "Redo Add Point light 1" => w.undo(),
        "Pause" | "Stop" => w.play(),
        "Step one tick" => {
            w.play();
            w.pause();
        }
        _ => {}
    }
    w
}

#[test]
fn every_old_menu_item_is_a_command_that_can_be_done_and_the_search_lists_it() {
    for (was, name, _) in MOVED {
        let mut w = at_the_moment_for(name);
        let command = ShellCommand::named(name, &w)
            .unwrap_or_else(|| panic!("{was} has no command called {name:?}"));
        assert_eq!(command.why_not(&w), None, "{was}");
        assert!(
            w.search_items().iter().any(|item| item.name == name),
            "{was}: the search lists {name:?}"
        );
        // And it does something: asks the app, or changes the editor.
        let ctx = egui::Context::default();
        let before = format!("{:?}", snapshot(&w));
        let asked = do_command(&mut w, &ctx, name);
        let after = format!("{:?}", snapshot(&w));
        assert!(
            !asked.is_empty() || before != after || QUIET.contains(&name),
            "{was}: {name:?} did nothing"
        );
    }
}

/// Commands whose doing shows nowhere this test looks: Rename starts
/// typing in the Area panel, the manual opens in the web browser, with
/// one entity in the area Select all selects what already is, a step
/// moves the game on by a tick (`play_tests.rs` watches that), and the
/// panels of an untouched arrangement are already where Reset layout
/// puts them (`file_pages/settings_tests.rs` moves one first).
const QUIET: [&str; 5] = [
    "Rename",
    "Open the manual ↗",
    "Select all",
    "Step one tick",
    "Reset layout",
];

/// What a command may change, as far as these tests look.
fn snapshot(w: &Workbench) -> impl std::fmt::Debug + use<> {
    (
        (
            w.open.document.revision(),
            w.open.document.selection.primary(),
            w.play_state(),
            w.ribbon_tab(),
            w.file_pages.page,
        ),
        (
            (w.file_pages.prefs_tab, w.file_pages.project_tab),
            w.open.input_page.is_some(),
            w.open.export.is_some(),
            w.viewport.overlays,
            w.open.outbox.len(),
            // Leaving the area asks about unsaved changes first.
            w.open.unsaved_prompt.is_some(),
        ),
    )
}

#[test]
fn every_old_menu_item_is_drawn_where_the_table_says() {
    for (was, name, place) in MOVED {
        let mut h = harness(at_the_moment_for(name));
        h.run();
        match place {
            Place::Search => {
                // Typed into the search, it is the first thing found.
                h.get_by_label("Search or run a command").click();
                h.run();
                h.run();
                h.get_by_label("Search").type_text(name);
                h.run();
                assert!(
                    h.query_all_by_label(name).next().is_some(),
                    "{was}: found by typing {name:?}"
                );
            }
            Place::Ribbon(tab) => {
                h.state_mut().0.show_tab(tab);
                h.run();
                h.run();
                // (A panel may have a button of the same name too.)
                let band = h.get_by_label("Simulate").rect().y_range();
                let on_the_ribbon = h
                    .query_all_by_label(name)
                    .any(|node| band.contains(node.rect().center().y));
                assert!(
                    on_the_ribbon,
                    "{was}: {name:?} is on {}'s ribbon",
                    tab.name()
                );
            }
            Place::RibbonList(tab, opener, words) => {
                h.state_mut().0.show_tab(tab);
                h.run();
                h.get_by_label(opener).click();
                h.run();
                assert!(
                    h.query_by_label(words).is_some(),
                    "{was}: {words:?} is in the list {opener:?} opens"
                );
            }
            Place::File(words) => {
                h.get_by_label("File tab").click();
                h.run();
                h.run();
                assert!(
                    h.query_all_by_label(words).next().is_some(),
                    "{was}: File has {words:?}"
                );
            }
        }
    }
}

#[test]
fn the_file_page_s_items_do_what_the_menus_did() {
    let mut h = harness(ready());
    h.run();
    // Preferences, Project settings and its Input are pages now, not
    // windows: everything the windows held is on them.
    use crate::file_pages::{Page, Tab};
    crate::test_support::file_item(&mut h, "Preferences");
    assert_eq!(h.state().0.file_pages.page, Page::Preferences);
    for tab in ["Appearance", "Viewport", "Shortcuts", "General"] {
        h.get_by_role_and_label(egui::accesskit::Role::Button, tab)
            .click();
        h.run();
    }
    assert!(
        h.query_all_by_label("Hide unfinished parts")
            .next()
            .is_some()
    );
    crate::test_support::file_item(&mut h, "Project settings");
    assert_eq!(h.state().0.file_pages.page, Page::ProjectSettings);
    for (tab, shows) in [
        ("2D", "Base resolution"),
        ("Build", "Game's name"),
        ("Input", "Jump"),
    ] {
        h.get_by_role_and_label(egui::accesskit::Role::Button, tab)
            .click();
        h.run();
        assert!(
            h.query_all_by_label_contains(shows).next().is_some(),
            "{tab}: {shows}"
        );
    }
    assert_eq!(h.state().0.file_pages.project_tab, Tab::Input);
    crate::test_support::file_item(&mut h, "About");
    assert_eq!(h.state().0.file_pages.page, crate::file_pages::Page::About);
    assert!(
        h.query_by_label("Open LICENSE").is_some(),
        "About is a page"
    );
    crate::test_support::file_item(&mut h, "Save all");
    assert!(h.state().1.actions.contains(&WorkbenchAction::SaveArea));
    crate::test_support::file_item(&mut h, "Back to editor");
    assert_eq!(h.state().0.ribbon_tab(), RibbonTab::Home);
    // Exit asks about the unsaved light first, as Quit did.
    crate::test_support::file_item(&mut h, "Exit");
    assert!(h.state().0.open.unsaved_prompt.is_some());
    assert!(!h.state().1.actions.contains(&WorkbenchAction::Quit));
}

#[test]
fn each_shortcut_in_the_table_does_what_its_command_does() {
    use egui::{Key, Modifiers};
    let keys = |text: &str| -> (Modifiers, Key) {
        let mut modifiers = Modifiers::NONE;
        let mut key = None;
        for part in text.split('+') {
            match part {
                "Ctrl" => modifiers |= Modifiers::COMMAND,
                "Shift" => modifiers |= Modifiers::SHIFT,
                "Esc" => key = Some(Key::Escape),
                other => key = Key::from_name(other),
            }
        }
        (
            modifiers,
            key.unwrap_or_else(|| panic!("{text} names no key")),
        )
    };
    let mut with_keys = 0;
    for command in ShellCommand::ALL {
        if command.keys().is_empty() {
            continue;
        }
        with_keys += 1;
        let (modifiers, key) = keys(command.keys());
        let name = |w: &Workbench| command.name(w);
        // Done by its key, with the pointer over the viewport...
        let mut w = at_the_moment_for("");
        if command == ShellCommand::Redo {
            w.undo();
        }
        let mut by_key = crate::test_support::rest_over(harness(w), crate::panel::Panel::Viewport);
        by_key.key_press_modifiers(modifiers, key);
        by_key.run();
        // ...and by its name.
        let mut w = at_the_moment_for("");
        if command == ShellCommand::Redo {
            w.undo();
        }
        let mut by_name = crate::test_support::rest_over(harness(w), crate::panel::Panel::Viewport);
        let called = name(&by_name.state().0);
        crate::test_support::run(&mut by_name, &called);
        let (a, b) = (by_key.state(), by_name.state());
        assert_eq!(
            a.1.actions, b.1.actions,
            "{command:?} asks the app the same"
        );
        assert_eq!(
            a.0.open.document.area().len(),
            b.0.open.document.area().len(),
            "{command:?}"
        );
        assert_eq!(
            a.0.open.document.selection.primary().is_some(),
            b.0.open.document.selection.primary().is_some(),
            "{command:?}"
        );
        assert_eq!(
            a.0.open.document.history().undo_len(),
            b.0.open.document.history().undo_len(),
            "{command:?}"
        );
        assert_eq!(a.0.shelf_folded(), b.0.shelf_folded(), "{command:?}");
    }
    assert_eq!(with_keys, 17, "every command with a shortcut was tried");
}
