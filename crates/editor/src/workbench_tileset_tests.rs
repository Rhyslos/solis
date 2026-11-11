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

/// A project with `farm.tileset.lst` (tilled 16-tile, its art on
/// `tiles.png`, every slot but 6), `tiles.png` read and `farm.tilemap.lst`.
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
        item("farm.tileset.lst", LibraryKind::Tileset, set_id),
        item("farm.tilemap.lst", LibraryKind::TileMap, map_id),
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
        "farm.tileset.lst".into(),
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
        "farm.tilemap.lst".into(),
        MapFileState::Read(Arc::new(map), Vec::new()),
    );
    sheets.paths.insert(set_id, "farm.tileset.lst".into());
    sheets.paths.insert(map_id, "farm.tilemap.lst".into());
    sheets.paths.insert(png, "tiles.png".into());
    w.set_sprite_sheets(sheets);
    (w, set_id, map_id)
}

fn saved(actions: &[WorkbenchAction]) -> Vec<String> {
    saved_files(actions, |f| matches!(f, crate::OwnFile::Tileset(_)))
        .into_iter()
        .map(|(_, text)| text)
        .collect()
}

#[test]
fn double_clicking_a_tileset_opens_its_tab_first_beside_the_viewport() {
    let (mut w, _, _) = with_tileset();
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::Open(LibraryKind::Tileset, "farm.tileset.lst".into()),
        &mut actions,
    );
    assert!(w.open.tilesets.tab().is_some());
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
        "farm.tileset.lst",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    assert!(h.query_by_label_contains("1 missing").is_some());
    // A saved layout never keeps it, and its document tab closes it.
    let restored = layout::restore(Some(h.state().0.dock.clone()));
    assert!(restored.find_tab(&Panel::Tileset).is_none());
    h.get_by_label("Close farm.tileset.lst").click();
    h.run();
    h.run();
    assert!(h.state().0.open.tilesets.tab().is_none());
}

#[test]
fn a_change_is_saved_shown_at_once_and_undone_over_the_tab() {
    let (mut w, set_id, _) = with_tileset();
    w.open_tileset("farm.tileset.lst");
    w.open.tilesets.tab_mut().unwrap().assign(6, [2, 1]);
    w.save_open_tileset();
    let texts = saved(&w.open.outbox);
    assert_eq!(texts.len(), 1);
    assert!(texts[0].contains("slot = 6"), "{}", texts[0]);
    let set = w.viewport.sheets.tileset(set_id).unwrap();
    assert!(tiles::autotile::missing_slots(set, &set.terrains[1]).is_empty());
    save_as_the_app(&mut w, &mut SpriteSheets::default());
    w.tileset_undo(FARM_SET, false, true);
    assert!(!saved(&w.open.outbox)[0].contains("slot = 6"));
    // Saved by the app, and read back: nothing changes. A change by hand
    // is taken in.
    let mut sheets = (*w.viewport.sheets).clone();
    save_as_the_app(&mut w, &mut sheets);
    w.set_sprite_sheets(sheets.clone());
    assert!(w.console.lines().iter().all(|l| !l.text.contains("disk")));
    let mut by_hand = w.open.tilesets.tab().unwrap().file.clone();
    by_hand.tile = [16, 16];
    sheets.tilesets.insert(
        "farm.tileset.lst".into(),
        TilesetState::Read(Arc::new(by_hand), Vec::new()),
    );
    w.set_sprite_sheets(sheets.clone());
    assert_eq!(w.open.tilesets.tab().unwrap().file.tile, [16, 16]);
    assert!(
        w.console
            .lines()
            .iter()
            .any(|l| l.text.contains("changed on disk"))
    );
    // Gone from disk: the tab closes.
    sheets.tilesets.clear();
    w.set_sprite_sheets(sheets);
    assert!(w.open.tilesets.tab().is_none());
}

#[test]
fn a_new_tileset_opens_in_its_tab_and_stays_until_it_s_read() {
    let (mut w, _, _) = with_tileset();
    w.new_tileset("sprites");
    let e = w.open.tilesets.tab().unwrap();
    assert_eq!(e.path, "sprites/tileset.tileset.lst");
    // A read of the project from before it was written doesn't close it.
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.tilesets.remove("sprites/tileset.tileset.lst");
    w.set_sprite_sheets(sheets);
    assert!(w.open.tilesets.tab().is_some());
}

#[test]
fn a_template_is_saved_beside_the_tileset_as_a_png() {
    let (mut w, _, _) = with_tileset();
    w.open_tileset("farm.tileset.lst");
    w.save_template(TileKind::Blob47);
    let (path, bytes) = w
        .open
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
        .open
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

const FARM_SET: &str = "farm.tileset.lst";

/// The farm's tileset with water made a terrain of its own rules, with a
/// rule and a tile, beside tilled's fourteen tiles. Returns the tileset's
/// ID and its file's text with it.
fn with_rules() -> (Workbench, AssetId, String) {
    let (mut w, set_id, _) = with_tileset();
    let mut set = (**w.viewport.sheets.tileset(set_id).unwrap()).clone();
    let water = set.terrains[2].id;
    set.terrains[2].kind = TileKind::Rules;
    let mut pattern = vec![tiles::Cond::Any; 9];
    pattern[4] = tiles::Cond::Same;
    set.rules.push(tiles::Rule {
        id: reflect::Id::random(),
        terrain: water,
        size: 3,
        pattern,
        slot: 0,
    });
    set.pieces.push(tiles::Piece {
        id: reflect::Id::random(),
        terrain: water,
        slot: 0,
        at: [3, 3],
        weight: 1.0,
        animation: tiles::Animation::None,
    });
    let text = tiles::tileset::write(&set);
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.tilesets.insert(
        FARM_SET.into(),
        TilesetState::Read(Arc::new(set), Vec::new()),
    );
    w.set_sprite_sheets(sheets);
    (w, set_id, text)
}

/// The whole editor with the tileset picked in the Library, so Properties
/// shows its terrains, and the pointer resting over Properties.
fn properties_showing(mut w: Workbench) -> egui_kittest::Harness<'static, State> {
    let selection = w.open.document.selection.clone();
    w.open.sound_shown = Some((FARM_SET.into(), selection));
    rest_over(harness(w), Panel::Properties)
}

/// The tileset's file as last saved since `from` actions were taken.
fn last_saved(h: &egui_kittest::Harness<'static, State>, from: usize) -> Option<String> {
    saved(&h.state().1.actions[from..]).pop()
}

#[test]
fn a_terrain_removed_in_properties_comes_back_whole_with_ctrl_z() {
    // The review (BUG-17): the × beside a terrain in a tileset's Properties
    // wrote its tiles and rules out of the file, and Ctrl+Z did nothing.
    let (w, set_id, whole) = with_rules();
    let area_before = w.open.document.area().len();
    let mut h = properties_showing(w);
    let from = h.state().1.actions.len();
    h.get_by_label("Remove water").click();
    h.run();
    let without = last_saved(&h, from).expect("saved");
    let left = tiles::tileset::read(&without).unwrap().tileset;
    assert_eq!(left.terrains.len(), 2, "water went");
    assert_eq!(
        (left.pieces.len(), left.rules.len()),
        (14, 0),
        "with its art"
    );
    // Undone with the pointer still over Properties: the file is back
    // exactly, the terrain's tile and rule with it.
    let from = h.state().1.actions.len();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(last_saved(&h, from).as_deref(), Some(whole.as_str()));
    let set = h.state().0.viewport.sheets.tileset(set_id).unwrap();
    assert_eq!(tiles::tileset::write(set), whole, "shown at once too");
    // Redone: gone again.
    let from = h.state().1.actions.len();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    h.run();
    assert_eq!(last_saved(&h, from), Some(without));
    assert_eq!(
        h.state().0.open.document.area().len(),
        area_before,
        "the area's own steps are untouched"
    );
}

/// Asks for `edit` as the tileset's Properties does.
fn edit_in_properties(w: &mut Workbench, edit: crate::map_tab::TerrainEdit) {
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::EditTileset(FARM_SET.into(), edit),
        &mut actions,
    );
    assert!(actions.is_empty());
}

/// Takes the tileset's saves waiting for the app, oldest first, each
/// saved as the app would.
fn take_saved(w: &mut Workbench) -> Vec<String> {
    let texts = saved(&w.open.outbox);
    save_as_the_app(w, &mut SpriteSheets::default());
    texts
}

#[test]
fn each_change_in_properties_is_one_undo_step_that_puts_the_file_back_exactly() {
    use crate::map_tab::TerrainEdit;
    let (mut w, set_id, whole) = with_rules();
    let water = w.viewport.sheets.tileset(set_id).unwrap().terrains[2].id;
    let set_it = |name: Option<&str>, code, colour| TerrainEdit::Set {
        id: water,
        name: name.map(str::to_owned),
        code,
        colour,
    };
    for (edit, what) in [
        (TerrainEdit::Add, "a terrain added"),
        (TerrainEdit::Remove(water), "a terrain removed"),
        (set_it(Some("lake"), None, None), "renamed"),
        (set_it(None, Some('w'), None), "its code"),
        (set_it(None, None, Some([1.0, 0.5, 0.0])), "its colour"),
    ] {
        edit_in_properties(&mut w, edit);
        let changed = take_saved(&mut w);
        assert_eq!(changed.len(), 1, "{what}: saved once");
        assert_ne!(changed[0], whole, "{what}: changed");
        let shown = w.viewport.sheets.tileset(set_id).unwrap();
        assert_eq!(tiles::tileset::write(shown), changed[0], "{what}: shown");
        w.tileset_undo(FARM_SET, false, true);
        assert_eq!(
            take_saved(&mut w),
            std::slice::from_ref(&whole),
            "{what}: undone"
        );
        w.tileset_undo(FARM_SET, true, false);
        assert_eq!(take_saved(&mut w), changed, "{what}: redone");
        w.tileset_undo(FARM_SET, false, true);
        assert_eq!(
            take_saved(&mut w),
            std::slice::from_ref(&whole),
            "{what}: undone again"
        );
    }
    // What the editor refuses is no step and no save: a name or a code
    // another terrain has, a code a map file can't write.
    for refused in [
        set_it(Some("grass"), None, None),
        set_it(None, Some('.'), None),
        set_it(None, Some(' '), None),
        set_it(None, None, None),
        TerrainEdit::Remove(reflect::Id::random()),
    ] {
        edit_in_properties(&mut w, refused.clone());
        assert_eq!(take_saved(&mut w), Vec::<String>::new(), "{refused:?}");
    }
    w.tileset_undo(FARM_SET, false, true);
    assert_eq!(take_saved(&mut w), Vec::<String>::new(), "no step left");
}

#[test]
fn dragging_a_terrain_s_colour_in_properties_is_one_step_and_one_save() {
    // The review (BUG-17): the file was written every frame of the drag.
    let (w, set_id, whole) = with_rules();
    let grass = w.viewport.sheets.tileset(set_id).unwrap().terrains[0].colour;
    // The tileset's Properties alone, with what they ask for made by the
    // workbench as in the editor.
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(460.0, 640.0))
        .build_ui_state(
            |ui, w: &mut Workbench| {
                let sheets = Arc::clone(&w.viewport.sheets);
                let Some(TilesetState::Read(set, notes)) = sheets.tilesets.get(FARM_SET) else {
                    return;
                };
                for edit in crate::map_tab::tileset_properties(ui, FARM_SET, set, notes) {
                    edit_in_properties(w, edit);
                }
            },
            w,
        );
    h.run();
    // Grass's colour button opens the picker under it.
    let button = h
        .get_all_by_role(egui::accesskit::Role::ColorWell)
        .next()
        .expect("a colour button")
        .rect();
    h.get_all_by_role(egui::accesskit::Role::ColorWell)
        .next()
        .unwrap()
        .click();
    h.run();
    // A drag across the picker's square, a frame at a time.
    let from = button.left_bottom() + egui::vec2(120.0, 160.0);
    h.drag_at(from);
    h.run();
    for frame in 1..=8 {
        h.hover_at(from + egui::vec2(8.0, 6.0) * frame as f32);
        h.run();
        assert!(h.state().open.outbox.is_empty(), "nothing saved mid-drag");
    }
    h.drop_at(from + egui::vec2(70.0, 54.0));
    h.run();
    let w = h.state_mut();
    assert_eq!(take_saved(w).len(), 1, "saved once, when let go");
    let now = w.viewport.sheets.tileset(set_id).unwrap().terrains[0].colour;
    assert_ne!(now, grass, "the colour changed");
    // One step: one undo puts the file back, and there's no second.
    w.tileset_undo(FARM_SET, false, true);
    assert_eq!(take_saved(w), [whole]);
    w.tileset_undo(FARM_SET, false, true);
    assert_eq!(take_saved(w), Vec::<String>::new());
}

#[test]
fn a_change_in_properties_is_undone_over_the_tileset_tab_and_the_other_way_round() {
    let (mut w, set_id, whole) = with_rules();
    let water = w.viewport.sheets.tileset(set_id).unwrap().terrains[2].id;
    // Removed in Properties while the tab is open on the same file: the
    // tab shows it at once, and Ctrl+Z over the tab brings it back.
    w.open_tileset(FARM_SET);
    edit_in_properties(&mut w, crate::map_tab::TerrainEdit::Remove(water));
    assert_eq!(w.open.tilesets.tab().unwrap().file.terrains.len(), 2);
    let mut h = rest_over(harness(w), Panel::Tileset);
    let from = h.state().1.actions.len();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(last_saved(&h, from).as_deref(), Some(whole.as_str()));

    // Changed in the tab, the tab then closed (its steps are the file's,
    // not the tab's): Ctrl+Z over Properties showing the tileset undoes it.
    let w = &mut h.state_mut().0;
    assert!(w.open.tilesets.tab_mut().unwrap().assign(6, [2, 1]));
    w.save_open_tileset();
    w.close_tileset();
    let selection = w.open.document.selection.clone();
    w.open.sound_shown = Some((FARM_SET.into(), selection));
    let mut h = rest_over(h, Panel::Properties);
    assert!(h.query_by_label("Remove water").is_some(), "Properties");
    let from = h.state().1.actions.len();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(last_saved(&h, from).as_deref(), Some(whole.as_str()));
}

#[test]
fn ctrl_z_over_properties_showing_a_tileset_is_never_the_area_s() {
    // Nothing of the tileset's to undo: the area's last step stays, as it
    // would over the Tileset tab. Elsewhere it's the area's as always.
    let (mut w, _, _) = with_rules();
    w.spawn_selected(area::EntityData::new("Crate"));
    let entities = w.open.document.area().len();
    let mut h = properties_showing(w);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(h.state().0.open.document.area().len(), entities);
    assert!(saved(&h.state().1.actions).is_empty());
    // Every other key over Properties is the area's still.
    h.key_press(egui::Key::Delete);
    h.run();
    assert_eq!(h.state().0.open.document.area().len(), entities - 1);
    let mut h = rest_over(h, Panel::Viewport);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(h.state().0.open.document.area().len(), entities);
}

#[test]
fn solid_set_in_the_map_tab_is_a_step_of_the_tileset_s() {
    let (mut w, set_id, map_id) = with_tileset();
    w.place_tile_map(map_id, None).unwrap();
    w.follow_map_selection();
    let whole = tiles::tileset::write(w.viewport.sheets.tileset(set_id).unwrap());
    w.map_requests_done(vec![crate::map_tab::MapRequest::SetSolid(b'~', true)]);
    let solid = take_saved(&mut w);
    assert!(solid[0].contains("solid = true"), "{}", solid[0]);
    // The tab opened afterwards has the change, and its undo takes it back.
    w.open_tileset(FARM_SET);
    assert!(w.open.tilesets.tab().unwrap().file.terrains[2].solid);
    w.tileset_undo(FARM_SET, false, true);
    assert_eq!(take_saved(&mut w), [whole]);
}

#[test]
fn a_tileset_s_steps_follow_its_file_on_disk_and_go_with_it() {
    let (mut w, set_id, whole) = with_rules();
    let water = w.viewport.sheets.tileset(set_id).unwrap().terrains[2].id;
    edit_in_properties(&mut w, crate::map_tab::TerrainEdit::Remove(water));
    let without = take_saved(&mut w).remove(0);
    // Solis's own save read back is no step; the file changed by hand is
    // one, though no tab shows it.
    let sheets = (*w.viewport.sheets).clone();
    w.set_sprite_sheets(sheets.clone());
    let mut by_hand = sheets.clone();
    let mut set = (**sheets.tileset(set_id).unwrap()).clone();
    set.tile = [16, 16];
    let edited = tiles::tileset::write(&set);
    by_hand.tilesets.insert(
        FARM_SET.into(),
        TilesetState::Read(Arc::new(set), Vec::new()),
    );
    w.set_sprite_sheets(by_hand);
    w.tileset_undo(FARM_SET, false, true);
    assert_eq!(take_saved(&mut w), [without], "the change by hand undone");
    w.tileset_undo(FARM_SET, false, true);
    assert_eq!(take_saved(&mut w), [whole], "then the terrain is back");
    w.tileset_undo(FARM_SET, true, false);
    w.tileset_undo(FARM_SET, true, false);
    assert_eq!(take_saved(&mut w).pop(), Some(edited));
    // The file gone from the project (deleted or moved): its steps go too,
    // and aren't carried out on a file that later takes its name.
    let mut gone = sheets.clone();
    gone.tilesets.clear();
    w.set_sprite_sheets(gone);
    w.set_sprite_sheets(sheets);
    w.tileset_undo(FARM_SET, false, true);
    assert_eq!(take_saved(&mut w), Vec::<String>::new());
}

/// The × on the Tileset tab closes the tileset's tab; its undo steps stay
/// the file's.
#[test]
fn the_x_on_its_tab_closes_the_tileset() {
    let (mut w, _, _) = with_tileset();
    w.open_tileset("farm.tileset.lst");
    let mut h = harness(w);
    click_the_x(&mut h, Panel::Tileset);
    let w = &h.state().0;
    assert!(w.open.tilesets.tab().is_none());
    assert!(w.dock.find_tab(&Panel::Tileset).is_none());
}
