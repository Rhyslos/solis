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
    h.state_mut().0.show_panel(crate::panel::Panel::Viewport);
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
    // Home, Create, Camera.
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

/// The review, BUG-51: a spawn that can't be made says why in the
/// Console, once a play for each assembly, however often it's asked for.
mod spawns_that_cannot_be_made {
    use super::*;
    use crate::game_code::GameEdit;
    use crate::library::{LibraryItem, LibraryKind};
    use area::text::{AreaFile, FileKind};
    use area::{Area, EntityData, Origin};
    use reflect::{AssetId, Id};
    use std::sync::Arc;

    /// How many frames each test plays.
    const FRAMES: u64 = 40;

    fn id(n: u64) -> EntityId {
        EntityId(Id::from_raw(n).unwrap())
    }

    fn asset(n: u64) -> AssetId {
        AssetId(Id::from_raw(n).unwrap())
    }

    /// An assembly of one entity, and one more for each of `places`: a
    /// copy of that assembly, under the root.
    fn assembly(places: &[AssetId]) -> AreaFile {
        let mut area = Area::new();
        area.insert(id(1), EntityData::new("Root"));
        for (n, &placed) in places.iter().enumerate() {
            let instance = id(100 + n as u64);
            let mut copy = EntityData::new("Copy");
            copy.parent = Some(id(1));
            copy.origin = Some(Origin {
                assembly: placed,
                instance,
                path: Vec::new(),
            });
            area.insert(instance, copy);
        }
        AreaFile {
            kind: FileKind::Assembly { root: id(1) },
            area,
            overrides: Default::default(),
        }
    }

    /// A workbench that knows `files` as assemblies, each in the Library
    /// under its name, playing a new area.
    fn playing(files: Vec<(AssetId, &str, AreaFile)>) -> Workbench {
        let mut w = knowing(files);
        w.play();
        w
    }

    /// A workbench that knows `files` as assemblies, each in the Library
    /// under its name.
    fn knowing(files: Vec<(AssetId, &str, AreaFile)>) -> Workbench {
        let mut w = Workbench::new(info());
        let mut library = Vec::new();
        for (asset, name, file) in files {
            w.set_assembly(asset, Arc::new(file));
            library.push(LibraryItem {
                id: Some(asset),
                kind: LibraryKind::Assembly,
                name: name.into(),
                path: format!("{name}.solasm"),
                thumbnail: None,
            });
        }
        w.set_library(library);
        w
    }

    /// Game code asks for a copy of `name` every frame, as entity `first`
    /// and the ones after it.
    fn spawn_every_frame(w: &mut Workbench, name: &str, first: u64) {
        for frame in 0..FRAMES {
            let asked = GameEdit::Spawn(id(first + frame), name.into(), Vec3::ZERO);
            w.open.playing.as_mut().unwrap().pending.push(asked);
            w.run_game(1.0 / 60.0);
        }
    }

    /// The Console's lines about spawning `name`.
    fn said_about<'a>(w: &'a Workbench, name: &str) -> Vec<&'a str> {
        let about = format!("spawned \"{name}\"");
        w.log_lines()
            .iter()
            .filter(|l| l.text.contains(&about))
            .map(|l| l.text.as_str())
            .collect()
    }

    /// Spawning "squad" every frame says `why` once, as a warning, and
    /// makes nothing.
    fn says_once(mut w: Workbench, why: &str) {
        let before = w.open.document.area().len();
        spawn_every_frame(&mut w, "squad", 5000);
        let said = said_about(&w, "squad");
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].contains(why), "{}", said[0]);
        assert!(said[0].ends_with("so nothing was made."), "{}", said[0]);
        assert_eq!(w.open.document.area().len(), before);
        // Plain words: no ID stands where a name is known.
        assert!(!said[0].contains(&asset(10).to_string()), "{}", said[0]);
    }

    #[test]
    fn a_part_whose_file_is_missing_is_said_once() {
        let w = playing(vec![(asset(10), "squad", assembly(&[asset(999)]))]);
        says_once(w, "isn't in the project");
    }

    /// An exported game lands its spawns through the same code.
    #[test]
    fn an_exported_game_says_it_once_too() {
        let mut w = knowing(vec![(asset(10), "squad", assembly(&[asset(999)]))]);
        w.set_play_only();
        w.play();
        says_once(w, "isn't in the project");
    }

    #[test]
    fn a_part_whose_file_cannot_be_read_is_named() {
        // In the Library, but never read as an assembly.
        let mut w = playing(vec![(asset(10), "squad", assembly(&[asset(11)]))]);
        let mut library = w.open.library.clone();
        library.push(LibraryItem {
            id: Some(asset(11)),
            kind: LibraryKind::Assembly,
            name: "scout".into(),
            path: "scout.solasm".into(),
            thumbnail: None,
        });
        w.set_library(library);
        says_once(w, "it places \"scout\", whose file can't be read");
    }

    #[test]
    fn an_area_spawned_as_an_assembly_is_said_once() {
        let mut town = assembly(&[]);
        town.kind = FileKind::Area;
        let w = playing(vec![(asset(10), "squad", town)]);
        says_once(w, "its file is an area, not an assembly");
    }

    #[test]
    fn a_part_that_is_an_area_is_said_once() {
        let mut town = assembly(&[]);
        town.kind = FileKind::Area;
        let w = playing(vec![
            (asset(10), "squad", assembly(&[asset(11)])),
            (asset(11), "town", town),
        ]);
        says_once(w, "\"town\", which is an area, not an assembly");
    }

    #[test]
    fn assemblies_placing_each_other_in_a_loop_are_said_once() {
        let w = playing(vec![
            (asset(10), "squad", assembly(&[asset(11)])),
            (asset(11), "scout", assembly(&[asset(10)])),
        ]);
        says_once(w, "in a loop (\"squad\" → \"scout\" → \"squad\")");
    }

    #[test]
    fn assemblies_placed_too_deep_are_said_once() {
        let deep = area::text::MAX_NESTING as u64 + 2;
        let mut files: Vec<(AssetId, String, AreaFile)> = (0..deep)
            .map(|level| {
                let name = if level == 0 {
                    "squad".to_string()
                } else {
                    format!("level{level}")
                };
                (asset(10 + level), name, assembly(&[asset(11 + level)]))
            })
            .collect();
        files.push((asset(10 + deep), "last".into(), assembly(&[])));
        let files = files
            .iter()
            .map(|(asset, name, file)| (*asset, name.as_str(), file.clone()))
            .collect();
        says_once(playing(files), "placed more than 16 deep");
    }

    #[test]
    fn a_copy_too_big_to_make_is_said_once() {
        // Ten copies of ten copies… eight times over: a hundred million.
        let mut files = vec![(asset(20), "one".to_string(), assembly(&[]))];
        for level in 1..=8u64 {
            let name = if level == 8 {
                "squad".to_string()
            } else {
                format!("crowd{level}")
            };
            files.push((asset(20 + level), name, assembly(&[asset(19 + level); 10])));
        }
        let files = files
            .iter()
            .map(|(asset, name, file)| (*asset, name.as_str(), file.clone()))
            .collect();
        says_once(playing(files), "more than 1,000,000 entities");
    }

    #[test]
    fn two_parts_that_would_share_an_id_are_said_once() {
        // A part of its own with the ID a placed copy's part gets.
        let mut scout = assembly(&[]);
        let mut part = EntityData::new("Part");
        part.parent = Some(id(1));
        scout.area.insert(id(2), part);
        let mut squad = assembly(&[asset(11)]);
        let mut twin = EntityData::new("Twin");
        twin.parent = Some(id(1));
        squad
            .area
            .insert(area::assembly::derive(id(100), &[id(2)]), twin);
        let w = playing(vec![
            (asset(10), "squad", squad),
            (asset(11), "scout", scout),
        ]);
        says_once(w, "two of its entities would share an ID");
    }

    #[test]
    fn a_copy_whose_id_is_taken_is_said_once_and_others_are_still_made() {
        let mut w = playing(vec![(asset(10), "squad", assembly(&[]))]);
        let before = w.open.document.area().len();
        // The first is made; asking for the same entity again can't be.
        for _ in 0..FRAMES {
            let asked = GameEdit::Spawn(id(5000), "squad".into(), Vec3::ZERO);
            w.open.playing.as_mut().unwrap().pending.push(asked);
            w.run_game(1.0 / 60.0);
        }
        assert!(w.open.document.area().contains(id(5000)));
        assert_eq!(w.open.document.area().len(), before + 1);
        let said = said_about(&w, "squad");
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].contains("already in the area"), "{}", said[0]);
        // Another entity of the same assembly is still made.
        spawn_every_frame(&mut w, "squad", 6000);
        assert_eq!(w.open.document.area().len(), before + 1 + FRAMES as usize);
        assert_eq!(said_about(&w, "squad").len(), 1);
    }

    #[test]
    fn each_assembly_s_reason_is_its_own_and_a_new_play_says_it_again() {
        let mut w = playing(vec![
            (asset(10), "squad", assembly(&[asset(999)])),
            (asset(12), "patrol", assembly(&[asset(12)])),
            (asset(13), "scout", assembly(&[])),
        ]);
        let before = w.open.document.area().len();
        for frame in 0..FRAMES {
            let pending = &mut w.open.playing.as_mut().unwrap().pending;
            for (n, name) in ["squad", "patrol", "nobody", "scout"]
                .into_iter()
                .enumerate()
            {
                let entity = id(7000 + frame * 10 + n as u64);
                pending.push(GameEdit::Spawn(entity, name.into(), Vec3::ZERO));
            }
            w.run_game(1.0 / 60.0);
        }
        assert_eq!(said_about(&w, "squad").len(), 1);
        assert_eq!(said_about(&w, "patrol").len(), 1);
        assert_eq!(said_about(&w, "nobody").len(), 1);
        assert!(said_about(&w, "patrol")[0].contains("in a loop"));
        assert!(said_about(&w, "nobody")[0].contains("no assembly by that name"));
        assert!(said_about(&w, "scout").is_empty());
        assert_eq!(w.open.document.area().len(), before + FRAMES as usize);
        // The next play is a new one: it says so again, once.
        w.stop();
        w.play();
        spawn_every_frame(&mut w, "squad", 5000);
        assert_eq!(said_about(&w, "squad").len(), 2);
    }
}
