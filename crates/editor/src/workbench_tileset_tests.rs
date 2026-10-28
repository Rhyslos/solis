//! The Tileset tab in the whole editor: opened from the Library beside the
//! viewport, saved at once and shown on every map, undo over the tab, a
//! file changed on disk, templates saved, and maps whose tileset misses
//! tiles listed in Problems.

use super::*;
use crate::library::{LibraryItem, LibraryKind, LibraryRequest, ProjectInfo};
use crate::sprite_sheets::{ArtState, MapFileState, SpriteSheets};
use crate::test_support::*;
use assets::tile_art::TileArt;
use egui_kittest::kittest::Queryable;
use reflect::AssetId;

/// A project with `farm.soltileset` (tilled 16-tile, its art on
/// `tiles.png`, every slot but 6), `tiles.png` read and `farm.soltilemap`.
fn with_tileset() -> (Workbench, AssetId, AssetId) {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    let (set_id, map_id, png) = (AssetId::random(), AssetId::random(), AssetId::random());
    let item = |path: &str, kind, id| LibraryItem {
        id: Some(id),
        kind,
        name: crate::sprite_sheets::name_of(path),
        path: path.into(),
        thumbnail: None,
    };
    w.set_library(vec![
        item("farm.soltileset", LibraryKind::Tileset, set_id),
        item("farm.soltilemap", LibraryKind::TileMap, map_id),
        item("tiles.png", LibraryKind::Texture, png),
    ]);
    let mut set = crate::workbench_tiles::starter_tileset();
    set.terrains[1].kind = TileKind::Dual16;
    set.picture = Some(png);
    set.tile = [8, 8];
    let tilled = set.terrains[1].id;
    for slot in (1..16).filter(|&s| s != 6) {
        set.pieces.push(tiles::Piece {
            id: reflect::Id::random(),
            terrain: tilled,
            slot,
            at: [slot % 4, slot / 4],
            weight: 1.0,
            animation: tiles::Animation::None,
        });
    }
    let mut sheets = SpriteSheets::default();
    sheets.tilesets.insert(
        "farm.soltileset".into(),
        TilesetState::Read(Arc::new(set), Vec::new()),
    );
    sheets.tile_art.insert(
        "tiles.png".into(),
        ArtState::Read(Arc::new(TileArt {
            size: [32, 32],
            frames: vec![vec![200; 32 * 32 * 4]],
            durations: vec![100],
        })),
    );
    let mut map = tiles::TileMap::new([8, 8], Some(set_id), Some(b'.'));
    map.set_code(0, 2, 2, b't').unwrap();
    sheets.tile_maps.insert(
        "farm.soltilemap".into(),
        MapFileState::Read(Arc::new(map), Vec::new()),
    );
    sheets.paths.insert(set_id, "farm.soltileset".into());
    sheets.paths.insert(map_id, "farm.soltilemap".into());
    sheets.paths.insert(png, "tiles.png".into());
    w.set_sprite_sheets(sheets);
    (w, set_id, map_id)
}

fn saved(actions: &[WorkbenchAction]) -> Vec<String> {
    actions
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::SaveTileset { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn double_clicking_a_tileset_opens_its_tab_first_beside_the_viewport() {
    let (mut w, _, _) = with_tileset();
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::Open(LibraryKind::Tileset, "farm.soltileset".into()),
        &mut actions,
    );
    assert!(w.tileset_editor.is_some());
    let path = w.dock.find_tab(&Panel::Tileset).expect("open");
    let tabs = w.dock[path.surface][path.node].tabs().unwrap().to_vec();
    assert_eq!(tabs[0], Panel::Tileset);
    assert!(tabs.contains(&Panel::Viewport));
    let mut h = harness(w);
    h.run();
    h.run();
    for label in [
        "Terrain tilled",
        "Tileset picture",
        "Slot 6",
        "Try-it patch",
        "farm.soltileset",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    assert!(h.query_by_label_contains("1 missing").is_some());
    // A saved layout never keeps it, and its document tab closes it.
    let restored = layout::restore(Some(h.state().0.dock.clone()));
    assert!(restored.find_tab(&Panel::Tileset).is_none());
    h.get_by_label("Close farm.soltileset").click();
    h.run();
    h.run();
    assert!(h.state().0.tileset_editor.is_none());
}

#[test]
fn a_change_is_saved_shown_at_once_and_undone_over_the_tab() {
    let (mut w, set_id, _) = with_tileset();
    w.open_tileset("farm.soltileset");
    w.tileset_editor.as_mut().unwrap().assign(6, [2, 1]);
    w.save_open_tileset();
    let texts = saved(&w.outbox);
    assert_eq!(texts.len(), 1);
    assert!(texts[0].contains("slot = 6;"), "{}", texts[0]);
    let set = w.viewport.sheets.tileset(set_id).unwrap();
    assert!(tiles::autotile::missing_slots(set, &set.terrains[1]).is_empty());
    w.outbox.clear();
    w.tileset_hovered = true;
    w.tileset_undo(false, true);
    assert!(!saved(&w.outbox)[0].contains("slot = 6;"));
    // Its own save read back changes nothing; a change by hand is taken in.
    let mut sheets = (*w.viewport.sheets).clone();
    let mut by_hand = w.tileset_editor.as_ref().unwrap().file.clone();
    by_hand.tile = [16, 16];
    sheets.tilesets.insert(
        "farm.soltileset".into(),
        TilesetState::Read(Arc::new(by_hand), Vec::new()),
    );
    w.set_sprite_sheets(sheets.clone());
    assert_eq!(w.tileset_editor.as_ref().unwrap().file.tile, [16, 16]);
    assert!(
        w.console
            .lines()
            .iter()
            .any(|l| l.text.contains("changed on disk"))
    );
    // Gone from disk: the tab closes.
    sheets.tilesets.clear();
    w.set_sprite_sheets(sheets);
    assert!(w.tileset_editor.is_none());
}

#[test]
fn a_new_tileset_opens_in_its_tab_and_stays_until_it_s_read() {
    let (mut w, _, _) = with_tileset();
    w.new_tileset("sprites");
    let e = w.tileset_editor.as_ref().unwrap();
    assert_eq!(e.path, "sprites/tileset.soltileset");
    // A read of the project from before it was written doesn't close it.
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.tilesets.remove("sprites/tileset.soltileset");
    w.set_sprite_sheets(sheets);
    assert!(w.tileset_editor.is_some());
}

#[test]
fn a_template_is_saved_beside_the_tileset_as_a_png() {
    let (mut w, _, _) = with_tileset();
    w.open_tileset("farm.soltileset");
    w.save_template(TileKind::Blob47);
    let (path, bytes) = w
        .outbox
        .iter()
        .find_map(|a| match a {
            WorkbenchAction::SavePicture { path, bytes } => Some((path.clone(), bytes.clone())),
            _ => None,
        })
        .expect("saved");
    assert_eq!(path, std::path::PathBuf::from("47-tile template.png"));
    let art = assets::tile_art::read_tile_art(&bytes, false).unwrap();
    assert_eq!(art.size, [8 * 8, 6 * 8], "8 × 6 slots of 8 pixels");
}

#[test]
fn a_map_whose_tileset_misses_tiles_is_in_problems_and_drawn_tile_by_tile() {
    let (mut w, _, map_id) = with_tileset();
    let placed = w.place_tile_map(map_id, None).unwrap();
    w.find_problems();
    let found: Vec<&crate::problems::Problem> = w
        .problems
        .list
        .iter()
        .filter(|p| p.kind == crate::problems::ProblemKind::MissingTiles)
        .collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].entity, placed);
    assert!(
        found[0].text.contains("tilled needs slot 6"),
        "{}",
        found[0].text
    );
    // The viewport draws it from the picture: one atlas, tiles not quads.
    let mut h = harness(w);
    h.run();
    h.run();
    let w = &h.state().0;
    let view = w.viewport.view([400, 300], 1.0);
    assert_eq!(view.tile_pictures.len(), 1, "the picture's atlas");
    let tiles: usize = view
        .sprites
        .iter()
        .filter(|b| b.sheet == view.tile_pictures[0].id)
        .map(|b| b.sprites.len())
        .sum();
    // 63 grass cells in their colour, 4 tilled corners round the other.
    assert_eq!(tiles, 63 + 4);
}
