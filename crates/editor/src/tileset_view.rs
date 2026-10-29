//! The Tileset tab (Phase 9, D5), as the project lead chose it (mock-up
//! A): the terrains on the left with the picture and its tile size; the
//! picture in the middle, cut into its grid, under a strip of the steps
//! of setting a terrain up; and on the right what the chosen terrain's
//! kind needs (its template, missing slots dashed red, or its rules) above
//! a small patch to try its edges on. Properties shows the picked slot's
//! pieces (variations, weights, animation).

use crate::theme;
use crate::tileset_editor::{Step, TRY_SIZE, TilesetEditor};
use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2, pos2, vec2};
use reflect::{AssetId, Id};
use tiles::autotile::{self, Art};
use tiles::{Animation, Cond, TileKind, Tileset};

/// The picture as the tab draws it: its texture (every frame stacked, then
/// a white patch, as the viewport's atlas), and the picture's own size.
pub(crate) struct Picture<'a> {
    /// The texture.
    pub(crate) texture: &'a egui::TextureHandle,
    /// The texture's size in texels.
    pub(crate) atlas: [u32; 2],
    /// The picture's size (one frame), in pixels.
    pub(crate) size: [u32; 2],
    /// Its frames' times (one for a PNG).
    pub(crate) durations: &'a [u32],
    /// The picture, for finding its grid and which tiles show anything.
    pub(crate) art: &'a assets::tile_art::TileArt,
}

/// What the tab and its Properties draw from.
pub(crate) struct TilesetUi<'a> {
    /// The file open.
    pub(crate) editor: &'a mut TilesetEditor,
    /// Its picture, if chosen and read.
    pub(crate) picture: Option<Picture<'a>>,
    /// Every picture it can take its art from: path and ID.
    pub(crate) pictures: Vec<(String, AssetId)>,
    /// Why the picture couldn't be read, if it couldn't.
    pub(crate) picture_problem: Option<String>,
    /// The project's tile size (from its 2D settings), preferred when
    /// finding a picture's grid.
    pub(crate) project_tile: [u32; 2],
    /// What was put right reading the file.
    pub(crate) notes: Vec<String>,
    /// The time animated tiles show, in milliseconds.
    pub(crate) ms: u64,
    /// The accent colour.
    pub(crate) accent: Color32,
}

/// What the tab asked for this frame.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct TilesetOut {
    /// The file changed: save it.
    pub(crate) changed: bool,
    /// Save the template of this kind, at the tileset's tile size, beside it.
    pub(crate) save_template: Option<TileKind>,
}

const PINK: Color32 = Color32::from_rgb(255, 0, 255);
const MISSING_LINE: Color32 = Color32::from_rgb(0xd0, 0x60, 0x5a);
/// What's dragged from the picture to a slot: a tile's column and row.
#[derive(Debug, Clone, Copy)]
struct TileDrag([u32; 2]);

fn colour(c: [f32; 3]) -> Color32 {
    let [r, g, b] = c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
    Color32::from_rgb(r, g, b)
}

/// The UV rectangle of tile `at` (frame `frame`, column `col` further
/// along for an animation) in the texture.
fn tile_uv(p: &Picture<'_>, tile: [u32; 2], at: [u32; 2], frame: u32, col: u32) -> Rect {
    let (aw, ah) = (p.atlas[0].max(1) as f32, p.atlas[1].max(1) as f32);
    let x = ((at[0] + col) * tile[0]) as f32;
    let y = (frame * p.size[1] + at[1] * tile[1]) as f32;
    Rect::from_min_max(
        pos2(x / aw, y / ah),
        pos2((x + tile[0] as f32) / aw, (y + tile[1] as f32) / ah),
    )
}

/// The UV rectangle of the white patch.
fn white_uv(p: &Picture<'_>) -> Rect {
    let (aw, ah) = (p.atlas[0].max(1) as f32, p.atlas[1].max(1) as f32);
    let y = (p.size[1] * p.durations.len().max(1) as u32) as f32;
    Rect::from_min_max(
        pos2(0.5 / aw, (y + 0.5) / ah),
        pos2(1.5 / aw, (y + 1.5) / ah),
    )
}

/// The picture's grid: columns and rows of whole tiles.
fn grid(set: &Tileset, p: &Picture<'_>) -> [u32; 2] {
    [
        p.size[0] / set.tile[0].max(1),
        p.size[1] / set.tile[1].max(1),
    ]
}

/// Draws piece `index` of `set` in `rect`, at the time `ms`.
pub(crate) fn paint_piece(
    ui: &Ui,
    rect: Rect,
    set: &Tileset,
    p: &Picture<'_>,
    index: usize,
    ms: u64,
) {
    let piece = &set.pieces[index];
    let frame = autotile::frame_at(piece.animation, ms, p.durations);
    let (row, col) = match piece.animation {
        Animation::Frames => (frame, 0),
        Animation::Columns { .. } => (0, frame),
        Animation::None => (0, 0),
    };
    ui.painter().image(
        p.texture.id(),
        rect,
        tile_uv(p, set.tile, piece.at, row, col),
        Color32::WHITE,
    );
}

/// A slot's pictogram: which parts of its tile are the terrain (in its
/// colour) and which are something else (dark): quarters for a 16-tile
/// slot (the four cells at its corners), thirds for the rest (the cell in
/// the middle, its neighbours round it).
fn paint_pictogram(ui: &Ui, rect: Rect, kind: TileKind, slot: u32, on: Color32) {
    let off = Color32::from_rgb(0x3c, 0x44, 0x38);
    if kind == TileKind::Dual16 {
        let half = rect.size() / 2.0;
        for (i, bit) in [8u32, 4, 2, 1].into_iter().enumerate() {
            let r = Rect::from_min_size(
                rect.min + vec2((i % 2) as f32 * half.x, (i / 2) as f32 * half.y),
                half,
            )
            .shrink(1.0);
            ui.painter()
                .rect_filled(r, 1.0, if slot & bit != 0 { on } else { off });
        }
        return;
    }
    let picture = autotile::slot_picture(kind, slot);
    let third = rect.size() / 3.0;
    for (i, &part) in picture.iter().enumerate() {
        let r = Rect::from_min_size(
            rect.min + vec2((i % 3) as f32 * third.x, (i / 3) as f32 * third.y),
            third,
        )
        .shrink(1.0);
        ui.painter()
            .rect_filled(r, 1.0, if part { on } else { off });
    }
}

/// A dashed outline round `rect`.
fn dashed(ui: &Ui, rect: Rect, colour: Color32) {
    let stroke = Stroke::new(2.0, colour);
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        ui.painter()
            .add(egui::Shape::dashed_line(&[a, b], stroke, 5.0, 4.0));
    }
}

/// The Tileset tab.
pub(crate) fn tileset_tab(ui: &mut Ui, t: &mut TilesetUi<'_>) -> TilesetOut {
    let mut out = TilesetOut::default();
    let full = ui.max_rect();
    let left = Rect::from_min_size(full.min, vec2(220.0, full.height()));
    let right_w = (full.width() * 0.32).clamp(300.0, 380.0);
    let right = Rect::from_min_max(pos2(full.max.x - right_w, full.min.y), full.max);
    let middle = Rect::from_min_max(pos2(left.max.x, full.min.y), pos2(right.min.x, full.max.y));
    let line = Stroke::new(1.0, theme::LINE);
    ui.painter().vline(left.max.x, full.y_range(), line);
    ui.painter().vline(right.min.x, full.y_range(), line);
    let mut column = |rect: Rect, salt: &str, add: &mut dyn FnMut(&mut Ui)| {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink(10.0))
                .id_salt(salt),
        );
        egui::ScrollArea::vertical()
            .id_salt(salt)
            .auto_shrink(false)
            .show(&mut child, |ui| add(ui));
    };
    column(left, "tileset terrains", &mut |ui| {
        out.changed |= terrains(ui, t)
    });
    column(middle, "tileset picture", &mut |ui| {
        out.changed |= picture_column(ui, t)
    });
    column(right, "tileset needs", &mut |ui| {
        needs_column(ui, t, &mut out)
    });
    out
}

/// The terrains, the picture and its tile size.
fn terrains(ui: &mut Ui, t: &mut TilesetUi<'_>) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Terrains").color(theme::TEXT_WEAK));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let add = ui
                .add_enabled(
                    t.editor.file.terrains.len() < tiles::tileset::MAX_TERRAINS,
                    egui::Button::new("+"),
                )
                .on_hover_text("Add a terrain: something a cell can be");
            add.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Add a terrain")
            });
            if add.clicked() {
                changed |= t.editor.add_terrain();
            }
        });
    });
    let chosen = t.editor.terrain;
    for terrain in t.editor.file.terrains.clone() {
        let selected = chosen == Some(terrain.id);
        let (rect, r) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
        if selected {
            ui.painter()
                .rect_filled(rect, 3.0, t.accent.gamma_multiply(0.25));
        } else if r.hovered() {
            ui.painter().rect_filled(rect, 3.0, theme::HOVER);
        }
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min + vec2(4.0, 5.0), vec2(16.0, 16.0)),
            2.0,
            colour(terrain.colour),
        );
        ui.painter().text(
            rect.min + vec2(28.0, 13.0),
            egui::Align2::LEFT_CENTER,
            &terrain.name,
            egui::FontId::proportional(theme::TEXT_SIZE),
            theme::TEXT,
        );
        ui.painter().text(
            rect.min + vec2(28.0 + 8.0 * terrain.name.chars().count() as f32 + 6.0, 13.0),
            egui::Align2::LEFT_CENTER,
            terrain.code.to_string(),
            egui::FontId::monospace(12.0),
            theme::TEXT_WEAK,
        );
        let missing = autotile::missing_slots(&t.editor.file, &terrain).len();
        let has_art = t.editor.file.has_art(terrain.id);
        ui.painter().text(
            rect.right_center() - vec2(6.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            terrain.kind.word(),
            egui::FontId::proportional(12.0),
            if has_art && missing > 0 {
                MISSING_LINE
            } else {
                theme::TEXT_WEAK
            },
        );
        let r = r.on_hover_text(if !has_art {
            format!("{}: no art yet, drawn in its colour", terrain.name)
        } else if missing > 0 {
            format!(
                "{}: {missing} slots still need a tile (pink on maps)",
                terrain.name
            )
        } else {
            format!("{}: every slot has its tile", terrain.name)
        });
        r.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                true,
                selected,
                format!("Terrain {}", terrain.name),
            )
        });
        if r.clicked() {
            t.editor.terrain = Some(terrain.id);
            t.editor.slot = None;
            t.editor.rule = None;
        }
    }
    ui.add_space(14.0);
    ui.label(RichText::new("Picture").color(theme::TEXT_WEAK));
    let shown = match t.editor.file.picture {
        None => "None".to_owned(),
        Some(id) => t.pictures.iter().find(|(_, i)| *i == id).map_or_else(
            || format!("Missing picture {id}"),
            |(p, _)| crate::sprite_sheets::name_of(p) + extension(p),
        ),
    };
    let mut picked: Option<Option<AssetId>> = None;
    egui::ComboBox::from_id_salt("tileset picture")
        .selected_text(shown)
        .width(ui.available_width() - 8.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(t.editor.file.picture.is_none(), "None")
                .clicked()
            {
                picked = Some(None);
            }
            for (path, id) in &t.pictures {
                if ui
                    .selectable_label(t.editor.file.picture == Some(*id), path)
                    .clicked()
                {
                    picked = Some(Some(*id));
                }
            }
            if t.pictures.is_empty() {
                ui.label("No pictures yet: put a PNG or an Aseprite file in the project's assets.");
            }
        });
    if let Some(p) = picked
        && p != t.editor.file.picture
    {
        // A new picture's grid is found from its art the next frame (it
        // has to be read first); the project's tile size until then.
        changed |= t.editor.set_picture(p, t.project_tile);
    }
    if let Some(why) = &t.picture_problem {
        ui.label(RichText::new(why).small().color(theme::WARNING));
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Tile size").color(theme::TEXT_WEAK));
        let [w, h] = t.editor.file.tile;
        let most = f64::from(tiles::tileset::MAX_TILE);
        let a = crate::frames_view::settled_whole_drag(ui, "tile w", f64::from(w), 1.0..=most, 0.1);
        ui.label(RichText::new("×").color(theme::TEXT_WEAK));
        let b = crate::frames_view::settled_whole_drag(ui, "tile h", f64::from(h), 1.0..=most, 0.1);
        ui.label(RichText::new("px").color(theme::TEXT_WEAK));
        if a.is_some() || b.is_some() {
            let tile = [a.map_or(w, |v| v as u32), b.map_or(h, |v| v as u32)];
            changed |= t.editor.set_picture(t.editor.file.picture, tile);
        }
    });
    if let Some(p) = &t.picture {
        let [cols, rows] = grid(&t.editor.file, p);
        ui.label(RichText::new(format!("Grid: {cols} × {rows} tiles")).color(theme::TEXT_WEAK));
        let found = autotile::detect_tile(p.size, &p.art.frames[0], t.project_tile);
        if found != t.editor.file.tile {
            let b = ui
                .button(format!("Use {} × {} px (found)", found[0], found[1]))
                .on_hover_text("The size the picture's art changes at");
            if b.clicked() {
                changed |= t.editor.set_picture(t.editor.file.picture, found);
            }
        }
    }
    ui.add_space(10.0);
    ui.label(
        RichText::new(
            "Click a tile in the picture, then the slot it fills on the right, or drag it there. \
             A second tile in a slot is a variation.",
        )
        .small()
        .color(theme::TEXT_WEAK),
    );
    for note in &t.notes {
        ui.label(RichText::new(note).small().color(theme::WARNING));
    }
    changed
}

fn extension(path: &str) -> &str {
    path.rfind('.').map_or("", |i| &path[i..])
}

/// The steps strip and the picture cut into its grid.
fn picture_column(ui: &mut Ui, t: &mut TilesetUi<'_>) -> bool {
    let mut changed = false;
    let step = t.editor.step();
    let chosen = t.editor.chosen().cloned();
    ui.horizontal_wrapped(|ui| {
        let done = |s: Step| match step {
            Step::Picture => false,
            Step::Grid => s == Step::Picture,
            Step::Kind => matches!(s, Step::Picture | Step::Grid),
            Step::MarkUp => s != Step::MarkUp,
        };
        let pill = |ui: &mut Ui, s: Step, text: String| {
            let now = s == step;
            let text = if done(s) { format!("{text} ✓") } else { text };
            let colour = if now { t.accent } else { theme::TEXT_WEAK };
            egui::Frame::new()
                .fill(if now {
                    t.accent.gamma_multiply(0.18)
                } else {
                    theme::HOVER
                })
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(8, 2))
                .show(ui, |ui| ui.label(RichText::new(text).color(colour).small()));
        };
        pill(ui, Step::Picture, "① Picture".into());
        pill(ui, Step::Grid, "② Grid".into());
        let kind = chosen.as_ref().map_or("—", |c| c.kind.word());
        pill(ui, Step::Kind, format!("③ Kind: {kind}"));
        let mark = chosen.as_ref().map_or_else(
            || "④ Mark up".to_owned(),
            |c| {
                let needed = autotile::needed_slots(&t.editor.file, c).len();
                let missing = autotile::missing_slots(&t.editor.file, c).len();
                format!("④ Mark up {} of {needed}", needed - missing)
            },
        );
        pill(ui, Step::MarkUp, mark);
    });
    ui.add_space(8.0);
    let Some(p) = &t.picture else {
        ui.label(
            RichText::new(if t.editor.file.picture.is_some() {
                "The picture is being read."
            } else {
                "Choose a picture on the left: a PNG or an Aseprite file with the tiles drawn \
                 in a grid. Until then, each terrain is drawn in its colour."
            })
            .color(theme::TEXT_WEAK),
        );
        return false;
    };
    // The kind, guessed from the picture while the terrain has no art.
    if let Some(c) = &chosen
        && step == Step::Kind
    {
        let g = grid(&t.editor.file, p);
        let (size, tile, rgba) = (p.size, t.editor.file.tile, &p.art.frames[0]);
        let from = t.editor.tile.unwrap_or([0, 0]);
        let guess = autotile::guess_kind(|at| autotile::tile_fill(size, rgba, tile, at), from, g);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!(
                    "From the tile picked (or the top left), it looks like a {} set. Is that right?",
                    guess.word()
                ))
                    .color(theme::TEXT),
            );
            if ui.button(format!("Make {} {}", c.name, guess.word())).clicked() {
                changed |= t.editor.set_kind(c.id, guess);
            }
        });
        ui.add_space(6.0);
    }
    ui.horizontal(|ui| {
        if ui.small_button("−").on_hover_text("Smaller").clicked() {
            t.editor.zoom = (t.editor.zoom - 1.0).max(1.0);
        }
        ui.label(
            RichText::new(format!("{:.0}%", t.editor.zoom * 100.0))
                .small()
                .color(theme::TEXT_WEAK),
        );
        if ui.small_button("+").on_hover_text("Bigger").clicked() {
            t.editor.zoom = (t.editor.zoom + 1.0).min(8.0);
        }
        ui.label(
            RichText::new("Numbers mark the chosen terrain's slots.")
                .small()
                .color(theme::TEXT_WEAK),
        );
    });
    let set = &t.editor.file;
    let [cols, rows] = grid(set, p);
    let z = t.editor.zoom;
    let size = vec2(p.size[0] as f32 * z, p.size[1] as f32 * z);
    let mut clicked_tile = None;
    let mut dragged = None;
    egui::ScrollArea::both()
        .id_salt("picture scroll")
        .max_height(ui.available_height() - 20.0)
        .show(ui, |ui| {
            let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
            let uv = Rect::from_min_max(
                pos2(0.0, 0.0),
                pos2(
                    p.size[0] as f32 / p.atlas[0].max(1) as f32,
                    p.size[1] as f32 / p.atlas[1].max(1) as f32,
                ),
            );
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_rgb(0x12, 0x12, 0x12));
            ui.painter().image(p.texture.id(), rect, uv, Color32::WHITE);
            let (tw, th) = (set.tile[0] as f32 * z, set.tile[1] as f32 * z);
            let grid_line = Stroke::new(1.0, Color32::from_white_alpha(46));
            for c in 0..=cols {
                ui.painter()
                    .vline(rect.min.x + c as f32 * tw, rect.y_range(), grid_line);
            }
            for r in 0..=rows {
                ui.painter()
                    .hline(rect.x_range(), rect.min.y + r as f32 * th, grid_line);
            }
            if let Some(c) = &chosen {
                for piece in set.pieces.iter().filter(|x| x.terrain == c.id) {
                    let at = rect.min
                        + vec2(piece.at[0] as f32 * tw + 3.0, piece.at[1] as f32 * th + 2.0);
                    ui.painter().text(
                        at,
                        egui::Align2::LEFT_TOP,
                        piece.slot.to_string(),
                        egui::FontId::proportional(10.0),
                        Color32::WHITE,
                    );
                }
            }
            if let Some([x, y]) = t.editor.tile {
                ui.painter().rect_stroke(
                    Rect::from_min_size(
                        rect.min + vec2(x as f32 * tw, y as f32 * th),
                        vec2(tw, th),
                    ),
                    0.0,
                    Stroke::new(2.0, t.accent),
                    egui::StrokeKind::Inside,
                );
            }
            let at_pointer = |pos: Pos2| -> Option<[u32; 2]> {
                let local = pos - rect.min;
                let (x, y) = ((local.x / tw).floor(), (local.y / th).floor());
                (x >= 0.0 && y >= 0.0 && (x as u32) < cols && (y as u32) < rows)
                    .then_some([x as u32, y as u32])
            };
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                clicked_tile = at_pointer(pos);
            }
            if response.drag_started()
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(at) = at_pointer(pos)
            {
                dragged = Some(at);
            }
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Tileset picture")
            });
        });
    if let Some(at) = clicked_tile {
        t.editor.tile = Some(at);
    }
    if let Some(at) = dragged {
        t.editor.tile = Some(at);
        egui::DragAndDrop::set_payload(ui.ctx(), TileDrag(at));
    }
    changed
}

/// What the chosen terrain needs, and the patch to try it on.
fn needs_column(ui: &mut Ui, t: &mut TilesetUi<'_>, out: &mut TilesetOut) {
    let Some(c) = t.editor.chosen().cloned() else {
        ui.label(RichText::new("Add a terrain on the left.").color(theme::TEXT_WEAK));
        return;
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{} · {}", c.name, c.kind.word())).color(theme::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut kind = c.kind;
            egui::ComboBox::from_id_salt("terrain kind")
                .selected_text("Kind")
                .width(80.0)
                .show_ui(ui, |ui| {
                    for k in TileKind::ALL {
                        ui.selectable_value(&mut kind, k, k.word());
                    }
                });
            if kind != c.kind {
                out.changed |= t.editor.set_kind(c.id, kind);
            }
        });
    });
    let c = t.editor.chosen().cloned().unwrap_or(c);
    if c.kind == TileKind::Rules {
        rules(ui, t, &c, out);
    } else {
        template(ui, t, &c, out);
    }
    ui.add_space(10.0);
    try_it(ui, t);
}

/// A slot's box: its art (or its pictogram, dashed when missing), its
/// variations' count, and what a click or a dropped tile asks for.
fn slot_box(
    ui: &mut Ui,
    t: &mut TilesetUi<'_>,
    c: &tiles::Terrain,
    slot: u32,
    size: f32,
    out: &mut TilesetOut,
) {
    let (rect, r) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let set = &t.editor.file;
    let pieces: Vec<usize> = set.pieces_of(c.id, slot).map(|(i, _)| i).collect();
    ui.painter().rect_filled(rect, 3.0, theme::FIELD);
    let needed = c.kind != TileKind::Dual16 || slot != 0;
    match (pieces.first(), &t.picture) {
        (Some(&i), Some(p)) => paint_piece(ui, rect.shrink(3.0), set, p, i, t.ms),
        _ => {
            paint_pictogram(ui, rect.shrink(5.0), c.kind, slot, colour(c.colour));
            if needed {
                dashed(ui, rect.shrink(1.0), MISSING_LINE);
            }
        }
    }
    if pieces.len() > 1 {
        ui.painter().text(
            rect.right_top() + vec2(-3.0, 2.0),
            egui::Align2::RIGHT_TOP,
            format!("×{}", pieces.len()),
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );
    }
    ui.painter().text(
        rect.left_top() + vec2(3.0, 2.0),
        egui::Align2::LEFT_TOP,
        slot.to_string(),
        egui::FontId::proportional(9.0),
        Color32::from_white_alpha(170),
    );
    if t.editor.slot == Some(slot) {
        ui.painter().rect_stroke(
            rect,
            3.0,
            Stroke::new(2.0, t.accent),
            egui::StrokeKind::Inside,
        );
    } else if r.hovered() {
        ui.painter().rect_stroke(
            rect,
            3.0,
            Stroke::new(1.0, theme::TEXT_WEAK),
            egui::StrokeKind::Inside,
        );
    }
    let r = r.on_hover_text(match pieces.len() {
        0 if needed => "Missing: click a tile in the picture, then here".to_owned(),
        0 => "Not needed (none of its corners): never drawn".to_owned(),
        1 => "Click to see it in Properties; drop another tile on it for a variation".to_owned(),
        n => format!("{n} variations, picked by weight: click to see them in Properties"),
    });
    r.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            true,
            t.editor.slot == Some(slot),
            format!("Slot {slot}"),
        )
    });
    if let Some(TileDrag(at)) = r.dnd_release_payload::<TileDrag>().map(|d| *d) {
        out.changed |= t.editor.assign(slot, at);
    } else if r.clicked() {
        match t.editor.tile.take() {
            Some(at) => out.changed |= t.editor.assign(slot, at),
            None => t.editor.slot = Some(slot),
        }
    }
}

/// The kind's template: every slot at its place.
fn template(ui: &mut Ui, t: &mut TilesetUi<'_>, c: &tiles::Terrain, out: &mut TilesetOut) {
    let [cols, _] = autotile::template_size(c.kind);
    let count = autotile::slot_count(c.kind);
    let gap = 6.0;
    let size = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).clamp(24.0, 72.0);
    ui.add_space(4.0);
    let rows = count.div_ceil(cols);
    for row in 0..rows {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for col in 0..cols {
                let slot = row * cols + col;
                if slot < count {
                    slot_box(ui, t, c, slot, size, out);
                }
            }
        });
        ui.add_space(gap - 4.0);
    }
    let missing = autotile::missing_slots(&t.editor.file, c).len();
    if t.editor.file.has_art(c.id) && missing > 0 {
        ui.label(
            RichText::new(format!(
                "{missing} missing: shown dashed, and pink on the map"
            ))
            .color(MISSING_LINE),
        );
    } else if !t.editor.file.has_art(c.id) {
        ui.label(
            RichText::new("No tiles yet: drawn in its colour on maps.")
                .small()
                .color(theme::TEXT_WEAK),
        );
    }
    ui.horizontal_wrapped(|ui| {
        if let Some(p) = &t.picture {
            let from = t.editor.tile.unwrap_or([0, 0]);
            let b = ui
                .button(format!(
                    "Mark up from the template at column {}, row {}",
                    from[0] + 1,
                    from[1] + 1
                ))
                .on_hover_text(format!(
                    "For a picture drawn over the {} template: each slot takes the tile at its \
                     place, starting from the tile picked (or the top left). Replaces this \
                     terrain's tiles.",
                    c.kind.word()
                ));
            if b.clicked() {
                let (size, tile, rgba) = (p.size, t.editor.file.tile, &p.art.frames[0]);
                out.changed |= t
                    .editor
                    .mark_up(from, |at| autotile::tile_fill(size, rgba, tile, at).0);
            }
        }
        if c.kind != TileKind::Plain
            && ui
                .button(format!("Save {} template…", c.kind.word()))
                .on_hover_text("A picture of every slot at the tile size, to draw your tiles over")
                .clicked()
        {
            out.save_template = Some(c.kind);
        }
    });
}

/// A rule terrain's rules, each a pattern of the cells around one, and
/// the slots they name.
fn rules(ui: &mut Ui, t: &mut TilesetUi<'_>, c: &tiles::Terrain, out: &mut TilesetOut) {
    ui.label(
        RichText::new(
            "Tried in order: the first whose pattern matches gives the slot. Click a cell of a \
             pattern to change it: ? anything, = the same, ! not the same, _ nothing, or a terrain.",
        )
        .small()
        .color(theme::TEXT_WEAK),
    );
    let codes: Vec<char> = t.editor.file.terrains.iter().map(|x| x.code).collect();
    let cycle = |cond: Cond| -> Cond {
        let order: Vec<Cond> = [Cond::Any, Cond::Same, Cond::Other, Cond::Empty]
            .into_iter()
            .chain(codes.iter().map(|&c| Cond::Terrain(c)))
            .collect();
        let i = order.iter().position(|o| *o == cond).unwrap_or(0);
        order[(i + 1) % order.len()]
    };
    let list: Vec<tiles::Rule> = t.editor.file.rules_of(c.id).cloned().collect();
    for (n, rule) in list.iter().enumerate() {
        let picked = t.editor.rule == Some(rule.id);
        egui::Frame::new()
            .fill(if picked {
                t.accent.gamma_multiply(0.15)
            } else {
                theme::FIELD
            })
            .corner_radius(4.0)
            .inner_margin(6.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let cell = (90.0 / rule.size as f32).clamp(10.0, 22.0);
                    let (rect, _) = ui.allocate_exact_size(
                        vec2(cell * rule.size as f32, cell * rule.size as f32),
                        Sense::hover(),
                    );
                    for (i, cond) in rule.pattern.iter().enumerate() {
                        let r = Rect::from_min_size(
                            rect.min
                                + vec2(
                                    (i % rule.size) as f32 * cell,
                                    (i / rule.size) as f32 * cell,
                                ),
                            vec2(cell, cell),
                        )
                        .shrink(1.0);
                        let middle = i == rule.pattern.len() / 2;
                        let resp =
                            ui.interact(r, ui.id().with(("rule cell", rule.id, i)), Sense::click());
                        let fill = if middle {
                            colour(c.colour)
                        } else if resp.hovered() {
                            theme::HOVER
                        } else {
                            Color32::from_rgb(0x2c, 0x2c, 0x2c)
                        };
                        ui.painter().rect_filled(r, 2.0, fill);
                        ui.painter().text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            if middle { '*' } else { cond.char() }.to_string(),
                            egui::FontId::monospace((cell * 0.6).max(8.0)),
                            theme::TEXT,
                        );
                        if !middle && resp.clicked() {
                            out.changed |= t.editor.set_cond(rule.id, i, cycle(*cond));
                        }
                    }
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{}.", n + 1)).color(theme::TEXT_WEAK));
                            ui.label("Shows slot");
                            let mut slot = rule.slot;
                            if ui
                                .add(
                                    egui::DragValue::new(&mut slot)
                                        .range(0..=autotile::RULE_SLOTS - 1),
                                )
                                .changed()
                            {
                                out.changed |= t.editor.set_rule_slot(rule.id, slot);
                            }
                        });
                        ui.horizontal(|ui| {
                            let mut size = rule.size;
                            egui::ComboBox::from_id_salt(("rule size", rule.id))
                                .selected_text(format!("{0} × {0}", rule.size))
                                .width(60.0)
                                .show_ui(ui, |ui| {
                                    for s in [1, 3, 5, 7, 9] {
                                        ui.selectable_value(&mut size, s, format!("{s} × {s}"));
                                    }
                                });
                            if size != rule.size {
                                out.changed |= t.editor.set_rule_size(rule.id, size);
                            }
                            if ui
                                .small_button("↑")
                                .on_hover_text("Try it sooner")
                                .clicked()
                            {
                                out.changed |= t.editor.move_rule(rule.id, true);
                            }
                            if ui.small_button("↓").on_hover_text("Try it later").clicked() {
                                out.changed |= t.editor.move_rule(rule.id, false);
                            }
                            let x = ui.small_button("×").on_hover_text("Take this rule away");
                            x.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    true,
                                    format!("Remove rule {}", n + 1),
                                )
                            });
                            if x.clicked() {
                                out.changed |= t.editor.remove_rule(rule.id);
                            }
                        });
                    });
                });
            });
        ui.add_space(4.0);
    }
    if ui.button("+ Rule").clicked() {
        out.changed |= t.editor.add_rule();
    }
    // The slots the rules name, to fill.
    let needed = autotile::needed_slots(&t.editor.file, c);
    if !needed.is_empty() {
        ui.add_space(6.0);
        ui.label(RichText::new("Slots the rules show").color(theme::TEXT_WEAK));
        ui.horizontal_wrapped(|ui| {
            for slot in needed {
                slot_box(ui, t, c, slot, 48.0, out);
            }
        });
    }
}

/// The patch to try edges on: click cells to turn the terrain on or off.
fn try_it(ui: &mut Ui, t: &mut TilesetUi<'_>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Try it").color(theme::TEXT_WEAK));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Clear").clicked() {
                t.editor.try_cells.iter_mut().for_each(|c| *c = false);
            }
        });
    });
    let [w, h] = TRY_SIZE;
    let cell = ((ui.available_width() - 4.0) / w as f32).clamp(16.0, 48.0);
    let (rect, response) =
        ui.allocate_exact_size(vec2(cell * w as f32, cell * h as f32), Sense::click());
    ui.painter()
        .rect_filled(rect, 0.0, Color32::from_rgb(0x12, 0x12, 0x12));
    if let (Some(map), None) = (t.editor.try_map(), &t.picture) {
        // No picture yet: each cell in its terrain's colour.
        for layer in 0..2 {
            for y in 0..h {
                for x in 0..w {
                    let code = map.code(layer, x as u32, y as u32).unwrap_or(0);
                    if let Some(terrain) = t
                        .editor
                        .file
                        .by_code(char::from(code))
                        .filter(|_| code != 0)
                    {
                        let r = Rect::from_min_size(
                            rect.min + vec2(x as f32 * cell, y as f32 * cell),
                            Vec2::splat(cell),
                        );
                        ui.painter().rect_filled(r, 0.0, colour(terrain.colour));
                    }
                }
            }
        }
    }
    if let (Some(map), Some(p)) = (t.editor.try_map(), &t.picture) {
        let set = &t.editor.file;
        let clip = ui.painter().with_clip_rect(rect);
        for layer in 0..2 {
            let mut tiles = autotile::chunk_tiles(&map, layer, set, [0, 0]);
            tiles.sort_by_key(|x| x.pass);
            for placed in tiles {
                let r = Rect::from_min_size(
                    rect.min + vec2(placed.x as f32 * 0.5 * cell, placed.y as f32 * 0.5 * cell),
                    Vec2::splat(cell),
                );
                match placed.art {
                    Art::Piece(i) => {
                        let piece = &set.pieces[i];
                        let frame = autotile::frame_at(piece.animation, t.ms, p.durations);
                        let (row, col) = match piece.animation {
                            Animation::Frames => (frame, 0),
                            Animation::Columns { .. } => (0, frame),
                            Animation::None => (0, 0),
                        };
                        clip.image(
                            p.texture.id(),
                            r,
                            tile_uv(p, set.tile, piece.at, row, col),
                            Color32::WHITE,
                        );
                    }
                    Art::Missing => {
                        clip.image(p.texture.id(), r, white_uv(p), PINK);
                    }
                    Art::Colour(i) => {
                        clip.image(
                            p.texture.id(),
                            r,
                            white_uv(p),
                            colour(set.terrains[i].colour),
                        );
                    }
                }
            }
        }
    }
    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let local = pos - rect.min;
        let (x, y) = ((local.x / cell) as usize, (local.y / cell) as usize);
        if x < w && y < h {
            let i = y * w + x;
            t.editor.try_cells[i] = !t.editor.try_cells[i];
        }
    }
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Try-it patch"));
    ui.label(
        RichText::new("Click cells to see the edges; not saved.")
            .small()
            .color(theme::TEXT_WEAK),
    );
}

/// Properties while the Tileset tab is in front: the picked slot's pieces
/// (each with its weight and animation), or the tileset's summary.
pub(crate) fn tileset_properties(ui: &mut Ui, t: &mut TilesetUi<'_>) -> bool {
    let mut changed = false;
    let Some(c) = t.editor.chosen().cloned() else {
        ui.label(RichText::new("No terrains yet.").color(theme::TEXT_WEAK));
        return false;
    };
    let Some(slot) = t.editor.slot else {
        ui.horizontal(|ui| {
            ui.label(RichText::new("▦").color(theme::kind::SPRITE));
            ui.label(RichText::new(crate::sprite_sheets::name_of(&t.editor.path)).strong());
        });
        // The chosen terrain: its name, code and colour.
        ui.add_space(4.0);
        ui.label(RichText::new(format!("Terrain · {}", c.kind.word())).color(theme::TEXT_WEAK));
        crate::inspector::row(ui, "Name", "What game code will call it.", |ui| {
            if let Some(name) =
                crate::map_tab::name_field(ui, ("terrain name", c.id), &c.name, 120.0)
            {
                changed |= t.editor.set_terrain(c.id, Some(name), None, None);
            }
        });
        crate::inspector::row(
            ui,
            "Code",
            "The one character a map file writes for it.",
            |ui| {
                let id = ui.id().with(("terrain code", c.id));
                let mut code: String = ui
                    .data(|d| d.get_temp(id))
                    .unwrap_or_else(|| c.code.to_string());
                let r = ui.add(
                    egui::TextEdit::singleline(&mut code)
                        .desired_width(18.0)
                        .char_limit(1),
                );
                if r.has_focus() {
                    ui.data_mut(|d| d.insert_temp(id, code.clone()));
                } else {
                    ui.data_mut(|d| d.remove::<String>(id));
                    if let Some(ch) = code.chars().next()
                        && r.lost_focus()
                        && ch != c.code
                    {
                        changed |= t.editor.set_terrain(c.id, None, Some(ch), None);
                    }
                }
            },
        );
        crate::inspector::row(ui, "Colour", "Drawn where it has no art.", |ui| {
            let mut rgb = c.colour.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
            if ui.color_edit_button_srgb(&mut rgb).changed() {
                changed |=
                    t.editor
                        .set_terrain(c.id, None, None, Some(rgb.map(|v| f32::from(v) / 255.0)));
            }
        });
        let x = ui
            .button("Remove terrain")
            .on_hover_text("Take it away with its tiles (maps keep its code, drawn pink)");
        if x.clicked() {
            changed |= t.editor.remove_terrain(c.id);
        }
        ui.add_space(8.0);
        let set = &t.editor.file;
        ui.label(
            RichText::new(format!(
                "{} terrains · {} tiles placed · {} rules",
                set.terrains.len(),
                set.pieces.len(),
                set.rules.len()
            ))
            .color(theme::TEXT_WEAK),
        );
        if let Some(at) = t.editor.tile {
            let used: Vec<String> = set
                .pieces
                .iter()
                .filter(|p| p.at == at)
                .filter_map(|p| {
                    set.terrain(p.terrain)
                        .map(|x| format!("{} slot {}", x.name, p.slot))
                })
                .collect();
            ui.add_space(6.0);
            ui.label(format!("Tile: column {}, row {}", at[0] + 1, at[1] + 1));
            ui.label(
                RichText::new(if used.is_empty() {
                    "Not used yet: click a slot of the chosen terrain to fill it.".to_owned()
                } else {
                    format!("Fills {}", used.join(", "))
                })
                .color(theme::TEXT_WEAK),
            );
        } else {
            ui.label(
                RichText::new("Click a slot on the right to see its tiles here.")
                    .small()
                    .color(theme::TEXT_WEAK),
            );
        }
        return changed;
    };
    ui.label(RichText::new(format!("{} · slot {slot}", c.name)).strong());
    let pieces: Vec<tiles::Piece> = t
        .editor
        .file
        .pieces_of(c.id, slot)
        .map(|(_, p)| p.clone())
        .collect();
    if pieces.is_empty() {
        ui.label(
            RichText::new("No tile yet: click a tile in the picture, then this slot.")
                .color(theme::TEXT_WEAK),
        );
    }
    let aseprite = t.picture.as_ref().is_some_and(|p| p.durations.len() > 1);
    let total: f32 = pieces.iter().map(|p| p.weight).sum();
    for (n, piece) in pieces.iter().enumerate() {
        egui::Frame::new()
            .fill(theme::FIELD)
            .corner_radius(4.0)
            .inner_margin(6.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(p) = &t.picture
                        && let Some(i) = t.editor.file.pieces.iter().position(|x| x.id == piece.id)
                    {
                        let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                        paint_piece(ui, r, &t.editor.file, p, i, t.ms);
                    }
                    ui.vertical(|ui| {
                        ui.label(format!(
                            "Tile: column {}, row {}{}",
                            piece.at[0] + 1,
                            piece.at[1] + 1,
                            if pieces.len() > 1 && total > 0.0 {
                                format!(
                                    " · picked {:.0}% of the time",
                                    piece.weight / total * 100.0
                                )
                            } else {
                                String::new()
                            }
                        ));
                        let mut weight = piece.weight;
                        let mut animation = piece.animation;
                        ui.horizontal(|ui| {
                            ui.label("Weight");
                            ui.add(
                                egui::DragValue::new(&mut weight)
                                    .range(0.0..=tiles::tileset::MAX_WEIGHT)
                                    .speed(0.05),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Animation");
                            let word = match animation {
                                Animation::None => "none",
                                Animation::Columns { .. } => "tiles to the right",
                                Animation::Frames => "the picture's frames",
                            };
                            egui::ComboBox::from_id_salt(("piece animation", piece.id))
                                .selected_text(word)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut animation, Animation::None, "none");
                                    if !matches!(animation, Animation::Columns { .. }) {
                                        let cols = Animation::Columns { count: 4, ms: 150 };
                                        if ui
                                            .selectable_label(false, "tiles to the right")
                                            .clicked()
                                        {
                                            animation = cols;
                                        }
                                    }
                                    if aseprite {
                                        ui.selectable_value(
                                            &mut animation,
                                            Animation::Frames,
                                            "the picture's frames",
                                        );
                                    }
                                });
                        });
                        if let Animation::Columns { count, ms } = &mut animation {
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(count)
                                        .range(2..=tiles::tileset::MAX_FRAMES)
                                        .suffix(" frames"),
                                );
                                ui.add(
                                    egui::DragValue::new(ms)
                                        .range(
                                            tiles::tileset::FRAME_MS.0..=tiles::tileset::FRAME_MS.1,
                                        )
                                        .suffix(" ms each"),
                                );
                            });
                        }
                        if weight != piece.weight || animation != piece.animation {
                            changed |= t.editor.set_piece(piece.id, weight, animation);
                        }
                        let x = ui
                            .small_button("Remove")
                            .on_hover_text("Take this tile out of the slot");
                        x.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                format!("Remove tile {}", n + 1),
                            )
                        });
                        if x.clicked() {
                            changed |= t.editor.remove_piece(piece.id);
                        }
                    });
                });
            });
        ui.add_space(4.0);
    }
    ui.label(
        RichText::new(
            "A slot with more than one tile picks one per cell by weight, the same for the same \
             cell every time.",
        )
        .small()
        .color(theme::TEXT_WEAK),
    );
    let _ = Id::random;
    changed
}

#[cfg(test)]
#[path = "tileset_view_tests.rs"]
mod tests;
