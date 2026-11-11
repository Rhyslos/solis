//! Play mode: Play, edits while playing, Pause and Stop, what waits for
//! Stop, the Game panel, cameras, and the buttons and commands.

use super::*;
use crate::console::LogLevel;
use crate::properties::PropertyRequest;
use crate::test_support::*;
use crate::viewport::{GAME_IMAGE, VIEWPORT_IMAGE};
use area::builtin::{Light, Transform};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use reflect::{EntityId, Value};
use std::path::{Path, PathBuf};

fn set_position(w: &mut Workbench, id: EntityId, at: Vec3, gesture: u64) {
    w.apply_property(
        PropertyRequest::SetField {
            entity: id,
            component: "Transform".into(),
            field: "position".into(),
            value: Value::vec3(at),
        },
        gesture,
    );
}

fn position(w: &Workbench, id: EntityId) -> Vec3 {
    w.open
        .document
        .area()
        .get(id)
        .unwrap()
        .get::<Transform>()
        .unwrap()
        .position
}

#[test]
fn stop_puts_the_area_its_history_and_the_selection_back_exactly() {
    // The phase gate, as a test: build, Play, change things, Stop.
    let (mut w, crate_id) = with_a_crate();
    let lamp = w.add_point_light();
    set_position(&mut w, crate_id, Vec3::new(1.0, 0.0, 0.0), 1);
    w.open.document.selection.select_only(crate_id);
    let area = w.open.document.area().clone();
    let steps = w.open.document.history().undo_len();
    let dirty = w.open.document.is_dirty();
    let selection = w.open.document.selection.clone();

    w.play();
    assert_eq!(w.play_state(), PlayState::Playing);
    // Everything changes while playing…
    set_position(&mut w, crate_id, Vec3::new(9.0, 9.0, 9.0), 2);
    w.add_entity();
    w.open.document.selection.select_only(lamp);
    w.delete_selection();
    w.undo();
    w.redo();
    assert_ne!(*w.open.document.area(), area);

    // …and Stop puts it all back.
    w.stop();
    assert_eq!(w.play_state(), PlayState::Editing);
    assert_eq!(*w.open.document.area(), area);
    assert_eq!(w.open.document.history().undo_len(), steps);
    assert_eq!(w.open.document.is_dirty(), dirty);
    assert_eq!(w.open.document.selection, selection);
    assert!(w.world().is_none());
    // Undo still undoes what was done before Play.
    w.undo();
    assert_eq!(position(&w, crate_id), Vec3::ZERO);
}

#[test]
fn the_game_follows_changes_made_while_playing() {
    let (mut w, crate_id) = with_a_crate();
    w.play();
    let before = w.world().unwrap().len();
    w.add_entity();
    w.run_game(0.0);
    assert_eq!(w.world().unwrap().len(), before + 1);
    // Every frame of a drag reaches the game, though it's one undo step.
    for (i, x) in [1.0, 2.0, 3.0].into_iter().enumerate() {
        set_position(&mut w, crate_id, Vec3::new(x, 0.0, 0.0), 7);
        w.run_game(0.0);
        let at = w.world().unwrap().global(crate_id).unwrap();
        assert_eq!(at.transform_point3(Vec3::ZERO).x, x, "frame {i}");
    }
}

#[test]
fn pause_stops_the_clock_and_play_carries_on() {
    let (mut w, _) = with_a_crate();
    w.run_game(1.0);
    assert!(w.world().is_none(), "nothing runs before Play");
    w.play();
    w.run_game(0.1);
    let ticks = w.world().unwrap().clock().ticks;
    assert!(ticks > 0);
    w.pause();
    assert_eq!(w.play_state(), PlayState::Paused);
    w.run_game(0.1);
    assert_eq!(w.world().unwrap().clock().ticks, ticks, "paused");
    w.play();
    assert_eq!(w.play_state(), PlayState::Playing);
    w.run_game(0.1);
    assert!(w.world().unwrap().clock().ticks > ticks);
    // Play again while playing changes nothing.
    w.play();
    assert_eq!(w.play_state(), PlayState::Playing);
}

#[test]
fn stop_and_pause_with_nothing_running_just_say_so() {
    let (mut w, _) = with_a_crate();
    w.stop();
    w.pause();
    assert_eq!(w.play_state(), PlayState::Editing);
    assert_eq!(w.log_lines().last().unwrap().text, "Nothing is running.");
}

#[test]
fn saving_and_opening_wait_until_the_game_stops() {
    let (mut w, crate_id) = with_a_crate();
    w.play();
    for action in [
        WorkbenchAction::SaveArea,
        WorkbenchAction::SaveAreaAs,
        WorkbenchAction::NewArea,
        WorkbenchAction::OpenArea,
        WorkbenchAction::OpenProject,
        WorkbenchAction::CreateAssembly(crate_id),
    ] {
        let mut actions = Vec::new();
        w.request(action.clone(), &mut actions);
        w.hold_while_playing(&mut actions);
        assert!(actions.is_empty(), "{action:?} waits");
        let last = w.log_lines().last().unwrap();
        assert_eq!(last.level, LogLevel::Warning);
        assert!(last.text.starts_with("Stop playing first"));
    }
    assert!(w.open.unsaved_prompt.is_none(), "no question about saving");
    // Other actions go through.
    let mut actions = vec![WorkbenchAction::ImportModel];
    w.hold_while_playing(&mut actions);
    assert_eq!(actions, [WorkbenchAction::ImportModel]);
    // After Stop, saving works again.
    w.stop();
    let mut actions = Vec::new();
    w.request(WorkbenchAction::SaveArea, &mut actions);
    w.hold_while_playing(&mut actions);
    assert_eq!(actions, [WorkbenchAction::SaveArea]);
}

#[test]
fn quitting_while_playing_stops_first() {
    let (mut w, crate_id) = with_a_crate();
    w.play();
    set_position(&mut w, crate_id, Vec3::ONE, 1);
    let mut actions = Vec::new();
    w.request(WorkbenchAction::Quit, &mut actions);
    assert_eq!(w.play_state(), PlayState::Editing);
    assert_eq!(position(&w, crate_id), Vec3::ZERO);
    // The move made while playing is asked about first (Keep or Undo
    // all): nothing closes, and one question is asked at a time.
    assert_eq!(actions, []);
    assert!(w.play_changes().is_some());
    assert!(w.open.unsaved_prompt.is_none());
    // Undo all: the area is as it was before Play, where the crate was
    // placed and not saved, so the question about saving comes next.
    w.undo_play_changes();
    assert_eq!(w.open.unsaved_prompt, Some(WorkbenchAction::Quit));
    assert!(w.open.outbox.is_empty(), "still not closed");
    // With nothing changed while playing and nothing unsaved, Quit stops
    // the game and closes.
    let (mut w, _) = with_a_crate();
    w.open.document.mark_saved("harbour.solarea".into());
    w.play();
    let mut actions = Vec::new();
    w.request(WorkbenchAction::Quit, &mut actions);
    assert_eq!(w.play_state(), PlayState::Editing);
    assert_eq!(actions, [WorkbenchAction::Quit]);
    // Closing the window with unsaved changes asks first, and the game
    // plays on meanwhile (Cancel leaves it running); with nothing unsaved,
    // it stops and closes.
    let (mut w, _) = with_a_crate();
    w.play();
    assert!(!w.close_requested());
    assert_eq!(w.open.unsaved_prompt, Some(WorkbenchAction::Quit));
    assert_ne!(w.play_state(), PlayState::Editing, "still playing");
    let (mut w, _) = with_a_crate();
    w.open.document.mark_saved("harbour.solarea".into());
    w.play();
    assert!(w.close_requested());
    assert_eq!(w.play_state(), PlayState::Editing);
}

#[test]
fn a_file_changed_on_disk_while_playing_is_offered_after_stop() {
    let (mut w, _) = with_a_crate();
    w.open.document.mark_saved("harbour.solarea".into());
    w.play();
    let mut theirs = area::builtin::starter_area();
    let theirs_id = theirs.spawn(area::EntityData::new("Theirs"));
    w.offer_reload(theirs);
    assert!(
        !w.open.document.area().contains(theirs_id),
        "not while playing"
    );
    w.stop();
    assert!(
        w.open.document.area().contains(theirs_id),
        "loaded after Stop (nothing was unsaved)"
    );
}

#[test]
fn an_assembly_changed_while_playing_updates_its_copies_after_stop() {
    let (mut w, asset, _, lamp) = with_a_lamp_post();
    w.open.document.mark_saved("harbour.solarea".into());
    w.play();
    let (_, file) = lamp_post_file();
    let mut file = (*file).clone();
    let two = EntityId(reflect::Id::from_raw(2).unwrap());
    assert!(area::assembly::set_in_file(
        &mut file,
        &[two],
        "Light",
        "strength",
        Value::Float(5.0)
    ));
    w.set_assembly(asset, std::sync::Arc::new(file));
    let strength = |w: &Workbench| {
        w.open
            .document
            .area()
            .get(lamp)
            .unwrap()
            .get::<Light>()
            .unwrap()
            .strength
    };
    assert_eq!(strength(&w), 5.0, "the game sees it at once");
    w.stop();
    assert_eq!(strength(&w), 5.0, "and it's still there after Stop");
    assert!(
        !w.open.document.is_dirty(),
        "nothing to save: the file has it"
    );
}

#[test]
fn a_new_camera_looks_the_way_the_editor_does() {
    let (mut w, _) = with_a_crate();
    let id = w.add_camera();
    let data = w.open.document.area().get(id).unwrap();
    assert_eq!(data.name, "Camera 1");
    let camera = data.get::<area::builtin::Camera>().unwrap();
    assert!((camera.fov - 50.0).abs() < 1e-3);
    let (view, position) = crate::game_panel::camera_view(w.open.document.area().world_matrix(id));
    assert!(position.distance(w.viewport.camera.position) < 1e-4);
    let editor = w.viewport.camera.view_matrix();
    for (a, b) in view.to_cols_array().iter().zip(editor.to_cols_array()) {
        assert!((a - b).abs() < 1e-4, "{view:?} vs {editor:?}");
    }
    assert_eq!(w.add_camera(), w.open.document.selection.primary().unwrap());
    assert_eq!(
        w.open
            .document
            .area()
            .get(w.open.document.selection.primary().unwrap())
            .unwrap()
            .name,
        "Camera 2"
    );
}

#[test]
fn the_game_panel_draws_through_the_camera_without_editor_lines() {
    let (mut w, crate_id) = with_a_crate();
    let camera = w.add_camera();
    // Moved after it was added: the game looks from where it is now.
    set_position(&mut w, camera, Vec3::new(0.0, -20.0, 3.0), 1);
    let mut h = harness(w);
    h.run();
    h.query_all_by_label("Play")
        .next()
        .expect("the Play button")
        .click();
    h.run();
    h.run();
    assert_eq!(h.state().0.play_state(), PlayState::Playing);
    let _ = h.get_by_label_contains("Playing ·");
    let views = &h.state().1.views;
    let game = views
        .iter()
        .rev()
        .find(|v| v.image == GAME_IMAGE)
        .expect("the Game panel drew");
    assert!(!game.editor_lines);
    assert_eq!(game.camera_position, Vec3::new(0.0, -20.0, 3.0));
    assert_eq!(game.models.len(), 1, "the crate");
    assert!(game.models.iter().all(|m| m.pick == 0), "nothing picked");
    let _ = crate_id;
    // The viewport still draws, with its lines.
    h.get_by_label("Viewport").click();
    h.run();
    let viewport = h.state().1.views.last().unwrap();
    assert_eq!(viewport.image, VIEWPORT_IMAGE);
    assert!(viewport.editor_lines);
}

#[test]
fn without_a_camera_the_game_shows_the_editors_view_with_a_note() {
    let (w, _) = with_a_crate();
    let mut h = harness(w);
    h.run();
    h.query_all_by_label("Play")
        .next()
        .expect("the Play button")
        .click();
    h.run();
    h.run();
    let _ = h.get_by_label("No camera in the area: showing the editor's view");
    let game = h
        .state()
        .1
        .views
        .iter()
        .rev()
        .find(|v| v.image == GAME_IMAGE)
        .unwrap();
    assert_eq!(game.camera_position, h.state().0.viewport.camera.position);
}

#[test]
fn the_buttons_at_the_top_play_pause_and_stop() {
    let (w, crate_id) = with_a_crate();
    let mut h = harness(w);
    h.run();
    h.query_all_by_label("Play")
        .next()
        .expect("the Play button")
        .click();
    h.run();
    assert_eq!(h.state().0.play_state(), PlayState::Playing);
    set_position(&mut h.state_mut().0, crate_id, Vec3::ONE, 99);
    h.get_by_label("Pause").click();
    h.run();
    assert_eq!(h.state().0.play_state(), PlayState::Paused);
    let _ = h.get_by_label_contains("Paused ·");
    h.get_by_label("Stop").click();
    h.run();
    assert_eq!(h.state().0.play_state(), PlayState::Editing);
    assert_eq!(position(&h.state().0, crate_id), Vec3::ZERO);
    // Ctrl+S while playing is held back.
    h.query_all_by_label("Play")
        .next()
        .expect("the Play button")
        .click();
    h.run();
    h.state_mut().1.actions.clear();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run();
    assert!(!h.state().1.actions.contains(&WorkbenchAction::SaveArea));
}

#[test]
fn the_console_plays_pauses_and_stops() {
    let (mut w, _) = with_a_crate();
    let mut actions = Vec::new();
    let ctx = egui::Context::default();
    w.run_command("play", &ctx, &mut actions);
    assert_eq!(w.play_state(), PlayState::Playing);
    w.run_command("pause", &ctx, &mut actions);
    assert_eq!(w.play_state(), PlayState::Paused);
    w.run_command("stop", &ctx, &mut actions);
    assert_eq!(w.play_state(), PlayState::Editing);
    w.run_command("play", &ctx, &mut actions);
    w.run_command("save", &ctx, &mut actions);
    w.hold_while_playing(&mut actions);
    assert!(actions.is_empty(), "saving waits from the console too");
}

#[test]
fn a_camera_is_listed_with_its_icon_and_added_from_the_entity_menu() {
    let (w, _) = with_a_crate();
    let mut h = harness(w);
    h.run();
    // The top bar's Entity menu (the Area panel has one too).
    h.query_all_by_label("Entity").next().unwrap().click();
    h.run();
    h.get_by_label("Add camera").click();
    h.run();
    let w = &h.state().0;
    let id = w.open.document.selection.primary().unwrap();
    assert!(w.open.document.area().get(id).unwrap().has("Camera"));
    assert!(h.query_all_by_label("Camera 1").next().is_some(), "listed");
}

#[test]
fn a_cancelled_save_calls_off_the_quit_so_a_later_close_asks_again() {
    let (mut w, _) = with_a_crate();
    assert!(w.open.document.is_dirty());
    assert!(!w.close_requested(), "asks first");
    // "Save" was chosen: the quit may go ahead once the save works...
    w.may_close = true;
    assert!(w.close_requested());
    // ...but the Save window was cancelled.
    w.quit_cancelled();
    assert!(!w.close_requested(), "asks again");
    assert_eq!(w.open.unsaved_prompt, Some(WorkbenchAction::Quit));
}

#[test]
fn a_file_changed_on_disk_is_asked_about_only_after_stop() {
    let (mut w, _) = with_a_crate();
    w.open.pending_reload = Some(area::builtin::starter_area());
    w.play();
    let mut h = harness(w);
    h.run();
    assert!(
        h.query_by_label("Load the file").is_none(),
        "not while playing"
    );
    h.state_mut().0.stop();
    h.run();
    let _ = h.get_by_label("Load the file");
}

// The open area's file moved or deleted in the Library while playing
// (the review, BUG-05): only the document the game runs on followed, so
// Stop put back the copy from before Play with the old path, and Ctrl+S
// made the old file again beside the moved one (or, after a delete, the
// area read as saved though its file was gone).

const AREA_FILE: &str = "/p/assets/areas/harbour.solarea";

/// What the Library did: what it moved, where to (`None`: deleted), and
/// where the open area's file is afterwards (`None`: nowhere).
type FileCase = (
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
);

const FILE_CASES: [FileCase; 5] = [
    (
        "renamed",
        AREA_FILE,
        Some("/p/assets/areas/port.solarea"),
        Some("/p/assets/areas/port.solarea"),
    ),
    (
        "moved into a folder",
        AREA_FILE,
        Some("/p/assets/areas/docks/harbour.solarea"),
        Some("/p/assets/areas/docks/harbour.solarea"),
    ),
    (
        "a parent folder renamed",
        "/p/assets/areas",
        Some("/p/assets/levels"),
        Some("/p/assets/levels/harbour.solarea"),
    ),
    ("deleted", AREA_FILE, None, None),
    ("its folder deleted", "/p/assets/areas", None, None),
];

/// A saved area with a crate, open from [`AREA_FILE`].
fn saved_area() -> (Workbench, EntityId) {
    let (mut w, crate_id) = with_a_crate();
    w.mark_saved(AREA_FILE.into());
    (w, crate_id)
}

/// Does to `w` what the Library did in `case`, as the app tells it.
fn library_does(w: &mut Workbench, case: &FileCase) {
    let (what, from, to, after) = *case;
    assert_eq!(
        w.file_moved(Path::new(from), to.map(Path::new)),
        Some(after.map(PathBuf::from)),
        "{what}: the app is told where the file is now"
    );
}

/// The open area is where `case` left its file, and saving (Ctrl+S, as
/// the app does it) goes there: `unsaved` says whether there are changes
/// to save. With no file left, the area counts as unsaved, so saving
/// asks where.
fn saves_where_the_file_is(w: &mut Workbench, case: &FileCase, unsaved: bool, when: &str) {
    let (what, _, _, after) = *case;
    assert_eq!(w.play_state(), PlayState::Editing);
    assert_eq!(w.document().file(), after.map(Path::new), "{what}, {when}");
    match after {
        Some(file) => {
            assert_eq!(w.document().is_dirty(), unsaved, "{what}, {when}");
            w.mark_saved(file.into());
            assert_eq!(w.document().file(), Some(Path::new(file)));
            assert!(!w.document().is_dirty());
        }
        None => assert!(w.document().is_dirty(), "{what}, {when}: unsaved"),
    }
}

#[test]
fn the_area_s_file_moved_or_deleted_while_playing_stays_so_after_stop() {
    for case in &FILE_CASES {
        let (mut w, _) = saved_area();
        w.play();
        library_does(&mut w, case);
        w.stop();
        assert!(w.play_changes().is_none());
        saves_where_the_file_is(&mut w, case, false, "after Stop");
    }
}

#[test]
fn the_area_s_file_moved_or_deleted_while_playing_stays_so_after_keep_and_undo_all() {
    for case in &FILE_CASES {
        // An edit before the move and one after it, then Stop and Keep.
        let (mut w, crate_id) = saved_area();
        w.play();
        set_position(&mut w, crate_id, Vec3::X, 1);
        library_does(&mut w, case);
        let lamp = w.add_point_light();
        w.stop();
        assert_eq!(w.play_changes().unwrap().len(), 2);
        w.keep_play_changes();
        assert!(w.document().area().contains(lamp));
        saves_where_the_file_is(&mut w, case, true, "after Stop and Keep");
        // Undo all: nothing of the user's is left to save.
        let (mut w, crate_id) = saved_area();
        w.play();
        library_does(&mut w, case);
        set_position(&mut w, crate_id, Vec3::X, 1);
        w.stop();
        w.undo_play_changes();
        saves_where_the_file_is(&mut w, case, false, "after Stop and Undo all");
    }
}

#[test]
fn the_area_s_file_moved_or_deleted_while_keep_is_asked_stays_so_after_the_answer() {
    for case in &FILE_CASES {
        let (mut w, _) = saved_area();
        w.play();
        let lamp = w.add_point_light();
        w.stop();
        assert!(w.play_changes().is_some());
        library_does(&mut w, case);
        w.keep_play_changes();
        assert!(w.document().area().contains(lamp));
        saves_where_the_file_is(&mut w, case, true, "moved while asked, kept");
    }
}

#[test]
fn another_file_moved_while_playing_changes_nothing() {
    let (mut w, _) = saved_area();
    w.play();
    assert_eq!(
        w.file_moved(
            Path::new("/p/assets/crate.glb"),
            Some(Path::new("/p/assets/box.glb"))
        ),
        None
    );
    // A file whose name only starts the same isn't the area's folder.
    assert_eq!(w.file_moved(Path::new("/p/assets/are"), None), None);
    w.stop();
    assert_eq!(w.document().file(), Some(Path::new(AREA_FILE)));
    assert!(!w.document().is_dirty());
}
