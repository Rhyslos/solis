//! Tile maps drawn (Phase 9, D4 and D5).
//!
//! A map whose tileset has a picture is drawn tile by tile: each visible
//! 32 × 32 chunk's tiles worked out by autotiling (`tiles::autotile`), kept
//! until the chunk or one beside it changes, and drawn as sprites from one
//! atlas per picture (every frame of the picture stacked, and a small
//! white patch for terrains without art, tinted their colour, and missing
//! pieces, tinted pink). A 16-tile terrain's pieces sit half a cell off.
//! Animated pieces show the frame for the time given.
//!
//! A map without a picture, or seen from so far that too many chunks are
//! in view, is drawn as before: each layer one quad whose picture has a
//! texel per cell in its terrains' colours, made again only when its layer
//! changes. Layers under sprites are drawn first, under everything, and
//! layers over sprites last, over everything (`render::SpriteLayer`). The
//! app uploads a picture only when its key changes.

use crate::sprite_sheets::{SpriteBatch, SpriteSheets, ViewSprite};
use glam::{Mat4, Vec2, Vec3};
use reflect::{AssetId, EntityId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tiles::TileMap;
use tiles::autotile::{Art, Placed};

/// A code with no terrain in the map's tileset, or a slot with no piece:
/// drawn in this, so a missing terrain shows instead of quietly looking
/// like another.
pub(crate) const MISSING: [u8; 4] = [255, 0, 255, 255];

/// `render::SpriteInstance::UNDER` and `OVER`.
const UNDER: u32 = 16;
const OVER: u32 = 32;

/// The most chunks drawn tile by tile in one view: past this (seen from
/// far away), a map's layers are drawn as their colour pictures.
pub const MOST_CHUNKS: usize = 256;

/// The white patch at the bottom of a picture's atlas, in texels.
const WHITE: u32 = 4;

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

/// One picture the app uploads: a layer's colours, or a tileset's art.
#[derive(Debug, Clone, PartialEq)]
pub struct TilePicture {
    /// The ID it's drawn under.
    pub id: AssetId,
    /// Changes whenever the picture does: the app uploads it again then.
    pub key: u64,
    /// Its size in texels.
    pub size: [u32; 2],
    /// Straight sRGB RGBA, a row at a time.
    pub rgba: Arc<Vec<u8>>,
}

/// One chunk's tiles, as autotiling placed them, with what they were
/// worked out from.
type ChunkTiles = (u64, Arc<Vec<Placed>>);

/// Which chunk: a map's entity, a layer's ID, and the chunk's column and
/// row.
type ChunkId = (EntityId, reflect::Id, [u32; 2]);

/// Each layer's picture and each chunk's tiles, kept until they change.
#[derive(Debug, Default)]
pub struct TilePictures {
    made: Mutex<HashMap<(EntityId, usize), Arc<TilePicture>>>,
    atlases: Mutex<HashMap<u64, Arc<TilePicture>>>,
    chunks: Mutex<HashMap<ChunkId, ChunkTiles>>,
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

/// A picture's atlas: every frame stacked top to bottom, then a white
/// patch [`WHITE`] texels square. Its size and pixels.
pub(crate) fn atlas_rgba(art: &assets::tile_art::TileArt) -> ([u32; 2], Vec<u8>) {
    let [w, h] = art.size;
    let width = w.max(WHITE);
    let frames = art.frames.len() as u32;
    let height = h * frames + WHITE;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for (f, frame) in art.frames.iter().enumerate() {
        for y in 0..h {
            let from = (y * w * 4) as usize;
            let to = (((f as u32 * h + y) * width) * 4) as usize;
            if let (Some(src), Some(dst)) = (
                frame.get(from..from + (w * 4) as usize),
                rgba.get_mut(to..to + (w * 4) as usize),
            ) {
                dst.copy_from_slice(src);
            }
        }
    }
    for y in 0..WHITE {
        for x in 0..WHITE {
            let i = (((h * frames + y) * width + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&[255; 4]);
        }
    }
    ([width, height], rgba)
}

/// What a map is drawn with: its tileset and, if it has one, its picture.
struct Look<'a> {
    set: Option<&'a Arc<tiles::Tileset>>,
    art: Option<(&'a Arc<assets::tile_art::TileArt>, Arc<TilePicture>)>,
}

/// A number that changes when any of these chunks (the one asked for and
/// those beside it, which its edges read) changes.
fn chunk_key(map: &TileMap, chunk: [u32; 2], layer_cells: usize, set: usize) -> u64 {
    let mut key = reflect::Checksum::new();
    for dy in -1i64..=1 {
        for dx in -1i64..=1 {
            let (x, y) = (i64::from(chunk[0]) + dx, i64::from(chunk[1]) + dy);
            key.u64(if x < 0 || y < 0 {
                0
            } else {
                map.chunk_revision([x as u32, y as u32])
            });
        }
    }
    key.u64(layer_cells as u64);
    key.u64(set as u64);
    key.finish()
}

/// The chunks of a map within `bounds` (world, on the ground), or all of
/// them.
fn chunks_in(
    map: &TileMap,
    transform: Mat4,
    cell: [f32; 2],
    bounds: Option<[Vec2; 2]>,
) -> Vec<[u32; 2]> {
    let [across, down] = map.chunks();
    let (mut x0, mut x1, mut y0, mut y1) = (0, across, 0, down);
    if let Some([lo, hi]) = bounds {
        let inverse = transform.inverse();
        let corners = [
            Vec3::new(lo.x, lo.y, 0.0),
            Vec3::new(hi.x, lo.y, 0.0),
            Vec3::new(lo.x, hi.y, 0.0),
            Vec3::new(hi.x, hi.y, 0.0),
        ]
        .map(|p| inverse.transform_point3(p));
        let span = tiles::CHUNK as f32;
        let xs = corners.map(|c| c.x / cell[0] / span);
        let ys = corners.map(|c| -c.y / cell[1] / span);
        let fmin = |v: [f32; 4]| v.into_iter().fold(f32::INFINITY, f32::min);
        let fmax = |v: [f32; 4]| v.into_iter().fold(f32::NEG_INFINITY, f32::max);
        // A cell's art can reach half a cell past it: a chunk more each way.
        let clamp = |v: f32, most: u32| (v.max(0.0) as u32).min(most);
        if !(fmin(xs).is_finite()
            && fmax(xs).is_finite()
            && fmin(ys).is_finite()
            && fmax(ys).is_finite())
        {
            return Vec::new();
        }
        x0 = clamp(fmin(xs).floor() - 1.0, across);
        x1 = clamp(fmax(xs).floor() + 2.0, across);
        y0 = clamp(fmin(ys).floor() - 1.0, down);
        y1 = clamp(fmax(ys).floor() + 2.0, down);
    }
    let mut out = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            out.push([x, y]);
        }
    }
    out
}

impl TilePictures {
    /// The pictures and the batches that draw `maps`, cells `cell` metres
    /// across, leaving out layers `hidden` in the editor (by ID). `ground`
    /// gives the part of the ground at a height that's in view (none for
    /// all of it); `ms` is the time animated tiles show.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        maps: &[ViewTileMap],
        sheets: &SpriteSheets,
        cell: [f32; 2],
        hidden: &std::collections::HashSet<reflect::Id>,
        ground: &dyn Fn(f32) -> Option<[Vec2; 2]>,
        ms: u64,
        meaning: Option<EntityId>,
    ) -> (Vec<Arc<TilePicture>>, Vec<SpriteBatch>) {
        let mut pictures: Vec<Arc<TilePicture>> = Vec::new();
        let mut under = Vec::new();
        let mut over = Vec::new();
        let mut used = std::collections::HashSet::new();
        let mut used_atlases = std::collections::HashSet::new();
        let mut used_chunks = std::collections::HashSet::new();
        for m in maps {
            let set = m.map.tileset.and_then(|t| sheets.tileset(t));
            let look = Look {
                set,
                // Its meaning shown (D6): each terrain in its colour.
                art: set.filter(|_| meaning != Some(m.entity)).and_then(|s| {
                    let art = sheets.tile_art(s.picture?)?;
                    let key = Arc::as_ptr(art) as u64;
                    used_atlases.insert(key);
                    let atlas = self.atlas(key, art);
                    Some((art, atlas))
                }),
            };
            let bounds = ground(m.transform.w_axis.z);
            let chunks = if look.art.is_some() {
                chunks_in(&m.map, m.transform, cell, bounds)
            } else {
                Vec::new()
            };
            let tile_by_tile = look.art.is_some() && chunks.len() <= MOST_CHUNKS;
            if let (true, Some((_, atlas))) = (tile_by_tile, &look.art)
                && !pictures.iter().any(|p| p.id == atlas.id)
            {
                pictures.push(Arc::clone(atlas));
            }
            for (i, layer) in m.map.layers().iter().enumerate() {
                if hidden.contains(&layer.id) {
                    continue;
                }
                let batch = if tile_by_tile {
                    self.tiles_of(m, i, &look, &chunks, cell, ms, &mut used_chunks)
                } else {
                    used.insert((m.entity, i));
                    let picture = self.layer_picture(m, i, set);
                    let batch = colour_batch(m, i, &picture, cell);
                    pictures.push(picture);
                    batch
                };
                if layer.over {
                    over.push(batch);
                } else {
                    under.push(batch);
                }
            }
        }
        lock(&self.made).retain(|k, _| used.contains(k));
        lock(&self.atlases).retain(|k, _| used_atlases.contains(k));
        lock(&self.chunks).retain(|k, _| used_chunks.contains(k));
        under.extend(over);
        under.retain(|b| !b.sprites.is_empty());
        (pictures, under)
    }

    /// The atlas of picture `art` (by `key`), made once.
    fn atlas(&self, key: u64, art: &Arc<assets::tile_art::TileArt>) -> Arc<TilePicture> {
        let mut atlases = lock(&self.atlases);
        Arc::clone(atlases.entry(key).or_insert_with(|| {
            let (size, rgba) = atlas_rgba(art);
            let mut sum = reflect::Checksum::new();
            sum.bytes(b"tile art");
            sum.u64(key);
            Arc::new(TilePicture {
                id: AssetId(
                    reflect::Id::from_raw(sum.finish() | 1).unwrap_or_else(reflect::Id::random),
                ),
                key,
                size,
                rgba: Arc::new(rgba),
            })
        }))
    }

    /// Layer `i` of `m` as its colour picture, made again only when it
    /// changes.
    fn layer_picture(
        &self,
        m: &ViewTileMap,
        i: usize,
        set: Option<&Arc<tiles::Tileset>>,
    ) -> Arc<TilePicture> {
        let size = m.map.size();
        let mut key = reflect::Checksum::new();
        key.u64(m.map.layer_cells(i).map_or(0, |c| c.as_ptr() as u64));
        key.u64(m.map.layer_revision(i));
        key.u64(set.map_or(0, |t| Arc::as_ptr(t) as u64));
        key.u64(u64::from(size[0]) << 32 | u64::from(size[1]));
        let key = key.finish();
        let mut made = lock(&self.made);
        match made.get(&(m.entity, i)) {
            Some(p) if p.key == key => Arc::clone(p),
            _ => {
                let p = Arc::new(TilePicture {
                    id: picture_id(m.entity, i),
                    key,
                    size,
                    rgba: Arc::new(layer_rgba(&m.map, i, set.map(|t| &**t))),
                });
                made.insert((m.entity, i), Arc::clone(&p));
                p
            }
        }
    }

    /// Layer `i` of `m` tile by tile, in `chunks`.
    #[allow(clippy::too_many_arguments)]
    fn tiles_of(
        &self,
        m: &ViewTileMap,
        i: usize,
        look: &Look<'_>,
        chunks: &[[u32; 2]],
        cell: [f32; 2],
        ms: u64,
        used: &mut std::collections::HashSet<ChunkId>,
    ) -> SpriteBatch {
        let (Some(set), Some((art, atlas))) = (look.set, &look.art) else {
            return SpriteBatch {
                sheet: AssetId(reflect::Id::random()),
                see_through: false,
                sprites: Vec::new(),
                picks: Vec::new(),
            };
        };
        let layer = &m.map.layers()[i];
        let cells_ptr = m.map.layer_cells(i).map_or(0, |c| c.as_ptr() as usize);
        let set_ptr = Arc::as_ptr(set) as usize;
        let mut placed: Vec<Placed> = Vec::new();
        {
            let mut cache = lock(&self.chunks);
            for &c in chunks {
                let key = chunk_key(&m.map, c, cells_ptr, set_ptr);
                let id = (m.entity, layer.id, c);
                used.insert(id);
                let tiles = match cache.get(&id) {
                    Some((k, tiles)) if *k == key => Arc::clone(tiles),
                    _ => {
                        let tiles = Arc::new(tiles::autotile::chunk_tiles(&m.map, i, set, c));
                        cache.insert(id, (key, Arc::clone(&tiles)));
                        tiles
                    }
                };
                placed.extend(tiles.iter().copied());
            }
        }
        // Later terrains over earlier ones, across chunks.
        placed.sort_by_key(|p| p.pass);
        let flags = if layer.over { OVER } else { UNDER };
        let [tw, th] = set.tile;
        let frames = art.frames.len() as u32;
        let white_y = art.size[1] * frames;
        let across = m.transform.transform_vector3(Vec3::X);
        let down = m.transform.transform_vector3(-Vec3::Y);
        let origin = m.transform.transform_point3(Vec3::ZERO);
        let sprites: Vec<ViewSprite> = placed
            .iter()
            .map(|p| {
                let corner = origin
                    + across * (p.x as f32 * 0.5 * cell[0])
                    + down * (p.y as f32 * 0.5 * cell[1]);
                let (rect, tint, texels) = match p.art {
                    Art::Piece(index) => {
                        let piece = &set.pieces[index];
                        let frame = tiles::autotile::frame_at(piece.animation, ms, &art.durations);
                        let (col, row) = match piece.animation {
                            tiles::Animation::Columns { .. } => (frame, 0),
                            tiles::Animation::Frames => (0, frame.min(frames.saturating_sub(1))),
                            tiles::Animation::None => (0, 0),
                        };
                        (
                            [
                                ((piece.at[0] + col) * tw) as f32,
                                (row * art.size[1] + piece.at[1] * th) as f32,
                                tw as f32,
                                th as f32,
                            ],
                            [1.0, 1.0, 1.0],
                            [tw as f32, th as f32],
                        )
                    }
                    Art::Missing => (
                        [0.0, white_y as f32, WHITE as f32, WHITE as f32],
                        [1.0, 0.0, 1.0],
                        [WHITE as f32; 2],
                    ),
                    Art::Colour(t) => (
                        [0.0, white_y as f32, WHITE as f32, WHITE as f32],
                        crate::sprite_sheets::linear(set.terrains[t].colour),
                        [WHITE as f32; 2],
                    ),
                };
                let ax = across * (cell[0] / texels[0]);
                let ay = down * (cell[1] / texels[1]);
                ViewSprite {
                    corner: corner.to_array(),
                    flags,
                    axis_x: [ax.x, ax.y],
                    axis_y: [ay.x, ay.y],
                    rect,
                    tint: [tint[0], tint[1], tint[2], 0.0],
                    outline: [0.0; 4],
                    foot: [corner.x, corner.y],
                }
            })
            .collect();
        SpriteBatch {
            sheet: atlas.id,
            see_through: false,
            picks: vec![0; sprites.len()],
            sprites,
        }
    }
}

/// A layer drawn as one quad of its colour picture.
fn colour_batch(m: &ViewTileMap, i: usize, picture: &TilePicture, cell: [f32; 2]) -> SpriteBatch {
    let layer = &m.map.layers()[i];
    let size = m.map.size();
    let corner = m.transform.transform_point3(Vec3::ZERO);
    let across = m.transform.transform_vector3(Vec3::X * cell[0]);
    let down = m.transform.transform_vector3(-Vec3::Y * cell[1]);
    SpriteBatch {
        sheet: picture.id,
        see_through: false,
        sprites: vec![ViewSprite {
            corner: corner.to_array(),
            flags: if layer.over { OVER } else { UNDER },
            axis_x: [across.x, across.y],
            axis_y: [down.x, down.y],
            rect: [0.0, 0.0, size[0] as f32, size[1] as f32],
            tint: [1.0, 1.0, 1.0, 0.0],
            outline: [0.0; 4],
            foot: [corner.x, corner.y],
        }],
        picks: vec![0],
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The part of the ground (at height `z`) a view sees, from where the
/// rays through its corners land (by its `view_projection`); `None` when
/// a corner looks above the horizon (then everything may be in view).
pub fn ground_in_view(view_projection: Mat4, z: f32) -> Option<[Vec2; 2]> {
    let inverse = view_projection.inverse();
    let mut lo = Vec2::splat(f32::INFINITY);
    let mut hi = Vec2::splat(f32::NEG_INFINITY);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        // Depth is reversed: 1 is near, 0 far (or at infinity).
        let near = inverse.project_point3(Vec3::new(x, y, 1.0));
        let far = inverse.project_point3(Vec3::new(x, y, 0.001));
        let d = far - near;
        if !(near.is_finite() && far.is_finite()) || d.z.abs() < 1e-9 {
            return None;
        }
        let t = (z - near.z) / d.z;
        if !(t.is_finite() && t >= 0.0) {
            return None;
        }
        let p = near + d * t;
        lo = lo.min(p.truncate());
        hi = hi.max(p.truncate());
    }
    Some([lo, hi])
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

/// The cell of a map placed by `transform` (cells `cell` metres across)
/// at `point` on the ground, as whole column and row, even past the
/// map's edges (a rectangle painted from outside the map).
pub fn cell_off_map(transform: Mat4, cell: [f32; 2], point: Vec3) -> Option<[i64; 2]> {
    let local = transform.inverse().transform_point3(point);
    let (x, y) = ((local.x / cell[0]).floor(), (-local.y / cell[1]).floor());
    let far = 1.0e6;
    (x.is_finite() && y.is_finite() && x.abs() < far && y.abs() < far)
        .then_some([x as i64, y as i64])
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
    /// The tile painter's settings (D6).
    pub paint: crate::tile_paint::PaintSettings,
    /// The cell under the pointer, even off the map's edge (painting a
    /// rectangle that starts outside), found while drawing.
    pub pointer: Option<[i64; 2]>,
    /// The mouse this frame, while painting.
    pub input: crate::tile_paint::PaintInput,
    /// The cells a drag will paint (or copy), as rectangles of cells
    /// (top-left and bottom-right, inclusive), drawn over the map.
    pub preview: Vec<[[i64; 2]; 2]>,
    /// The preview empties cells (drawn in red).
    pub preview_erase: bool,
    /// A stamp button pressed this frame: a turn, or `None` to drop it.
    pub turn_asked: Option<Option<crate::tile_paint::Turn>>,
}

/// The most preview rectangles drawn (a long line's cells).
const MOST_PREVIEWED: usize = 4096;

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
        overlay.pointer = cell_off_map(m.transform, cell, ground);
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
    // What a drag will paint, or the brush under the pointer.
    let cells_rect = |[a, b]: [[i64; 2]; 2]| -> Option<egui::Rect> {
        let corner = |x: i64, y: i64| {
            to_screen(m.transform.transform_point3(Vec3::new(
                x as f32 * cell[0],
                -(y as f32) * cell[1],
                0.0,
            )))
        };
        Some(egui::Rect::from_two_pos(
            corner(a[0], a[1])?,
            corner(b[0] + 1, b[1] + 1)?,
        ))
    };
    let paint = &overlay.paint;
    let colour = if overlay.preview_erase {
        egui::Color32::from_rgb(0xe0, 0x50, 0x50)
    } else {
        accent.colors().fill
    };
    if paint.painting() {
        let mut rects = overlay.preview.clone();
        if rects.is_empty()
            && let Some(p) = overlay.pointer
        {
            // The brush: the stamp held, round the pointer, or one cell.
            let (size, handle) = match &paint.stamp {
                Some(s) if paint.tool != Some(crate::tile_paint::PaintTool::Eyedropper) => {
                    (s.size, s.handle())
                }
                _ => ([1, 1], [0, 0]),
            };
            let a = [p[0] - handle[0], p[1] - handle[1]];
            rects.push([
                a,
                [a[0] + i64::from(size[0]) - 1, a[1] + i64::from(size[1]) - 1],
            ]);
        }
        for r in rects
            .iter()
            .take(MOST_PREVIEWED)
            .filter_map(|&r| cells_rect(r))
        {
            painter.rect_filled(r, 0.0, colour.gamma_multiply(0.25));
            painter.rect_stroke(
                r,
                0.0,
                egui::Stroke::new(1.5, colour),
                egui::StrokeKind::Inside,
            );
        }
    }
    if let Some(at) = overlay.hovered
        && !paint.painting()
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
