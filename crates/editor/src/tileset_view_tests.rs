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
    /// How many frames asked for the file to be saved.
    saves: usize,
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
        editor: TilesetEditor::new("farm.tileset.lst".into(), set),
        art: with_picture.then(art),
        texture: None,
        out: TilesetOut::default(),
        properties: false,
        saves: 0,
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
                    let changed = tileset_properties(ui, &mut t);
                    s.out.changed |= changed;
                    s.saves += usize::from(changed);
                } else {
                    // The Terrains shelf tab's column at the left, the
                    // document beside it, as LIBRARY TOOLS · Tileset has
                    // them (round 3).
                    let full = ui.max_rect();
                    let (left, right) = full.split_left_right_at_x(full.left() + 220.0);
                    let mut shelf = ui.new_child(egui::UiBuilder::new().max_rect(left));
                    let changed = terrains(&mut shelf, &mut t);
                    s.out.changed |= changed;
                    s.saves += usize::from(changed);
                    let mut document = ui.new_child(egui::UiBuilder::new().max_rect(right));
                    let out = tileset_tab(&mut document, &mut t);
                    s.out.changed |= out.changed;
                    s.saves += usize::from(out.changed);
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

/// How many undo steps the editor holds (undoing them all).
fn steps(editor: &mut TilesetEditor) -> usize {
    std::iter::from_fn(|| editor.undo().then_some(())).count()
}

/// BUG-20: a tile's weight dragged for ten frames is one undo step and one
/// save, made when the mouse is let go; so are its frames and their time.
#[test]
fn a_dragged_weight_or_frame_count_is_one_step_and_one_save() {
    use crate::test_support::{drag_for_frames, number_field_beside};
    let mut h = harness(true);
    h.state_mut().properties = true;
    assert!(h.state_mut().editor.assign(5, [1, 1]));
    h.run();
    let piece = h.state().editor.file.pieces[0].clone();
    let field = number_field_beside(&h, "Weight");
    h.state_mut().saves = 0;
    h.hover_at(field.center());
    h.run();
    h.drag_at(field.center());
    h.run();
    for frame in 1..=10 {
        h.hover_at(field.center() + vec2(6.0, 0.0) * frame as f32);
        h.run();
        assert_eq!(h.state().saves, 0, "nothing saved mid-drag");
        assert_eq!(
            h.state().editor.file.pieces[0].weight,
            piece.weight,
            "nor changed"
        );
    }
    h.drop_at(field.center() + vec2(60.0, 0.0));
    h.run();
    let weight = h.state().editor.file.pieces[0].weight;
    assert!(weight > piece.weight + 1.0, "{weight}");
    assert_eq!(h.state().saves, 1, "saved once, when let go");
    // Its animation's two numbers, each dragged the same way.
    let columns = Animation::Columns { count: 4, ms: 150 };
    assert!(h.state_mut().editor.set_piece(piece.id, weight, columns));
    h.run();
    h.state_mut().saves = 0;
    // The number fields are the weight, the frames and their time.
    for (field, by) in [(1, 3.0), (2, 4.0)] {
        let field = h
            .get_all_by_role(egui::accesskit::Role::SpinButton)
            .nth(field)
            .expect("the animation's number")
            .rect();
        drag_for_frames(&mut h, field.center(), vec2(by, 0.0), 10);
    }
    let Animation::Columns { count, ms } = h.state().editor.file.pieces[0].animation else {
        panic!("still tiles to the right");
    };
    assert!(count > 4 && ms > 150, "{count} frames, {ms} ms");
    assert_eq!(h.state().saves, 2, "one save a drag");
    // The tile assigned, the weight, the animation set, and two drags.
    assert_eq!(steps(&mut h.state_mut().editor), 5);
}

/// BUG-20: a terrain's colour dragged in the picker is one undo step and
/// one save.
#[test]
fn a_dragged_terrain_colour_is_one_step_and_one_save() {
    let mut h = harness(true);
    h.state_mut().properties = true;
    h.run();
    let before = h.state().editor.chosen().unwrap().colour;
    // The colour button opens the picker under it.
    let well = h
        .get_all_by_role(egui::accesskit::Role::ColorWell)
        .next()
        .expect("a colour button");
    let button = well.rect();
    well.click();
    h.run();
    let from = button.left_bottom() + vec2(60.0, 120.0);
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    for frame in 1..=8 {
        h.hover_at(from + vec2(8.0, 6.0) * frame as f32);
        h.run();
        assert_eq!(h.state().saves, 0, "nothing saved mid-drag");
    }
    h.drop_at(from + vec2(64.0, 48.0));
    h.run();
    assert_ne!(h.state().editor.chosen().unwrap().colour, before);
    assert_eq!(h.state().saves, 1, "saved once, when let go");
    assert_eq!(steps(&mut h.state_mut().editor), 1);
}

/// BUG-20: the slot a rule shows, dragged, is one undo step and one save.
#[test]
fn a_dragged_rule_slot_is_one_step_and_one_save() {
    use crate::test_support::{drag_for_frames, number_field_beside};
    let mut h = harness(true);
    let t = h.state().editor.file.by_name("tilled").unwrap().id;
    assert!(h.state_mut().editor.set_kind(t, TileKind::Rules));
    assert!(h.state_mut().editor.add_rule());
    h.run();
    h.state_mut().saves = 0;
    let field = number_field_beside(&h, "Shows slot");
    drag_for_frames(&mut h, field.center(), vec2(4.0, 0.0), 10);
    let slot = h.state().editor.file.rules[0].slot;
    assert!(slot > 10, "{slot}");
    assert_eq!(h.state().saves, 1, "saved once, when let go");
    // The kind, the rule and the drag.
    assert_eq!(steps(&mut h.state_mut().editor), 3);
    // A typed number counts when Enter is pressed, not a letter at a time.
    let mut h = harness(true);
    let t = h.state().editor.file.by_name("tilled").unwrap().id;
    assert!(h.state_mut().editor.set_kind(t, TileKind::Rules));
    assert!(h.state_mut().editor.add_rule());
    h.run();
    h.state_mut().saves = 0;
    let field = number_field_beside(&h, "Shows slot");
    h.hover_at(field.center());
    h.drag_at(field.center());
    h.drop_at(field.center());
    h.run();
    h.event(egui::Event::Text("12".into()));
    h.run();
    assert_eq!(h.state().saves, 0, "nothing saved while typing");
    assert_eq!(h.state().editor.file.rules[0].slot, 0);
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state().editor.file.rules[0].slot, 12);
    assert_eq!(h.state().saves, 1);
}
