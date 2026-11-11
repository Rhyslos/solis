//! Opening another project: nothing of the old one is used with the new
//! one, whatever was open, half done or waiting for an answer.

use crate::layout;
use crate::library::{LibraryItem, LibraryKind, LibraryRequest, ProjectInfo};
use crate::map_tab::MapRequest;
use crate::panel::Panel;
use crate::project_state::ProjectState;
use crate::sprite_sheets::{FramesState, MapFileState, SheetState, SpriteSheets, TilesetState};
use crate::test_support::*;
use crate::workbench::{Workbench, WorkbenchAction};
use crate::workbench_tiles::tests::{farm_map, saved_maps};
use egui_kittest::kittest::Queryable;
use reflect::AssetId;
use std::sync::Arc;

const SET: &str = "maps/town.tileset.lst";
const MAP: &str = "maps/town.tilemap.lst";
const SHEET: &str = "walker.aseprite";
const CLIPS: &str = "walker.solframes";

/// The IDs of a project's files. A project and its copy share them, as
/// they share their paths.
#[derive(Clone, Copy)]
struct Ids {
    set: AssetId,
    map: AssetId,
    sheet: AssetId,
    clips: AssetId,
}

impl Ids {
    fn new() -> Self {
        Self {
            set: AssetId::random(),
            map: AssetId::random(),
            sheet: AssetId::random(),
            clips: AssetId::random(),
        }
    }
}

fn project(name: &str) -> ProjectInfo {
    ProjectInfo {
        name: name.into(),
        root: format!("/projects/{name}").into(),
        assets: format!("/projects/{name}/assets").into(),
    }
}

/// Opens the project `name` and hands over its files as the app does: a
/// tileset and a map of it, a sprite sheet and a clips file playing it.
fn open_farm(w: &mut Workbench, name: &str, ids: Ids) {
    w.set_project(Some(project(name)));
    let item = |path: &str, kind, id| LibraryItem {
        id: Some(id),
        kind,
        name: crate::sprite_sheets::name_of(path),
        path: path.into(),
        thumbnail: None,
    };
    w.set_library(vec![
        item(SET, LibraryKind::Tileset, ids.set),
        item(MAP, LibraryKind::TileMap, ids.map),
        item(SHEET, LibraryKind::SpriteSheet, ids.sheet),
        item(CLIPS, LibraryKind::SpriteClips, ids.clips),
    ]);
    w.set_folders(vec!["maps".into()]);
    let clips = animation::FramesFile {
        sheet: Some(ids.sheet),
        first: "wave".into(),
        tags: Vec::new(),
        clips: vec![animation::OwnClip {
            name: "wave".into(),
            speed: 1.0,
            direction: animation::Direction::Forward,
            repeats: 0,
            frames: (0..2)
                .map(|frame| animation::ClipFrame {
                    frame,
                    ms: 100,
                    events: Vec::new(),
                })
                .collect(),
        }],
    };
    let mut sheets = SpriteSheets::default();
    sheets.tilesets.insert(
        SET.into(),
        TilesetState::Read(
            Arc::new(crate::workbench_tiles::starter_tileset()),
            Vec::new(),
        ),
    );
    sheets.tile_maps.insert(
        MAP.into(),
        MapFileState::Read(Arc::new(farm_map(ids.set)), Vec::new()),
    );
    sheets.by_path.insert(
        SHEET.into(),
        SheetState::Read(Arc::new(crate::sprite_sheets::tests::test_sheet())),
    );
    sheets
        .frames
        .insert(CLIPS.into(), FramesState::Read(Arc::new(clips), Vec::new()));
    for (id, path) in [
        (ids.set, SET),
        (ids.map, MAP),
        (ids.sheet, SHEET),
        (ids.clips, CLIPS),
    ] {
        sheets.paths.insert(id, path.into());
    }
    w.set_sprite_sheets(sheets);
}

#[test]
fn undo_over_the_map_tab_never_reaches_into_another_project() {
    // The review (BUG-07): the Map tab's undo steps, kept by path, outlived
    // their project. A map changed in one project, then a copy of the
    // project opened (the same paths): Ctrl+Z over the Map tab wrote the
    // first project's old map over the copy's file.
    let ids = Ids::new();
    let mut w = Workbench::new(info(), None);
    open_farm(&mut w, "farm", ids);
    w.place_tile_map(ids.map, None).unwrap();
    w.follow_map_selection();
    w.map_requests_done(vec![MapRequest::AddLayer]);
    assert_eq!(w.map_file(MAP).unwrap().layers().len(), 4);

    open_farm(&mut w, "copy", ids);
    assert_eq!(w.map_file(MAP).unwrap().layers().len(), 3, "the copy's own");
    w.place_tile_map(ids.map, None).unwrap();
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(h.state().0.dock.find_tab(&Panel::Map).is_some());
    h.state_mut().1.actions.clear();
    // With the pointer over the Map tab, Ctrl+Z is the map's.
    let over_the_tab = h.get_by_label("Add a layer").rect().center();
    h.hover_at(over_the_tab);
    h.run();
    assert!(h.state().0.map_hovered);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    let (w, picker) = h.state();
    assert_eq!(saved_maps(&picker.actions), Vec::new(), "nothing written");
    assert_eq!(saved_maps(&w.open.outbox), Vec::new(), "nor about to be");
    assert_eq!(w.map_file(MAP).unwrap().layers().len(), 3, "nothing undone");
    assert_eq!(
        w.open.document.area().len(),
        3,
        "nor the area's own last step"
    );
}

#[test]
fn files_open_in_tabs_close_with_their_project() {
    // The review (BUG-07): the Clips and Frames tabs stayed open on the old
    // project's file, and the next change there was saved into the new one.
    let ids = Ids::new();
    let mut w = Workbench::new(info(), None);
    open_farm(&mut w, "farm", ids);
    w.open_frames(CLIPS);
    w.open_tileset(SET);
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::AtlasCrop("sprites/tools.png".into()),
        &mut actions,
    );
    assert!(w.open.frames_editor.is_some() && w.open.tileset_editor.is_some());
    assert!(w.open.atlas_editor.is_some());
    for panel in [
        Panel::Clips,
        Panel::Frames,
        Panel::Tileset,
        Panel::AtlasCrop,
    ] {
        assert!(w.dock.find_tab(&panel).is_some(), "{panel:?} open");
    }

    open_farm(&mut w, "copy", ids);
    assert!(w.open.frames_editor.is_none(), "the clips file closed");
    assert!(w.open.tileset_editor.is_none(), "the tileset closed");
    assert!(w.open.atlas_editor.is_none(), "the atlas closed");
    for panel in Panel::OPEN_WITH_A_FILE {
        assert!(w.dock.find_tab(&panel).is_none(), "{panel:?} closed");
    }
    assert!(layout::is_complete(&w.dock));
    // Drawn, nothing of the old project's is saved into the new one.
    let mut h = harness(w);
    h.run();
    h.run();
    let (w, picker) = h.state();
    assert!(picker.actions.is_empty(), "{:?}", picker.actions);
    assert!(w.open.outbox.is_empty(), "{:?}", w.open.outbox);
}

/// Two things print the same: for state that can be printed but not
/// compared.
fn same<T: std::fmt::Debug>(now: &T, fresh: &T, what: &str) {
    assert_eq!(format!("{now:?}"), format!("{fresh:?}"), "{what}");
}

/// The names of the entities in `document`'s area, sorted.
fn names(document: &crate::document::Document) -> Vec<String> {
    let mut names: Vec<String> = document
        .area()
        .iter()
        .map(|(_, e)| e.name.clone())
        .collect();
    names.sort();
    names
}

/// The components `document` knows, by name, sorted.
fn known(document: &crate::document::Document) -> Vec<String> {
    let mut names: Vec<String> = document
        .registry
        .iter()
        .map(|schema| schema.name.clone())
        .collect();
    names.sort();
    names
}

/// `now` holds nothing that `fresh` (the state of a workbench that never
/// had another project open) doesn't. Every field is named, so a field
/// added to `ProjectState` must be given its line here.
fn assert_fresh(now: &ProjectState, fresh: &ProjectState) {
    let ProjectState {
        project,
        document,
        models,
        export_window,
        last_export,
        last_export_folder,
        library,
        assemblies,
        outbox,
        unsaved_prompt,
        pending_reload,
        moving,
        area_panel,
        copied,
        folders,
        library_state,
        sounds,
        sound_events,
        preview,
        sound_shown,
        text_file,
        mixer,
        pixels,
        code_checked,
        play_changes,
        cue_sounds,
        frames_editor,
        animator_editor,
        tileset_editor,
        atlas_editor,
        atlas_texture,
        tile_textures,
        map_edits,
        sheet_pictures,
        sheet_pictures_from,
        place_targets,
        pictures_placing,
        problems,
        playing,
        code_problems,
        program,
        game_components,
        editor_view,
        bindings,
        input_window,
        fixed_seed,
        game_screens,
        screen_pictures,
        screen_doc,
        screen_from,
        screen_game,
        game_scene,
        game_sounds,
        left_out,
        told_code_notes,
        rust_components,
        assembly_part_kept,
        quit_asked,
        preview_forgotten,
    } = now;
    assert_eq!(project, &fresh.project);
    // The area: a new one, saved nowhere, with nothing to undo or redo.
    assert_eq!(names(document), names(&fresh.document));
    assert_eq!(document.file(), None);
    assert!(!document.is_dirty());
    assert_eq!(document.selection.primary(), None);
    assert_eq!(
        document.history().state(),
        fresh.document.history().state(),
        "no undo steps"
    );
    assert_eq!(known(document), known(&fresh.document), "the components");
    assert!(models.is_empty() && library.is_empty() && folders.is_empty());
    assert!(assemblies.is_empty() && cue_sounds.is_empty());
    assert!(export_window.is_none() && last_export.is_none());
    assert_eq!(last_export_folder, &None);
    assert_eq!(outbox, &Vec::new(), "nothing still to be carried out");
    assert_eq!(unsaved_prompt, &None);
    assert!(pending_reload.is_none() && play_changes.is_none());
    assert!(moving.is_none() && copied.is_empty());
    same(area_panel, &fresh.area_panel, "the Area panel");
    assert_eq!(library_state, &fresh.library_state);
    assert!(sounds.is_empty() && sound_events.is_empty());
    assert!(preview.is_none() && sound_shown.is_none() && text_file.is_none());
    same(mixer, &fresh.mixer, "the Mixer");
    assert_eq!(pixels, &fresh.pixels);
    assert_eq!(code_checked, &None);
    assert!(frames_editor.is_none() && animator_editor.is_none());
    assert!(tileset_editor.is_none() && atlas_editor.is_none());
    assert!(atlas_texture.is_none() && tile_textures.is_empty());
    same(
        map_edits,
        &fresh.map_edits,
        "the Map tab and its undo steps",
    );
    assert!(sheet_pictures.is_empty() && sheet_pictures_from.is_empty());
    assert!(place_targets.is_empty() && pictures_placing.is_empty());
    assert!(problems.list.is_empty());
    assert!(playing.is_none() && game_scene.is_none());
    assert!(game_sounds.is_empty() && !quit_asked);
    assert!(code_problems.is_empty() && program.is_none());
    assert!(game_components.is_empty() && left_out.is_empty());
    assert!(told_code_notes.is_empty() && rust_components.is_empty());
    assert_eq!(editor_view, &None);
    assert_eq!(bindings, &fresh.bindings);
    assert!(input_window.is_none());
    assert_eq!(fixed_seed, &None);
    assert!(game_screens.pictures.is_empty() && game_screens.sent.is_empty());
    same(screen_pictures, &fresh.screen_pictures, "screens' pictures");
    assert!(screen_doc.is_none() && screen_game.is_none());
    assert_eq!(screen_from, &None);
    assert!(assembly_part_kept.borrow().is_none());
    assert_eq!(preview_forgotten, &fresh.preview_forgotten);
}

/// A component the program itself registers (as a plugin would): not any
/// project's.
fn wobble() -> reflect::ComponentSchema {
    reflect::ComponentSchema {
        name: "Wobble".into(),
        help: "Wobbles.".into(),
        fields: Vec::new(),
    }
}

/// A workbench as the program starts it, before any project.
fn started() -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.register_component(wobble()).unwrap();
    w
}

/// Opens the farm and uses everything the editor has on it: files handed
/// over, the game's code, a map placed and changed, files open in every
/// kind of tab, values copied, an entity hidden, windows open, a seed
/// chosen, and a play with an edit, stopped and not yet answered.
fn work_on_the_farm(w: &mut Workbench, ids: Ids) {
    open_farm(w, "farm", ids);
    w.set_program(Some(crate::workbench_screens::tests::program(
        crate::screen_doc::tests::MENU,
    )));
    w.set_code_problems(Vec::new(), &[]);
    w.set_pixels(area::pixels::PixelSettings {
        pixels_per_unit: 48,
        ..area::pixels::PixelSettings::default()
    });
    let mut mixer = sound::Mixer::default();
    mixer.buses[0].volume_db = -6.0;
    w.set_mixer(mixer);
    w.set_last_export_folder(Some("/exports/farm".into()));
    w.set_sounds(std::collections::HashMap::from([(
        "moo.ogg".to_string(),
        crate::sound_view::SoundState::Measuring,
    )]));
    // The area: a map placed and changed, hidden, its values copied.
    let placed = w.place_tile_map(ids.map, None).unwrap();
    w.follow_map_selection();
    w.map_requests_done(vec![MapRequest::AddLayer, MapRequest::PaintWith(b'~')]);
    w.copy_components(placed, None);
    w.open.area_panel.hidden.insert(placed);
    w.open
        .place_targets
        .insert(ids.sheet, (glam::Vec3::ONE, None));
    // A file open in every kind of tab.
    w.open_frames(CLIPS);
    w.open_tileset(SET);
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::AtlasCrop("sprites/tools.png".into()),
        &mut actions,
    );
    w.library_request(
        LibraryRequest::Open(LibraryKind::Code, "ui/pauseMenu.lst".into()),
        &mut actions,
    );
    assert!(w.open.frames_editor.is_some() && w.open.tileset_editor.is_some());
    assert!(w.open.atlas_editor.is_some() && w.open.screen_doc.is_some());
    // The Library, windows, a seed.
    w.pick_in_library(MAP);
    w.open_export();
    w.open_input_window();
    w.open.fixed_seed = Some(5);
    // A play with an edit, stopped: Keep or Undo all waits.
    w.play();
    w.open.document.selection.select_only(placed);
    w.delete_selection();
    w.stop();
    assert!(w.play_changes().is_some());
    w.open.unsaved_prompt = Some(WorkbenchAction::NewArea);
    w.offer_reload(area::builtin::starter_area());
    // Drawn, so everything worked out for the farm is.
    let mut h = harness(std::mem::replace(w, started()));
    h.run();
    *w = std::mem::replace(&mut h.state_mut().0, started());
}

#[test]
fn another_project_starts_as_it_would_in_a_workbench_just_started() {
    // Everything of the open project is in `ProjectState`, replaced whole
    // by `set_project`: after any amount of work on one project, another
    // starts exactly as in a workbench that never had one open.
    for next in [Some(project("other")), None] {
        let mut w = started();
        work_on_the_farm(&mut w, Ids::new());
        w.set_project(next.clone());
        let mut fresh = started();
        fresh.set_project(next);
        assert_fresh(&w.open, &fresh.open);
        // What the viewport holds of the project's goes too.
        assert_eq!(*w.viewport.sheets, SpriteSheets::default());
        same(
            &w.viewport.tile_overlay,
            &fresh.viewport.tile_overlay,
            "the selected map and the painter",
        );
        assert!(w.viewport.area.models.is_empty());
        assert_eq!(w.viewport.entities, 0);
        // The layouts keep no tab of the old project's files.
        for dock in std::iter::once(&w.dock).chain(w.other_layouts.values()) {
            for panel in Panel::OPEN_WITH_A_FILE {
                assert!(dock.find_tab(&panel).is_none(), "{panel:?}");
            }
        }
        assert!(layout::is_complete(&w.dock));
        assert_eq!(w.title(), fresh.title());
        // What is the program's stays: its own component, and the Console.
        assert!(w.open.document.registry.get("Wobble").is_some());
        assert!(
            w.log_lines()
                .iter()
                .any(|l| l.text == "Opened the project farm.")
        );
        // And it draws, asking nothing of the app for the old project.
        let mut h = harness(w);
        h.run();
        h.run();
        let (w, picker) = h.state();
        assert_eq!(picker.actions, Vec::new());
        assert_eq!(w.open.outbox, Vec::new());
    }
}

#[test]
fn opening_the_open_project_again_changes_nothing() {
    let ids = Ids::new();
    let mut w = started();
    open_farm(&mut w, "farm", ids);
    let placed = w.place_tile_map(ids.map, None).unwrap();
    w.open_frames(CLIPS);
    w.set_project(Some(project("farm")));
    assert!(w.open.document.area().contains(placed));
    assert!(w.open.document.is_dirty());
    assert!(w.open.frames_editor.is_some());
    assert!(w.dock.find_tab(&Panel::Clips).is_some());
    assert!(w.map_file(MAP).is_some());
}

#[test]
fn a_game_still_running_goes_with_its_project() {
    // Opening a project waits for Stop, so the app never does this; if it
    // did, Stop afterwards would put the old project's area back.
    let mut w = started();
    open_farm(&mut w, "farm", Ids::new());
    w.play();
    w.game_input = true;
    w.set_project(Some(project("other")));
    assert_eq!(w.play_state(), crate::play::PlayState::Editing);
    assert!(!w.game_input, "the mouse and keys are the editor's again");
    w.stop();
    assert_eq!(w.open.project, Some(project("other")));
    assert_eq!(names(&w.open.document), names(&started().open.document));
}
