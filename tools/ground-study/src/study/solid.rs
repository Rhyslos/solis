//! Collision: each chunk's collider made, and the scale target's 2,000
//! characters walking over the whole ground and into its hills.

use super::scale::mesh_all;
use super::stats::{Spread, Table, count, ms, timed};
use glam::{IVec3, Quat, Vec2, Vec3};
use ground_study::collision::{add_chunk, entity, solid};
use ground_study::ground::Ground;
use ground_study::mesh::{ChunkMesh, Method};
use ground_study::store::Full;
use physics::{CharacterSpec, GRAVITY, Physics, SolidMesh};

/// Colliders made for every surface chunk, one way.
pub(crate) fn colliders(full: &Ground<Full>) {
    let table = Table::start(
        "Colliders: every surface chunk of the target ground made solid",
        &[
            "Way",
            "Collider, mean (ms)",
            "95th (ms)",
            "Worst (ms)",
            "Triangles a collider, mean",
        ],
    );
    for method in Method::ALL {
        let (meshes, _) = mesh_all(full, method);
        let mut physics = Physics::new();
        let mut times = Vec::new();
        let mut triangles = 0;
        for (n, (_, mesh)) in meshes.iter().enumerate() {
            let (time, made) = timed(|| {
                let solid = solid(mesh);
                triangles += solid.triangles.len();
                add_chunk(&mut physics, n as u64 + 1, solid)
            });
            if made {
                times.push(time);
            }
        }
        let spread = Spread::of(&times);
        table.row(&[
            method.name().into(),
            ms(spread.mean),
            ms(spread.p95),
            ms(spread.worst),
            count(triangles / meshes.len().max(1)),
        ]);
    }
}

const WALKERS: usize = 2_000;
const TICKS: usize = 120;

/// Where each walker starts and which way it walks.
fn walkers(full: &Ground<Full>) -> Vec<(Vec3, Vec3)> {
    let side = (WALKERS as f32).sqrt().ceil() as usize;
    let size = full.size();
    let step = (size.x - 20) as f32 / side as f32;
    (0..WALKERS)
        .map(|i| {
            let flat = Vec2::new(
                10.0 + (i % side) as f32 * step,
                10.0 + (i / side) as f32 * step,
            );
            let angle = i as f32 * 2.399;
            (
                flat.extend(top(full, flat) + 0.3),
                Vec3::new(angle.cos(), angle.sin(), 0.0) * 4.0,
            )
        })
        .collect()
}

/// The top of the ground at `at`.
fn top(full: &Ground<Full>, at: Vec2) -> f32 {
    let (x, y) = (at.x.round() as i32, at.y.round() as i32);
    (0..full.size().z)
        .rev()
        .find(|z| full.get(IVec3::new(x, y, *z)).is_solid())
        .map_or(0.0, |z| z as f32 + 0.5)
}

/// What the walkers did.
struct Walked {
    tick: Spread,
    fell_through: usize,
    stopped: usize,
    dropped: usize,
    standing: usize,
}

fn walk_all(
    physics: &mut Physics,
    starts: &[(Vec3, Vec3)],
    ground: Option<&Ground<Full>>,
) -> Walked {
    let ids: Vec<_> = (0..starts.len())
        .filter_map(|i| entity(1_000_000 + i as u64))
        .collect();
    for (id, (start, _)) in ids.iter().zip(starts) {
        physics.add_character(*id, (*start, Quat::IDENTITY), CharacterSpec::default());
    }
    let dt = 1.0 / 60.0;
    let mut fall = vec![0.0f32; ids.len()];
    let mut last: Vec<Vec3> = starts.iter().map(|(s, _)| *s).collect();
    let mut grounded = vec![false; ids.len()];
    let mut dropped = vec![false; ids.len()];
    let mut ticks = Vec::with_capacity(TICKS);
    for _ in 0..TICKS {
        let moves: Vec<_> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, (starts[i].1 + Vec3::Z * fall[i]) * dt))
            .collect();
        let (time, _) = timed(|| {
            physics.move_characters(&moves, dt);
            physics.step(dt);
            let _ = physics.moved();
        });
        ticks.push(time);
        for (i, id) in ids.iter().enumerate() {
            let now = physics.state().character(*id).is_some_and(|c| c.grounded);
            let feet = physics.state().pose(*id).map_or(last[i], |(p, _)| p);
            if grounded[i] && last[i].z - feet.z > 0.5 {
                dropped[i] = true;
            }
            fall[i] = if now { 0.0 } else { fall[i] + GRAVITY * dt };
            grounded[i] = now;
            last[i] = feet;
        }
    }
    let wanted = 4.0 * dt * TICKS as f32;
    let mut walked = Walked {
        tick: Spread::of(&ticks),
        fell_through: 0,
        stopped: 0,
        dropped: dropped.iter().filter(|d| **d).count(),
        standing: grounded.iter().filter(|g| **g).count(),
    };
    for (i, (start, _)) in starts.iter().enumerate() {
        if inside(ground, last[i]) {
            walked.fell_through += 1;
        }
        if (last[i] - *start).truncate().length() < wanted * 0.5 {
            walked.stopped += 1;
        }
    }
    walked
}

/// Whether feet at `p` are buried in the ground or below it: fallen
/// through. (Two voxels above the feet must both be ground, so a walker
/// on a steep slope, or in a cave under the top, isn't counted.)
fn inside(ground: Option<&Ground<Full>>, p: Vec3) -> bool {
    if p.z < -1.0 {
        return true;
    }
    ground.is_some_and(|g| {
        [1.5, 2.5]
            .iter()
            .all(|up| g.get((p + Vec3::Z * *up).round().as_ivec3()).is_solid())
    })
}

/// The ground's surface under `p` (looking a little above it), read
/// from the voxels: where the distance crosses zero between a voxel in
/// the ground and the one above it.
fn surface_near(ground: &Ground<Full>, p: Vec3) -> Option<f32> {
    let (x, y) = (p.x.round() as i32, p.y.round() as i32);
    let from = p.z.round() as i32 + 2;
    let mut above = ground.get(IVec3::new(x, y, from + 1));
    for z in (from - 6..=from).rev() {
        let here = ground.get(IVec3::new(x, y, z));
        if here.is_solid() && !above.is_solid() {
            let (d0, d1) = (here.distance(), above.distance());
            return Some(z as f32 + d0 / (d0 - d1));
        }
        above = here;
    }
    None
}

/// 2,000 units kept on the ground by reading its voxels, as a strategy
/// game's units can be, instead of each being a physics character.
fn follow_all(ground: &Ground<Full>, starts: &[(Vec3, Vec3)]) -> Walked {
    let dt = 1.0 / 60.0;
    let mut at: Vec<Vec3> = starts.iter().map(|(s, _)| *s).collect();
    let mut stopped = vec![false; at.len()];
    let mut ticks = Vec::with_capacity(TICKS);
    for _ in 0..TICKS {
        let (time, _) = timed(|| {
            for (i, (_, way)) in starts.iter().enumerate() {
                let next = at[i] + *way * dt;
                match surface_near(ground, next) {
                    // A rise of more than half a metre in one step is a
                    // wall: the unit waits there.
                    Some(z) if z - at[i].z <= 0.5 => at[i] = next.truncate().extend(z),
                    Some(_) => stopped[i] = true,
                    None => at[i] = next.truncate().extend(at[i].z - 0.1),
                }
            }
        });
        ticks.push(time);
    }
    Walked {
        tick: Spread::of(&ticks),
        fell_through: at.iter().filter(|p| inside(Some(ground), **p)).count(),
        stopped: stopped.iter().filter(|s| **s).count(),
        dropped: 0,
        standing: at.len(),
    }
}

fn flat_floor(size: f32) -> SolidMesh {
    SolidMesh {
        points: vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(size, 0.0, 0.0),
            Vec3::new(size, size, 0.0),
            Vec3::new(0.0, size, 0.0),
        ],
        triangles: vec![[0, 1, 2], [0, 2, 3]],
    }
}

/// The 2,000 walkers on the ground made solid each way, and on a flat
/// floor for comparison.
pub(crate) fn walking(full: &Ground<Full>) {
    let table = Table::start(
        "2,000 characters walking 2 s over the whole target ground (every surface chunk solid)",
        &[
            "Ground",
            "A tick, median (ms)",
            "Worst (ms)",
            "Standing at the end",
            "Stopped by hills",
            "Dropped > 0.5 m in a tick",
            "Fell through",
        ],
    );
    let starts = walkers(full);
    let row = |name: &str, walked: Walked| {
        table.row(&[
            name.into(),
            ms(walked.tick.median),
            ms(walked.tick.worst),
            count(walked.standing),
            count(walked.stopped),
            count(walked.dropped),
            count(walked.fell_through),
        ]);
    };
    let mut physics = Physics::new();
    add_chunk(&mut physics, 1, flat_floor(full.size().x as f32));
    let flat_starts: Vec<_> = starts
        .iter()
        .map(|(s, w)| (s.truncate().extend(0.3), *w))
        .collect();
    row(
        "A flat floor (two triangles)",
        walk_all(&mut physics, &flat_starts, None),
    );
    for method in [
        Method::SurfaceNets,
        Method::DualContouring,
        Method::GreedyCubes,
    ] {
        let (meshes, _) = mesh_all(full, method);
        let physics = &mut solid_ground(&meshes);
        row(method.name(), walk_all(physics, &starts, Some(full)));
    }
    row(
        "No physics: each unit reads the voxels under it",
        follow_all(full, &starts),
    );
}

fn solid_ground(meshes: &[(IVec3, ChunkMesh)]) -> Physics {
    let mut physics = Physics::new();
    for (n, (_, mesh)) in meshes.iter().enumerate() {
        add_chunk(&mut physics, n as u64 + 1, solid(mesh));
    }
    physics
}
