//! Tile maps in the whole editor: placed from the Library, the Map tab
//! following the selection as mock-up A has it, its changes saved at once
//! and undone over the tab, new tilesets and maps, a map file's tileset
//! and size from Properties, the viewport's menu of values, and the map
//! the game changes shown while playing.

use super::*;
use crate::library::{LibraryItem, LibraryKind, LibraryRequest, ProjectInfo};
use crate::map_tab::MapRequest;
use crate::sprite_sheets::{SpriteSheets, TilesetState};
use crate::test_support::*;
use crate::workbench::WorkbenchAction;
use egui_kittest::kittest::Queryable;
use tiles::{CellValue, ValueKind};

/// grass `.`, tilled `t`, water `~`.
fn farm_set() -> Tileset {
    starter_tileset()
}

/// A 40 × 36 map of grass with a tilled patch and crop, growth and
/// watered values.
pub(crate) fn farm_map(tileset: AssetId) -> TileMap {
    let mut map = TileMap::new([40, 36], Some(tileset), Some(b'.'));
    for y in 4..8 {
        for x in 4..10 {
            map.set_code(0, x, y, b't').unwrap();
        }
    }
    map.add_value(
        "crop",
        ValueKind::Choice(vec!["wheat".into(), "corn".into()]),
    )
    .unwrap();
    map.add_value("growth", ValueKind::Number).unwrap();
    map.add_value("watered", ValueKind::Switch).unwrap();
    map.set_value(0, 5, 5, CellValue::Choice(Some(1))).unwrap();
    map.set_value(1, 5, 5, CellValue::Number(3)).unwrap();
    map.set_value(2, 5, 5, CellValue::Switch(true)).unwrap();
    map
}

/// A workbench with a project holding `farm.tileset.lst` and
/// `farm.tilemap.lst`; the map's and tileset's asset IDs.
pub(crate) fn with_map() -> (Workbench, AssetId, AssetId) {
    let mut w = Workbench::new(info());
    w.set_project(Some(ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    let (set, map) = (AssetId::random(), AssetId::random());
    let item = |path: &str, kind, id| LibraryItem {
        id: Some(id),
        kind,
        name: crate::sprite_sheets::name_of(path),
        path: path.into(),
        thumbnail: None,
    };
    w.set_library(vec![
        item("farm.tileset.lst", LibraryKind::Tileset, set),
        item("farm.tilemap.lst", LibraryKind::TileMap, map),
    ]);
    let mut sheets = SpriteSheets::default();
    sheets.tilesets.insert(
        "farm.tileset.lst".into(),
        TilesetState::Read(Arc::new(farm_set()), Vec::new()),
    );
    sheets.tile_maps.insert(
        "farm.tilemap.lst".into(),
        MapFileState::Read(Arc::new(farm_map(set)), Vec::new()),
    );
    sheets.paths.insert(set, "farm.tileset.lst".into());
    sheets.paths.insert(map, "farm.tilemap.lst".into());
    w.set_sprite_sheets(sheets);
    (w, map, set)
}

pub(crate) fn saved_maps(actions: &[WorkbenchAction]) -> Vec<(String, String)> {
    saved_files(actions, |f| matches!(f, crate::OwnFile::TileMap(_)))
}

/// The tilesets saved among `actions`: each one's path and text.
fn saved_tilesets(actions: &[WorkbenchAction]) -> Vec<(String, String)> {
    saved_files(actions, |f| matches!(f, crate::OwnFile::Tileset(_)))
}

pub(crate) fn the_map(w: &Workbench) -> Arc<TileMap> {
    w.map_file("farm.tilemap.lst").unwrap()
}

#[test]
fn placing_a_map_selects_it_and_opens_the_map_tab_beside_properties() {
    let (mut w, map, _) = with_map();
    let mut actions = Vec::new();
    w.library_request(LibraryRequest::Place(map), &mut actions);
    let placed = w.open.document.selection.primary().expect("selected");
    let entity = w.open.document.area().get(placed).unwrap();
    assert_eq!(entity.name, "farm");
    let component = entity.get::<area::builtin::TileMap>().unwrap();
    assert_eq!(component.map.unwrap().asset, map);
    // Its corner on a whole tile.
    let at = entity.get::<area::builtin::Transform>().unwrap().position;
    let [tx, ty] = w.open.pixels.tile_size();
    assert_eq!((at.x / tx).fract(), 0.0);
    assert_eq!((at.y / ty).fract(), 0.0);
    // Home's Map shelf tab, in front.
    let mut h = harness(w);
    h.run();
    h.run();
    let w = &h.state().0;
    assert_eq!(w.shelf_tab(), Some(crate::shell::ShelfTab::Map));
    for label in [
        "Add a layer",
        "Add a value",
        "Show Ground",
        "Remove watered",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    assert!(h.query_by_label_contains("40 × 36 cells").is_some());
    // Nothing selected: the tab goes.
    h.state_mut().0.open.document.selection.clear();
    h.run();
    h.run();
    assert!(!has_panel(&h.state().0, crate::panel::Panel::Map));
}

#[test]
fn a_change_in_the_tab_is_saved_at_once_and_undone_and_redone() {
    let (mut w, map, _) = with_map();
    w.place_tile_map(map, None).unwrap();
    w.follow_map_selection();
    w.open.outbox.clear();
    w.map_requests_done(vec![
        MapRequest::AddLayer,
        MapRequest::RenameValue(1, "age".into()),
    ]);
    let saves = saved_maps(&w.open.outbox);
    assert_eq!(saves.len(), 2, "each its own step, each saved");
    assert_eq!(saves[0].0, "farm.tilemap.lst");
    assert!(saves[1].1.contains("name = \"age\""), "{}", saves[1].1);
    let now = the_map(&w);
    assert_eq!(now.layers().len(), 4);
    assert_eq!(now.layers()[3].name, "Layer1");
    assert!(now.value_named("age").is_some());
    // Shown at once: the game's set of maps has it.
    assert_eq!(w.viewport.sheets.tile_map(map).unwrap().layers().len(), 4);
    // Undo twice, redo once.
    w.map_undo(false, true);
    assert!(the_map(&w).value_named("growth").is_some());
    w.map_undo(false, true);
    assert_eq!(the_map(&w).layers().len(), 3);
    w.map_undo(true, false);
    assert_eq!(the_map(&w).layers().len(), 4);
    assert!(the_map(&w).value_named("age").is_none());
    // A change that changes nothing isn't a step: renaming to a name in
    // use, or a layer over the top.
    let steps = w.open.map_edits.undo.len();
    w.map_requests_done(vec![
        MapRequest::RenameValue(0, "watered".into()),
        MapRequest::MoveLayer(3, true),
        MapRequest::SetLayer(1, "Ground".into(), false),
    ]);
    assert_eq!(w.open.map_edits.undo.len(), steps);
}

/// How long the file of the map at `path` is, as it's saved now.
pub(crate) fn file_len(w: &Workbench) -> usize {
    tiles::map_file::write(&the_map(w)).unwrap().len()
}

/// The Console's last line, if it's a warning.
pub(crate) fn last_warning(w: &Workbench) -> Option<&str> {
    let last = w.log_lines().last()?;
    (last.level == LogLevel::Warning).then_some(last.text.as_str())
}

#[test]
fn a_change_that_would_make_a_map_too_big_to_open_again_is_refused() {
    // The review of Phase 9, BUG-55: a map's own limits allow far more
    // than a file that's read back can hold, and the editor saved such a
    // map all the same: it could never be opened again. Here the reader
    // takes files just big enough for the farm with one more layer, to
    // the byte, so the edge is reached with a small map.
    let (mut w, map, set) = with_map();
    w.place_tile_map(map, None).unwrap();
    w.follow_map_selection();
    w.open.outbox.clear();
    let path = "farm.tilemap.lst";
    let start = the_map(&w);
    // (A layer's ID is always as long, so any fourth layer called Layer1
    // makes a file this long.)
    let mut with_a_layer = (*start).clone();
    with_a_layer.add_layer("Layer1", false).unwrap();
    let edge = tiles::map_file::write(&with_a_layer).unwrap().len();
    w.open.map_edits.most_bytes = edge;
    // To the edge: saved, an undo step, and what's saved opens again.
    w.map_requests_done(vec![MapRequest::AddLayer]);
    let at_the_edge = the_map(&w);
    assert_eq!(at_the_edge.layers().len(), 4);
    let saves = saved_maps(&w.open.outbox);
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].1.len(), edge);
    let opened = tiles::map_file::read(&saves[0].1).unwrap();
    assert_eq!(opened.map, *at_the_edge);
    assert_eq!(w.open.map_edits.undo.len(), 1);
    assert_eq!(last_warning(&w), None);
    // One step past, each way a map grows: refused as it's made. Nothing
    // is saved, the map is as it was, and the undo steps are as they were.
    w.open.outbox.clear();
    let said = w.log_lines().len();
    let grown = [
        "another layer",
        "another value",
        "a choice's options",
        "a cell painted on a layer with nothing on it",
        "a cell's value",
        "an exact tile",
        "the map made bigger",
        "a layer's longer name",
    ];
    for (n, what) in grown.into_iter().enumerate() {
        let made = match n {
            0 => w.change_map(path, |m| m.add_layer("Layer2", true).is_ok()),
            1 => w.change_map(path, |m| m.add_value("ripe", ValueKind::Switch).is_ok()),
            2 => w.change_map(path, |m| {
                let options = (0..12).map(|n| format!("kind{n:02}")).collect();
                m.set_value_kind(2, ValueKind::Choice(options))
            }),
            3 => w.change_map(path, |m| m.set_code(1, 2, 2, b'~').unwrap_or(false)),
            4 => w.change_map(path, |m| {
                m.set_value(1, 9, 9, CellValue::Number(5)).unwrap_or(false)
            }),
            5 => w.change_map(path, |m| {
                m.set_exact(0, 1, 1, Some(reflect::Id::random()))
                    .unwrap_or(false)
            }),
            6 => {
                let mut actions = Vec::new();
                let bigger = LibraryRequest::ShapeTileMap(path.into(), Some(set), [70, 36]);
                w.library_request(bigger, &mut actions);
                false
            }
            _ => w.change_map(path, |m| m.set_layer(3, "A much longer name", false)),
        };
        assert!(!made, "{what}");
        assert!(Arc::ptr_eq(&the_map(&w), &at_the_edge), "{what}");
        assert!(saved_maps(&w.open.outbox).is_empty(), "{what}");
        assert_eq!(w.open.map_edits.undo.len(), 1, "{what}");
        assert!(w.open.map_edits.redo.is_empty(), "{what}");
        assert_eq!(w.log_lines().len(), said + n + 1, "{what}");
        let why = last_warning(&w).expect(what);
        assert!(
            why.starts_with("farm.tilemap.lst wasn't changed: saved like that, the map couldn't")
                && why.contains("the map is too big to read")
                && why.contains("It's as it was. Make the map smaller"),
            "{what}: {why}"
        );
    }
    // A change that leaves the file no bigger is still made, at the edge.
    assert!(w.change_map(path, |m| m.set_code(0, 2, 2, b'~').unwrap_or(false)));
    assert_eq!(w.open.map_edits.undo.len(), 2);
    assert_eq!(file_len(&w), edge);
    // And undo goes back through every step, to the map as it began.
    w.map_undo(false, true);
    assert_eq!(*the_map(&w), *at_the_edge);
    w.map_undo(false, true);
    assert_eq!(*the_map(&w), *start);
    assert!(w.open.map_edits.undo.is_empty());
    // A step back or forward to a map that couldn't be opened again (a
    // reader taking less than when the step was made) isn't taken, and
    // stays a step.
    w.open.outbox.clear();
    w.open.map_edits.most_bytes = 10;
    w.map_undo(true, false);
    assert_eq!(*the_map(&w), *start);
    assert_eq!(w.open.map_edits.redo.len(), 2);
    assert!(saved_maps(&w.open.outbox).is_empty());
    assert!(last_warning(&w).is_some_and(|why| why.contains("wasn't changed")));
}

#[test]
fn a_kind_the_map_has_no_room_for_changes_nothing_like_a_value_too_many() {
    // The code review: on a full map a value could still be made a bigger
    // kind, past the map's memory. Refused now, it's shown as a value too
    // many is: the tab as it was, no step to undo, nothing saved. The
    // biggest map, with every layer and as many switches as it has room for.
    let (mut w, map, _) = with_map();
    w.place_tile_map(map, None).unwrap();
    w.follow_map_selection();
    let side = tiles::map::MAX_SIDE;
    let mut full = TileMap::bare([side, side], None);
    for n in 0..tiles::map::MAX_LAYERS {
        full.add_layer(&format!("Layer{n}"), false).unwrap();
    }
    for n in 0..16 {
        full.add_value(&format!("value{n}"), ValueKind::Switch)
            .unwrap();
    }
    let sheets = Arc::make_mut(&mut w.viewport.sheets);
    sheets.tile_maps.insert(
        "farm.tilemap.lst".into(),
        MapFileState::Read(Arc::new(full), Vec::new()),
    );
    w.open.outbox.clear();
    let steps = w.open.map_edits.undo.len();
    w.map_requests_done(vec![
        MapRequest::SetValueKind(0, ValueKind::Number),
        MapRequest::AddValue,
    ]);
    assert_eq!(w.open.map_edits.undo.len(), steps);
    assert!(saved_maps(&w.open.outbox).is_empty());
    let now = the_map(&w);
    assert_eq!(now.values().len(), 16);
    assert_eq!(now.values()[0].kind, ValueKind::Switch);
}

#[test]
fn hiding_a_layer_is_the_editor_s_alone() {
    let (mut w, map, _) = with_map();
    w.place_tile_map(map, None).unwrap();
    w.follow_map_selection();
    let ground = the_map(&w).layers()[0].id;
    w.open.outbox.clear();
    w.map_requests_done(vec![MapRequest::ToggleHidden(ground)]);
    assert!(w.open.map_edits.hidden.contains(&ground));
    assert!(saved_maps(&w.open.outbox).is_empty(), "never saved");
    w.set_tile_overlay();
    assert!(w.viewport.tile_overlay.hidden.contains(&ground));
    w.map_requests_done(vec![MapRequest::ToggleHidden(ground)]);
    assert!(w.open.map_edits.hidden.is_empty());
}

#[test]
fn the_viewport_s_menu_chooses_the_values_laid_over_and_the_chunks() {
    let (mut w, map, _) = with_map();
    let placed = w.place_tile_map(map, None).unwrap();
    w.follow_map_selection();
    w.set_tile_overlay();
    assert!(w.viewport.tile_overlay.values.is_empty());
    let overlay = &mut w.viewport.tile_overlay;
    overlay.asked = vec![("watered".into(), true), ("growth".into(), true)];
    overlay.chunks_asked = true;
    overlay.hovered = Some([5, 5]);
    w.tile_overlay_done();
    assert_eq!(w.open.map_edits.shown_values, ["watered", "growth"]);
    assert!(w.open.map_edits.chunks);
    assert_eq!(w.open.map_edits.hovered, Some((placed, [5, 5])));
    w.set_tile_overlay();
    assert_eq!(w.viewport.tile_overlay.values, [2, 1]);
    assert!(w.viewport.tile_overlay.chunks);
    // Off again.
    w.viewport.tile_overlay.asked = vec![("watered".into(), false)];
    w.tile_overlay_done();
    assert_eq!(w.open.map_edits.shown_values, ["growth"]);
    // Drawn in the whole editor, with the cell inspector in the tab.
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(h.query_by_label_contains("Values: growth").is_some());
    assert!(h.query_by_label("Chunks").is_some());
}

#[test]
fn new_tilesets_and_maps_are_made_saved_and_shown_with_free_names() {
    let (mut w, _, set) = with_map();
    let mut actions = Vec::new();
    w.library_request(LibraryRequest::NewTileset("sprites".into()), &mut actions);
    w.library_request(LibraryRequest::NewTileset("sprites".into()), &mut actions);
    let paths: Vec<String> = saved_tilesets(&w.open.outbox)
        .into_iter()
        .map(|(path, text)| {
            assert!(text.contains("grass"), "{text}");
            path
        })
        .collect();
    assert_eq!(
        paths,
        [
            "sprites/tileset.tileset.lst",
            "sprites/tileset 2.tileset.lst"
        ]
    );
    w.open.outbox.clear();
    w.library_request(LibraryRequest::NewTileMap(String::new()), &mut actions);
    let saves = saved_maps(&w.open.outbox);
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].0, "map.tilemap.lst");
    let made = w.map_file("map.tilemap.lst").unwrap();
    assert_eq!(made.size(), tiles::map_file::STANDARD_SIZE);
    assert_eq!(made.tileset, Some(set), "the project's first tileset");
    assert_eq!(
        made.code(0, 0, 0),
        Some(b'.'),
        "filled with its first terrain"
    );
}

#[test]
fn a_map_with_no_tileset_says_how_to_get_one() {
    let mut w = Workbench::new(info());
    w.new_tile_map("");
    let made = w.map_file("map.tilemap.lst").unwrap();
    assert_eq!(made.tileset, None);
    assert_eq!(made.code(0, 0, 0), Some(0), "empty");
    assert!(
        w.console
            .lines()
            .iter()
            .any(|l| l.text.contains("has no tileset yet"))
    );
}

#[test]
fn properties_change_a_map_file_s_tileset_and_size() {
    let (mut w, _, set) = with_map();
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::ShapeTileMap("farm.tilemap.lst".into(), Some(set), [20, 10]),
        &mut actions,
    );
    let map = the_map(&w);
    assert_eq!(map.size(), [20, 10]);
    assert_eq!(map.code(0, 5, 5), Some(b't'), "cells kept");
    assert_eq!(map.value(1, 5, 5), Some(CellValue::Number(3)));
    // An empty map given a tileset is filled with its first terrain.
    w.new_tile_map("");
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.tile_maps.insert(
        "bare.tilemap.lst".into(),
        MapFileState::Read(Arc::new(TileMap::new([4, 4], None, None)), Vec::new()),
    );
    w.set_sprite_sheets(sheets);
    w.library_request(
        LibraryRequest::ShapeTileMap("bare.tilemap.lst".into(), Some(set), [4, 4]),
        &mut actions,
    );
    let bare = w.map_file("bare.tilemap.lst").unwrap();
    assert_eq!(bare.tileset, Some(set));
    assert_eq!(bare.counts(0)[usize::from(b'.')], 16);
    // A tileset's terrains changed in Properties are saved.
    let grass = w.viewport.sheets.tileset(set).unwrap().terrains[0].id;
    w.open.outbox.clear();
    w.library_request(
        LibraryRequest::EditTileset(
            "farm.tileset.lst".into(),
            crate::map_tab::TerrainEdit::Set {
                id: grass,
                name: Some("meadow".into()),
                code: None,
                colour: None,
            },
        ),
        &mut actions,
    );
    assert!(
        saved_tilesets(&w.open.outbox)
            .iter()
            .any(|(_, text)| text.contains("meadow"))
    );
    assert_eq!(
        w.viewport.sheets.tileset(set).unwrap().terrains[0].name,
        "meadow"
    );
}

#[test]
fn properties_show_a_tileset_and_a_map_file_from_the_library() {
    let (mut w, _, _) = with_map();
    let selection = w.open.document.selection.clone();
    w.open.sound_shown = Some(("farm.tileset.lst".into(), selection.clone()));
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(h.query_by_label("Remove grass").is_some());
    assert!(h.query_by_label("+ Terrain").is_some());
    h.state_mut().0.open.sound_shown = Some(("farm.tilemap.lst".into(), selection));
    h.run();
    h.run();
    assert!(h.query_by_label_contains("3 layers · 3 values").is_some());
}

#[test]
fn while_playing_the_tab_shows_the_game_s_map() {
    let (mut w, map, _) = with_map();
    let placed = w.place_tile_map(map, None).unwrap();
    w.play();
    w.run_game(0.05);
    w.open
        .playing
        .as_mut()
        .unwrap()
        .world
        .set_cell_value(placed, 1, [5, 5], CellValue::Number(9))
        .unwrap();
    let (shown, tileset) = w.map_shown().unwrap();
    assert_eq!(shown.value(1, 5, 5), Some(CellValue::Number(9)));
    assert!(tileset.is_some());
    // The file is untouched.
    assert_eq!(the_map(&w).value(1, 5, 5), Some(CellValue::Number(3)));
    w.set_tile_overlay();
    let overlay = w.viewport.tile_overlay.map.as_ref().unwrap();
    assert_eq!(overlay.map.value(1, 5, 5), Some(CellValue::Number(9)));
}

#[test]
fn a_terrain_made_solid_is_saved_in_its_tileset_and_its_walls_shown() {
    let (mut w, map, set) = with_map();
    w.place_tile_map(map, None).unwrap();
    w.follow_map_selection();
    // Water (`~`) made solid from the Map tab's right-click.
    w.map_requests_done(vec![crate::map_tab::MapRequest::SetSolid(b'~', true)]);
    let saved = w.viewport.sheets.tileset(set).unwrap();
    assert!(saved.by_code('~').unwrap().solid);
    assert!(!saved.by_code('.').unwrap().solid, "grass isn't");
    let (_, text) = saved_tilesets(&w.open.outbox).swap_remove(0);
    assert!(text.contains("solid = true"), "{text}");
    // Asking again changes nothing.
    let before = w.open.outbox.len();
    w.map_requests_done(vec![crate::map_tab::MapRequest::SetSolid(b'~', true)]);
    assert_eq!(w.open.outbox.len(), before);
    // The viewport's Walls switch shows them: the tileset's solid codes.
    w.set_tile_overlay();
    assert!(!w.viewport.tile_overlay.walls);
    assert!(w.viewport.tile_overlay.solid.unwrap()[b'~' as usize]);
    w.viewport.tile_overlay.walls_asked = true;
    w.tile_overlay_done();
    assert!(w.open.map_edits.walls);
    w.set_tile_overlay();
    assert!(w.viewport.tile_overlay.walls);
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(h.query_by_label("Walls").is_some());
    // Nothing solid: nothing to show.
    let w = &mut h.state_mut().0;
    w.map_requests_done(vec![crate::map_tab::MapRequest::SetSolid(b'~', false)]);
    w.set_tile_overlay();
    assert!(w.viewport.tile_overlay.solid.is_none());
}

#[test]
fn game_code_waters_the_farm_through_the_map_s_own_file() {
    let (mut w, map, _) = with_map();
    let placed = w.place_tile_map(map, None).unwrap();
    // The map's own file, as the app writes it, and a component using it.
    let names = w.sprite_sheets().map_names();
    assert_eq!(names.len(), 1);
    let (path, text) = script::map_code::map_file("Test 1.0", "solis", &names[0])
        .0
        .unwrap();
    assert!(
        text.contains("state FarmCrop = [None, Wheat, Corn];"),
        "{text}"
    );
    let mut files = script::describe::descriptions("Test 1.0", "solis");
    files.push((path, text));
    files.push((
        "main.lst".into(),
        "fetch \"solis/maps/farm\":[FarmMap, FarmCrop];\n\
         component Waterer {\n    has {\n        FarmMap farm;\n    }\n\
         \x20   method update(float dt) {\n\
         \x20       is not farm:watered(4, 4)? then {\n            farm:setWatered(4, 4, true);\n\
         \x20           farm:setGrowth(4, 4, farm:growth(5, 5) + 1);\n            farm:setCrop(4, 4, FarmCrop.Wheat);\n        }\n\
         \x20   }\n}\npass Waterer;\nend\n"
            .into(),
    ));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let program = match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    w.set_program(Some(program));
    w.apply_property(
        crate::properties::PropertyRequest::AddComponent {
            entity: placed,
            component: "Waterer".into(),
        },
        1,
    );
    w.play();
    w.run_game(2.0 / 60.0);
    let played = w.world().unwrap().tile_map(placed).unwrap();
    assert_eq!(played.value(2, 4, 4), Some(CellValue::Switch(true)));
    assert_eq!(played.value(1, 4, 4), Some(CellValue::Number(4)));
    assert_eq!(played.value(0, 4, 4), Some(CellValue::Choice(Some(0))));
    // The map file itself is left as it was: only the game's copy changed.
    let file = w.sprite_sheets().tile_map(map).unwrap();
    assert_eq!(file.value(2, 4, 4), Some(CellValue::Switch(false)));
}

#[test]
fn game_code_finds_the_cell_at_a_place_wherever_the_map_is() {
    let (mut w, map, _) = with_map();
    let placed = w.place_tile_map(map, None).unwrap();
    let names = w.sprite_sheets().map_names();
    let (path, text) = script::map_code::map_file("Test 1.0", "solis", &names[0])
        .0
        .unwrap();
    let mut files = script::describe::descriptions("Test 1.0", "solis");
    files.push((path, text));
    // The middle of cell 6, 2 is on the map, and its cell is 6, 2 again,
    // wherever the map was placed; past the map's box is off it.
    files.push((
        "main.lst".into(),
        "fetch \"solis/maps/farm\":FarmMap;\nfetch \"solis/tiles\":[MapCell, MapBounds];\n\
         component Finder {\n    has {\n        FarmMap farm;\n    }\n\
         \x20   method update(float dt) {\n\
         \x20       MapCell cell = farm:cellAt(farm:placeOf(6, 2));\n\
         \x20       MapBounds box = mapBounds();\n\
         \x20       bool off = farm:cellAt(box.high + [1, 1, 0]).inside;\n\
         \x20       is cell.inside && box.found && not off? then {\n            farm:setWatered(cell.x, cell.y, true);\n        }\n\
         \x20   }\n}\npass Finder;\nend\n"
            .into(),
    ));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let program = match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    w.set_program(Some(program));
    w.apply_property(
        crate::properties::PropertyRequest::AddComponent {
            entity: placed,
            component: "Finder".into(),
        },
        1,
    );
    w.play();
    w.run_game(2.0 / 60.0);
    let played = w.world().unwrap().tile_map(placed).unwrap();
    assert_eq!(played.value(2, 6, 2), Some(CellValue::Switch(true)));
    assert_eq!(played.value(2, 2, 6), Some(CellValue::Switch(false)));
}

#[test]
fn big_maps_keep_fewer_undo_steps() {
    // The stability review: fifty steps of a 4096 × 4096 map would hold
    // gigabytes.
    let mut edits = MapEdits::default();
    for _ in 0..30 {
        edits.push_undo("big.tilemap.lst".into(), TileMap::bare([4096, 4096], None));
    }
    assert_eq!(edits.undo.len(), 16);
    let mut small = MapEdits::default();
    for _ in 0..60 {
        small.push_undo("small.tilemap.lst".into(), TileMap::bare([64, 64], None));
    }
    assert_eq!(small.undo.len(), 50, "small maps keep them all");
}
