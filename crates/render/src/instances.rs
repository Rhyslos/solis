//! Instances: where each copy of a mesh is drawn. Every frame the
//! placements of every model are gathered into one buffer, grouped so each
//! mesh is drawn once for all its copies (decision 0021). Mirrored copies
//! (a negative scale) are grouped apart: their triangles wind the other
//! way, so they're drawn with pipelines that treat the other side as the
//! front.

use crate::gpu_model::{Drawn, GpuModel};
use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3, Vec4};
use std::ops::Range;

/// One instance exactly as the shader's `Instance` inputs expect it: 52
/// bytes. The shader works out the normals' matrix from the placement
/// itself, which is quicker than making and sending it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct InstanceRaw {
    /// The placement's first three rows (the last is always 0, 0, 0, 1).
    rows: [[f32; 4]; 3],
    /// The placement's number for picking (0: not pickable).
    pick: u32,
}

impl InstanceRaw {
    pub(crate) fn new(model: Mat4, pick: u32) -> Self {
        Self {
            rows: [
                model.row(0).to_array(),
                model.row(1).to_array(),
                model.row(2).to_array(),
            ],
            pick,
        }
    }
}

/// How an instance is laid out in the instance buffer (vertex buffer 1).
const ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
    3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 10 => Uint32,
];

pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRIBUTES,
    }
}

/// One mesh of one model, drawn for a run of instances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Batch {
    /// Which placed model (index into the frame's placements' models).
    pub(crate) model: usize,
    /// Which of its meshes.
    pub(crate) mesh: usize,
    /// Which level of detail (0 for the full mesh).
    pub(crate) lod: usize,
    /// Whether these copies are mirrored.
    pub(crate) mirrored: bool,
    /// Whether these copies are in the camera's view (drawn).
    pub(crate) seen: bool,
    /// Whether these copies can cast a shadow into the view (drawn into
    /// the sun's shadow map).
    pub(crate) casts: bool,
    /// Their instances in the buffer.
    pub(crate) instances: Range<u32>,
}

/// A model placed in the world: one copy of it, all its meshes, or one
/// part of it (a node: its own meshes only, at the transform given, for a
/// model unpacked into entities).
#[derive(Debug, Clone, Copy)]
pub struct Placement<'a> {
    /// The model.
    pub model: &'a GpuModel,
    /// One node's meshes only, or `None` for the whole model.
    pub node: Option<usize>,
    /// Where it is.
    pub transform: Mat4,
    /// Its number in the picking image ([`crate::PickResult`]), or 0 when
    /// it can't be picked.
    pub pick: u32,
}

/// What can be seen: the camera's view, and where shadows are drawn.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cull {
    /// The view's sides as planes (`n · p + d ≥ 0` inside), each with its
    /// normal of length 1.
    planes: Vec<Vec4>,
    /// Where the sun's shadows are drawn, and the way its light goes, when
    /// they are: copies whose shadow can fall in that box cast.
    shadows: Option<(Vec3, Vec3, Vec3)>,
    /// How big things look, for choosing levels of detail: the view's
    /// last row (a point's distance) and how many pixels a metre is at
    /// distance 1; `None` to draw everything in full.
    detail: Option<(Vec4, f32)>,
}

impl Cull {
    /// The view seen through `view_projection` (depth 0 to 1, either way
    /// round, the far side possibly endless), with the sun's shadows
    /// falling in the box `shadows` (lowest and highest corners) along
    /// `light`.
    pub(crate) fn new(view_projection: Mat4, shadows: Option<(Vec3, Vec3, Vec3)>) -> Self {
        let r = |i| view_projection.row(i);
        let planes = [
            r(3) + r(0),
            r(3) - r(0),
            r(3) + r(1),
            r(3) - r(1),
            r(2),
            r(3) - r(2),
        ]
        .into_iter()
        .filter_map(|p| {
            // A side at infinity (an endless far side) has no normal:
            // nothing is beyond it.
            let length = p.truncate().length();
            (length > 1e-6 && p.is_finite()).then(|| p / length)
        })
        .collect();
        Self {
            planes,
            shadows,
            detail: None,
        }
    }

    /// The same view, choosing levels of detail for an image
    /// `height_px` pixels tall.
    pub(crate) fn with_detail(mut self, view_projection: Mat4, height_px: u32) -> Self {
        // A view's second row turns metres up the screen into its -1 to 1
        // across the image (divided by the distance, its last row).
        let per_metre = view_projection.row(1).truncate().length() * height_px as f32 * 0.5;
        if per_metre.is_finite() && per_metre > 0.0 {
            self.detail = Some((view_projection.row(3), per_metre));
        }
        self
    }

    /// The level of detail for a model whose ball is at `centre` with
    /// `radius`, whose levels stray by `errors` (a share of its size; the
    /// full model first), when it was at level `before` last frame: the
    /// simplest whose straying is under [`LOD_PIXELS`] on screen, keeping
    /// the one before while it's still nearly so, so a copy at the
    /// boundary doesn't flicker between two.
    pub(crate) fn level(&self, centre: Vec3, radius: f32, errors: &[f32], before: u8) -> u8 {
        let Some((distance_row, per_metre)) = self.detail else {
            return 0;
        };
        if errors.len() < 2 {
            return 0;
        }
        let distance = distance_row.dot(centre.extend(1.0));
        if distance.is_nan() || distance <= 1e-6 {
            // Behind the camera (only its shadow can show): the simplest.
            return (errors.len() - 1) as u8;
        }
        let size_px = 2.0 * radius * per_metre / distance;
        let fits = |level: usize, pixels: f32| errors[level] * size_px <= pixels;
        let before = usize::from(before);
        let keep = before < errors.len()
            && fits(before, LOD_PIXELS * LOD_BAND)
            && !(before + 1 < errors.len() && fits(before + 1, LOD_PIXELS / LOD_BAND));
        if keep {
            return before as u8;
        }
        (0..errors.len())
            .rev()
            .find(|&level| fits(level, LOD_PIXELS))
            .unwrap_or(0) as u8
    }

    /// Whether a ball at `centre` of `radius` is at least partly in view.
    pub(crate) fn sees(&self, centre: Vec3, radius: f32) -> bool {
        self.planes
            .iter()
            .all(|p| p.truncate().dot(centre) + p.w >= -radius)
    }

    /// Whether a ball at `centre` of `radius` can cast a shadow into the
    /// shadows' box: the ball, swept along the light down to below the
    /// box, touches it.
    pub(crate) fn casts(&self, centre: Vec3, radius: f32) -> bool {
        let Some((min, max, light)) = self.shadows else {
            return false;
        };
        let (min, max) = (min - Vec3::splat(radius), max + Vec3::splat(radius));
        // How far the light goes from the ball before it's below the box
        // (or across it, for a light that doesn't go down).
        let across = (max - min).length();
        let length = if light.z < -1e-3 {
            ((centre.z - min.z) / -light.z).clamp(0.0, 4.0 * across)
        } else {
            4.0 * across
        };
        segment_meets_box(centre, light * length, min, max)
    }
}

/// How many pixels a level of detail may stray on screen before a more
/// detailed one is drawn.
pub const LOD_PIXELS: f32 = 1.0;

/// How far past [`LOD_PIXELS`] a copy keeps its level from last frame (and
/// how far under it must be to go simpler), so it doesn't flicker.
const LOD_BAND: f32 = 1.25;

/// Whether the segment from `start` along `along` meets the box `min`–`max`
/// (the slab method).
fn segment_meets_box(start: Vec3, along: Vec3, min: Vec3, max: Vec3) -> bool {
    let (mut enter, mut leave) = (0.0_f32, 1.0_f32);
    for axis in 0..3 {
        let (s, d, lo, hi) = (start[axis], along[axis], min[axis], max[axis]);
        if d.abs() < 1e-9 {
            if s < lo || s > hi {
                return false;
            }
        } else {
            let (a, b) = ((lo - s) / d, (hi - s) / d);
            enter = enter.max(a.min(b));
            leave = leave.min(a.max(b));
            if enter > leave {
                return false;
            }
        }
    }
    true
}

/// One mesh copy on its way into the buffer: its batch's key (whether it's
/// seen and casts, the model, mesh and whether it's mirrored, packed so
/// it sorts in that order), the copy itself, and the ball around it.
struct Entry {
    key: u64,
    raw: InstanceRaw,
    centre: Vec3,
    radius: f32,
}

/// Where a copy's key keeps whether it's seen and casts: in view only,
/// both, or casting only, in that order.
const SEEN_ONLY: u64 = 0;
const SEEN_AND_CASTS: u64 = 1;
const CASTS_ONLY: u64 = 2;

/// What [`gather`] found.
#[derive(Debug, Default)]
pub(crate) struct Gathered {
    /// Every copy drawn, in batches.
    pub(crate) raw: Vec<InstanceRaw>,
    /// The batches, each one mesh drawn for a run of copies.
    pub(crate) batches: Vec<Batch>,
    /// For each batch model number, the placement that holds that model.
    pub(crate) models: Vec<usize>,
    /// Copies in view.
    pub(crate) seen: u32,
    /// Copies drawn into the shadow map.
    pub(crate) casting: u32,
    /// The box around the copies that cast, if any do.
    pub(crate) casters: Option<(Vec3, Vec3)>,
    /// Each placement's level of detail, to pass back next frame.
    pub(crate) levels: Vec<u8>,
}

/// Every mesh copy of `placements` that `cull` lets through (all of them,
/// seen and casting, without one), grouped into batches. Models are told
/// apart by address, so the same model placed twice shares its batches.
/// `before` is each placement's level of detail last frame (from the last
/// [`Gathered::levels`]; anything for a new one).
pub(crate) fn gather(placements: &[Placement<'_>], cull: Option<&Cull>, before: &[u8]) -> Gathered {
    use rayon::prelude::*;
    // Each distinct model once, in the order first seen, by the placement
    // that first showed it. Copies of one model usually come together, so
    // the last one is checked before asking the map.
    let mut models: Vec<usize> = Vec::new();
    let mut known: std::collections::HashMap<*const GpuModel, usize> = Default::default();
    let mut last: Option<(*const GpuModel, usize)> = None;
    let numbers: Vec<usize> = placements
        .iter()
        .enumerate()
        .map(|(i, placement)| {
            let model = std::ptr::from_ref(placement.model);
            if let Some((m, n)) = last
                && m == model
            {
                return n;
            }
            let n = *known.entry(model).or_insert_with(|| {
                models.push(i);
                models.len() - 1
            });
            last = Some((model, n));
            n
        })
        .collect();
    // Each placement's level of detail, from how big it looks.
    let levels: Vec<u8> = match cull.filter(|c| c.detail.is_some()) {
        None => vec![0; placements.len()],
        Some(cull) => placements
            .par_iter()
            .enumerate()
            .map(|(i, p)| {
                let errors = p.model.lod_errors();
                if errors.len() < 2 || !p.transform.is_finite() {
                    return 0;
                }
                let (centre, radius) = p.model.sphere();
                let turn = Mat3::from_mat4(p.transform);
                let scale = turn
                    .x_axis
                    .length()
                    .max(turn.y_axis.length())
                    .max(turn.z_axis.length());
                cull.level(
                    p.transform.transform_point3(centre),
                    radius * scale,
                    errors,
                    before.get(i).copied().unwrap_or(0),
                )
            })
            .collect(),
    };
    // Each placement's mesh copies, in jobs of a run of placements each,
    // kept in the placements' order.
    let copies_of = |placement: &Placement<'_>, number: usize, level: u8, out: &mut Vec<Entry>| {
        // A place that isn't numbers isn't drawn.
        let number = placement.transform.is_finite().then_some(number);
        let meshes = match (number, placement.node) {
            (None, _) => rayon::iter::Either::Left(rayon::iter::Either::Left(std::iter::empty())),
            (Some(_), None) => rayon::iter::Either::Left(rayon::iter::Either::Right(
                placement.model.placements().iter().copied(),
            )),
            (Some(_), Some(node)) => {
                rayon::iter::Either::Right(placement.model.node_meshes(node).iter().map(|&m| {
                    Drawn::new(
                        m,
                        Mat4::IDENTITY,
                        placement.model.sphere_of(m),
                        placement.model.lods_of(m),
                    )
                }))
            }
        };
        let model = number.unwrap_or(0) as u64;
        let place = placement.transform;
        let turn = Mat3::from_mat4(place);
        let scale = turn
            .x_axis
            .length()
            .max(turn.y_axis.length())
            .max(turn.z_axis.length());
        let flipped = turn.determinant() < 0.0;
        out.extend(meshes.filter_map(|drawn: Drawn| {
            let centre = place.transform_point3(drawn.centre);
            let radius = drawn.radius * scale;
            let class = match cull {
                None => SEEN_AND_CASTS,
                Some(cull) => match (cull.sees(centre, radius), cull.casts(centre, radius)) {
                    (true, false) => SEEN_ONLY,
                    (true, true) => SEEN_AND_CASTS,
                    (false, true) => CASTS_ONLY,
                    (false, false) => return None,
                },
            };
            let world = if drawn.at_origin {
                place
            } else {
                place * drawn.local
            };
            let mirrored = flipped != drawn.mirrored;
            let lod = u64::from(level.min(drawn.lods));
            Some(Entry {
                key: class << 62
                    | (model & 0x03ff_ffff) << 36
                    | (drawn.mesh as u64 & 0xffff_ffff) << 4
                    | lod << 1
                    | u64::from(mirrored),
                raw: InstanceRaw::new(world, placement.pick),
                centre,
                radius,
            })
        }));
    };
    const RUN: usize = 2048;
    let runs: Vec<Vec<Entry>> = placements
        .par_chunks(RUN)
        .zip(numbers.par_chunks(RUN))
        .zip(levels.par_chunks(RUN))
        .map(|((placements, numbers), levels)| {
            let mut out = Vec::with_capacity(placements.len() * 2);
            for ((p, &n), &level) in placements.iter().zip(numbers).zip(levels) {
                copies_of(p, n, level, &mut out);
            }
            out
        })
        .collect();
    // Grouped into batches by sorting keys with where each copy is, so
    // copies keep the placements' order within a batch, whatever the
    // number of cores.
    let mut order: Vec<(u64, u32, u32)> = runs
        .par_iter()
        .enumerate()
        .flat_map_iter(|(r, run)| {
            run.iter()
                .enumerate()
                .map(move |(i, e)| (e.key, r as u32, i as u32))
        })
        .collect();
    order.par_sort_unstable();
    let entry = |&(_, r, i): &(u64, u32, u32)| &runs[r as usize][i as usize];
    let raw: Vec<InstanceRaw> = order.par_iter().map(|o| entry(o).raw).collect();
    let mut batches: Vec<Batch> = Vec::new();
    let (mut seen, mut casting) = (0, 0);
    for (index, &(key, _, _)) in order.iter().enumerate() {
        let class = key >> 62;
        let (model, mesh, lod, mirrored) = (
            ((key >> 36) & 0x03ff_ffff) as usize,
            ((key >> 4) & 0xffff_ffff) as usize,
            ((key >> 1) & 0b111) as usize,
            key & 1 == 1,
        );
        let (is_seen, casts) = (class != CASTS_ONLY, class != SEEN_ONLY);
        seen += u32::from(is_seen);
        casting += u32::from(casts);
        let index = index as u32;
        match batches.last_mut() {
            Some(b)
                if b.model == model
                    && b.mesh == mesh
                    && b.lod == lod
                    && b.mirrored == mirrored
                    && b.seen == is_seen
                    && b.casts == casts =>
            {
                b.instances.end = index + 1;
            }
            _ => batches.push(Batch {
                model,
                mesh,
                lod,
                mirrored,
                seen: is_seen,
                casts,
                instances: index..index + 1,
            }),
        }
    }
    // The box around what casts, for the sun's view to take in.
    let casters = cull.filter(|c| c.shadows.is_some()).and_then(|_| {
        runs.par_iter()
            .flat_map_iter(|run| run.iter())
            .filter(|e| e.key >> 62 != SEEN_ONLY)
            .map(|e| {
                let r = Vec3::splat(e.radius);
                (e.centre - r, e.centre + r)
            })
            .reduce_with(|(a, b), (c, d)| (a.min(c), b.max(d)))
    });
    Gathered {
        raw,
        batches,
        models,
        seen,
        casting,
        casters,
        levels,
    }
}

/// The box around every placement, in the world, or `None` if there's
/// nothing.
pub(crate) fn world_bounds(placements: &[Placement<'_>]) -> Option<(Vec3, Vec3)> {
    use rayon::prelude::*;
    placements
        .par_iter()
        .filter(|p| p.transform.is_finite())
        .filter_map(|p| {
            let (min, max) = p.model.bounds();
            if !(min.is_finite() && max.is_finite()) {
                return None;
            }
            // The box's middle and half-size, turned: along each world
            // axis, the half-size is the sum of the turned half-sizes'
            // parts.
            let middle = p.transform.transform_point3((min + max) * 0.5);
            let half = (max - min) * 0.5;
            let turn = Mat3::from_mat4(p.transform);
            let reach = turn.x_axis.abs() * half.x
                + turn.y_axis.abs() * half.y
                + turn.z_axis.abs() * half.z;
            Some((middle - reach, middle + reach))
        })
        .reduce_with(|(a_min, a_max), (b_min, b_max)| (a_min.min(b_min), a_max.max(b_max)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instances_match_the_shader_layout() {
        // Three rows of 4 floats, then the pick number.
        assert_eq!(std::mem::size_of::<InstanceRaw>(), 13 * 4);
        let last = ATTRIBUTES.last().unwrap();
        assert_eq!(last.offset + 4, std::mem::size_of::<InstanceRaw>() as u64);
    }

    #[test]
    fn an_instance_keeps_the_placement_row_by_row() {
        let place = Mat4::from_scale_rotation_translation(
            Vec3::new(2.0, 1.0, 3.0),
            glam::Quat::from_rotation_z(0.5),
            Vec3::new(4.0, 5.0, 6.0),
        );
        let raw = InstanceRaw::new(place, 7);
        let back = Mat4::from_cols_array_2d(&[
            raw.rows[0],
            raw.rows[1],
            raw.rows[2],
            [0.0, 0.0, 0.0, 1.0],
        ])
        .transpose();
        assert!(back.abs_diff_eq(place, 1e-6));
        assert_eq!(raw.pick, 7);
    }

    /// A camera at the origin looking along +Y, 90° wide, depth reversed
    /// and endless (as the editor's is).
    fn camera() -> Mat4 {
        // Right is +X, up is +Z, forward is +Y (the camera looks down its
        // own -Z).
        let view = Mat4::from_cols(
            Vec4::new(1.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 0.0, -1.0, 0.0),
            Vec4::new(0.0, 1.0, 0.0, 0.0),
            Vec4::W,
        );
        let near = 0.1;
        let projection = Mat4::from_cols(
            Vec4::new(1.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 1.0, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 0.0, -1.0),
            Vec4::new(0.0, 0.0, near, 0.0),
        );
        projection * view
    }

    #[test]
    fn the_view_sees_what_is_in_front_and_not_what_is_behind() {
        let cull = Cull::new(camera(), None);
        assert!(cull.sees(Vec3::new(0.0, 10.0, 0.0), 1.0));
        assert!(
            cull.sees(Vec3::new(0.0, 100_000.0, 0.0), 1.0),
            "no far side"
        );
        assert!(!cull.sees(Vec3::new(0.0, -10.0, 0.0), 1.0));
        assert!(
            !cull.sees(Vec3::new(30.0, 10.0, 0.0), 1.0),
            "off to the side"
        );
        // Just outside the edge, but big enough to reach in.
        assert!(cull.sees(Vec3::new(12.0, 10.0, 0.0), 2.0));
        assert!(!cull.sees(Vec3::new(12.0, 10.0, 0.0), 1.0));
    }

    #[test]
    fn a_flat_view_sees_its_box() {
        // Looking straight down from 50 m, 20 m across, depth 0.1 m (1) to
        // 100 m (0).
        let view = Mat4::from_translation(Vec3::new(0.0, 0.0, -50.0));
        let projection = Mat4::from_cols(
            Vec4::new(0.1, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 0.1, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0 / 99.9, 0.0),
            Vec4::new(0.0, 0.0, 100.0 / 99.9, 1.0),
        );
        let flat = projection * view;
        let cull = Cull::new(flat, None);
        assert!(cull.sees(Vec3::ZERO, 0.5));
        assert!(!cull.sees(Vec3::new(20.0, 0.0, 0.0), 0.5));
    }

    #[test]
    fn things_cast_into_the_box_only_along_the_light() {
        // Shadows drawn in a box on the ground around the origin; light
        // slanting down along +X.
        let light = Vec3::new(1.0, 0.0, -1.0).normalize();
        let cull = Cull::new(
            camera(),
            Some((Vec3::new(-5.0, -5.0, 0.0), Vec3::new(5.0, 5.0, 2.0), light)),
        );
        // Up and back towards the sun: its shadow falls in.
        assert!(cull.casts(Vec3::new(-20.0, 0.0, 20.0), 1.0));
        // Up and the other way: the shadow falls further away.
        assert!(!cull.casts(Vec3::new(20.0, 0.0, 20.0), 1.0));
        // Off to the side.
        assert!(!cull.casts(Vec3::new(-20.0, 30.0, 20.0), 1.0));
        // Inside.
        assert!(cull.casts(Vec3::ZERO, 0.1));
        // Without shadows, nothing casts.
        assert!(!Cull::new(camera(), None).casts(Vec3::ZERO, 1.0));
    }

    #[test]
    fn a_damaged_view_hides_nothing_rather_than_breaking() {
        let cull = Cull::new(Mat4::from_cols_array(&[f32::NAN; 16]), None);
        assert!(cull.sees(Vec3::ZERO, 1.0));
        let flat = Cull::new(Mat4::ZERO, None);
        assert!(flat.sees(Vec3::ZERO, 1.0));
    }

    #[test]
    fn far_copies_take_simpler_levels_and_near_ones_full() {
        // 1080 pixels tall; a model 2 m across whose levels stray by 0.5%,
        // 2% and 8% of its size.
        let cull = Cull::new(camera(), None).with_detail(camera(), 1080);
        let errors = [0.0, 0.005, 0.02, 0.08];
        let level = |distance: f32| cull.level(Vec3::new(0.0, distance, 0.0), 1.0, &errors, 0);
        assert_eq!(level(2.0), 0, "close: in full");
        let mut last = 0;
        for d in [5.0, 20.0, 50.0, 200.0, 1000.0, 10_000.0] {
            let l = level(d);
            assert!(l >= last, "never more detailed further away");
            last = l;
        }
        assert_eq!(last, 3, "far: the simplest");
        // Each level is used only when its straying is under a pixel.
        for d in [5.0_f32, 20.0, 50.0, 200.0] {
            let l = usize::from(level(d));
            let size_px = 2.0 * 1080.0 * 0.5 / d;
            assert!(errors[l] * size_px <= LOD_PIXELS, "{d} m: level {l}");
        }
        // Behind the camera: the simplest (only its shadow shows).
        assert_eq!(cull.level(Vec3::new(0.0, -5.0, 0.0), 1.0, &errors, 0), 3);
        // Without levels, or without detail, always full.
        assert_eq!(cull.level(Vec3::new(0.0, 1e4, 0.0), 1.0, &[0.0], 0), 0);
        let plain = Cull::new(camera(), None);
        assert_eq!(plain.level(Vec3::new(0.0, 1e4, 0.0), 1.0, &errors, 0), 0);
    }

    #[test]
    fn a_copy_at_a_boundary_keeps_its_level_rather_than_flicker() {
        let cull = Cull::new(camera(), None).with_detail(camera(), 1000);
        let errors = [0.0, 0.01];
        // Level 1 strays exactly a pixel at 10 m (2 m across: 100 px).
        let at = |d: f32, before| cull.level(Vec3::new(0.0, d, 0.0), 1.0, &errors, before);
        // Near the boundary, each keeps what it had.
        assert_eq!(at(10.5, 0), 0, "just past it, from full: kept full");
        assert_eq!(at(10.5, 1), 1, "just past it, from simpler: kept");
        assert_eq!(at(9.5, 1), 1, "just inside, from simpler: kept");
        assert_eq!(at(9.5, 0), 0, "just inside, from full: kept");
        // Well past either way, it changes.
        assert_eq!(at(13.0, 0), 1, "well past: simpler");
        assert_eq!(at(7.0, 1), 0, "well inside: full again");
        // A level from last frame that no longer exists is let go.
        assert_eq!(at(5.0, 9), 0);
    }
}
