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
fn with_clips() -> (Workbench, AssetId, AssetId) {
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
    actions
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::SaveFrames { text, .. } => Some(text.clone()),
            _ => None,
        })
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
    assert!(w.frames_editor.is_some());
    assert_eq!(w.frames_editor.as_ref().unwrap().clip, "wave", "the clip played first");
    let beside = |w: &Workbench, panel, other| {
        let path = w.dock.find_tab(&panel).expect("open");
        w.dock[path.surface][path.node]
            .tabs()
            .unwrap()
            .contains(&other)
    };
    assert!(beside(&w, Panel::Clips, Panel::Properties));
    assert!(beside(&w, Panel::Frames, Panel::Library));
    assert!(layout::is_complete(&w.dock), "they don't count as missing panels");
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(h.query_by_label("wave").is_some(), "the Clips tab lists it");
    assert!(h.query_by_label("Frame 1 of wave").is_some(), "the Frames tab shows it");
    // A saved layout never keeps them.
    let restored = layout::restore(Some(h.state().0.dock.clone()));
    assert!(restored.find_tab(&Panel::Clips).is_none());
    h.state_mut().0.close_frames();
    let w = &h.state().0;
    assert!(w.frames_editor.is_none());
    assert!(w.dock.find_tab(&Panel::Frames).is_none());
}

#[test]
fn a_change_is_saved_shown_on_sprites_at_once_and_undone_over_the_tabs() {
    let (mut w, _, clips) = with_clips();
    w.open_frames("walker.solframes");
    let editor = w.frames_editor.as_mut().unwrap();
    assert!(editor.set_speed(2.0));
    let mut actions = Vec::new();
    w.frames_tabs_done(
        crate::panels::FramesOut {
            changed: true,
            ..Default::default()
        },
        &mut actions,
    );
    let texts = saved(&actions);
    assert_eq!(texts.len(), 1);
    assert!(texts[0].contains("clip1 = [\"wave\", 2.0"), "{}", texts[0]);
    // Every sprite playing the file sees it at once.
    let set = w.viewport.sheets.clips(clips).unwrap();
    assert_eq!(set.get("wave").unwrap().speed, 2.0);
    // Undo over the tabs undoes the file's change, and saves it.
    w.frames_hovered = true;
    w.frames_undo(false, true);
    let undone = saved(&w.outbox);
    assert!(undone.last().unwrap().contains("clip1 = [\"wave\", 1.0"));
    assert_eq!(
        w.viewport.sheets.clips(clips).unwrap().get("wave").unwrap().speed,
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
    w.library_request(LibraryRequest::MakeClipsFile("crops.aseprite".into()), &mut actions);
    let made = saved(&w.outbox);
    assert_eq!(made.len(), 1);
    assert!(made[0].contains(&format!("sheet = \"{crops}\"")));
    assert_eq!(
        w.frames_editor.as_ref().unwrap().path,
        "crops.solframes",
        "opened beside it"
    );
    // One that's there already is just opened.
    w.outbox.clear();
    w.library_request(LibraryRequest::MakeClipsFile("walker.aseprite".into()), &mut actions);
    assert!(saved(&w.outbox).is_empty());
    assert_eq!(w.frames_editor.as_ref().unwrap().path, "walker.solframes");
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
    assert!(w.frames_editor.is_none());
    assert!(
        w.console
            .lines()
            .iter()
            .any(|l| l.text.contains("Couldn't open walker.solframes: line 3"))
    );
}
