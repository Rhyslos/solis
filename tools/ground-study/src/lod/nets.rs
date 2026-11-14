//! Smooth and Sharp at any level: surface nets (sharpened for Sharp)
//! over a chunk of any level, and three ways of meeting a neighbour of
//! another level.
//!
//! The chunks of one level make one surface between them, as the ground
//! study's surface nets do: a point in each cube the surface passes
//! through, a square round each edge it crosses. Where two levels meet,
//! their surfaces don't line up, and a crack opens. The ways compared:
//!
//! - [`Seams::None`]: nothing done, to count the cracks.
//! - [`Seams::Skirts`]: each chunk hangs a strip down from its open edges
//!   ([`super::skirts`]), hiding the crack; meshes never depend on the
//!   neighbours.
//! - [`Seams::Transition`]: a band of joining cells. On the border with
//!   a coarser column, the finer chunk's samples take the coarser
//!   level's values (its samples there, and halfway between them the
//!   average), and its border cells are joined straight to the coarser
//!   chunk's cells, a square or a triangle round each crossed edge; the
//!   coarser chunk leaves those out. The surface is closed, but both
//!   chunks are made again when a neighbour's level changes.
//! - [`Seams::Stitched`]: the same joins, kept apart from the chunk's
//!   own mesh (`border` in [`Made`]), so that when a neighbour's level
//!   changes only the border is made again.
//!
//! Every sample's value comes from one rule, whichever chunk asks: a
//! place on the edge of a coarser column takes that level's value. So
//! two chunks making the same cell make the same point, to the bit.

use super::layout::Layout;
use super::{Levels, chunk_metres, skirts, voxel};
use crate::ground::Samples;
use crate::mesh::{ChunkMesh, sharpen};
use crate::voxel::{Dense, SIZE_I, Voxel};
use glam::{IVec2, IVec3, Vec3};
use std::collections::HashMap;

/// How a chunk meets a neighbour of another level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Seams {
    /// Nothing: cracks show.
    None,
    /// A strip hanging down from the chunk's open edges.
    Skirts,
    /// Joining cells, in the finer chunk's mesh.
    Transition,
    /// Joining cells, in a mesh of their own beside the chunk's.
    Stitched,
}

impl Seams {
    /// Every way, in the report's order.
    pub const ALL: [Self; 4] = [Self::None, Self::Skirts, Self::Transition, Self::Stitched];

    /// Its name in the report.
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "Nothing",
            Self::Skirts => "Skirts",
            Self::Transition => "Transition band",
            Self::Stitched => "Stitched seams",
        }
    }
}

/// The smooth looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Look {
    /// Surface nets.
    Smooth,
    /// Surface nets sharpened, hard edges where faces meet steeply.
    Sharp,
}

/// A chunk's meshes, in metres.
#[derive(Debug, Default, Clone)]
pub struct Made {
    /// The chunk's own triangles.
    pub mesh: ChunkMesh,
    /// With [`Seams::Stitched`], the joins at its borders (empty
    /// otherwise).
    pub border: ChunkMesh,
}

impl Made {
    /// Triangles in both.
    pub fn triangles(&self) -> usize {
        self.mesh.triangles() + self.border.triangles()
    }
}

const HARD_EDGE_DEGREES: f32 = 40.0;
const PULL_STEPS: usize = 4;
const AXES: [IVec3; 3] = [IVec3::X, IVec3::Y, IVec3::Z];
/// Samples cached round a chunk: from one before its first to two after
/// its last.
const LO: i32 = -1;
const SPAN: i32 = SIZE_I + 3;

/// A sample's distance (in its level's voxels) and kind.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Sample {
    d: f32,
    kind: u8,
}

impl Sample {
    fn of(v: Voxel) -> Self {
        Self {
            d: v.distance(),
            kind: v.material,
        }
    }

    fn solid(self) -> bool {
        self.d < 0.0
    }
}

/// Which levels each place is seen at: the layout's, or one level
/// everywhere (for the seams that don't look at neighbours).
#[derive(Clone, Copy)]
struct Scope<'a> {
    layout: Option<&'a Layout>,
    level: u32,
}

impl Scope<'_> {
    /// The level of the column holding `centre` (metres).
    fn level_at(&self, centre: Vec3) -> u32 {
        self.layout
            .map_or(self.level, |l| l.level_at(centre.truncate()))
    }

    /// The level whose values the sample `g` of `level` takes.
    fn value_level(&self, level: u32, g: IVec3) -> u32 {
        match self.layout {
            None => level,
            Some(layout) => layout.coarsest_at(g.truncate() << level).max(level),
        }
    }
}

/// The value of sample `g` of `level` as every chunk sees it.
fn value(levels: &impl Levels, scope: Scope, level: u32, g: IVec3) -> Sample {
    let top = scope.value_level(level, g);
    if top == level {
        return Sample::of(levels.voxel(level, g));
    }
    // A coarser level's samples are every 2^k of this one's.
    let k = top - level;
    let step = 1 << k;
    let base = g.div_euclid(IVec3::splat(step));
    let rest = (g - base * step).as_vec3() / step as f32;
    let at = |d: IVec3| levels.voxel(top, base + d);
    let near = at((rest * 2.0).floor().as_ivec3().min(IVec3::ONE));
    if rest == Vec3::ZERO {
        let v = at(IVec3::ZERO);
        return Sample {
            d: v.distance() * step as f32,
            kind: v.material,
        };
    }
    let mut d = 0.0;
    for corner in 0..8 {
        let c = IVec3::new(corner & 1, (corner >> 1) & 1, corner >> 2);
        let w = Vec3::select(c.cmpeq(IVec3::ONE), rest, Vec3::ONE - rest);
        d += w.x * w.y * w.z * at(c).distance();
    }
    Sample {
        d: d * step as f32,
        kind: near.material,
    }
}

/// One chunk's meshing.
struct Mesher<'a, L: Levels> {
    levels: &'a L,
    scope: Scope<'a>,
    level: u32,
    key: IVec3,
    look: Look,
    seams: Seams,
    only_border: bool,
    cache: Vec<Sample>,
    chunks: IVec3,
    size: IVec3,
}

/// The chunk `key` of `level`, meshed in `look`, meeting its neighbours
/// (as `layout` places them) by `seams`. Points are in metres.
pub fn mesh(
    levels: &impl Levels,
    layout: &Layout,
    level: u32,
    key: IVec3,
    look: Look,
    seams: Seams,
) -> Made {
    let mut made = Mesher::new(levels, layout, level, key, look, seams, false).emit();
    if look == Look::Sharp {
        made.mesh = sharpen(&made.mesh, HARD_EDGE_DEGREES);
        made.border = sharpen(&made.border, HARD_EDGE_DEGREES);
    }
    if seams == Seams::Skirts {
        let lo = (key.truncate() * chunk_metres(level)).as_vec2();
        let hi = lo + chunk_metres(level) as f32;
        skirts::hang(&mut made.mesh, lo, hi, 2.0 * voxel(level + 1));
    }
    made
}

/// Just the border of the chunk `key` of `level` with [`Seams::Stitched`]:
/// what is made again when a neighbour's level changes.
pub fn border(
    levels: &impl Levels,
    layout: &Layout,
    level: u32,
    key: IVec3,
    look: Look,
) -> ChunkMesh {
    let made = Mesher::new(levels, layout, level, key, look, Seams::Stitched, true).emit();
    match look {
        Look::Smooth => made.border,
        Look::Sharp => sharpen(&made.border, HARD_EDGE_DEGREES),
    }
}

impl<'a, L: Levels> Mesher<'a, L> {
    fn new(
        levels: &'a L,
        layout: &'a Layout,
        level: u32,
        key: IVec3,
        look: Look,
        seams: Seams,
        only_border: bool,
    ) -> Self {
        let neighbours_matter = matches!(seams, Seams::Transition | Seams::Stitched);
        let size = levels.size(level);
        let mut mesher = Self {
            levels,
            scope: Scope {
                layout: neighbours_matter.then_some(layout),
                level,
            },
            level,
            key,
            look,
            seams,
            only_border,
            cache: Vec::new(),
            chunks: (size + SIZE_I - 1).div_euclid(IVec3::splat(SIZE_I)),
            size,
        };
        mesher.fill_cache();
        mesher
    }

    fn fill_cache(&mut self) {
        let mut samples = Samples::default();
        let mut scratch = Dense::filled(Voxel::AIR);
        self.levels
            .gather(self.level, self.key, &mut scratch, &mut samples);
        self.cache = Vec::with_capacity((SPAN * SPAN * SPAN) as usize);
        let origin = self.key * SIZE_I;
        for z in LO..LO + SPAN {
            for y in LO..LO + SPAN {
                for x in LO..LO + SPAN {
                    let p = IVec3::new(x, y, z);
                    let edge = |v: i32| v <= 0 || v >= SIZE_I;
                    let sample = if self.scope.layout.is_some() && (edge(x) || edge(y)) {
                        value(self.levels, self.scope, self.level, origin + p)
                    } else {
                        Sample::of(samples.get(p))
                    };
                    self.cache.push(sample);
                }
            }
        }
    }

    /// The sample at `g` of `level`: from the cache when it's there.
    fn sample(&self, level: u32, g: IVec3) -> Sample {
        if level == self.level {
            let q = g - self.key * SIZE_I - LO;
            if q.cmpge(IVec3::ZERO).all() && q.cmplt(IVec3::splat(SPAN)).all() {
                return self.cache[(q.x + SPAN * (q.y + SPAN * q.z)) as usize];
            }
        }
        value(self.levels, self.scope, level, g)
    }

    /// The chunk of `level` holding the cell centred at `centre`, held
    /// to the world (cells past its edges belong to the edge's chunk).
    fn owner(&self, centre: Vec3) -> IVec3 {
        let chunk = chunk_metres(self.level) as f32;
        let key = (centre / chunk).floor().as_ivec3();
        key.clamp(IVec3::ZERO, self.chunks - 1)
    }

    fn emit(&self) -> Made {
        let mut made = Made::default();
        let mut points: [HashMap<(u32, IVec3), u32>; 2] = [HashMap::new(), HashMap::new()];
        let origin = self.key * SIZE_I;
        let width = voxel(self.level);
        for z in 0..=SIZE_I {
            for y in 0..=SIZE_I {
                for x in 0..=SIZE_I {
                    let s = IVec3::new(x, y, z);
                    let inner = |v: i32| (2..SIZE_I - 1).contains(&v);
                    if self.only_border && inner(x) && inner(y) {
                        continue;
                    }
                    let g = origin + s;
                    if g.cmplt(IVec3::ZERO).any() || g.cmpge(self.size).any() {
                        continue;
                    }
                    let here = self.sample(self.level, g);
                    for (a, axis) in AXES.iter().enumerate() {
                        if s[a] == SIZE_I
                            || here.solid() == self.sample(self.level, g + *axis).solid()
                        {
                            continue;
                        }
                        let (b, c) = (AXES[(a + 1) % 3], AXES[(a + 2) % 3]);
                        let ring = [g - b - c, g - c, g, g - b];
                        let Some((mut cells, interior)) = self.ring(ring, width) else {
                            continue;
                        };
                        if !here.solid() {
                            cells.reverse();
                        }
                        let target = usize::from(self.seams == Seams::Stitched && !interior);
                        let out = if target == 0 {
                            &mut made.mesh
                        } else {
                            &mut made.border
                        };
                        let mut corners: Vec<u32> = Vec::with_capacity(4);
                        for id in cells {
                            let i = match points[target].get(&id) {
                                Some(i) => *i,
                                None => {
                                    let i = self.point(out, id);
                                    points[target].insert(id, i);
                                    i
                                }
                            };
                            if corners.last() != Some(&i) && corners.first() != Some(&i) {
                                corners.push(i);
                            }
                        }
                        push(out, &corners);
                    }
                }
            }
        }
        made
    }

    /// The cells round an edge, as each is drawn (its level and its
    /// index there), if this chunk makes the edge's polygon; and whether
    /// all of them are this chunk's own, away from its borders.
    fn ring(&self, ring: [IVec3; 4], width: f32) -> Option<(Vec<(u32, IVec3)>, bool)> {
        let mut first = None;
        let mut cells = Vec::with_capacity(4);
        let mut interior = true;
        for g in ring {
            let centre = (g.as_vec3() + 0.5) * width;
            let level = self.scope.level_at(centre);
            if level < self.level {
                return None; // the finer side makes it
            }
            if level == self.level {
                first.get_or_insert(centre);
                cells.push((level, g));
                let local = g - self.key * SIZE_I;
                interior &= local.x >= 1 && local.x < SIZE_I - 1;
                interior &= local.y >= 1 && local.y < SIZE_I - 1;
            } else {
                cells.push((level, (centre / voxel(level)).floor().as_ivec3()));
                interior = false;
            }
        }
        let first = first?;
        (self.owner(first) == self.key).then_some((cells, interior))
    }

    /// Makes the point of the cell `id` (its level and lowest corner).
    fn point(&self, out: &mut ChunkMesh, (level, cell): (u32, IVec3)) -> u32 {
        let corners: [Sample; 8] = std::array::from_fn(|i| {
            let i = i as i32;
            let c = IVec3::new(i & 1, (i >> 1) & 1, i >> 2);
            self.sample(level, cell + c)
        });
        let (local, normal, kind) = place(&corners, self.look);
        let position = (cell.as_vec3() + local) * voxel(level);
        out.push_point(position, normal, kind)
    }
}

/// Where a cell's point goes (in the cell, 0 to 1), its facing and its
/// kind, from its eight corners (x fastest, then y, then z).
fn place(corners: &[Sample; 8], look: Look) -> (Vec3, Vec3, u8) {
    let mut places = [Vec3::ZERO; 12];
    let mut count = 0;
    for (i, here) in corners.iter().enumerate() {
        let c = IVec3::new(i as i32 & 1, (i as i32 >> 1) & 1, i as i32 >> 2);
        for (a, axis) in AXES.iter().enumerate() {
            if c[a] == 1 {
                continue;
            }
            let there = corners[i + (1 << a)];
            if here.solid() == there.solid() {
                continue;
            }
            let t = (here.d / (here.d - there.d)).clamp(0.0, 1.0);
            places[count] = c.as_vec3() + axis.as_vec3() * t;
            count += 1;
        }
    }
    let middle = if count == 0 {
        Vec3::splat(0.5)
    } else {
        places[..count].iter().sum::<Vec3>() / count as f32
    };
    let local = match look {
        Look::Smooth => middle,
        Look::Sharp => pulled(corners, &places[..count], middle),
    }
    .clamp(Vec3::ZERO, Vec3::ONE);
    let normal = gradient(corners, local).normalize_or(Vec3::Z);
    (local, normal, most_common(corners))
}

/// The middle pulled a few steps towards the planes the surface crosses
/// the cube's edges at.
fn pulled(corners: &[Sample; 8], places: &[Vec3], middle: Vec3) -> Vec3 {
    if places.is_empty() {
        return middle;
    }
    let normals: Vec<Vec3> = places
        .iter()
        .map(|p| gradient(corners, *p).normalize_or_zero())
        .collect();
    let mut x = middle;
    for _ in 0..PULL_STEPS {
        let mut step = Vec3::ZERO;
        for (p, n) in places.iter().zip(&normals) {
            step += *n * n.dot(*p - x);
        }
        x += step / places.len() as f32;
    }
    x
}

/// How the distance grows at `at` in the cube (towards the air), from
/// its corners blended across it.
fn gradient(corners: &[Sample; 8], at: Vec3) -> Vec3 {
    let d = |i: usize| corners[i].d;
    let (u, v, w) = (at.x, at.y, at.z);
    let blend = |a: f32, b: f32, c: f32, e: f32, s: f32, t: f32| {
        a * (1.0 - s) * (1.0 - t) + b * s * (1.0 - t) + c * (1.0 - s) * t + e * s * t
    };
    Vec3::new(
        blend(d(1) - d(0), d(3) - d(2), d(5) - d(4), d(7) - d(6), v, w),
        blend(d(2) - d(0), d(3) - d(1), d(6) - d(4), d(7) - d(5), u, w),
        blend(d(4) - d(0), d(5) - d(1), d(6) - d(2), d(7) - d(3), u, v),
    )
}

/// The most common kind among the solid corners, the lowest of equals.
fn most_common(corners: &[Sample; 8]) -> u8 {
    let mut best = (0, 0u8);
    for here in corners.iter().filter(|c| c.solid()) {
        let n = corners
            .iter()
            .filter(|c| c.solid() && c.kind == here.kind)
            .count();
        if n > best.0 || (n == best.0 && here.kind < best.1) {
            best = (n, here.kind);
        }
    }
    best.1
}

/// A square (split along its shorter diagonal) or a triangle.
fn push(out: &mut ChunkMesh, corners: &[u32]) {
    match *corners {
        [a, b, c, d] => {
            let p = |i: u32| out.positions[i as usize];
            if p(a).distance_squared(p(c)) <= p(b).distance_squared(p(d)) {
                out.indices.extend_from_slice(&[a, b, c, a, c, d]);
            } else {
                out.indices.extend_from_slice(&[b, c, d, b, d, a]);
            }
        }
        [a, b, c] => out.indices.extend_from_slice(&[a, b, c]),
        _ => {}
    }
}

/// The xy square of the chunk `key` of `level`, in metres.
pub fn square(level: u32, key: IVec3) -> (IVec2, IVec2) {
    let lo = key.truncate() * chunk_metres(level);
    (lo, lo + chunk_metres(level))
}

#[cfg(test)]
#[path = "nets_tests.rs"]
mod tests;
