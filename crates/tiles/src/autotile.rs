//! Autotiling (Phase 9, D5): which piece of art each cell shows, worked
//! out from its neighbours, so a map stores only what each cell means and
//! its edges come out smooth whatever the art.
//!
//! Each kind of terrain has numbered **slots**, one for each way its
//! neighbours can be ([`slot_count`]):
//!
//! - **Plain:** one slot, whatever the neighbours.
//! - **16-tile (dual grid):** art drawn half a tile off the cells. Each
//!   piece sits where four cells meet and is chosen by which of them are
//!   the terrain: top left 8, top right 4, bottom left 2, bottom right 1.
//!   Slot 0 (none of them) is never drawn.
//! - **47-tile ("blob"):** each cell's piece chosen by its eight
//!   neighbours, a corner counting only where both its sides are the same
//!   terrain: 47 ways in all.
//! - **Edges:** each cell's piece chosen by which of its four sides join
//!   the same terrain: north 1, east 2, south 4, west 8.
//! - **Rules:** the tileset's own patterns, tried in order; the first
//!   that matches gives the slot.
//!
//! A slot with more than one piece picks among them by their weights, the
//! same for the same cell every time (a hash of its place, never a
//! random number), so a map always looks the same. A slot with no piece
//! shows as missing (pink), never quietly as another.
//!
//! Everything here is pure arithmetic on the map and the tileset: no GPU,
//! no time.

use crate::map::{CHUNK, TileMap};
use crate::tileset::{Animation, Cond, Terrain, TileKind, Tileset};

/// How many slots a kind has.
pub fn slot_count(kind: TileKind) -> u32 {
    match kind {
        TileKind::Plain => 1,
        TileKind::Dual16 | TileKind::Edges => 16,
        TileKind::Blob47 => 47,
        TileKind::Rules => RULE_SLOTS,
    }
}

/// The most slots a terrain of its own rules can use.
pub const RULE_SLOTS: u32 = 64;

/// 47-tile neighbours: north, north-east, east, south-east, south,
/// south-west, west, north-west.
pub const N: u8 = 1;
/// North-east.
pub const NE: u8 = 2;
/// East.
pub const E: u8 = 4;
/// South-east.
pub const SE: u8 = 8;
/// South.
pub const S: u8 = 16;
/// South-west.
pub const SW: u8 = 32;
/// West.
pub const W: u8 = 64;
/// North-west.
pub const NW: u8 = 128;

/// A neighbour mask with corners kept only where both their sides are set.
pub const fn blob_reduce(mask: u8) -> u8 {
    let mut m = mask & (N | E | S | W);
    if mask & NE != 0 && mask & N != 0 && mask & E != 0 {
        m |= NE;
    }
    if mask & SE != 0 && mask & S != 0 && mask & E != 0 {
        m |= SE;
    }
    if mask & SW != 0 && mask & S != 0 && mask & W != 0 {
        m |= SW;
    }
    if mask & NW != 0 && mask & N != 0 && mask & W != 0 {
        m |= NW;
    }
    m
}

/// The 47 reduced masks, smallest first: slot `i` is `BLOB_MASKS[i]`.
pub const BLOB_MASKS: [u8; 47] = blob_masks();

const fn blob_masks() -> [u8; 47] {
    let mut out = [0u8; 47];
    let mut n = 0;
    let mut m = 0u16;
    while m < 256 {
        if blob_reduce(m as u8) == m as u8 {
            out[n] = m as u8;
            n += 1;
        }
        m += 1;
    }
    out
}

/// Each of the 256 masks' slot (after reducing it).
const BLOB_SLOT: [u8; 256] = blob_slots();

const fn blob_slots() -> [u8; 256] {
    let mut out = [0u8; 256];
    let mut m = 0u16;
    while m < 256 {
        let r = blob_reduce(m as u8);
        let mut i = 0;
        while i < 47 {
            if BLOB_MASKS[i] == r {
                out[m as usize] = i as u8;
            }
            i += 1;
        }
        m += 1;
    }
    out
}

/// The 47-tile slot of a neighbour mask.
pub fn blob_slot(mask: u8) -> u32 {
    u32::from(BLOB_SLOT[usize::from(mask)])
}

/// The slots a terrain needs pieces for: every one of its kind (a 16-tile
/// terrain's slot 0 is never drawn, so it isn't needed), or the slots its
/// rules name.
pub fn needed_slots(set: &Tileset, terrain: &Terrain) -> Vec<u32> {
    match terrain.kind {
        TileKind::Dual16 => (1..16).collect(),
        TileKind::Rules => {
            let mut slots: Vec<u32> = set.rules_of(terrain.id).map(|r| r.slot).collect();
            slots.sort_unstable();
            slots.dedup();
            slots
        }
        kind => (0..slot_count(kind)).collect(),
    }
}

/// The slots `terrain` needs that no piece fills: what the Tileset tab
/// shows dashed, and maps draw pink.
pub fn missing_slots(set: &Tileset, terrain: &Terrain) -> Vec<u32> {
    needed_slots(set, terrain)
        .into_iter()
        .filter(|&s| set.pieces_of(terrain.id, s).next().is_none())
        .collect()
}

/// A number from a cell's place, the same every time (a mixing hash).
fn place_hash(x: i64, y: i64, salt: u64) -> u64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ salt.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^ (h >> 33)
}

/// The piece (by index in the tileset) `terrain`'s `slot` shows at
/// (`x`, `y`): one of its variations by weight, the same for the same
/// place every time. `None` when the slot has no piece.
pub fn pick(set: &Tileset, terrain: &Terrain, slot: u32, x: i64, y: i64) -> Option<usize> {
    let mut total = 0.0f64;
    let mut first = None;
    let mut count = 0;
    for (i, p) in set.pieces_of(terrain.id, slot) {
        first.get_or_insert(i);
        total += f64::from(p.weight.max(0.0));
        count += 1;
    }
    let first = first?;
    if count == 1 {
        return Some(first);
    }
    let salt = terrain.id.raw() ^ u64::from(slot);
    let h = place_hash(x, y, salt);
    if total <= 0.0 {
        // Every weight 0: picked evenly.
        let n = (h % count as u64) as usize;
        return set.pieces_of(terrain.id, slot).nth(n).map(|(i, _)| i);
    }
    let mut at = (h >> 11) as f64 / (1u64 << 53) as f64 * total;
    let mut last = first;
    for (i, p) in set.pieces_of(terrain.id, slot) {
        let w = f64::from(p.weight.max(0.0));
        if w <= 0.0 {
            continue;
        }
        last = i;
        if at < w {
            return Some(i);
        }
        at -= w;
    }
    Some(last)
}

/// What a placed tile shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Art {
    /// A piece of the picture (by index in the tileset).
    Piece(usize),
    /// Nothing fills the slot: drawn pink, so it's noticed.
    Missing,
    /// The terrain has no art: drawn in its colour (by terrain index).
    Colour(usize),
}

/// One tile to draw: where (in half cells from the map's top-left corner,
/// so a 16-tile piece can sit half a cell off), and what.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Placed {
    /// When it's drawn among the layer's tiles: lower first, so later
    /// terrains' art lies over earlier ones'.
    pub pass: u16,
    /// Its top-left corner, in half cells (a cell's is twice its column).
    pub x: i32,
    /// Its top-left corner, in half cells (a cell's is twice its row).
    pub y: i32,
    /// What it shows.
    pub art: Art,
}

/// The terrain (by index) of each code, for a fast look-up.
fn terrain_of(set: &Tileset) -> [Option<u8>; 256] {
    let mut out = [None; 256];
    for (i, t) in set.terrains.iter().enumerate() {
        if let Ok(c) = u8::try_from(u32::from(t.code)) {
            out[usize::from(c)] = Some(i as u8);
        }
    }
    out
}

/// The tiles of `layer` of `map` within chunk `chunk` (column and row of
/// 32 × 32 cells), as `set` draws them. A 16-tile terrain's pieces at the
/// chunk's right and bottom edges are the next chunk's, except at the
/// map's own edge.
pub fn chunk_tiles(map: &TileMap, layer: usize, set: &Tileset, chunk: [u32; 2]) -> Vec<Placed> {
    let mut out = Vec::new();
    let Some(cells) = map.layer_cells(layer) else {
        return out;
    };
    let [w, h] = map.size();
    let (x0, y0) = (chunk[0] * CHUNK, chunk[1] * CHUNK);
    if x0 >= w || y0 >= h {
        return out;
    }
    let (x1, y1) = ((x0 + CHUNK).min(w), (y0 + CHUNK).min(h));
    let terrains = terrain_of(set);
    let code_at = |x: i64, y: i64| -> Option<u8> {
        (x >= 0 && y >= 0 && x < i64::from(w) && y < i64::from(h))
            .then(|| cells[y as usize * w as usize + x as usize])
    };
    let mut duals = false;
    // Cells painted with an exact tile (D6): drawn over everything else
    // of the layer, as they are; their cell's own art isn't drawn.
    let exact_pass = 1 + set.terrains.len() as u16;
    let mut exact = [false; (CHUNK * CHUNK) as usize];
    let local = |x: u32, y: u32| ((y - y0) * CHUNK + (x - x0)) as usize;
    for y in y0..y1 {
        for (x, piece) in map.exact_in_row(layer, y, x0..x1) {
            exact[local(x, y)] = true;
            let art = set
                .pieces
                .iter()
                .position(|p| p.id == piece)
                .map_or(Art::Missing, Art::Piece);
            out.push(Placed {
                pass: exact_pass,
                x: 2 * x as i32,
                y: 2 * y as i32,
                art,
            });
        }
    }
    for y in y0..y1 {
        for x in x0..x1 {
            let code = cells[y as usize * w as usize + x as usize];
            if code == 0 || exact[local(x, y)] {
                continue;
            }
            let (px, py) = (2 * x as i32, 2 * y as i32);
            let Some(ti) = terrains[usize::from(code)] else {
                out.push(Placed {
                    pass: 0,
                    x: px,
                    y: py,
                    art: Art::Missing,
                });
                continue;
            };
            let t = &set.terrains[usize::from(ti)];
            if !set.has_art(t.id) {
                out.push(Placed {
                    pass: 0,
                    x: px,
                    y: py,
                    art: Art::Colour(usize::from(ti)),
                });
                continue;
            }
            let (xi, yi) = (i64::from(x), i64::from(y));
            // Past the map's edge counts as the same terrain, so a map's
            // edges don't show a border.
            let same = |dx: i64, dy: i64| code_at(xi + dx, yi + dy).is_none_or(|c| c == code);
            let slot = match t.kind {
                TileKind::Dual16 => {
                    duals = true;
                    continue;
                }
                TileKind::Plain => Some(0),
                TileKind::Edges => Some(
                    u32::from(same(0, -1))
                        | u32::from(same(1, 0)) << 1
                        | u32::from(same(0, 1)) << 2
                        | u32::from(same(-1, 0)) << 3,
                ),
                TileKind::Blob47 => {
                    let mut m = 0u8;
                    for (bit, dx, dy) in [
                        (N, 0, -1),
                        (NE, 1, -1),
                        (E, 1, 0),
                        (SE, 1, 1),
                        (S, 0, 1),
                        (SW, -1, 1),
                        (W, -1, 0),
                        (NW, -1, -1),
                    ] {
                        if same(dx, dy) {
                            m |= bit;
                        }
                    }
                    Some(blob_slot(m))
                }
                TileKind::Rules => rule_slot(set, t, code, &code_at, &terrains, xi, yi),
            };
            let art = slot
                .and_then(|s| pick(set, t, s, xi, yi))
                .map_or(Art::Missing, Art::Piece);
            out.push(Placed {
                pass: if t.kind == TileKind::Plain {
                    0
                } else {
                    1 + u16::from(ti)
                },
                x: px,
                y: py,
                art,
            });
        }
    }
    if duals {
        dual_tiles(map, cells, set, &terrains, [x0, y0, x1, y1], &mut out);
    }
    out
}

/// A cell's slot from its terrain's rules: the first whose pattern holds.
fn rule_slot(
    set: &Tileset,
    t: &Terrain,
    code: u8,
    code_at: &impl Fn(i64, i64) -> Option<u8>,
    terrains: &[Option<u8>; 256],
    x: i64,
    y: i64,
) -> Option<u32> {
    'rules: for r in set.rules_of(t.id) {
        let mid = (r.size / 2) as i64;
        for (i, cond) in r.pattern.iter().enumerate() {
            let dx = (i % r.size) as i64 - mid;
            let dy = (i / r.size) as i64 - mid;
            let there = code_at(x + dx, y + dy).filter(|&c| c != 0);
            let holds = match cond {
                Cond::Any => true,
                Cond::Same => there == Some(code),
                Cond::Other => there != Some(code),
                Cond::Empty => there.is_none(),
                Cond::Terrain(c) => there.is_some_and(|there| {
                    u8::try_from(u32::from(*c)).is_ok_and(|c| c == there)
                        && terrains[usize::from(there)].is_some()
                }),
            };
            if !holds {
                continue 'rules;
            }
        }
        return Some(r.slot);
    }
    None
}

/// The 16-tile terrains' pieces at the corners (vertices) within a chunk:
/// vertex (`vx`, `vy`) is the top-left corner of cell (`vx`, `vy`), and
/// its piece covers the four cells around it.
fn dual_tiles(
    map: &TileMap,
    cells: &[u8],
    set: &Tileset,
    terrains: &[Option<u8>; 256],
    [x0, y0, x1, y1]: [u32; 4],
    out: &mut Vec<Placed>,
) {
    let [w, h] = map.size();
    let code_at = |x: i64, y: i64| -> u8 {
        if x >= 0 && y >= 0 && x < i64::from(w) && y < i64::from(h) {
            cells[y as usize * w as usize + x as usize]
        } else {
            0
        }
    };
    // The map's own last corners belong to its last chunk.
    let vx1 = if x1 == w { w + 1 } else { x1 };
    let vy1 = if y1 == h { h + 1 } else { y1 };
    let mut seen: Vec<u8> = Vec::with_capacity(4);
    for vy in y0..vy1 {
        for vx in x0..vx1 {
            let (xi, yi) = (i64::from(vx), i64::from(vy));
            let corners = [
                code_at(xi - 1, yi - 1),
                code_at(xi, yi - 1),
                code_at(xi - 1, yi),
                code_at(xi, yi),
            ];
            seen.clear();
            for &c in &corners {
                if c != 0 && !seen.contains(&c) {
                    seen.push(c);
                }
            }
            for &code in &seen {
                let Some(ti) = terrains[usize::from(code)] else {
                    continue;
                };
                let t = &set.terrains[usize::from(ti)];
                if t.kind != TileKind::Dual16 || !set.has_art(t.id) {
                    continue;
                }
                let slot = corners
                    .iter()
                    .fold(0u32, |m, &c| m << 1 | u32::from(c == code));
                let art = pick(set, t, slot, xi, yi).map_or(Art::Missing, Art::Piece);
                out.push(Placed {
                    pass: 1 + u16::from(ti),
                    x: 2 * vx as i32 - 1,
                    y: 2 * vy as i32 - 1,
                    art,
                });
            }
        }
    }
}

/// The frame of an animation to show at `ms` milliseconds: a column of
/// the picture (`Columns`), or a frame of an Aseprite picture (`Frames`,
/// with each frame's time). 0 when it doesn't animate.
pub fn frame_at(animation: Animation, ms: u64, picture_frames: &[u32]) -> u32 {
    match animation {
        Animation::None => 0,
        Animation::Columns { count, ms: each } => {
            if count < 2 || each == 0 {
                0
            } else {
                ((ms / u64::from(each)) % u64::from(count)) as u32
            }
        }
        Animation::Frames => {
            let total: u64 = picture_frames.iter().map(|&d| u64::from(d.max(1))).sum();
            if picture_frames.len() < 2 || total == 0 {
                return 0;
            }
            let mut at = ms % total;
            for (i, &d) in picture_frames.iter().enumerate() {
                let d = u64::from(d.max(1));
                if at < d {
                    return i as u32;
                }
                at -= d;
            }
            0
        }
    }
}

/// Where each slot of a kind sits on its template (and, for a picture
/// drawn over the template, which tile fills it): columns and rows.
pub fn template_size(kind: TileKind) -> [u32; 2] {
    match kind {
        TileKind::Plain => [1, 1],
        TileKind::Dual16 | TileKind::Edges => [4, 4],
        TileKind::Blob47 => [8, 6],
        TileKind::Rules => [8, RULE_SLOTS / 8],
    }
}

/// Slot `slot`'s place on its kind's template.
pub fn template_place(kind: TileKind, slot: u32) -> [u32; 2] {
    let [cols, _] = template_size(kind);
    [slot % cols, slot / cols]
}

/// Which parts of a slot's tile are the terrain, as a 3 × 3 picture (row
/// by row from the top left): what the template draws for it. For a
/// 16-tile slot, only the four corners count (the tile sits where four
/// cells meet); for 47-tile and edges, the middle is the cell and the
/// rest its neighbours.
pub fn slot_picture(kind: TileKind, slot: u32) -> [bool; 9] {
    match kind {
        TileKind::Plain | TileKind::Rules => [true; 9],
        TileKind::Dual16 => {
            let b = |bit: u32| slot & bit != 0;
            [
                b(8),
                b(8) && b(4),
                b(4),
                b(8) && b(2),
                slot != 0,
                b(4) && b(1),
                b(2),
                b(2) && b(1),
                b(1),
            ]
        }
        TileKind::Edges => {
            let b = |bit: u32| slot & bit != 0;
            [false, b(1), false, b(8), true, b(2), false, b(4), false]
        }
        TileKind::Blob47 => {
            let m = BLOB_MASKS.get(slot as usize).copied().unwrap_or(0);
            let b = |bit: u8| m & bit != 0;
            [b(NW), b(N), b(NE), b(W), true, b(E), b(SW), b(S), b(SE)]
        }
    }
}

/// A template to draw over, for a kind: each slot's tile at its place,
/// split in thirds with the terrain's parts in `on` and the rest in `off`
/// (straight RGBA), and a thin line round each tile. Its size in pixels,
/// and its pixels.
pub fn template_rgba(
    kind: TileKind,
    tile: [u32; 2],
    on: [u8; 3],
    off: [u8; 3],
) -> ([u32; 2], Vec<u8>) {
    let [cols, rows] = template_size(kind);
    let slots = slot_count(kind).min(cols * rows);
    let (w, h) = (cols * tile[0], rows * tile[1]);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for slot in 0..slots {
        let [cx, cy] = template_place(kind, slot);
        let picture = slot_picture(kind, slot);
        for py in 0..tile[1] {
            for px in 0..tile[0] {
                let third = |p: u32, size: u32| ((p * 3) / size.max(1)).min(2) as usize;
                let part = picture[third(py, tile[1]) * 3 + third(px, tile[0])];
                let edge = px == 0 || py == 0 || px + 1 == tile[0] || py + 1 == tile[1];
                let c = if edge {
                    [40, 40, 40]
                } else if part {
                    on
                } else {
                    off
                };
                let i = (((cy * tile[1] + py) * w + cx * tile[0] + px) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
    }
    ([w, h], rgba)
}

/// How a tile of a picture is filled: whether any pixel shows, and
/// whether every pixel is solid.
pub fn tile_fill(size: [u32; 2], rgba: &[u8], tile: [u32; 2], at: [u32; 2]) -> (bool, bool) {
    let mut any = false;
    let mut all = true;
    for py in 0..tile[1] {
        let y = at[1] * tile[1] + py;
        if y >= size[1] {
            return (any, false);
        }
        for px in 0..tile[0] {
            let x = at[0] * tile[0] + px;
            if x >= size[0] {
                return (any, false);
            }
            let a = rgba
                .get(((y * size[0] + x) * 4 + 3) as usize)
                .copied()
                .unwrap_or(0);
            any |= a > 0;
            all &= a == 255;
        }
    }
    (any, all)
}

/// The tile size a picture is most likely cut into: of the usual sizes
/// that divide it, the one whose seams differ most from the rest of the
/// picture (art changes at tiles' edges), the `preferred` size (the
/// project's) winning a near tie. Falls back to `preferred`.
pub fn detect_tile(size: [u32; 2], rgba: &[u8], preferred: [u32; 2]) -> [u32; 2] {
    let [w, h] = size;
    if w == 0 || h == 0 || rgba.len() < (w as usize * h as usize * 4) {
        return preferred;
    }
    let px = |x: u32, y: u32| -> [i32; 4] {
        let i = ((y * w + x) * 4) as usize;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]].map(i32::from)
    };
    let diff = |a: [i32; 4], b: [i32; 4]| -> u64 {
        a.iter()
            .zip(b)
            .map(|(a, b)| (a - b).unsigned_abs() as u64)
            .sum()
    };
    // How different neighbouring columns are, summed down the picture.
    let step = (h / 256).max(1);
    let mut cols = vec![0u64; w as usize];
    for x in 1..w {
        let mut s = 0;
        let mut y = 0;
        while y < h {
            s += diff(px(x - 1, y), px(x, y));
            y += step;
        }
        cols[x as usize] = s;
    }
    let step = (w / 256).max(1);
    let mut rows = vec![0u64; h as usize];
    for y in 1..h {
        let mut s = 0;
        let mut x = 0;
        while x < w {
            s += diff(px(x, y - 1), px(x, y));
            x += step;
        }
        rows[y as usize] = s;
    }
    let score = |line: &[u64], t: u32| -> f64 {
        let (mut seam, mut ns, mut rest, mut nr) = (0u64, 0u64, 0u64, 0u64);
        for (i, &v) in line.iter().enumerate().skip(1) {
            if (i as u32).is_multiple_of(t) {
                seam += v;
                ns += 1;
            } else {
                rest += v;
                nr += 1;
            }
        }
        if ns == 0 {
            return 0.0;
        }
        (seam as f64 / ns as f64 + 1.0) / (rest as f64 / nr.max(1) as f64 + 1.0)
    };
    let mut best: Option<([u32; 2], f64)> = None;
    let mut sizes: Vec<u32> = vec![8, 12, 16, 24, 32, 48, 64];
    if preferred[0] == preferred[1] && !sizes.contains(&preferred[0]) {
        sizes.push(preferred[0]);
    }
    for t in sizes {
        if t == 0 || w % t != 0 || h % t != 0 || (w / t) * (h / t) < 2 {
            continue;
        }
        let mut s = score(&cols, t) + score(&rows, t);
        if [t, t] == preferred {
            s *= 1.15;
        }
        if best.is_none_or(|(_, b)| s > b * 1.001) {
            best = Some(([t, t], s));
        }
    }
    match best {
        Some((t, s)) if s > 2.0 => t,
        _ if w % preferred[0].max(1) == 0 && h % preferred[1].max(1) == 0 => preferred,
        Some((t, _)) => t,
        None => preferred,
    }
}

/// The kind a terrain's art most likely is, from the picture's tiles
/// starting at `from` (the template's top-left tile): a block of 8 × 6
/// tiles nearly all drawn is 47-tile; a block of 4 × 4 drawn is 16-tile
/// when its tiles are mostly solid, edges when they're mostly see-through
/// (a fence over the ground); otherwise plain. `fill` says, for a tile,
/// whether any pixel shows and whether every one is solid; `grid` is the
/// picture's size in tiles.
pub fn guess_kind(
    fill: impl Fn([u32; 2]) -> (bool, bool),
    from: [u32; 2],
    grid: [u32; 2],
) -> TileKind {
    let block = |w: u32, h: u32| -> (usize, usize) {
        let (mut shown, mut solid) = (0, 0);
        for y in 0..h {
            for x in 0..w {
                let at = [from[0] + x, from[1] + y];
                if at[0] >= grid[0] || at[1] >= grid[1] {
                    continue;
                }
                let (any, all) = fill(at);
                shown += usize::from(any);
                solid += usize::from(all);
            }
        }
        (shown, solid)
    };
    let (shown, _) = block(8, 6);
    if shown >= 44 {
        return TileKind::Blob47;
    }
    let (shown, solid) = block(4, 4);
    if shown >= 15 {
        return if solid * 2 >= shown {
            TileKind::Dual16
        } else {
            TileKind::Edges
        };
    }
    TileKind::Plain
}

/// A terrain's pieces for a picture drawn over its kind's template: each
/// slot's tile from its place (offset by `from`, the template's top-left
/// tile in the picture), where that tile shows anything. Slots and
/// places.
pub fn mark_up(
    kind: TileKind,
    from: [u32; 2],
    shown: impl Fn([u32; 2]) -> bool,
) -> Vec<(u32, [u32; 2])> {
    (0..slot_count(kind))
        .filter(|_| kind != TileKind::Rules)
        .filter_map(|slot| {
            let [x, y] = template_place(kind, slot);
            let at = [from[0] + x, from[1] + y];
            shown(at).then_some((slot, at))
        })
        .collect()
}

#[cfg(test)]
#[path = "autotile_tests.rs"]
mod tests;
