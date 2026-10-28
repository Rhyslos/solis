//! The Tileset tab on its own: what it shows with and without a picture,
//! and what clicks ask for.

use super::*;
use crate::tileset_editor::TilesetEditor;
use assets::tile_art::TileArt;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use std::sync::Arc;

/// A 4 × 4 picture of 8-pixel tiles, every tile drawn.
fn art() -> Arc<TileArt> {
    let (w, h) = (32u32, 32u32);
    let mut rgba = vec![255u8; (w * h * 4) as usize];
    for (i, px) in rgba.chunks_mut(4).enumerate() {
        px[0] = (i % 251) as u8;
    }
    Arc::new(TileArt {
        size: [w, h],
        frames: vec![rgba],
        durations: vec![100],
    })
}

struct State {
    editor: TilesetEditor,
    art: Option<Arc<TileArt>>,
    texture: Option<(egui::TextureHandle, [u32; 2])>,
    out: TilesetOut,
    properties: bool,
}

fn harness(with_picture: bool) -> Harness<'static, State> {
    let mut set = crate::workbench_tiles::starter_tileset();
    set.terrains[1].kind = TileKind::Dual16;
    set.tile = [8, 8];
    let picture = AssetId::random();
    if with_picture {
        set.picture = Some(picture);
    }
    let state = State {
        editor: TilesetEditor::new("farm.soltileset".into(), set),
        art: with_picture.then(art),
        texture: None,
        out: TilesetOut::default(),
        properties: false,
    };
    Harness::builder()
        .with_size(vec2(1100.0, 700.0))
        .build_ui_state(
            move |ui, s: &mut State| {
                if let (Some(art), None) = (&s.art, &s.texture) {
                    let (size, rgba) = crate::tile_view::atlas_rgba(art);
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [size[0] as usize, size[1] as usize],
                        &rgba,
                    );
                    s.texture = Some((
                        ui.ctx()
                            .load_texture("t", image, egui::TextureOptions::NEAREST),
                        size,
                    ));
                }
                let picture = match (&s.art, &s.texture) {
                    (Some(art), Some((texture, atlas))) => Some(Picture {
                        texture,
                        atlas: *atlas,
                        size: art.size,
                        durations: &art.durations,
                        art,
                    }),
                    _ => None,
                };
                let mut t = TilesetUi {
                    editor: &mut s.editor,
                    picture,
                    pictures: vec![("tiles.png".into(), picture_id())],
                    picture_problem: None,
                    project_tile: [16, 16],
                    notes: Vec::new(),
                    ms: 0,
                    accent: Color32::GOLD,
                };
                if s.properties {
                    s.out.changed |= tileset_properties(ui, &mut t);
                } else {
                    let out = tileset_tab(ui, &mut t);
                    s.out.changed |= out.changed;
                    if out.save_template.is_some() {
                        s.out.save_template = out.save_template;
                    }
                }
            },
            state,
        )
}

fn picture_id() -> AssetId {
    AssetId(Id::from_raw(77).unwrap())
}

#[test]
fn without_a_picture_it_says_how_to_start() {
    let mut h = harness(false);
    h.run();
    for label in [
        "Terrain grass",
        "Terrain tilled",
        "Terrain water",
        "Add a terrain",
        "Try-it patch",
        "Slot 1",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    assert!(
        h.query_by_label_contains("Choose a picture on the left")
            .is_some()
    );
    assert!(h.query_by_label_contains("No tiles yet").is_some());
    h.get_by_label("Add a terrain").click();
    h.run();
    assert!(h.state().out.changed);
    assert_eq!(h.state().editor.file.terrains.len(), 4);
    // Turning a cell of the patch on.
    let before = h.state().editor.try_cells.clone();
    h.get_by_label("Try-it patch").click();
    h.run();
    assert_ne!(h.state().editor.try_cells, before);
}

#[test]
fn a_tile_picked_then_a_slot_fills_it() {
    let mut h = harness(true);
    h.run();
    assert!(h.query_by_label("Tileset picture").is_some());
    assert!(h.query_by_label_contains("Grid: 4 × 4 tiles").is_some());
    // Pick the picture's tile at its middle (2, 2) ...
    h.state_mut().editor.tile = Some([2, 2]);
    h.get_by_label("Slot 7").click();
    h.run();
    let e = &h.state().editor;
    let tilled = e.file.by_name("tilled").unwrap().id;
    let filled: Vec<[u32; 2]> = e.file.pieces_of(tilled, 7).map(|(_, p)| p.at).collect();
    assert_eq!(filled, [[2, 2]]);
    assert!(h.state().out.changed);
    // A slot clicked with no tile picked is picked itself.
    h.get_by_label("Slot 3").click();
    h.run();
    assert_eq!(h.state().editor.slot, Some(3));
    // The step strip counts what's filled.
    assert!(h.query_by_label_contains("Mark up 1 of 15").is_some());
}

#[test]
fn marking_up_and_saving_a_template_are_asked_for() {
    let mut h = harness(true);
    h.run();
    h.get_by_label_contains("Mark up from the template at column 1, row 1")
        .click();
    h.run();
    let e = &h.state().editor;
    assert_eq!(
        e.file.pieces.len(),
        16,
        "every tile of the picture is drawn"
    );
    h.get_by_label("Save 16-tile template…").click();
    h.run();
    assert_eq!(h.state().out.save_template, Some(TileKind::Dual16));
}

#[test]
fn a_plain_terrain_s_kind_is_guessed_from_the_picture() {
    let mut h = harness(true);
    let grass = h.state().editor.file.by_name("grass").unwrap().id;
    h.state_mut().editor.terrain = Some(grass);
    h.run();
    assert!(
        h.query_by_label_contains("it looks like a 16-tile set")
            .is_some()
    );
    h.get_by_label("Make grass 16-tile").click();
    h.run();
    assert_eq!(
        h.state().editor.file.terrain(grass).unwrap().kind,
        TileKind::Dual16
    );
}

#[test]
fn rules_are_drawn_and_added() {
    let mut h = harness(true);
    let t = h.state().editor.file.by_name("tilled").unwrap().id;
    h.state_mut().editor.set_kind(t, TileKind::Rules);
    h.run();
    h.get_by_label("+ Rule").click();
    h.run();
    assert_eq!(h.state().editor.file.rules.len(), 1);
    assert!(h.query_by_label("Remove rule 1").is_some());
    assert!(
        h.query_by_label("Slot 0").is_some(),
        "the slot it shows, to fill"
    );
    h.get_by_label("Remove rule 1").click();
    h.run();
    assert!(h.state().editor.file.rules.is_empty());
}

#[test]
fn properties_show_a_slot_s_tiles_and_the_terrain() {
    let mut h = harness(true);
    h.state_mut().properties = true;
    h.run();
    assert!(h.query_by_label("Remove terrain").is_some());
    assert!(
        h.query_by_label_contains("Click a slot on the right")
            .is_some()
    );
    let e = &mut h.state_mut().editor;
    e.assign(5, [1, 1]);
    e.assign(5, [2, 1]);
    h.run();
    assert!(
        h.query_by_label_contains("column 2, row 2 · picked 50% of the time")
            .is_some()
    );
    h.get_by_label("Remove tile 2").click();
    h.run();
    let e = &h.state().editor;
    assert_eq!(e.file.pieces.len(), 1);
    assert!(h.state().out.changed);
}
