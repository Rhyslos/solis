//! The UI editor in the workbench (decision 0089): opening a screen from
//! the Library onto the canvas, drawing it, picking, Properties, saving,
//! undo, and the file changed on disk.

use crate::library::{LibraryKind, LibraryRequest, ProjectInfo};
use crate::panel::Panel;
use crate::screen_doc::tests::MENU;
use crate::test_support::*;
use crate::workbench::{Workbench, WorkbenchAction};
use crate::workspace::Workspace;
use egui_kittest::kittest::Queryable;
use std::sync::Arc;

/// The program for `main.lst` and `ui/pauseMenu.lst` as `menu`.
pub(crate) fn program(menu: &str) -> Arc<script::Compiled> {
    let code = script::Code {
        files: vec![
            ("main.lst".to_string(), "end\n".to_string()),
            ("ui/pauseMenu.lst".to_string(), menu.to_string()),
        ],
        lua: Vec::new(),
        notes: Vec::new(),
    };
    match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    }
}

/// A workbench with a project and MENU's program, the screen open.
pub(crate) fn opened() -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    w.set_program(Some(program(MENU)));
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::Open(LibraryKind::Code, "ui/pauseMenu.lst".into()),
        &mut actions,
    );
    w
}

fn saved(w: &Workbench) -> Vec<String> {
    w.outbox
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::SaveScreen { path, text } => {
                assert_eq!(
                    path.to_string_lossy().replace('\\', "/"),
                    "ui/pauseMenu.lst"
                );
                Some(text.clone())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_screen_s_file_opens_on_the_canvas_in_the_ui_workspace() {
    let mut w = opened();
    assert_eq!(w.workspace(), Workspace::Ui);
    assert!(w.dock.find_tab(&Panel::Canvas).is_some());
    assert!(w.dock.find_tab(&Panel::Elements).is_some());
    w.refresh_screen();
    let doc = w.screen_doc.as_ref().unwrap();
    assert_eq!(
        (doc.path.as_str(), doc.screen.as_str()),
        ("ui/pauseMenu.lst", "PauseMenu")
    );
    let view = doc.view.as_ref().expect("drawn from the game's code");
    assert_eq!(view.children[0].kind, "stack");
    assert_eq!(doc.matches().len(), 5);
    // Another code file isn't a screen: it opens as code.
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::Open(LibraryKind::Code, "main.lst".into()),
        &mut actions,
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::OpenExternally(p) if p.ends_with("main.lst")))
    );
    // Leaving UI and coming back keeps the canvas there, not elsewhere.
    w.switch_workspace(Workspace::Level);
    assert!(w.dock.find_tab(&Panel::Canvas).is_none());
    w.switch_workspace(Workspace::Ui);
    assert!(w.dock.find_tab(&Panel::Canvas).is_some());
}

#[test]
fn the_canvas_draws_the_screen_and_properties_changes_the_piece_picked() {
    let mut w = opened();
    w.screen_doc.as_mut().unwrap().picked = vec![vec![0, 0, 0]];
    let mut h = harness(w);
    h.run();
    let views = &h.state().1.views;
    let canvas = views
        .iter()
        .rev()
        .find(|v| v.image == crate::canvas_view::CANVAS_IMAGE)
        .expect("the canvas asked the renderer for its picture");
    assert_eq!(canvas.ui.len(), 1);
    assert_eq!(canvas.ui[0].name, "PauseMenu");
    assert!(canvas.ui[0].view.is_some());
    // Properties shows the title.
    h.get_by_label_contains("ui/pauseMenu.lst, line 10");
    assert!(h.query_by_label("Text size").is_some());
    // Elements lists the pieces, and the loop as code.
    h.get_by_label("text \"Paused\"");
    h.get_by_label("button \"Quit\"");
    assert!(h.query_by_label_contains("‹code› loop 2").is_some());
    // Bold, ticked in Properties, is saved.
    h.state_mut().1.actions.clear();
    h.get_all_by_role(egui::accesskit::Role::CheckBox)
        .next()
        .unwrap()
        .click();
    h.run();
    let saves: Vec<String> = h
        .state()
        .1
        .actions
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::SaveScreen { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(saves.len(), 1);
    assert!(
        saves[0].contains("text \"Paused\" [size 28, bold]; -- the title"),
        "{}",
        saves[0]
    );
}

#[test]
fn undo_over_the_canvas_saves_again_and_a_change_on_disk_is_taken_in() {
    let mut w = opened();
    w.refresh_screen();
    let doc = w.screen_doc.as_mut().unwrap();
    doc.picked = vec![vec![0, 0, 2]];
    assert!(doc.delete_picked());
    w.save_screen();
    assert!(!saved(&w).last().unwrap().contains("Quit"));
    w.screen_undo(false);
    assert_eq!(saved(&w).last().unwrap(), MENU);
    // The code is checked again with the file as Solis saved it: nothing
    // to take in.
    w.set_program(Some(program(MENU)));
    w.refresh_screen();
    assert_eq!(w.screen_doc.as_ref().unwrap().text, MENU);
    // Someone else changes it.
    let theirs = MENU.replace("Quit", "Leave");
    w.set_program(Some(program(&theirs)));
    w.refresh_screen();
    assert_eq!(w.screen_doc.as_ref().unwrap().text, theirs);
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("changed on disk"))
    );
    // A screen gone from its file closes the canvas.
    let none = script::compile(&script::Code {
        files: vec![("main.lst".to_string(), "end\n".to_string())],
        lua: Vec::new(),
        notes: Vec::new(),
    })
    .unwrap()
    .unwrap();
    w.set_program(Some(Arc::new(none)));
    w.refresh_screen();
    assert!(w.screen_doc.is_none());
}

/// MENU's game with a Health component and a Pause event, Quit with an
/// empty { }.
fn tied_program(menu: &str) -> Arc<script::Compiled> {
    let main = "event Pause();\ncomponent Health {\n    has {\n        float hp = 20;\n        float maxHp = 20;\n    }\n}\npass Health, Pause;\nend\n";
    let code = script::Code {
        files: vec![
            ("main.lst".to_string(), main.to_string()),
            ("ui/pauseMenu.lst".to_string(), menu.to_string()),
        ],
        lua: Vec::new(),
        notes: Vec::new(),
    };
    match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}\n{menu}", problems[0]),
    }
}

/// The words of the title piece drawn.
fn title(w: &Workbench) -> String {
    let view = w.screen_doc.as_ref().unwrap().view.as_ref().unwrap();
    view.children[0].children[0].children[0].text.clone()
}

#[test]
fn a_tie_and_a_send_make_code_the_game_reads_and_preview_values_try_numbers() {
    let menu = MENU
        .replace(
            "fetch \"solis/screen\":Ui;",
            "fetch \"solis/screen\":Ui;\nfetch \"main\":Pause;",
        )
        .replace("button \"Quit\";", "button \"Quit\" {\n                }");
    let mut w = opened();
    w.set_program(Some(tied_program(&menu)));
    w.refresh_screen();
    let readable = crate::screen_ties::readable(w.program.as_deref(), &w.document.registry);
    let health = readable
        .iter()
        .find(|r| r.component == "Health")
        .expect("Health can be read")
        .clone();
    assert_eq!(health.module, "main");
    assert_eq!(health.reader, "health");
    assert!(
        health
            .fields
            .iter()
            .any(|(n, k, s)| n == "hp" && *k == crate::screen_ties::Kind::Number && *s == 20.0)
    );
    let events = &w.program.as_ref().unwrap().screens.as_ref().unwrap().events;
    assert_eq!(events, &[("Pause".to_string(), "main.lst".to_string())]);
    let doc = w.screen_doc.as_mut().unwrap();
    doc.picked = vec![vec![0, 0, 0]];
    assert!(doc.tie(&health, "hp", &crate::screen_ties::TieTo::Words));
    doc.picked = vec![vec![1, 0, 2]];
    assert_eq!(doc.sends(), Some(None));
    assert!(doc.set_sends("Pause", "main.lst"));
    // The game's own code reads what was written.
    let text = doc.text.clone();
    w.set_program(Some(tied_program(&text)));
    w.refresh_screen();
    assert_eq!(title(&w), "20.0", "the starting hp");
    let doc = w.screen_doc.as_mut().unwrap();
    doc.preview.insert(("Health".into(), "hp".into()), 7.0);
    doc.redraw = true;
    w.refresh_screen();
    assert_eq!(title(&w), "7.0", "a preview value");
    assert!(!w.screen_doc.as_ref().unwrap().redraw);
}
