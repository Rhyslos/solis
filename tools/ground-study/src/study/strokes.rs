//! Brush strokes on the target ground: what each frame of a stroke costs
//! from the mouse to a mesh ready to upload and a collider ready to use.

use super::scale::kept_as;
use super::stats::{Spread, Table, ms, timed};
use glam::{IVec3, Vec2};
use ground_study::brush::{Action, Brush};
use ground_study::collision::{add_chunk, solid};
use ground_study::ground::{Ground, Samples};
use ground_study::mesh::{ChunkMesh, Method};
use ground_study::shapes::SAND;
use ground_study::store::{Full, Octree, Sparse, Store};
use ground_study::voxel::{Dense, SIZE_I, Voxel};
use physics::Physics;
use rayon::prelude::*;

/// A kind of stroke.
#[derive(Clone, Copy)]
pub(crate) struct Kind {
    pub(crate) name: &'static str,
    pub(crate) radius: f32,
    pub(crate) action: Action,
}

/// The strokes measured: the mock-up's brush (3 m), a big one, and
/// smoothing.
pub(crate) const KINDS: [Kind; 3] = [
    Kind {
        name: "Add, 3 m brush",
        radius: 1.5,
        action: Action::Add(SAND),
    },
    Kind {
        name: "Dig, 12 m brush",
        radius: 6.0,
        action: Action::Dig,
    },
    Kind {
        name: "Smooth, 6 m brush",
        radius: 3.0,
        action: Action::Smooth,
    },
];

/// Dabs in a stroke: a second of dragging at 60 frames a second.
const DABS: usize = 60;
/// The stroke's way across the ground, over the corner where four
/// chunk columns meet.
const FROM: Vec2 = Vec2::new(500.0, 500.0);
const TO: Vec2 = Vec2::new(524.0, 520.0);

/// What one frame of a stroke cost.
#[derive(Clone, Copy, Default)]
struct Dab {
    edit: f64,
    chunks: usize,
    remesh_one_core: f64,
    remesh_all_cores: f64,
    collider: f64,
}

/// The height of the ground's top at `at`, as a ray from the mouse would
/// find it.
fn surface<S: Store>(ground: &Ground<S>, at: Vec2) -> f32 {
    let (x, y) = (at.x.round() as i32, at.y.round() as i32);
    (0..ground.size().z)
        .rev()
        .find(|z| ground.get(IVec3::new(x, y, *z)).is_solid())
        .map_or(0.0, |z| z as f32 + 0.5)
}

fn index_of(key: IVec3) -> u64 {
    (key.x + 64 * (key.y + 64 * key.z)) as u64 + 1
}

/// Physics with every surface chunk near the stroke made solid.
fn solid_near<S: Store>(ground: &Ground<S>, method: Method) -> Physics {
    let mut physics = Physics::new();
    let (mut scratch, mut samples) = (Dense::filled(Voxel::AIR), Samples::default());
    let middle = ((FROM + TO) * 0.5 / SIZE_I as f32).as_ivec2();
    for key in ground.mixed_keys() {
        if (key.truncate() - middle).abs().max_element() <= 3 {
            ground.gather(key, &mut scratch, &mut samples);
            add_chunk(&mut physics, index_of(key), solid(&method.mesh(&samples)));
        }
    }
    physics
}

fn stroke<S: Store>(
    ground: &mut Ground<S>,
    kind: Kind,
    method: Method,
    physics: &mut Physics,
) -> Vec<Dab> {
    let mut out = Vec::with_capacity(DABS);
    let (mut scratch, mut samples) = (Dense::filled(Voxel::AIR), Samples::default());
    for i in 0..DABS {
        let at = FROM.lerp(TO, i as f32 / (DABS - 1) as f32);
        let brush = Brush {
            center: at.extend(surface(ground, at)),
            radius: kind.radius,
            strength: 0.4,
            action: kind.action,
        };
        let (edit, keys) = timed(|| ground.apply(&brush));
        let (remesh_one_core, _) = timed(|| {
            keys.iter()
                .map(|key| {
                    ground.gather(*key, &mut scratch, &mut samples);
                    method.mesh(&samples)
                })
                .collect::<Vec<ChunkMesh>>()
        });
        let shared: &Ground<S> = ground;
        let (remesh_all_cores, meshes) = timed(|| {
            keys.par_iter()
                .map_init(
                    || (Dense::filled(Voxel::AIR), Samples::default()),
                    |(scratch, samples), key| {
                        shared.gather(*key, scratch, samples);
                        method.mesh(samples)
                    },
                )
                .collect::<Vec<ChunkMesh>>()
        });
        let (collider, _) = timed(|| {
            for (key, mesh) in keys.iter().zip(&meshes) {
                add_chunk(physics, index_of(*key), solid(mesh));
            }
        });
        out.push(Dab {
            edit,
            chunks: keys.len(),
            remesh_one_core,
            remesh_all_cores,
            collider,
        });
    }
    out
}

fn spread(dabs: &[Dab], pick: impl Fn(&Dab) -> f64) -> Spread {
    Spread::of(&dabs.iter().map(pick).collect::<Vec<_>>())
}

/// The stores compared on strokes (meshed as Smooth's surface nets).
pub(crate) fn by_store(full: &Ground<Full>) {
    let table = Table::start(
        "A stroke's frames by store: changing the voxels (60 dabs, strength 40 %)",
        &[
            "Store",
            "Stroke",
            "Chunks a dab, mean (worst)",
            "Edit, median (ms)",
            "Edit, worst (ms)",
        ],
    );
    store_rows::<Full>(&table, full);
    store_rows::<Sparse>(&table, full);
    store_rows::<Octree>(&table, full);
}

fn store_rows<S: Store>(table: &Table, full: &Ground<Full>) {
    for kind in KINDS {
        let mut ground: Ground<S> = kept_as(full);
        let mut physics = solid_near(&ground, Method::SurfaceNets);
        let dabs = stroke(&mut ground, kind, Method::SurfaceNets, &mut physics);
        let chunks = dabs.iter().map(|d| d.chunks).collect::<Vec<_>>();
        let mean = chunks.iter().sum::<usize>() as f64 / chunks.len() as f64;
        table.row(&[
            S::NAME.into(),
            kind.name.into(),
            format!("{mean:.1} ({})", chunks.iter().max().unwrap_or(&0)),
            ms(spread(&dabs, |d| d.edit).median),
            ms(spread(&dabs, |d| d.edit).worst),
        ]);
    }
}

/// The ways of meshing compared on strokes (fixed chunks).
pub(crate) fn by_method(full: &Ground<Full>) {
    let table = Table::start(
        "A stroke's frames by way of meshing (fixed chunks): median (worst), ms",
        &[
            "Way",
            "Stroke",
            "Edit",
            "Remesh, one core",
            "Remesh, both cores",
            "Colliders",
            "Frame's work (edit, remesh on both, colliders)",
        ],
    );
    for method in Method::ALL {
        for kind in KINDS {
            let mut ground: Ground<Full> = kept_as(full);
            let mut physics = solid_near(&ground, method);
            let dabs = stroke(&mut ground, kind, method, &mut physics);
            let both = |pick: &dyn Fn(&Dab) -> f64| {
                let s = spread(&dabs, pick);
                format!("{} ({})", ms(s.median), ms(s.worst))
            };
            table.row(&[
                method.name().into(),
                kind.name.into(),
                both(&|d| d.edit),
                both(&|d| d.remesh_one_core),
                both(&|d| d.remesh_all_cores),
                both(&|d| d.collider),
                both(&|d| d.edit + d.remesh_all_cores + d.collider),
            ]);
        }
    }
}
