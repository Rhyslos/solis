//! Physics only where it's needed: colliders built off the frame, a
//! character walking off the edge of the solid area, and 2,000 units
//! spread over 20 km against all together.

use super::super::stats::{Spread, Table, count, mb, ms, timed};
use glam::{IVec2, IVec3, Quat, UVec3, Vec2, Vec3};
use ground_study::collision::{entity, solid};
use ground_study::ground::{Ground, Samples};
use ground_study::mesh::{ChunkMesh, Method};
use ground_study::store::Full;
use ground_study::stream::region::{pack, unpack};
use ground_study::voxel::{Dense, SIZE_I, Voxel};
use ground_study::world;
use physics::{CharacterSpec, ColliderSpec, GRAVITY, Physics, Shape};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub(crate) fn colliders_and_units(corner: IVec2) {
    let offset = (corner / SIZE_I).extend(0);
    let ground: Ground<Full> = Ground::build(UVec3::new(8, 8, 4), |key, dense| {
        world::fill(0, key + offset, dense);
    });
    let mut samples = Samples::default();
    let mut scratch = Dense::filled(Voxel::AIR);
    let meshes: HashMap<IVec3, ChunkMesh> = ground
        .mixed_keys()
        .into_iter()
        .map(|key| {
            ground.gather(key, &mut scratch, &mut samples);
            (key, Method::SurfaceNets.mesh(&samples))
        })
        .collect();
    let per_collider = colliders(&meshes);
    walking_off_the_edge(&ground, &meshes);
    spread_or_together(&ground, per_collider);
}

/// A collider's two halves: welding the mesh (any thread) and handing it
/// to physics (on the game's thread today).
fn colliders(meshes: &HashMap<IVec3, ChunkMesh>) -> f64 {
    let mut physics = Physics::new();
    let mut weld = Vec::new();
    let mut add = Vec::new();
    let mut keys: Vec<&IVec3> = meshes.keys().collect();
    keys.sort_by_key(|k| (k.z, k.y, k.x));
    for (n, key) in keys.into_iter().enumerate() {
        let (w, shape) = timed(|| solid(&meshes[key]));
        let Some(id) = entity(n as u64 + 1) else {
            continue;
        };
        let (a, _) = timed(|| {
            physics.add(
                id,
                (Vec3::ZERO, Quat::IDENTITY),
                Vec3::ONE,
                None,
                Some(ColliderSpec {
                    shape: Shape::Mesh(Arc::new(shape)),
                    ..ColliderSpec::default()
                }),
            )
        });
        weld.push(w);
        add.push(a);
    }
    let table = Table::start(
        "A chunk's collider, in two halves (level 0, Smooth, every surface chunk of a 256 m square)",
        &["Half", "Where it can run", "Mean (ms)", "95th (ms)"],
    );
    let (w, a) = (Spread::of(&weld), Spread::of(&add));
    table.row(&[
        "Welding the mesh into the collider's points and triangles".into(),
        "a worker, today".into(),
        ms(w.mean),
        ms(w.p95),
    ]);
    table.row(&[
        "Building the collider's shape and adding it".into(),
        "the game's thread, today (a worker, if the physics crate builds the shape first)".into(),
        ms(a.mean),
        ms(a.p95),
    ]);
    w.mean + a.mean
}

/// The level-0 column under `p` (metres).
fn column(p: Vec2) -> IVec2 {
    (p / SIZE_I as f32).floor().as_ivec2()
}

/// A character running east at 20 m/s (a vehicle) across a 256 m square
/// while the columns round it are made solid a while after they're
/// wanted: with nothing done, and held at the edge of the solid ground
/// until it's ready.
fn walking_off_the_edge(ground: &Ground<Full>, meshes: &HashMap<IVec3, ChunkMesh>) {
    let table = Table::start(
        "A vehicle (a character at 20 m/s) while the columns round it become solid late",
        &[
            "Made solid after",
            "When it reaches ground not yet solid",
            "Ticks held",
            "Fell more than 2 m",
            "Got",
        ],
    );
    for delay in [30usize, 90, 240] {
        for hold in [false, true] {
            let r = run_across(ground, meshes, delay, hold);
            table.row(&[
                format!("{delay} ticks ({:.0} ms)", delay as f64 * 1000.0 / 60.0),
                if hold {
                    "held at the edge".into()
                } else {
                    "nothing done".to_string()
                },
                r.0.to_string(),
                if r.1 { "yes".into() } else { "no".to_string() },
                format!("{:.0} m", r.2),
            ]);
        }
    }
}

/// Returns ticks held, whether it fell, and how far east it got.
fn run_across(
    ground: &Ground<Full>,
    meshes: &HashMap<IVec3, ChunkMesh>,
    delay: usize,
    hold: bool,
) -> (usize, bool, f32) {
    let mut physics = Physics::new();
    let Some(walker) = entity(9_000_000) else {
        return (0, false, 0.0);
    };
    let height = |p: Vec2| {
        let (x, y) = (p.x.round() as i32, p.y.round() as i32);
        (0..ground.size().z)
            .rev()
            .find(|z| ground.get(IVec3::new(x, y, *z)).is_solid())
            .map_or(0.0, |z| z as f32 + 0.5)
    };
    let start = Vec2::new(10.0, 128.0);
    let mut solid_columns: HashSet<IVec2> = HashSet::new();
    let mut asked: HashMap<IVec2, usize> = HashMap::new();
    let mut next_id = 1;
    let dt = 1.0 / 60.0;
    let make_solid = |physics: &mut Physics, c: IVec2, next_id: &mut u64| {
        for z in 0..4 {
            if let Some(mesh) = meshes.get(&c.extend(z))
                && let Some(id) = entity(*next_id)
            {
                *next_id += 1;
                physics.add(
                    id,
                    (Vec3::ZERO, Quat::IDENTITY),
                    Vec3::ONE,
                    None,
                    Some(ColliderSpec {
                        shape: Shape::Mesh(Arc::new(solid(mesh))),
                        ..ColliderSpec::default()
                    }),
                );
            }
        }
    };
    // The start's own columns are solid before it's placed.
    for dy in -1..=1 {
        for dx in -1..=1 {
            let c = column(start) + IVec2::new(dx, dy);
            make_solid(&mut physics, c, &mut next_id);
            solid_columns.insert(c);
        }
    }
    let feet = start.extend(height(start) + 0.3);
    physics.add_character(walker, (feet, Quat::IDENTITY), CharacterSpec::default());
    let (mut fall, mut held, mut fell) = (0.0f32, 0, false);
    let mut at = feet;
    for tick in 0..60 * 30 {
        // Columns round the character are wanted; each becomes solid
        // `delay` ticks after it was first wanted.
        let here = column(at.truncate());
        for dy in -1..=1 {
            for dx in -1..=1 {
                let c = here + IVec2::new(dx, dy);
                if c.cmpge(IVec2::ZERO).all()
                    && c.cmplt(IVec2::splat(8)).all()
                    && !solid_columns.contains(&c)
                {
                    asked.entry(c).or_insert(tick);
                }
            }
        }
        let ready: Vec<IVec2> = asked
            .iter()
            .filter(|(_, t)| tick >= **t + delay)
            .map(|(c, _)| *c)
            .collect();
        for c in ready {
            asked.remove(&c);
            make_solid(&mut physics, c, &mut next_id);
            solid_columns.insert(c);
        }
        let grounded = physics
            .state()
            .character(walker)
            .is_some_and(|c| c.grounded);
        fall = if grounded { 0.0 } else { fall + GRAVITY * dt };
        let mut way = Vec3::X * 20.0;
        let ahead = column(at.truncate() + Vec2::X * 1.0);
        if hold && !solid_columns.contains(&ahead) {
            way = Vec3::ZERO;
            held += 1;
        }
        physics.move_character(walker, (way + Vec3::Z * fall) * dt, dt);
        physics.step(dt);
        let _ = physics.moved();
        at = physics.state().pose(walker).map_or(at, |(p, _)| p);
        if at.z < height(at.truncate()) - 2.0 {
            fell = true;
            break;
        }
        if at.x > 240.0 {
            break;
        }
    }
    (held, fell, at.x - start.x)
}

/// A small, steady random stream.
fn next(state: &mut u64) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 40) as f32 / (1u64 << 24) as f32
}

/// 2,000 units over 20 km against all in a square kilometre: what must
/// be kept and made for them, from measured costs.
fn spread_or_together(ground: &Ground<Full>, per_collider: f64) {
    // Measured: what a level-0 column holds, and reading one chunk.
    let columns = 8 * 8;
    let mixed = ground.mixed_keys();
    let mixed_per_column = mixed.len() as f64 / f64::from(columns);
    let column_bytes = ground.bytes() as f64 / f64::from(columns);
    let packed: Vec<_> = mixed
        .iter()
        .filter_map(|k| ground.chunk(*k))
        .map(pack)
        .collect();
    let (unpack_ms, _) = timed(|| {
        for p in &packed {
            let _ = std::hint::black_box(unpack(p));
        }
    });
    let read_ms = unpack_ms / packed.len().max(1) as f64;
    let table = Table::start(
        "2,000 units following the ground's voxels: spread over 20 km against all in one square kilometre",
        &[
            "Units",
            "Level-0 columns kept",
            "Their memory",
            "Columns new a second, walking at 4 m/s",
            "Reading them (worker ms a second)",
            "If all were physics characters: colliders",
            "Building them once (s)",
            "Colliders new a second",
        ],
    );
    for (name, side, reach) in [
        (
            "Spread over 20 × 20 km, each keeping the 3 × 3 columns round it",
            world::SIDE as f32,
            1,
        ),
        (
            "Spread over 20 × 20 km, each keeping the column it stands on",
            world::SIDE as f32,
            0,
        ),
        (
            "All in 1 × 1 km, each keeping the 3 × 3 columns round it",
            1024.0,
            1,
        ),
    ] {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let base = Vec2::splat((world::SIDE as f32 - side) / 2.0);
        let units: Vec<(Vec2, Vec2)> = (0..2000)
            .map(|_| {
                let p = base + Vec2::new(next(&mut state), next(&mut state)) * side;
                let a = next(&mut state) * std::f32::consts::TAU;
                (p, Vec2::new(a.cos(), a.sin()) * 4.0)
            })
            .collect();
        let around = |p: Vec2| -> Vec<IVec2> {
            let c = column(p);
            (-reach..=reach)
                .flat_map(|dy| (-reach..=reach).map(move |dx| c + IVec2::new(dx, dy)))
                .collect()
        };
        let kept: HashSet<IVec2> = units.iter().flat_map(|(p, _)| around(*p)).collect();
        // Ten seconds of walking: the columns that come in.
        let mut now = kept.clone();
        let mut new = 0;
        for second in 1..=10 {
            for (p, v) in &units {
                for c in around(*p + *v * second as f32) {
                    if now.insert(c) {
                        new += 1;
                    }
                }
            }
        }
        let new_a_second = new as f64 / 10.0;
        let colliders = kept.len() as f64 * mixed_per_column;
        table.row(&[
            name.into(),
            count(kept.len()),
            mb(kept.len() as f64 * column_bytes),
            format!("{new_a_second:.0}"),
            format!("{:.0}", new_a_second * mixed_per_column * read_ms),
            count(colliders as usize),
            format!("{:.1}", colliders * per_collider / 1000.0),
            format!("{:.0}", new_a_second * mixed_per_column),
        ]);
    }
    println!(
        "\n(Measured on the 256 m square: {mixed_per_column:.2} surface chunks a level-0 column, {} a column kept, {} ms to read a chunk from its packed bytes, {} ms a collider.)",
        mb(column_bytes),
        ms(read_ms),
        ms(per_collider)
    );
}
