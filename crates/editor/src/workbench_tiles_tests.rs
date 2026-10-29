//! Tile maps in the whole editor: placed from the Library, the Map tab
//! following the selection as mock-up A has it, its changes saved at once
//! and undone over the tab, new tilesets and maps, a map file's tileset
//! and size from Properties, the viewport's menu of values, and the map
//! the game changes shown while playing.

use super::*;
use crate::library::{LibraryItem, LibraryKind, LibraryRequest, ProjectInfo};
use crate::map_tab::MapRequest;
use crate::sprite_sheets::SpriteSheets;
use crate::test_support::*;
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

/// A workbench with a project holding `farm.soltileset` and
/// `farm.soltilemap`; the map's and tileset's asset IDs.
pub(crate) fn with_map() -> (Workbench, AssetId, AssetId) {
    let mut w = Workbench::new(info(), None);
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
        item("farm.soltileset", LibraryKind::Tileset, set),
        item("farm.soltilemap", LibraryKind::TileMap, map),
    ]);
    let mut sheets = SpriteSheets::default();
    sheets.tilesets.insert(
        "farm.soltileset".into(),
        TilesetState::Read(Arc::new(farm_set()), Vec::new()),
    );
    sheets.tile_maps.insert(
        "farm.soltilemap".into(),
        MapFileState::Read(Arc::new(farm_map(set)), Vec::new()),
    );
    sheets.paths.insert(set, "farm.soltileset".into());
    sheets.paths.insert(map, "farm.soltilemap".into());
    w.set_sprite_sheets(sheets);
    (w, map, set)
}

pub(crate) fn saved_maps(actions: &[WorkbenchAction]) -> Vec<(String, String)> {
    actions
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::SaveTileMap { path, text } => {
                Some((path.to_string_lossy().into_owned(), text.clone()))
            }
            _ => None,
        })
        .collect()
}

pub(crate) fn the_map(w: &Workbench) -> Arc<TileMap> {
    w.map_file("farm.soltilemap").unwrap()
}

#[test]
fn placing_a_map_selects_it_and_opens_the_map_tab_beside_properties() {
    let (mut w, map, _) = with_map();
    let mut actions = Vec::new();
    w.library_request(LibraryRequest::Place(map), &mut actions);
    let placed = w.document.selection.primary().expect("selected");
    let entity = w.document.area().get(placed).unwrap();
    assert_eq!(entity.name, "farm");
    let component = entity.get::<area::builtin::TileMap>().unwrap();
    assert_eq!(component.map.unwrap().asset, map);
    // Its corner on a whole tile.
    let at = entity.get::<area::builtin::Transform>().unwrap().position;
    let [tx, ty] = w.pixels.tile_size();
    assert_eq!((at.x / tx).fract(), 0.0);
    assert_eq!((at.y / ty).fract(), 0.0);
    // The tab beside Properties, as drawn in mock-up A.
    let mut h = harness(w);
    h.run();
    h.run();
    let w = &h.state().0;
    let path = w.dock.find_tab(&Panel::Map).expect("open");
    let tabs = w.dock[path.surface][path.node].tabs().unwrap().to_vec();
    assert!(tabs.contains(&Panel::Properties), "{tabs:?}");
    assert!(layout::is_complete(&w.dock));
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
    h.state_mut().0.document.selection.clear();
    h.run();
    h.run();
    assert!(h.state().0.dock.find_tab(&Panel::Map).is_none());
}

#[test]
fn a_change_in_the_tab_is_saved_at_once_and_undone_and_redone() {
    let (mut w, map, _) = with_map();
    w.place_tile_map(map, None).unwrap();
    w.follow_map_selection();
    w.outbox.clear();
    w.map_requests_done(vec![
        MapRequest::AddLayer,
        MapRequest::RenameValue(1, "age".into()),
    ]);
    let saves = saved_maps(&w.outbox);
    assert_eq!(saves.len(), 2, "each its own step, each saved");
    assert_eq!(saves[0].0, "farm.soltilemap");
    assert!(saves[1].1.contains("name = \"age\";"), "{}", saves[1].1);
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
    let steps = w.map_edits.undo.len();
    w.map_requests_done(vec![
        MapRequest::RenameValue(0, "watered".into()),
        MapRequest::MoveLayer(3, true),
        MapRequest::SetLayer(1, "Ground".into(), false),
    ]);
    assert_eq!(w.map_edits.undo.len(), steps);
}

#[test]
fn hiding_a_layer_is_the_editor_s_alone() {
    let (mut w, map, _) = with_map();
    w.place_tile_map(map, None).unwrap();
    w.follow_map_selection();
    let ground = the_map(&w).layers()[0].id;
    w.outbox.clear();
    w.map_requests_done(vec![MapRequest::ToggleHidden(ground)]);
    assert!(w.map_edits.hidden.contains(&ground));
    assert!(saved_maps(&w.outbox).is_empty(), "never saved");
    w.set_tile_overlay();
    assert!(w.viewport.tile_overlay.hidden.contains(&ground));
    w.map_requests_done(vec![MapRequest::ToggleHidden(ground)]);
    assert!(w.map_edits.hidden.is_empty());
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
    assert_eq!(w.map_edits.shown_values, ["watered", "growth"]);
    assert!(w.map_edits.chunks);
    assert_eq!(w.map_edits.hovered, Some((placed, [5, 5])));
    w.set_tile_overlay();
    assert_eq!(w.viewport.tile_overlay.values, [2, 1]);
    assert!(w.viewport.tile_overlay.chunks);
    // Off again.
    w.viewport.tile_overlay.asked = vec![("watered".into(), false)];
    w.tile_overlay_done();
    assert_eq!(w.map_edits.shown_values, ["growth"]);
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
    let paths: Vec<String> = w
        .outbox
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::SaveTileset { path, text } => {
                assert!(text.contains("grass"), "{text}");
                Some(path.to_string_lossy().into_owned())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        paths,
        ["sprites/tileset.soltileset", "sprites/tileset 2.soltileset"]
    );
    w.outbox.clear();
    w.library_request(LibraryRequest::NewTileMap(String::new()), &mut actions);
    let saves = saved_maps(&w.outbox);
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].0, "map.soltilemap");
    let made = w.map_file("map.soltilemap").unwrap();
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
    let mut w = Workbench::new(info(), None);
    w.new_tile_map("");
    let made = w.map_file("map.soltilemap").unwrap();
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
        LibraryRequest::ShapeTileMap("farm.soltilemap".into(), Some(set), [20, 10]),
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
        "bare.soltilemap".into(),
        MapFileState::Read(Arc::new(TileMap::new([4, 4], None, None)), Vec::new()),
    );
    w.set_sprite_sheets(sheets);
    w.library_request(
        LibraryRequest::ShapeTileMap("bare.soltilemap".into(), Some(set), [4, 4]),
        &mut actions,
    );
    let bare = w.map_file("bare.soltilemap").unwrap();
    assert_eq!(bare.tileset, Some(set));
    assert_eq!(bare.counts(0)[usize::from(b'.')], 16);
    // A tileset's terrains changed in Properties are saved.
    let mut changed = farm_set();
    changed.terrains[0].name = "meadow".into();
    w.outbox.clear();
    w.library_request(
        LibraryRequest::SaveTileset("farm.soltileset".into(), changed),
        &mut actions,
    );
    assert!(w.outbox.iter().any(|a| matches!(
        a,
        WorkbenchAction::SaveTileset { text, .. } if text.contains("meadow")
    )));
    assert_eq!(
        w.viewport.sheets.tileset(set).unwrap().terrains[0].name,
        "meadow"
    );
}

#[test]
fn properties_show_a_tileset_and_a_map_file_from_the_library() {
    let (mut w, _, _) = with_map();
    let selection = w.document.selection.clone();
    w.sound_shown = Some(("farm.soltileset".into(), selection.clone()));
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(h.query_by_label("Remove grass").is_some());
    assert!(h.query_by_label("+ Terrain").is_some());
    h.state_mut().0.sound_shown = Some(("farm.soltilemap".into(), selection));
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
    w.playing
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
