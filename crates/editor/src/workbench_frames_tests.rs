//! The Clips and Frames tabs in the whole editor: opened from the Library
//! and from a sheet's Properties, changes saved and shown at once on the
//! sprites, undo over the tabs, closing, and the layout without them.

use super::*;
use crate::WorkbenchAction;
use crate::library::{LibraryItem, LibraryKind, LibraryRequest, ProjectInfo};
use crate::sprite_sheets::SheetState;
use crate::test_support::*;
use egui_kittest::kittest::Queryable;
use reflect::AssetId;

/// A workbench with a project holding `walker.aseprite` (the test sheet:
/// two frames, no tags) and `walker.clips.lst` playing it, with a clip
/// made here, "wave", of both frames.
pub(crate) fn with_clips() -> (Workbench, AssetId, AssetId) {
    let mut w = Workbench::new(info());
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
        item("walker.clips.lst", LibraryKind::SpriteClips, clips),
    ]);
    let file = animation::ClipsFile::from_frames(&animation::frames_file::FramesFile {
        sheet: Some(sheet),
        first: "wave".into(),
        tags: Vec::new(),
        clips: vec![animation::frames_file::OwnClip {
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
    });
    let mut sheets = SpriteSheets::default();
    sheets.by_path.insert(
        "walker.aseprite".into(),
        SheetState::Read(Arc::new(crate::sprite_sheets::tests::test_sheet())),
    );
    sheets.frames.insert(
        "walker.clips.lst".into(),
        FramesState::Read(Arc::new(file), Vec::new()),
    );
    sheets.paths.insert(sheet, "walker.aseprite".into());
    sheets.paths.insert(clips, "walker.clips.lst".into());
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
        LibraryRequest::Open(LibraryKind::SpriteClips, "walker.clips.lst".into()),
        &mut actions,
    );
    assert!(w.open.frames_editor.is_some());
    assert_eq!(
        w.open.frames_editor.as_ref().unwrap().clip,
        "wave",
        "the clip played first"
    );
    // The Animate tab, with its Frames shelf tab in front.
    assert_eq!(w.ribbon_tab(), crate::shell::RibbonTab::Animate);
    assert_eq!(w.bottom_tab(), Some(crate::shell::BottomTab::Frames));
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(
        h.query_by_label("Frame 1 of wave").is_some(),
        "the Frames tab shows it"
    );
    h.state_mut().0.show_panel(Panel::Clips);
    h.run();
    assert!(h.query_by_label("wave").is_some(), "the Clips tab lists it");
    h.state_mut().0.close_frames();
    let w = &h.state().0;
    assert!(w.open.frames_editor.is_none());
    assert!(!has_panel(w, Panel::Frames));
}

#[test]
fn a_change_is_saved_shown_on_sprites_at_once_and_undone_over_the_tabs() {
    let (mut w, _, clips) = with_clips();
    w.open_frames("walker.clips.lst");
    let editor = w.open.frames_editor.as_mut().unwrap();
    assert!(editor.set_speed(2.0));
    w.frames_tabs_done(crate::panels::FramesOut {
        changed: true,
        ..Default::default()
    });
    let texts = saved(&w.open.outbox);
    assert_eq!(texts.len(), 1);
    assert!(
        texts[0].contains("new ClipSettings { name = \"wave\", speed = 2.0"),
        "{}",
        texts[0]
    );
    // Every sprite playing the file sees it at once.
    let set = w.viewport.sheets.clips(clips).unwrap();
    assert_eq!(set.get("wave").unwrap().speed, 2.0);
    // Undo over the tabs undoes the file's change, and saves it.
    w.frames_undo(false, true);
    let undone = saved(&w.open.outbox);
    assert!((undone.last().unwrap()).contains("new ClipSettings { name = \"wave\", speed = 1.0"));
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
    assert!(made[0].contains(&format!("picture = \"{crops}\"")));
    assert_eq!(
        w.open.frames_editor.as_ref().unwrap().path,
        "crops.clips.lst",
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
        "walker.clips.lst"
    );
    let _ = sheet;
}

#[test]
fn a_damaged_clips_file_isn_t_opened_and_says_why() {
    let (mut w, _, _) = with_clips();
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.frames.insert(
        "walker.clips.lst".into(),
        FramesState::Failed("line 3: a clip line should be …".into()),
    );
    w.set_sprite_sheets(sheets);
    w.open_frames("walker.clips.lst");
    assert!(w.open.frames_editor.is_none());
    assert!(
        w.console
            .lines()
            .iter()
            .any(|l| l.text.contains("Couldn't open walker.clips.lst: line 3"))
    );
}

/// BUG-20: how many times a clip plays, dragged for ten frames in the
/// Clips tab, is one undo step and one save, made when the mouse is let
/// go.
#[test]
fn a_dragged_number_in_the_clips_tab_is_one_step_and_one_save() {
    let (mut w, _, _) = with_clips();
    w.open_frames("walker.clips.lst");
    // A clip's settings are in Properties while its frames are in front
    // (Fixes 3).
    w.show_panel(Panel::Frames);
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
    assert!(wave.plays > 1, "{} times", wave.plays);
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

const CLIPS: &str = "walker.clips.lst";

/// The open clips file, as its editor has it.
fn open_file(w: &Workbench) -> animation::ClipsFile {
    w.open.frames_editor.as_ref().unwrap().file.clone()
}

/// The clips file changed outside Solis: another clip added, "blink".
fn changed_by_hand(store: &mut SpriteSheets, file: &animation::ClipsFile) -> animation::ClipsFile {
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
            "walker.clips.lst changed on disk, so the Clips and Frames tabs show it as it is now \
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
        [animation::clips_file::write(&before, "walker.aseprite")]
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
    assert_eq!(
        texts,
        [animation::clips_file::write(&wanted, "walker.aseprite")]
    );
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
        assert!(!has_panel(&w, panel), "{panel:?}");
    }
    let last = w.console.lines().last().unwrap();
    assert_eq!(
        last.text,
        "walker.clips.lst is gone, so the Clips and Frames tabs closed."
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
    // Double-clicked in the Library again, from another tab.
    w.show_tab(crate::shell::RibbonTab::Home);
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::Open(LibraryKind::SpriteClips, CLIPS.into()),
        &mut actions,
    );
    for panel in Panel::FRAMES_TABS {
        assert!(has_panel(&w, panel), "{panel:?} is open");
    }
    assert_eq!(w.ribbon_tab(), crate::shell::RibbonTab::Animate);
    let editor = w.open.frames_editor.as_ref().unwrap();
    assert_eq!(editor.zoom, 500.0, "the same editor");
    assert_eq!(editor.can_undo_redo(), (true, false), "with its undo step");
    w.frames_undo(false, true);
    assert_eq!(open_file(&w), before);
    // Another clips file takes its place, as before.
    let mut store = (*w.viewport.sheets).clone();
    store.put("other.clips.lst", crate::OwnFile::Frames(Arc::default()));
    w.set_sprite_sheets(store);
    w.open_frames("other.clips.lst");
    let editor = w.open.frames_editor.as_ref().unwrap();
    assert_eq!(editor.path, "other.clips.lst");
    assert_eq!(editor.can_undo_redo(), (false, false));
}

/// The review, BUG-10: a sheet's picture for the Library and the Frames
/// tab went by where the sheet was in memory. A sheet read again could
/// be put where the last one was, and the picture of the old one stayed.
#[test]
fn a_sheet_read_again_gets_its_picture_made_again() {
    let (mut w, _, _) = with_clips();
    let ctx = egui::Context::default();
    w.refresh_sheet_pictures(&ctx);
    let first = w.open.sheet_pictures["walker.aseprite"].id();
    w.refresh_sheet_pictures(&ctx);
    assert_eq!(w.open.sheet_pictures["walker.aseprite"].id(), first, "kept");
    // Read again: the sheet let go, and another just like it in its place.
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.by_path.remove("walker.aseprite");
    w.set_sprite_sheets(sheets.clone());
    sheets.by_path.insert(
        "walker.aseprite".into(),
        SheetState::Read(Arc::new(crate::sprite_sheets::tests::test_sheet())),
    );
    w.set_sprite_sheets(sheets.clone());
    w.refresh_sheet_pictures(&ctx);
    let again = w.open.sheet_pictures["walker.aseprite"].id();
    assert_ne!(again, first, "made again from the sheet as it is");
    w.refresh_sheet_pictures(&ctx);
    assert_eq!(w.open.sheet_pictures["walker.aseprite"].id(), again);
    // Gone from the project: its picture and what it was made from go.
    sheets.by_path.clear();
    w.set_sprite_sheets(sheets);
    w.refresh_sheet_pictures(&ctx);
    assert!(w.open.sheet_pictures.is_empty() && w.open.sheet_pictures_from.is_empty());
}

/// [`with_clips`] plus `knight.png` (one 8 × 8 frame), with Make clips…
/// pressed on it: its clips file made, opened, and Cut picture… open.
fn with_picture_cut_open() -> Workbench {
    let (mut w, _, _) = with_clips();
    let knight = AssetId::random();
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.by_path.insert(
        "knight.png".into(),
        SheetState::Read(Arc::new(crate::sprite_sheets::tests::test_sheet())),
    );
    sheets.paths.insert(knight, "knight.png".into());
    w.set_sprite_sheets(sheets);
    w.library_request(
        LibraryRequest::MakeClipsFile("knight.png".into()),
        &mut Vec::new(),
    );
    w.open.outbox.clear();
    w
}

#[test]
fn make_clips_on_a_picture_opens_cut_picture() {
    let w = with_picture_cut_open();
    let cut = w.open.cut_picture.as_ref().expect("Cut picture… open");
    assert_eq!(
        (cut.path.as_str(), cut.picture.as_str()),
        ("knight.clips.lst", "knight.png")
    );
    assert_eq!(cut.grid, animation::clips_file::Grid::default());
    // An Aseprite file's frames are its cells: no cutting.
    let (mut w, _, _) = with_clips();
    w.open_frames("walker.clips.lst");
    w.open_cut_picture();
    assert!(w.open.cut_picture.is_none());
}

#[test]
fn use_these_cells_is_one_undo_step_and_cancel_changes_nothing() {
    use crate::cut_picture::CutAnswer;
    let mut w = with_picture_cut_open();
    w.open.cut_picture.as_mut().unwrap().grid.cell = [4, 6];
    w.cut_picture_answered(CutAnswer::Cancel);
    assert!(w.open.cut_picture.is_none());
    assert!(saved(&w.open.outbox).is_empty(), "Cancel saves nothing");
    assert_eq!(open_file(&w).grid.cell, [16, 16]);
    w.open_cut_picture();
    let cut = w.open.cut_picture.as_mut().unwrap();
    cut.grid.cell = [4, 6];
    cut.grid.between = [1, 1];
    w.cut_picture_answered(CutAnswer::Use);
    assert_eq!(open_file(&w).grid.cell, [4, 6]);
    assert_eq!(saved(&w.open.outbox).len(), 1, "saved once");
    let editor = w.open.frames_editor.as_mut().unwrap();
    assert!(editor.undo(), "one step back");
    assert_eq!(editor.file.grid, animation::clips_file::Grid::default());
}

#[test]
fn cells_drawn_by_hand_are_kept_and_come_back_next_time() {
    use crate::cut_picture::CutAnswer;
    let mut w = with_picture_cut_open();
    let cut = w.open.cut_picture.as_mut().unwrap();
    cut.picked = Some(0);
    cut.nudge(0, 1, [16, 24]);
    w.cut_picture_answered(CutAnswer::Use);
    let cells = open_file(&w).cells;
    assert_eq!(cells.len(), 1);
    assert_eq!((cells[0].number, cells[0].rect), (0, [0, 1, 16, 16]));
    w.open_cut_picture();
    assert_eq!(
        w.open.cut_picture.as_ref().unwrap().drawn,
        [(0, [0, 1, 16, 16])]
    );
}

/// The sample project with `knight.png` (9 × 5 cells of 32, 2 apart, 1
/// from the edge) and its clips file, open: "walk" has eight directions,
/// mirrored, each row two cells.
pub(crate) fn with_knight_open() -> Workbench {
    use animation::clips_file::Facing;
    let mut w = crate::picture_support::sample_project();
    let knight = AssetId::random();
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.by_path.insert(
        "knight.png".into(),
        SheetState::Read(Arc::new(crate::cut_picture::tests::knight_sheet())),
    );
    sheets.paths.insert(knight, "knight.png".into());
    w.set_sprite_sheets(sheets);
    w.library_request(
        LibraryRequest::MakeClipsFile("knight.png".into()),
        &mut Vec::new(),
    );
    w.open.cut_picture = None;
    let editor = w.open.frames_editor.as_mut().unwrap();
    editor.change(|f| {
        f.grid = animation::clips_file::Grid {
            cell: [32, 32],
            between: [2, 2],
            edge: [1, 1],
        };
        let frame = |n: u32| animation::ClipFrame {
            frame: n,
            ms: 120,
            events: Vec::new(),
        };
        let mut walk = animation::clips_file::Clip::one_way(
            "walk".into(),
            1.0,
            animation::Direction::Forward,
            0,
            &[frame(0), frame(1)],
        );
        walk.mirror = true;
        walk.set_directions(8);
        for (row, facing) in [
            Facing::South,
            Facing::SouthEast,
            Facing::East,
            Facing::NorthEast,
            Facing::North,
        ]
        .into_iter()
        .enumerate()
        {
            let n = row as u32 * 9;
            walk.set_frames_of(
                facing,
                &[frame(n), frame(n + 1), frame(n + 2), frame(n + 3)],
            );
        }
        f.first = "walk".into();
        f.clips.push(walk);
    });
    w.save_frames();
    let editor = w.open.frames_editor.as_mut().unwrap();
    editor.choose("walk");
    w.open.outbox.clear();
    w
}

#[test]
fn directions_change_the_rows_and_edits_go_to_the_picked_row() {
    use animation::clips_file::Facing;
    let mut w = with_knight_open();
    let cells_sheet = Arc::clone(
        w.viewport
            .sheets
            .cut_sheet("knight.clips.lst")
            .expect("cut"),
    );
    assert_eq!(cells_sheet.frames.len(), 45);
    let editor = w.open.frames_editor.as_mut().unwrap();
    assert_eq!(editor.facing, Facing::South, "a clip's first row is picked");
    editor.pick_row(Some(Facing::East));
    editor.pick(0, false, false);
    assert!(editor.add_frames(&cells_sheet, &[44]));
    let walk = |e: &crate::frames_editor::FramesEditor| {
        e.file
            .clips
            .iter()
            .find(|c| c.name == "walk")
            .unwrap()
            .clone()
    };
    let cells = |e: &crate::frames_editor::FramesEditor, f: Facing| {
        walk(e)
            .frames_of(f, |_| 1)
            .iter()
            .map(|fr| fr.frame)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        cells(editor, Facing::East),
        [18, 44, 19, 20, 21],
        "added after the picked frame, in East"
    );
    assert_eq!(
        cells(editor, Facing::South),
        [0, 1, 2, 3],
        "South as it was"
    );
    assert!(editor.set_directions(4));
    assert_eq!(
        walk(editor).ways.len(),
        3,
        "South, East, North: West is mirrored"
    );
    assert!(editor.undo(), "one step back");
    assert_eq!(walk(editor).ways.len(), 5);
}

#[test]
fn the_clips_page_with_eight_directions() {
    use crate::picture_support::{drawn, editor_window, matches_approved};
    let mut w = with_knight_open();
    let editor = w.open.frames_editor.as_mut().unwrap();
    editor.pick_row(Some(animation::clips_file::Facing::NorthEast));
    editor.pick(1, false, false);
    editor.cells_picked = vec![31];
    let Some(mut h) = editor_window(
        w,
        crate::interface_look::InterfaceLook::default(),
        [1440.0, 900.0],
    ) else {
        return;
    };
    for _ in 0..4 {
        h.run();
    }
    matches_approved("clips-directions", &drawn(&mut h));
}

#[test]
fn a_run_of_cells_fills_a_row_and_a_clicked_cell_swaps_a_frame() {
    use animation::clips_file::Facing;
    let mut w = with_knight_open();
    let cells_sheet = Arc::clone(
        w.viewport
            .sheets
            .cut_sheet("knight.clips.lst")
            .expect("cut"),
    );
    let editor = w.open.frames_editor.as_mut().unwrap();
    let row = |e: &crate::frames_editor::FramesEditor, f: Facing| {
        let walk = e.file.clips.iter().find(|c| c.name == "walk").unwrap();
        walk.frames_of(f, |_| 1)
            .iter()
            .map(|fr| (fr.frame, fr.ms))
            .collect::<Vec<_>>()
    };
    // Five cells onto North's four frames: a fifth column, its time the
    // cell's own.
    assert!(editor.fill_row(Facing::North, &[40, 41, 42, 43, 44], &cells_sheet));
    assert_eq!(
        row(editor, Facing::North),
        [(40, 120), (41, 120), (42, 120), (43, 120), (44, 100)]
    );
    assert_eq!(editor.facing, Facing::North, "the row is picked");
    assert!(
        !editor.fill_row(Facing::North, &[99], &cells_sheet),
        "no such cell"
    );
    // Pick a frame, click a cell: swapped, its time kept.
    editor.pick(1, false, false);
    assert!(editor.swap_cell(7));
    assert_eq!(row(editor, Facing::North)[1], (7, 120));
    assert!(editor.undo());
    assert_eq!(row(editor, Facing::North)[1], (41, 120), "one step each");
}
