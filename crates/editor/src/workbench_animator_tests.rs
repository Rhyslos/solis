//! The Animator tab in the whole editor: opened from the Library, drawn
//! as the mock-up has it, a change saved and run at once, undo over the
//! tab, a new animator, a damaged file, a file changed on disk, and the
//! animator running in the game with the selected entity's state lit.

use super::*;
use crate::library::{LibraryItem, LibraryKind, LibraryRequest, ProjectInfo};
use crate::sprite_sheets::{FramesState, SheetState};
use crate::test_support::*;
use animation::{
    AnimatorFile, Condition, Facing, Parameter, ParameterKind, Plays, State, Transition,
};
use area::builtin::{Animator, Sprite, Transform};
use egui_kittest::kittest::Queryable;
use reflect::{AssetId, AssetRef, Id};

fn id(n: u64) -> Id {
    Id::from_raw(n).unwrap()
}

/// Idle and Wave (the test sheet's clip "wave", over and over), moving
/// to go between them.
fn farmer(sheet: AssetId) -> AnimatorFile {
    let state = |n, name: &str, x| State {
        id: id(n),
        name: name.into(),
        place: [x, 0.0],
        plays: Plays::Clip(if n == 1 { String::new() } else { "wave".into() }),
        speed: 1.0,
        speed_by: String::new(),
    };
    AnimatorFile {
        sheet: Some(sheet),
        entry: Some(id(1)),
        parameters: vec![Parameter {
            id: id(10),
            name: "moving".into(),
            kind: ParameterKind::Switch(false),
        }],
        states: vec![state(1, "Idle", 0.0), state(2, "Wave", 300.0)],
        transitions: vec![
            Transition {
                id: id(20),
                from: Some(id(1)),
                to: id(2),
                when: vec![Condition::Is("moving".into())],
            },
            Transition {
                id: id(21),
                from: Some(id(2)),
                to: id(1),
                when: vec![Condition::Not("moving".into())],
            },
        ],
        ..AnimatorFile::default()
    }
}

/// A workbench with a project holding the test sheet (`walker.aseprite`),
/// a clips file with the clip "wave" (`walker.solframes`) and
/// `farmer.solanim`.
fn with_animator() -> (Workbench, AssetId, AssetId) {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    let (sheet, clips, anim) = (AssetId::random(), AssetId::random(), AssetId::random());
    let item = |path: &str, kind, id| LibraryItem {
        id: Some(id),
        kind,
        name: crate::sprite_sheets::name_of(path),
        path: path.into(),
        thumbnail: None,
    };
    w.set_library(vec![
        item("walker.aseprite", LibraryKind::SpriteSheet, sheet),
        item("walker.solframes", LibraryKind::SpriteClips, clips),
        item("farmer.solanim", LibraryKind::Animator, anim),
    ]);
    let frames = animation::FramesFile {
        sheet: Some(sheet),
        first: "wave".into(),
        tags: Vec::new(),
        clips: vec![animation::OwnClip {
            name: "wave".into(),
            speed: 1.0,
            direction: animation::Direction::Forward,
            repeats: 0,
            frames: (0..2)
                .map(|f| animation::ClipFrame {
                    frame: f,
                    ms: 100,
                    events: Vec::new(),
                })
                .collect(),
        }],
    };
    let mut sheets = SpriteSheets::default();
    sheets.by_path.insert(
        "walker.aseprite".into(),
        SheetState::Read(Arc::new(crate::sprite_sheets::tests::test_sheet())),
    );
    sheets.frames.insert(
        "walker.solframes".into(),
        FramesState::Read(Arc::new(frames), Vec::new()),
    );
    sheets.animators.insert(
        "farmer.solanim".into(),
        AnimatorState::Read(Arc::new(farmer(clips)), Vec::new()),
    );
    sheets.paths.insert(sheet, "walker.aseprite".into());
    sheets.paths.insert(clips, "walker.solframes".into());
    sheets.paths.insert(anim, "farmer.solanim".into());
    w.set_sprite_sheets(sheets);
    (w, clips, anim)
}

fn saved(actions: &[WorkbenchAction]) -> Vec<String> {
    actions
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::SaveAnimator { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn it_opens_first_beside_the_viewport_as_drawn_in_the_mock_up() {
    let (mut w, _, _) = with_animator();
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::Open(LibraryKind::Animator, "farmer.solanim".into()),
        &mut actions,
    );
    assert!(w.animator_editor.is_some());
    let path = w.dock.find_tab(&Panel::Animator).expect("open");
    let tabs = w.dock[path.surface][path.node].tabs().unwrap().to_vec();
    assert_eq!(tabs[0], Panel::Animator, "first in its group");
    assert!(tabs.contains(&Panel::Viewport));
    assert!(layout::is_complete(&w.dock));
    let mut h = harness(w);
    h.run();
    h.run();
    for label in [
        "State Idle",
        "State Wave",
        "State Any state",
        "Transition moving",
        "Transition not moving",
        "farmer.solanim",
        "Graph canvas",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    // Properties shows the animator (nothing picked): its sheet.
    assert!(
        h.query_by_label_contains("2 states · 2 transitions · 1 parameters")
            .is_some()
    );
    // A saved layout never keeps it.
    let restored = layout::restore(Some(h.state().0.dock.clone()));
    assert!(restored.find_tab(&Panel::Animator).is_none());
    // Its document tab's × closes it.
    h.get_by_label("Close farmer.solanim").click();
    h.run();
    h.run();
    let w = &h.state().0;
    assert!(w.animator_editor.is_none());
    assert!(w.dock.find_tab(&Panel::Animator).is_none());
}

#[test]
fn clicking_a_state_and_an_arrow_shows_them_in_properties() {
    let (mut w, _, _) = with_animator();
    w.open_animator("farmer.solanim");
    let mut h = harness(w);
    h.run();
    h.run();
    h.get_by_label("State Wave").click();
    h.run();
    h.run();
    assert!(h.query_by_label("Use tool").is_none());
    assert!(h.query_by_label_contains("Clicking an arrow").is_none());
    assert!(
        h.query_by_label_contains("Click an arrow to set its conditions")
            .is_some(),
        "the state's Properties"
    );
    h.get_by_label("Transition moving").click();
    h.run();
    h.run();
    assert!(
        h.query_by_label_contains("Taken when all of these hold")
            .is_some()
    );
    assert!(h.query_by_label("Remove condition 1").is_some());
}

#[test]
fn a_change_is_saved_run_at_once_and_undone_over_the_tab() {
    let (mut w, _, anim) = with_animator();
    w.open_animator("farmer.solanim");
    let editor = w.animator_editor.as_mut().unwrap();
    assert!(editor.rename_state(id(2), "Hoe"));
    let mut actions = Vec::new();
    w.frames_tabs_done(
        crate::panels::FramesOut {
            animator_changed: true,
            ..Default::default()
        },
        &mut actions,
    );
    let texts = saved(&actions);
    assert_eq!(texts.len(), 1);
    assert!(texts[0].contains("name = \"Hoe\";"), "{}", texts[0]);
    // Ready to run at once, for the game too.
    let running = w.viewport.sheets.animator_set.get(&anim).unwrap();
    assert_eq!(running.state_name(1), "Hoe");
    // Undo over the tab undoes the file's change, and saves it.
    w.animator_hovered = true;
    w.animator_undo(false, true);
    assert!(
        saved(&w.outbox)
            .last()
            .unwrap()
            .contains("name = \"Wave\";")
    );
    // The app reading back Solis's own save changes nothing.
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.animators.insert(
        "farmer.solanim".into(),
        AnimatorState::Read(
            Arc::new(w.animator_editor.as_ref().unwrap().file.clone()),
            Vec::new(),
        ),
    );
    w.set_sprite_sheets(sheets);
    assert!(
        w.animator_editor.as_mut().unwrap().redo(),
        "its redo is still there"
    );
}

#[test]
fn a_file_changed_on_disk_is_taken_in_and_one_gone_closes() {
    let (mut w, clips, _) = with_animator();
    w.open_animator("farmer.solanim");
    let mut sheets = (*w.viewport.sheets).clone();
    let mut by_hand = farmer(clips);
    by_hand.states[0].name = "Rest".into();
    sheets.animators.insert(
        "farmer.solanim".into(),
        AnimatorState::Read(Arc::new(by_hand), Vec::new()),
    );
    w.set_sprite_sheets(sheets.clone());
    assert_eq!(
        w.animator_editor.as_ref().unwrap().file.states[0].name,
        "Rest"
    );
    assert!(
        w.console
            .lines()
            .iter()
            .any(|l| l.text.contains("changed on disk"))
    );
    sheets.animators.clear();
    w.set_sprite_sheets(sheets);
    assert!(w.animator_editor.is_none());
    assert!(w.dock.find_tab(&Panel::Animator).is_none());
}

#[test]
fn a_new_animator_is_made_saved_and_opened_and_a_damaged_one_says_why() {
    let (mut w, _, _) = with_animator();
    let mut actions = Vec::new();
    w.library_request(LibraryRequest::NewAnimator("sprites".into()), &mut actions);
    let made = saved(&w.outbox);
    assert_eq!(made.len(), 1);
    assert!(made[0].contains("name = \"Idle\";"));
    let editor = w.animator_editor.as_ref().unwrap();
    assert_eq!(editor.path, "sprites/animator.solanim");
    assert_eq!(editor.file.entry_state().unwrap().name, "Idle");
    // Not on disk yet: a read without it doesn't close it.
    let sheets = (*w.viewport.sheets).clone();
    let mut without = sheets.clone();
    without.animators.remove("sprites/animator.solanim");
    w.set_sprite_sheets(without);
    assert!(w.animator_editor.is_some());
    // A second one gets a free name.
    w.set_sprite_sheets(sheets);
    w.new_animator("sprites");
    assert_eq!(
        w.animator_editor.as_ref().unwrap().path,
        "sprites/animator 2.solanim"
    );
    // A damaged file isn't opened.
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.animators.insert(
        "farmer.solanim".into(),
        AnimatorState::Failed("line 4: the state block isn't closed".into()),
    );
    w.set_sprite_sheets(sheets);
    w.close_animator();
    w.open_animator("farmer.solanim");
    assert!(w.animator_editor.is_none());
    assert!(
        w.console
            .lines()
            .iter()
            .any(|l| l.text.contains("Couldn't open farmer.solanim: line 4"))
    );
}

#[test]
fn it_runs_in_the_game_and_the_selected_entity_s_state_is_live() {
    let (mut w, clips, anim) = with_animator();
    let walker = w.spawn_selected(
        area::EntityData::new("Walker")
            .with(&Transform::default())
            .with(&Sprite {
                sheet: Some(AssetRef {
                    asset: clips,
                    part: None,
                }),
                ..Sprite::default()
            })
            .with(&Animator {
                animator: Some(AssetRef {
                    asset: anim,
                    part: None,
                }),
                ..Animator::default()
            }),
    );
    w.document.selection.select_only(walker);
    w.open_animator("farmer.solanim");
    w.play();
    w.run_game(0.05);
    let world = w.world().unwrap();
    assert_eq!(world.get::<Animator>(walker).unwrap().state, "Idle");
    let (state, values) = w.live_animator().expect("it runs this file");
    assert_eq!(state, "Idle");
    assert_eq!(values, [("moving".to_owned(), ParamValue::Switch(false))]);
    // Set as game code will: the next tick takes it.
    w.playing
        .as_mut()
        .unwrap()
        .world
        .set_animator_value(walker, "moving", ParamValue::Switch(true))
        .unwrap();
    w.run_game(0.05);
    assert_eq!(w.live_animator().unwrap().0, "Wave");
    assert_eq!(
        w.world().unwrap().get::<Sprite>(walker).unwrap().clip,
        "wave"
    );
    // The tab shows the game's values (read-only) and lights its state
    // (Play brought the Game panel to the front of the group).
    layout::bring_to_front(&mut w.dock, Panel::Animator);
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(
        h.query_by_label_contains("The selected entity's values in the running game")
            .is_some()
    );
    let _ = Facing::Down;
}

#[test]
fn the_preview_s_values_are_set_beside_the_graph() {
    let (mut w, _, _) = with_animator();
    w.open_animator("farmer.solanim");
    w.animator_editor
        .as_mut()
        .unwrap()
        .add_parameter(ParameterKind::Trigger);
    let mut h = harness(w);
    h.run();
    h.run();
    h.get_by_label("moving").click();
    h.run();
    let editor = h.state().0.animator_editor.as_ref().unwrap();
    assert_eq!(editor.preview.values[0], ParamValue::Switch(true));
    h.get_by_label("Fire use").click();
    h.run();
    // Fired, then taken at the preview's next tick (nothing uses it).
    for _ in 0..10 {
        h.run();
    }
    let editor = h.state().0.animator_editor.as_ref().unwrap();
    assert_eq!(editor.preview.values[1], ParamValue::Trigger(false));
    assert_eq!(
        editor.running.state_name(editor.preview.state),
        "Wave",
        "moving"
    );
}
