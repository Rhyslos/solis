//! The Map tab and the tileset and map file Properties on their own: what
//! they show, and what a click asks for.

use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use reflect::{AssetId, Id};

fn set() -> Tileset {
    crate::workbench_tiles::starter_tileset()
}

fn map() -> TileMap {
    let mut map = TileMap::new([8, 6], Some(AssetId::random()), Some(b'.'));
    map.set_code(0, 1, 1, b't').unwrap();
    map.set_code(1, 2, 2, b'#').unwrap();
    map.add_value(
        "crop",
        ValueKind::Choice(vec!["wheat".into(), "corn".into()]),
    )
    .unwrap();
    map.add_value("growth", ValueKind::Number).unwrap();
    map.add_value("watered", ValueKind::Switch).unwrap();
    map.set_value(0, 1, 1, CellValue::Choice(Some(1))).unwrap();
    map.set_value(1, 1, 1, CellValue::Number(4)).unwrap();
    map.set_value(2, 1, 1, CellValue::Switch(true)).unwrap();
    map
}

/// The tab's map, tileset, hidden layers, and what it asked for.
type TabState = (TileMap, Option<Tileset>, HashSet<Id>, Vec<MapRequest>);

/// A map file's, and the tileset and size its Properties chose.
type FileState = (TileMap, Option<(Option<AssetId>, [u32; 2])>);

/// The tab drawn for `map` with the cell `hovered`, and what it asked
/// for in the last frame.
fn tab(
    map: TileMap,
    tileset: Option<Tileset>,
    hovered: Option<[u32; 2]>,
    hidden: HashSet<Id>,
) -> Harness<'static, TabState> {
    Harness::builder()
        .with_size(vec2(360.0, 720.0))
        .build_ui_state(
            move |ui, s: &mut TabState| {
                let shown = MapUi {
                    name: "farm".into(),
                    map: &s.0,
                    tileset: s.1.as_ref(),
                    hovered,
                    hidden: &s.2,
                    playing: false,
                };
                let asked = map_tab(ui, &shown);
                s.3.extend(asked);
            },
            (map, tileset, hidden, Vec::new()),
        )
}

#[test]
fn it_shows_layers_terrains_with_counts_values_and_the_cell() {
    let mut h = tab(map(), Some(set()), Some([1, 1]), HashSet::new());
    h.run();
    for label in [
        "farm",
        "8 × 6 cells",
        "Show Ground",
        "Show Decoration",
        "Show Above",
        "grass .",
        "tilled t",
        "Cell · x 1, y 1",
        "corn",
        "4",
        "✓",
        "Remove crop",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    // 47 grass and 1 tilled on Ground; the fence on Decoration isn't in
    // the tileset.
    assert!(
        h.query_by_label_contains("1 cells have a code the tileset hasn't")
            .is_some()
    );
    assert!(h.query_by_label("\"#\" (not in the tileset)").is_none());
    assert!(h.state().3.is_empty(), "nothing asked by drawing");
}

#[test]
fn clicks_ask_for_changes() {
    let m = map();
    let ground = m.layers()[0].id;
    let mut h = tab(m, Some(set()), None, HashSet::new());
    h.run();
    h.get_by_label("Add a layer").click();
    h.run();
    h.get_by_label("Show Ground").click();
    h.run();
    h.get_by_label("Add a value").click();
    h.run();
    h.get_by_label("Remove growth").click();
    h.run();
    assert_eq!(
        h.state().3,
        [
            MapRequest::AddLayer,
            MapRequest::ToggleHidden(ground),
            MapRequest::AddValue,
            MapRequest::RemoveValue(1),
        ]
    );
    assert!(
        h.query_by_label_contains("Point at a cell in the viewport")
            .is_some()
    );
}

#[test]
fn a_hidden_layer_shows_as_hidden() {
    let m = map();
    let ground = m.layers()[0].id;
    let mut h = tab(m, Some(set()), None, HashSet::from([ground]));
    h.run();
    let eye = h.get_by_label("Show Ground");
    assert_eq!(
        eye.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::False)
    );
    let eye = h.get_by_label("Show Above");
    assert_eq!(
        eye.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
}

#[test]
fn with_no_tileset_or_values_it_says_what_to_do() {
    let mut h = tab(
        TileMap::new([4, 4], None, None),
        None,
        Some([0, 0]),
        HashSet::new(),
    );
    h.run();
    assert!(
        h.query_by_label_contains("No tileset: choose one")
            .is_some()
    );
    assert!(h.query_by_label_contains("None yet. A value").is_some());
    assert!(h.query_by_label("Cell · x 0, y 0").is_some());
}

#[test]
fn a_tileset_s_properties_add_and_remove_terrains() {
    let mut h = Harness::builder()
        .with_size(vec2(360.0, 400.0))
        .build_ui_state(
            |ui, s: &mut (Tileset, Option<Tileset>)| {
                if let Some(changed) = tileset_properties(ui, "sprites/farm.soltileset", &s.0, &[])
                {
                    s.1 = Some(changed);
                }
            },
            (set(), None),
        );
    h.run();
    assert!(h.query_by_label("farm.soltileset").is_some());
    h.get_by_label("+ Terrain").click();
    h.run();
    let added = h.state().1.clone().unwrap();
    assert_eq!(added.terrains.len(), 4);
    assert_eq!(added.terrains[3].name, "terrain");
    assert_ne!(added.terrains[3].code, '.');
    h.state_mut().1 = None;
    h.get_by_label("Remove tilled").click();
    h.run();
    let removed = h.state().1.clone().unwrap();
    assert!(removed.by_name("tilled").is_none());
    assert_eq!(removed.terrains.len(), 2);
}

#[test]
fn a_map_file_s_properties_choose_its_tileset() {
    let (a, b) = (AssetId::random(), AssetId::random());
    let mut m = TileMap::new([10, 8], Some(a), None);
    m.add_value("crop", ValueKind::Number).unwrap();
    let tilesets = vec![("farm".to_owned(), a), ("town".to_owned(), b)];
    let mut h = Harness::builder()
        .with_size(vec2(360.0, 300.0))
        .build_ui_state(
            move |ui, s: &mut FileState| {
                if let Some(out) = map_file_properties(ui, "farm.soltilemap", &s.0, &tilesets, &[])
                {
                    s.1 = Some(out);
                }
            },
            (m, None),
        );
    h.run();
    assert!(h.query_by_label_contains("3 layers · 1 values").is_some());
    // Whole numbers of cells.
    assert!(h.query_by_value("10").is_some());
    assert!(h.query_by_value("10.0").is_none());
    h.get_by_value("farm").click();
    h.run();
    h.get_by_label("town").click();
    h.run();
    assert_eq!(h.state().1, Some((Some(b), [10, 8])));
}
