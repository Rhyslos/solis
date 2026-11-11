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
    opened_with(MENU)
}

/// A workbench with a project whose `ui/pauseMenu.lst` is `menu`, the
/// screen open.
pub(crate) fn opened_with(menu: &str) -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    w.set_program(Some(program(menu)));
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::Open(LibraryKind::Code, "ui/pauseMenu.lst".into()),
        &mut actions,
    );
    w
}

fn saved(w: &Workbench) -> Vec<String> {
    w.open
        .outbox
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
    let doc = w.open.screen_doc.as_ref().unwrap();
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
    w.open.screen_doc.as_mut().unwrap().picked = vec![vec![0, 0, 0]];
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
    let doc = w.open.screen_doc.as_mut().unwrap();
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
    assert_eq!(w.open.screen_doc.as_ref().unwrap().text, MENU);
    // Someone else changes it.
    let theirs = MENU.replace("Quit", "Leave");
    w.set_program(Some(program(&theirs)));
    w.refresh_screen();
    assert_eq!(w.open.screen_doc.as_ref().unwrap().text, theirs);
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
    assert!(w.open.screen_doc.is_none());
}

/// The review, BUG-09: the code is checked on a worker, so the program
/// from a check that began before a save can arrive after it. Its text of
/// the screen's file is then an older one of Solis's own, not a change
/// made on disk: it was reported as one, and took the newer change back.
#[test]
fn an_older_text_of_its_own_arriving_late_is_no_change_on_disk() {
    let mut w = opened();
    w.refresh_screen();
    let doc = w.open.screen_doc.as_mut().unwrap();
    doc.picked = vec![vec![0, 0, 2]];
    assert!(doc.delete_picked());
    w.save_screen();
    let first = w.open.screen_doc.as_ref().unwrap().text.clone();
    let doc = w.open.screen_doc.as_mut().unwrap();
    doc.picked = vec![vec![0, 0, 1]];
    assert!(doc.delete_picked());
    w.save_screen();
    let second = w.open.screen_doc.as_ref().unwrap().text.clone();
    assert!(first != MENU && second != first);
    // The checks finish late, oldest first: the file as it was opened,
    // then as first saved, then as it is.
    for text in [MENU, &first, &second] {
        w.set_program(Some(program(text)));
        w.refresh_screen();
        assert_eq!(w.open.screen_doc.as_ref().unwrap().text, second);
    }
    assert!(w.log_lines().iter().all(|l| !l.text.contains("on disk")));
    // Its undo steps are the two changes, and no more.
    let doc = w.open.screen_doc.as_mut().unwrap();
    assert!(doc.undo() && doc.undo());
    assert_eq!(doc.text, MENU);
    assert!(!doc.undo());
    // Once a change made on disk is taken in, its own older texts are no
    // longer the file's: one of them read after that is a change.
    w.set_program(Some(program(&MENU.replace("Quit", "Leave"))));
    w.refresh_screen();
    w.set_program(Some(program(&first)));
    w.refresh_screen();
    assert_eq!(w.open.screen_doc.as_ref().unwrap().text, first);
    let told = |l: &&crate::LogLine| l.text.contains("changed on disk");
    assert_eq!(w.log_lines().iter().filter(told).count(), 2);
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
    let view = w.open.screen_doc.as_ref().unwrap().view.as_ref().unwrap();
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
    let readable =
        crate::screen_ties::readable(w.open.program.as_deref(), &w.open.document.registry);
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
    let events = &w
        .open
        .program
        .as_ref()
        .unwrap()
        .screens
        .as_ref()
        .unwrap()
        .events;
    assert_eq!(events, &[("Pause".to_string(), "main.lst".to_string())]);
    let doc = w.open.screen_doc.as_mut().unwrap();
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
    let doc = w.open.screen_doc.as_mut().unwrap();
    doc.preview.insert(("Health".into(), "hp".into()), 7.0);
    doc.redraw = true;
    w.refresh_screen();
    assert_eq!(title(&w), "7.0", "a preview value");
    assert!(!w.open.screen_doc.as_ref().unwrap().redraw);
}

/// A workbench with a clips file and a screen both open, in the UI
/// workspace: Clips, Frames, Canvas and Elements all there.
fn with_clips_and_a_screen() -> Workbench {
    let (mut w, _, _) = crate::workbench_frames::tests::with_clips();
    w.set_program(Some(program(MENU)));
    w.open_frames("walker.solframes");
    assert!(w.open_screen("ui/pauseMenu.lst", None));
    for panel in [Panel::Clips, Panel::Frames, Panel::Canvas, Panel::Elements] {
        assert!(w.dock.find_tab(&panel).is_some(), "{panel:?} is open");
    }
    w
}

/// BUG-26: the × on Canvas or Elements closes the screen and both its
/// tabs, and leaves the clips file open with its undo steps.
#[test]
fn the_x_on_canvas_or_elements_closes_the_screen_and_not_the_clips_file() {
    for closed in [Panel::Canvas, Panel::Elements] {
        let mut w = with_clips_and_a_screen();
        assert!(w.open.frames_editor.as_mut().unwrap().set_speed(2.0));
        let mut h = harness(w);
        click_the_x(&mut h, closed);
        let w = &mut h.state_mut().0;
        assert!(w.open.screen_doc.is_none(), "{closed:?}: the screen closed");
        for panel in [Panel::Canvas, Panel::Elements] {
            assert!(
                w.dock.find_tab(&panel).is_none(),
                "{closed:?}: {panel:?} went with it"
            );
        }
        let clips = w
            .open
            .frames_editor
            .as_mut()
            .expect("the clips file is open");
        assert!(clips.undo(), "{closed:?}: with its undo steps");
        for panel in Panel::FRAMES_TABS {
            assert!(w.dock.find_tab(&panel).is_some(), "{closed:?}: {panel:?}");
        }
        // Gone for good: no tab comes back with the workspace, and the
        // file opens on the canvas again when asked.
        w.switch_workspace(Workspace::Level);
        w.switch_workspace(Workspace::Ui);
        assert!(w.dock.find_tab(&Panel::Canvas).is_none());
        assert!(w.dock.find_tab(&Panel::Elements).is_none());
        let mut actions = Vec::new();
        w.library_request(
            LibraryRequest::Open(LibraryKind::Code, "ui/pauseMenu.lst".into()),
            &mut actions,
        );
        assert!(w.open.screen_doc.is_some());
        assert!(actions.is_empty(), "on the canvas, not in another program");
    }
}

/// The × on Clips or Frames closes the clips file and both its tabs, and
/// leaves the screen open with its undo steps.
#[test]
fn the_x_on_clips_or_frames_closes_the_clips_file_and_not_the_screen() {
    for closed in Panel::FRAMES_TABS {
        let mut w = with_clips_and_a_screen();
        let doc = w.open.screen_doc.as_mut().unwrap();
        doc.picked = vec![vec![0, 0, 0]];
        assert!(doc.set_style("size", Some("40")));
        let mut h = harness(w);
        click_the_x(&mut h, closed);
        let w = &mut h.state_mut().0;
        assert!(
            w.open.frames_editor.is_none(),
            "{closed:?}: the file closed"
        );
        for panel in Panel::FRAMES_TABS {
            assert!(
                w.dock.find_tab(&panel).is_none(),
                "{closed:?}: {panel:?} went with it"
            );
        }
        let doc = w.open.screen_doc.as_mut().expect("the screen is open");
        assert!(doc.undo(), "{closed:?}: with its undo steps");
        for panel in [Panel::Canvas, Panel::Elements] {
            assert!(w.dock.find_tab(&panel).is_some(), "{closed:?}: {panel:?}");
        }
    }
}

/// Closing asks nothing and loses nothing: a change is saved the frame
/// it's made, so what's on disk is what was on the canvas.
#[test]
fn a_screen_closed_was_saved_as_it_was_changed() {
    let mut w = with_clips_and_a_screen();
    let doc = w.open.screen_doc.as_mut().unwrap();
    doc.picked = vec![vec![0, 0, 0]];
    assert!(doc.set_style("size", Some("40")));
    let text = doc.text.clone();
    // The change and the × in the same frame: saved, then closed.
    let mut actions = Vec::new();
    w.frames_tabs_done(
        crate::panels::FramesOut {
            screen_changed: true,
            screen_closed: true,
            ..Default::default()
        },
        &mut actions,
    );
    assert_eq!(saved(&w), vec![text]);
    assert!(w.open.screen_doc.is_none());
    assert!(w.open.unsaved_prompt.is_none(), "nothing asked");
}

/// A screen opened while already in the UI workspace gets both its tabs.
#[test]
fn a_screen_opened_in_the_ui_workspace_gets_elements_too() {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    w.set_program(Some(program(MENU)));
    w.switch_workspace(Workspace::Ui);
    assert!(w.dock.find_tab(&Panel::Elements).is_none());
    assert!(w.open_screen("ui/pauseMenu.lst", None));
    assert!(w.dock.find_tab(&Panel::Canvas).is_some());
    assert!(w.dock.find_tab(&Panel::Elements).is_some());
}

/// A screen that stops being one closes with its tabs, wherever they are.
#[test]
fn a_file_that_isn_t_a_screen_any_more_takes_its_tabs_with_it() {
    let mut w = opened();
    w.refresh_screen();
    // Left in the UI workspace's layout while another is in use.
    w.switch_workspace(Workspace::Level);
    let code = script::Code {
        files: vec![("main.lst".to_string(), "end\n".to_string())],
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let plain = match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        _ => panic!("the code compiles"),
    };
    w.set_program(Some(plain));
    w.refresh_screen();
    assert!(w.open.screen_doc.is_none());
    w.switch_workspace(Workspace::Ui);
    assert!(w.dock.find_tab(&Panel::Canvas).is_none());
    assert!(w.dock.find_tab(&Panel::Elements).is_none());
}

/// The review, BUG-10: what the open screen can be tied to was kept for
/// "this build of the game's code", told by where the program was in
/// memory. A new build could be put where the last one was, and the old
/// list stayed.
#[test]
fn what_a_screen_can_be_tied_to_is_worked_out_again_for_each_build_of_the_code() {
    let mut w = opened_with(MENU);
    w.set_program(Some(tied_program(MENU)));
    let mut h = harness(w);
    h.run();
    let first = h.state().0.open.screen_game.as_ref().unwrap().1.clone();
    h.run();
    let kept = h.state().0.open.screen_game.as_ref().unwrap().1.clone();
    assert!(Arc::ptr_eq(&first, &kept), "once per build");
    // The build let go, then the code checked again: another build.
    h.state_mut().0.set_program(None);
    h.run();
    h.state_mut().0.set_program(Some(tied_program(MENU)));
    h.run();
    let again = h.state().0.open.screen_game.as_ref().unwrap().1.clone();
    assert!(!Arc::ptr_eq(&first, &again), "worked out again");
    h.run();
    let kept = h.state().0.open.screen_game.as_ref().unwrap().1.clone();
    assert!(Arc::ptr_eq(&again, &kept));
}

/// A game with a clock, and MENU's screen with a view that stops each
/// time it runs: it tries to change the clock, which a screen can't.
fn program_whose_view_stops() -> Arc<script::Compiled> {
    let main = "object Clock {\n    has { int hour = 6; }\n}\npass Clock;\nend\n";
    let menu = MENU
        .replace(
            "fetch \"solis/screen\":Ui;",
            "fetch \"solis/screen\":Ui;\nfetch \"main\":Clock;",
        )
        .replace(
            "text \"Paused\" [size 28];",
            "is true? then { Clock.hour = 99; }\n                text \"Paused\" [size 28];",
        );
    assert_ne!(menu, MENU);
    let code = script::Code {
        files: vec![
            ("main.lst".to_string(), main.to_string()),
            ("ui/pauseMenu.lst".to_string(), menu),
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

/// How many times this test has started the game's screens' code.
fn starts() -> u32 {
    crate::game_screens::STARTS.with(std::cell::Cell::get)
}

/// The review, BUG-11: a screen whose view stopped was drawn again on
/// every frame, starting the game's screens and running its view each
/// time, for the same build of the code and the same result.
#[test]
fn a_screen_whose_view_stops_is_tried_once_for_each_build_of_the_code() {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    w.set_program(Some(program_whose_view_stops()));
    assert!(w.open_screen("ui/pauseMenu.lst", None));
    let before = starts();
    let stopped = |w: &Workbench| {
        let doc = w.open.screen_doc.as_ref().unwrap();
        assert!(doc.view.is_none());
        let why = doc.view_problem.as_deref().expect("it says why");
        assert!(why.starts_with("The screen's view stopped: "), "{why}");
        assert!(why.contains("can't be changed here"), "{why}");
    };
    // Twenty frames of the whole editor: its code is started once.
    let mut h = harness(w);
    for _ in 0..20 {
        h.run();
    }
    assert_eq!(starts() - before, 1, "once, not once a frame");
    stopped(&h.state().0);
    // The code is checked again and comes out the same: another build,
    // tried once more, and no more.
    h.state_mut()
        .0
        .set_program(Some(program_whose_view_stops()));
    for _ in 0..20 {
        h.run();
    }
    assert_eq!(starts() - before, 2, "once for the new build");
    stopped(&h.state().0);
    // A preview value changed: the view is asked for again, once.
    h.state_mut().0.open.screen_doc.as_mut().unwrap().redraw = true;
    for _ in 0..20 {
        h.run();
    }
    assert_eq!(starts() - before, 3, "once when asked to draw again");
    stopped(&h.state().0);
    // The code has mistakes: said on the canvas, with nothing to run.
    h.state_mut().0.set_program(None);
    for _ in 0..20 {
        h.run();
    }
    assert_eq!(starts() - before, 3);
    let doc = h.state().0.open.screen_doc.as_ref().unwrap();
    assert!(doc.view_problem.as_deref().unwrap().contains("mistakes"));
    // Put right: drawn, once, and the message goes.
    h.state_mut().0.set_program(Some(program(MENU)));
    for _ in 0..20 {
        h.run();
    }
    assert_eq!(starts() - before, 4);
    let doc = h.state().0.open.screen_doc.as_ref().unwrap();
    assert!(doc.view.is_some() && doc.view_problem.is_none());
}
