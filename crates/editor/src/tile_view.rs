//! Tile maps drawn (Phase 9, D4): each layer of a map becomes a picture
//! with one texel per cell, in its terrain's colour (its meaning; tileset
//! art comes with D5), drawn as one quad lying on the ground: layers under
//! sprites first, under everything, and layers over sprites last, over
//! everything (`render::SpriteLayer`). Pictures are made again only for a
//! layer that changed ([`TilePictures`]), and the app uploads a picture
//! only when its key changes.

use crate::sprite_sheets::{SpriteBatch, SpriteSheets, ViewSprite};
use glam::{Mat4, Vec3};
use reflect::{AssetId, EntityId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tiles::TileMap;

/// A code with no terrain in the map's tileset: drawn in this, so a
/// missing terrain shows instead of quietly looking like another.
pub(crate) const MISSING: [u8; 4] = [255, 0, 255, 255];

/// `render::SpriteInstance::UNDER` and `OVER`.
const UNDER: u32 = 16;
const OVER: u32 = 32;

/// A map in an area or the running game, to draw.
#[derive(Debug, Clone)]
pub struct ViewTileMap {
    /// Its entity.
    pub entity: EntityId,
    /// Where it is: its top-left corner at the entity.
    pub transform: Mat4,
    /// The map: its file's (editing), or the game's.
    pub map: Arc<TileMap>,
}

/// One layer's picture: a texel per cell, rows from the top left.
#[derive(Debug, Clone, PartialEq)]
pub struct TilePicture {
    /// The ID it's drawn under (made from its entity and layer).
    pub id: AssetId,
    /// Changes whenever the picture does: the app uploads it again then.
    pub key: u64,
    /// Its size in texels (the map's in cells).
    pub size: [u32; 2],
    /// Straight sRGB RGBA, a row at a time.
    pub rgba: Arc<Vec<u8>>,
}

/// Each layer's picture, kept until its layer or its tileset changes.
#[derive(Debug, Default)]
pub struct TilePictures {
    made: Mutex<HashMap<(EntityId, usize), Arc<TilePicture>>>,
}

/// The pictures kept are a cache, never part of what sheets are.
impl PartialEq for TilePictures {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// The ID a layer's picture is drawn under.
fn picture_id(entity: EntityId, layer: usize) -> AssetId {
    let mut sum = reflect::Checksum::new();
    sum.bytes(b"tile layer");
    sum.entity(entity);
    sum.u64(layer as u64);
    AssetId(reflect::Id::from_raw(sum.finish() | 1).unwrap_or_else(reflect::Id::random))
}

/// The picture of a layer of `map`, in `tileset`'s colours.
pub(crate) fn layer_rgba(map: &TileMap, layer: usize, tileset: Option<&tiles::Tileset>) -> Vec<u8> {
    let mut colours = [[0u8; 4]; 256];
    for (code, colour) in colours.iter_mut().enumerate().skip(1) {
        *colour = tileset
            .and_then(|t| t.by_code(char::from(code as u8)))
            .map_or(MISSING, |t| {
                let [r, g, b] = t.colour.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
                [r, g, b, 255]
            });
    }
    let cells = map.layer_cells(layer).unwrap_or_default();
    let mut rgba = Vec::with_capacity(cells.len() * 4);
    for &c in cells {
        rgba.extend_from_slice(&colours[usize::from(c)]);
    }
    rgba
}

impl TilePictures {
    /// The pictures and the batches that draw `maps`, cells `cell` metres
    /// across, each map in its tileset's colours, leaving out layers
    /// `hidden` in the editor (by ID).
    pub fn draw(
        &self,
        maps: &[ViewTileMap],
        sheets: &SpriteSheets,
        cell: [f32; 2],
        hidden: &std::collections::HashSet<reflect::Id>,
    ) -> (Vec<Arc<TilePicture>>, Vec<SpriteBatch>) {
        let mut made = self
            .made
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut pictures = Vec::new();
        let mut under = Vec::new();
        let mut over = Vec::new();
        let mut used = std::collections::HashSet::new();
        for m in maps {
            let tileset = m.map.tileset.and_then(|t| sheets.tileset(t));
            let size = m.map.size();
            for (i, layer) in m.map.layers().iter().enumerate() {
                if hidden.contains(&layer.id) {
                    continue;
                }
                let mut key = reflect::Checksum::new();
                key.u64(m.map.layer_cells(i).map_or(0, |c| c.as_ptr() as u64));
                key.u64(m.map.layer_revision(i));
                key.u64(tileset.map_or(0, |t| Arc::as_ptr(t) as u64));
                key.u64(u64::from(size[0]) << 32 | u64::from(size[1]));
                let key = key.finish();
                let picture = match made.get(&(m.entity, i)) {
                    Some(p) if p.key == key => Arc::clone(p),
                    _ => {
                        let p = Arc::new(TilePicture {
                            id: picture_id(m.entity, i),
                            key,
                            size,
                            rgba: Arc::new(layer_rgba(&m.map, i, tileset.map(|t| &**t))),
                        });
                        made.insert((m.entity, i), Arc::clone(&p));
                        p
                    }
                };
                used.insert((m.entity, i));
                let corner = m.transform.transform_point3(Vec3::ZERO);
                let across = m.transform.transform_vector3(Vec3::X * cell[0]);
                let down = m.transform.transform_vector3(-Vec3::Y * cell[1]);
                let sprite = ViewSprite {
                    corner: corner.to_array(),
                    flags: if layer.over { OVER } else { UNDER },
                    axis_x: [across.x, across.y],
                    axis_y: [down.x, down.y],
                    rect: [0.0, 0.0, size[0] as f32, size[1] as f32],
                    tint: [1.0, 1.0, 1.0, 0.0],
                    outline: [0.0; 4],
                    foot: [corner.x, corner.y],
                };
                let batch = SpriteBatch {
                    sheet: picture.id,
                    see_through: false,
                    sprites: vec![sprite],
                    picks: vec![0],
                };
                if layer.over {
                    over.push(batch);
                } else {
                    under.push(batch);
                }
                pictures.push(picture);
            }
        }
        made.retain(|k, _| used.contains(k));
        under.extend(over);
        (pictures, under)
    }
}

/// The cell of `map` (placed by `transform`, cells `cell` metres across)
/// at `point` on the ground, if it's on the map.
pub fn cell_at(map: &TileMap, transform: Mat4, cell: [f32; 2], point: Vec3) -> Option<[u32; 2]> {
    let local = transform.inverse().transform_point3(point);
    let (x, y) = (local.x / cell[0], -local.y / cell[1]);
    let [w, h] = map.size();
    (x.is_finite() && y.is_finite() && x >= 0.0 && y >= 0.0 && x < w as f32 && y < h as f32)
        .then_some([x as u32, y as u32])
}

/// The world place of a cell's corners: top left and bottom right.
pub fn cell_corners(transform: Mat4, cell: [f32; 2], at: [u32; 2]) -> [Vec3; 2] {
    let corner =
        |x: f32, y: f32| transform.transform_point3(Vec3::new(x * cell[0], -y * cell[1], 0.0));
    [
        corner(at[0] as f32, at[1] as f32),
        corner(at[0] as f32 + 1.0, at[1] as f32 + 1.0),
    ]
}

/// What the viewport lays over the selected tile map (Phase 9, D4), set
/// by the workbench each frame, and the cell under the pointer it finds.
#[derive(Debug, Clone, Default)]
pub struct TileOverlay {
    /// The selected map, if one is: drawn over.
    pub map: Option<ViewTileMap>,
    /// Its values laid over it, by index.
    pub values: Vec<usize>,
    /// Whether its 32 × 32 chunks are outlined.
    pub chunks: bool,
    /// Layers hidden in the editor, by ID.
    pub hidden: std::collections::HashSet<reflect::Id>,
    /// The cell under the pointer, found while drawing.
    pub hovered: Option<[u32; 2]>,
    /// The values asked for from the menu over the view this frame (by
    /// name, on or off), and the chunks switch.
    pub asked: Vec<(String, bool)>,
    /// The chunks switch pressed this frame.
    pub chunks_asked: bool,
}

/// The cells shown most: past this many in view, values aren't written.
const MOST_LABELLED: u64 = 4000;

/// Draws the selected map's overlay over the view in `rect` (seen by
/// `camera`): its values on its cells (a switch as a blue tint, a number
/// or a choice as text), its chunks, and an outline round the cell under
/// `mouse` (from the view's top left), which it notes; and the menu that
/// chooses what's shown, at the top of the view, whose place it gives
/// back (a press there is the menu's, never the view's).
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_overlay(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    camera: &crate::camera::Camera,
    overlay: &mut TileOverlay,
    cell: [f32; 2],
    mouse: Option<glam::Vec2>,
    accent: crate::theme::Accent,
) -> Option<egui::Rect> {
    overlay.hovered = None;
    overlay.asked.clear();
    overlay.chunks_asked = false;
    let m = overlay.map.clone()?;
    let size = glam::Vec2::new(rect.width(), rect.height());
    let to_screen = |p: Vec3| {
        camera
            .project(p, size)
            .map(|s| egui::pos2(rect.min.x + s.x, rect.min.y + s.y))
    };
    let painter = ui.painter_at(rect);
    let z = m.transform.w_axis.z;
    if let Some(at) = mouse
        && let Some(ground) = camera.ray_through(at, size).hit_ground(z)
    {
        overlay.hovered = cell_at(&m.map, m.transform, cell, ground);
    }
    let [w, h] = m.map.size();
    // The cells in view, from where the view's corners land on the map.
    let corners: Vec<[u32; 2]> = [
        glam::Vec2::ZERO,
        glam::Vec2::new(size.x, 0.0),
        glam::Vec2::new(0.0, size.y),
        size,
    ]
    .iter()
    .filter_map(|c| camera.ray_through(*c, size).hit_ground(z))
    .map(|p| {
        let local = m.transform.inverse().transform_point3(p);
        [
            (local.x / cell[0]).clamp(0.0, w as f32) as u32,
            (-local.y / cell[1]).clamp(0.0, h as f32) as u32,
        ]
    })
    .collect();
    let seen = if corners.len() == 4 {
        let x0 = corners.iter().map(|c| c[0]).min().unwrap_or(0);
        let x1 = corners
            .iter()
            .map(|c| c[0])
            .max()
            .unwrap_or(w)
            .min(w.saturating_sub(1));
        let y0 = corners.iter().map(|c| c[1]).min().unwrap_or(0);
        let y1 = corners
            .iter()
            .map(|c| c[1])
            .max()
            .unwrap_or(h)
            .min(h.saturating_sub(1));
        Some((x0, x1, y0, y1))
    } else {
        None
    };
    let screen_cell = |at: [u32; 2]| -> Option<egui::Rect> {
        let [a, b] = cell_corners(m.transform, cell, at);
        Some(egui::Rect::from_two_pos(to_screen(a)?, to_screen(b)?))
    };
    if let Some((x0, x1, y0, y1)) = seen
        && !overlay.values.is_empty()
    {
        let count = u64::from(x1.saturating_sub(x0) + 1) * u64::from(y1.saturating_sub(y0) + 1);
        let big_enough = screen_cell([x0, y0]).is_some_and(|r| r.width() >= 18.0);
        let font = egui::FontId::proportional(11.0);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let Some(r) = screen_cell([x, y]) else {
                    continue;
                };
                let mut words = Vec::new();
                for &v in &overlay.values {
                    match (m.map.value(v, x, y), m.map.values().get(v).map(|d| &d.kind)) {
                        (Some(tiles::CellValue::Switch(true)), _) => {
                            painter.rect_filled(
                                r,
                                0.0,
                                egui::Color32::from_rgba_unmultiplied(70, 150, 255, 120),
                            );
                        }
                        (Some(tiles::CellValue::Number(n)), _) if n != 0 => {
                            words.push(n.to_string())
                        }
                        (
                            Some(tiles::CellValue::Choice(Some(o))),
                            Some(tiles::ValueKind::Choice(options)),
                        ) => {
                            if let Some(name) = options.get(usize::from(o)) {
                                words.push(name.clone());
                            }
                        }
                        _ => {}
                    }
                }
                if !words.is_empty() && big_enough && count <= MOST_LABELLED {
                    painter.text(
                        r.left_top() + egui::vec2(4.0, 3.0),
                        egui::Align2::LEFT_TOP,
                        words.join(" "),
                        font.clone(),
                        egui::Color32::WHITE,
                    );
                }
            }
        }
    }
    if overlay.chunks {
        let line = egui::Stroke::new(2.0, accent.colors().fill.gamma_multiply(0.85));
        let [across, down] = m.map.chunks();
        let point = |x: u32, y: u32| {
            to_screen(m.transform.transform_point3(Vec3::new(
                x as f32 * cell[0],
                -(y as f32) * cell[1],
                0.0,
            )))
        };
        for cx in 0..=across {
            let x = (cx * tiles::CHUNK).min(w);
            if let (Some(a), Some(b)) = (point(x, 0), point(x, h)) {
                painter.line_segment([a, b], line);
            }
        }
        for cy in 0..=down {
            let y = (cy * tiles::CHUNK).min(h);
            if let (Some(a), Some(b)) = (point(0, y), point(w, y)) {
                painter.line_segment([a, b], line);
            }
        }
    }
    if let Some(at) = overlay.hovered
        && let Some(r) = screen_cell(at)
    {
        painter.rect_stroke(
            r,
            0.0,
            egui::Stroke::new(2.0, accent.colors().fill),
            egui::StrokeKind::Inside,
        );
    }
    // The menu over the view: which values, and the chunks.
    let names: Vec<String> = m.map.values().iter().map(|v| v.name.clone()).collect();
    let shown: Vec<&str> = overlay
        .values
        .iter()
        .filter_map(|&v| names.get(v).map(String::as_str))
        .collect();
    let label = if shown.is_empty() {
        "Values: none ▾".to_owned()
    } else {
        format!("Values: {} ▾", shown.join(", "))
    };
    let mut bar = ui.new_child(
        egui::UiBuilder::new().max_rect(egui::Rect::from_center_size(
            egui::pos2(rect.center().x, rect.top() + 18.0),
            egui::vec2(rect.width().min(420.0), 24.0),
        )),
    );
    bar.horizontal(|ui| {
        ui.menu_button(label, |ui| {
            if names.is_empty() {
                ui.label("This map has no values yet: add them in the Map tab.");
            }
            for (i, n) in names.iter().enumerate() {
                let mut on = overlay.values.contains(&i);
                if ui.checkbox(&mut on, n).changed() {
                    overlay.asked.push((n.clone(), on));
                }
            }
        });
        let chunks = ui.add(egui::Button::new("Chunks").selected(overlay.chunks));
        if chunks
            .on_hover_text("Outline the map's chunks of 32 × 32 cells, which save and draw apart")
            .clicked()
        {
            overlay.chunks_asked = true;
        }
    });
    Some(bar.min_rect())
}

#[cfg(test)]
#[path = "tile_view_tests.rs"]
mod tests;
