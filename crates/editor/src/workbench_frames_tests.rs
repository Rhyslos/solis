//! The Clips and Frames tabs in the whole editor: opened from the Library
//! and from a sheet's Properties, changes saved and shown at once on the
//! sprites, undo over the tabs, closing, and the layout without them.

use super::*;
use crate::library::{LibraryItem, LibraryKind, LibraryRequest, ProjectInfo};
use crate::sprite_sheets::SheetState;
use crate::test_support::*;
use egui_kittest::kittest::Queryable;
use reflect::AssetId;

/// A workbench with a project holding `walker.aseprite` (the test sheet:
/// two frames, no tags) and `walker.solframes` playing it, with a clip
/// made here, "wave", of both frames.
pub(crate) fn with_clips() -> (Workbench, AssetId, AssetId) {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    let (sheet, clips) = (AssetId::random(), AssetId::random());
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
    ]);
    let file = animation::FramesFile {
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
        FramesState::Read(Arc::new(file), Vec::new()),
    );
    sheets.paths.insert(sheet, "walker.aseprite".into());
    sheets.paths.insert(clips, "walker.solframes".into());
    w.set_sprite_sheets(sheets);
    (w, sheet, clips)
}

fn saved(actions: &[WorkbenchAction]) -> Vec<String> {
    saved_files(actions, |f| matches!(f, crate::OwnFile::Frames(_)))
        .into_iter()
        .map(|(_, text)| text)
        .collect()
}

#[test]
fn opening_a_clips_file_brings_its_tabs_and_closing_takes_them_away() {
    let (mut w, _, _) = with_clips();
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::Open(LibraryKind::SpriteClips, "walker.solframes".into()),
        &mut actions,
    );
    assert!(w.open.frames_editor.is_some());
    assert_eq!(
        w.open.frames_editor.as_ref().unwrap().clip,
        "wave",
        "the clip played first"
    );
    let beside = |w: &Workbench, panel, other| {
        let path = w.dock.find_tab(&panel).expect("open");
        w.dock[path.surface][path.node]
            .tabs()
            .unwrap()
            .contains(&other)
    };
    assert!(beside(&w, Panel::Clips, Panel::Properties));
    assert!(beside(&w, Panel::Frames, Panel::Library));
    assert!(
        layout::is_complete(&w.dock),
        "they don't count as missing panels"
    );
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(h.query_by_label("wave").is_some(), "the Clips tab lists it");
    assert!(
        h.query_by_label("Frame 1 of wave").is_some(),
        "the Frames tab shows it"
    );
    // A saved layout never keeps them.
    let restored = layout::restore(Some(h.state().0.dock.clone()));
    assert!(restored.find_tab(&Panel::Clips).is_none());
    h.state_mut().0.close_frames();
    let w = &h.state().0;
    assert!(w.open.frames_editor.is_none());
    assert!(w.dock.find_tab(&Panel::Frames).is_none());
}

#[test]
fn a_change_is_saved_shown_on_sprites_at_once_and_undone_over_the_tabs() {
    let (mut w, _, clips) = with_clips();
    w.open_frames("walker.solframes");
    let editor = w.open.frames_editor.as_mut().unwrap();
    assert!(editor.set_speed(2.0));
    let mut actions = Vec::new();
    w.frames_tabs_done(
        crate::panels::FramesOut {
            changed: true,
            ..Default::default()
        },
        &mut actions,
    );
    let texts = saved(&w.open.outbox);
    assert_eq!(texts.len(), 1);
    assert!(texts[0].contains("clip1 = [\"wave\", 2.0"), "{}", texts[0]);
    // Every sprite playing the file sees it at once.
    let set = w.viewport.sheets.clips(clips).unwrap();
    assert_eq!(set.get("wave").unwrap().speed, 2.0);
    // Undo over the tabs undoes the file's change, and saves it.
    w.frames_undo(false, true);
    let undone = saved(&w.open.outbox);
    assert!(undone.last().unwrap().contains("clip1 = [\"wave\", 1.0"));
    assert_eq!(
        w.viewport
            .sheets
            .clips(clips)
            .unwrap()
            .get("wave")
            .unwrap()
            .speed,
        1.0
    );
}

#[test]
fn a_sheet_s_properties_make_its_clips_file() {
    let (mut w, sheet, _) = with_clips();
    // A second sheet, with no clips file yet.
    let crops = AssetId::random();
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.by_path.insert(
        "crops.aseprite".into(),
        SheetState::Read(Arc::new(crate::sprite_sheets::tests::test_sheet())),
    );
    sheets.paths.insert(crops, "crops.aseprite".into());
    w.set_sprite_sheets(sheets);
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::MakeClipsFile("crops.aseprite".into()),
        &mut actions,
    );
    let made = saved(&w.open.outbox);
    assert_eq!(made.len(), 1);
    assert!(made[0].contains(&format!("sheet = \"{crops}\"")));
    assert_eq!(
        w.open.frames_editor.as_ref().unwrap().path,
        "crops.solframes",
        "opened beside it"
    );
    // One that's there already is just opened.
    w.open.outbox.clear();
    w.library_request(
        LibraryRequest::MakeClipsFile("walker.aseprite".into()),
        &mut actions,
    );
    assert!(saved(&w.open.outbox).is_empty());
    assert_eq!(
        w.open.frames_editor.as_ref().unwrap().path,
        "walker.solframes"
    );
    let _ = sheet;
}

#[test]
fn a_damaged_clips_file_isn_t_opened_and_says_why() {
    let (mut w, _, _) = with_clips();
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.frames.insert(
        "walker.solframes".into(),
        FramesState::Failed("line 3: a clip line should be …".into()),
    );
    w.set_sprite_sheets(sheets);
    w.open_frames("walker.solframes");
    assert!(w.open.frames_editor.is_none());
    assert!(
        w.console
            .lines()
            .iter()
            .any(|l| l.text.contains("Couldn't open walker.solframes: line 3"))
    );
}

/// BUG-20: how many times a clip plays, dragged for ten frames in the
/// Clips tab, is one undo step and one save, made when the mouse is let
/// go.
#[test]
fn a_dragged_number_in_the_clips_tab_is_one_step_and_one_save() {
    let (mut w, _, _) = with_clips();
    w.open_frames("walker.solframes");
    let mut h = harness(w);
    h.run();
    h.run();
    let saves = |h: &egui_kittest::Harness<'static, State>| saved(&h.state().1.actions).len();
    for (label, by) in [("Plays", 30.0), ("Speed", 12.0)] {
        h.state_mut().1.actions.clear();
        let field = number_field_beside(&h, label);
        h.hover_at(field.center());
        h.run();
        h.drag_at(field.center());
        h.run();
        for frame in 1..=10 {
            h.hover_at(field.center() + egui::vec2(by, 0.0) * frame as f32);
            h.run();
            assert_eq!(saves(&h), 0, "{label}: nothing saved mid-drag");
        }
        h.drop_at(field.center() + egui::vec2(by, 0.0) * 10.0);
        h.run();
        assert_eq!(saves(&h), 1, "{label}: saved once, when let go");
    }
    let editor = h.state_mut().0.open.frames_editor.as_mut().unwrap();
    let wave = &editor.file.clips[0];
    assert!(wave.repeats > 1, "{} times", wave.repeats);
    assert!(wave.speed > 1.5, "{} times as fast", wave.speed);
    // One step a drag.
    assert!(editor.undo());
    assert!(editor.undo());
    assert!(!editor.undo());
}

/// The clips file open in its tabs, and the project's files as the app
/// has them.
fn with_clips_open() -> (Workbench, SpriteSheets) {
    let (mut w, _, _) = with_clips();
    let store = (*w.viewport.sheets).clone();
    w.open_frames(CLIPS);
    (w, store)
}

const CLIPS: &str = "walker.solframes";

/// The open clips file, as its editor has it.
fn open_file(w: &Workbench) -> animation::FramesFile {
    w.open.frames_editor.as_ref().unwrap().file.clone()
}

/// The clips file changed outside Solis: another clip added, "blink".
fn changed_by_hand(
    store: &mut SpriteSheets,
    file: &animation::FramesFile,
) -> animation::FramesFile {
    let mut by_hand = file.clone();
    let mut blink = by_hand.clips[0].clone();
    blink.name = "blink".into();
    by_hand.clips.push(blink);
    store.put(CLIPS, crate::OwnFile::Frames(Arc::new(by_hand.clone())));
    by_hand
}

/// The review, BUG-08: the Clips and Frames tabs never looked at their
/// file again once it was open, unlike the Animator, Tileset and Atlas
/// Crop tabs. A file changed on disk (a Git pull) stayed as it had been
/// on screen.
#[test]
fn a_clips_file_changed_on_disk_is_taken_in_as_one_undo_step() {
    let (mut w, mut store) = with_clips_open();
    let before = open_file(&w);
    let by_hand = changed_by_hand(&mut store, &before);
    w.set_sprite_sheets(store.clone());
    assert_eq!(open_file(&w), by_hand, "shown as it is now");
    let told: Vec<&str> = w
        .console
        .lines()
        .iter()
        .filter(|l| l.text.contains("on disk"))
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(
        told,
        [
            "walker.solframes changed on disk, so the Clips and Frames tabs show it as it is now \
          (Ctrl+Z over them undoes that)."
        ]
    );
    // Read again unchanged: nothing more to take in or say.
    w.set_sprite_sheets(store.clone());
    assert_eq!(
        w.console
            .lines()
            .iter()
            .filter(|l| l.text.contains("on disk"))
            .count(),
        1
    );
    // One undo step: Ctrl+Z over the tabs puts the file back as it was
    // here, and saves that.
    assert!(
        saved(&w.open.outbox).is_empty(),
        "taking it in saves nothing"
    );
    w.frames_undo(false, true);
    assert_eq!(open_file(&w), before);
    assert_eq!(
        saved(&w.open.outbox),
        [animation::frames_file::write(&before)]
    );
    w.frames_undo(false, true);
    assert_eq!(open_file(&w), before, "one step, not two");
}

/// BUG-08's failure: the next change in the tab after the file changed on
/// disk saved the copy the tab still had over the new file, without a
/// word.
#[test]
fn a_change_after_the_file_changed_on_disk_never_writes_the_older_copy_over_it() {
    let (mut w, mut store) = with_clips_open();
    let by_hand = changed_by_hand(&mut store, &open_file(&w));
    w.set_sprite_sheets(store.clone());
    // A change in the Clips tab: the first clip twice as fast.
    let editor = w.open.frames_editor.as_mut().unwrap();
    assert!(editor.set_speed(2.0));
    w.save_frames();
    let mut wanted = by_hand;
    wanted.clips[0].speed = 2.0;
    let texts = saved(&w.open.outbox);
    assert_eq!(texts, [animation::frames_file::write(&wanted)]);
    assert!(texts[0].contains("blink"), "what was added by hand is kept");
    // And Ctrl+Z after such a change undoes it alone, the file by hand
    // still there.
    w.frames_undo(false, true);
    assert!(saved(&w.open.outbox)[1].contains("blink"));
}

/// BUG-08: a clips file deleted while open stayed open, and the next
/// change made the file again where it had been.
#[test]
fn a_clips_file_gone_from_the_project_closes_its_tabs() {
    let (mut w, mut store) = with_clips_open();
    store.frames.clear();
    w.set_sprite_sheets(store);
    assert!(w.open.frames_editor.is_none());
    for panel in Panel::FRAMES_TABS {
        assert!(w.dock.find_tab(&panel).is_none(), "{panel:?}");
    }
    let last = w.console.lines().last().unwrap();
    assert_eq!(
        last.text,
        "walker.solframes is gone, so the Clips and Frames tabs closed."
    );
    // Nothing is left to make the file again with.
    w.frames_undo(false, true);
    w.save_frames();
    assert!(saved(&w.open.outbox).is_empty());
}

/// BUG-08: opening the clips file that's already open made its editor
/// anew, and its undo steps were gone.
#[test]
fn opening_the_clips_file_already_open_keeps_its_editor_and_undo_steps() {
    let (mut w, _) = with_clips_open();
    let before = open_file(&w);
    let editor = w.open.frames_editor.as_mut().unwrap();
    assert!(editor.set_speed(2.0));
    editor.zoom = 500.0;
    w.save_frames();
    // Double-clicked in the Library again, with a tab closed meanwhile.
    layout::close(&mut w.dock, Panel::Frames);
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::Open(LibraryKind::SpriteClips, CLIPS.into()),
        &mut actions,
    );
    for panel in Panel::FRAMES_TABS {
        assert!(w.dock.find_tab(&panel).is_some(), "{panel:?} is open");
    }
    let editor = w.open.frames_editor.as_ref().unwrap();
    assert_eq!(editor.zoom, 500.0, "the same editor");
    assert_eq!(editor.can_undo_redo(), (true, false), "with its undo step");
    w.frames_undo(false, true);
    assert_eq!(open_file(&w), before);
    // Another clips file takes its place, as before.
    let mut store = (*w.viewport.sheets).clone();
    store.put("other.solframes", crate::OwnFile::Frames(Arc::default()));
    w.set_sprite_sheets(store);
    w.open_frames("other.solframes");
    let editor = w.open.frames_editor.as_ref().unwrap();
    assert_eq!(editor.path, "other.solframes");
    assert_eq!(editor.can_undo_redo(), (false, false));
}
