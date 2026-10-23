//! The benchmark behind decision 0024: hecs against a small storage of
//! Solis's own (`own.rs`), on what a game at the scale target does each
//! tick (DESIGN.md §2): 2,000 units at peak, and ten times that to see the
//! headroom. Projectiles get a packed pool of their own (decision 0016),
//! so they're not entities here.
//!
//! Each line is the median of many runs, in microseconds.

mod own;

use glam::{Mat4, Quat, Vec3};
use std::collections::HashMap;
use std::hint::black_box;
use std::time::Instant;

#[derive(Clone, Copy)]
struct Transform {
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
}
#[derive(Clone, Copy)]
struct Velocity(Vec3);
#[derive(Clone, Copy)]
struct Health {
    hp: i32,
    max: i32,
}
#[derive(Clone, Copy)]
struct Team(u8);
#[derive(Clone, Copy)]
struct Parent(u32);
#[derive(Clone, Copy)]
struct World(Mat4);
/// A component made while the program runs (Lua): fields by name.
#[derive(Clone)]
struct Script(HashMap<&'static str, f64>);

fn transform(i: usize) -> Transform {
    Transform {
        position: Vec3::new(i as f32, (i * 7 % 13) as f32, 0.0),
        rotation: Quat::from_rotation_z(i as f32 * 0.01),
        scale: Vec3::ONE,
    }
}

fn script() -> Script {
    Script(HashMap::from([("ammo", 30.0), ("cooldown", 0.5), ("damage", 12.0)]))
}

/// The median of `runs` timings of `f`, in microseconds.
fn time(runs: usize, mut f: impl FnMut()) -> f64 {
    let mut times: Vec<f64> = (0..runs)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed().as_secs_f64() * 1e6
        })
        .collect();
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

fn hecs_world(n: usize) -> (hecs::World, Vec<hecs::Entity>) {
    let mut world = hecs::World::new();
    let mut ids = Vec::with_capacity(n);
    for i in 0..n {
        let parent = Parent(if i % 10 == 0 { u32::MAX } else { (i / 10 * 10) as u32 });
        ids.push(world.spawn((
            transform(i),
            Velocity(Vec3::X),
            Health { hp: 100, max: 100 },
            Team((i % 4) as u8),
            parent,
            World(Mat4::IDENTITY),
            script(),
        )));
    }
    (world, ids)
}

fn own_world(n: usize) -> (own::World, Vec<own::Entity>) {
    let mut world = own::World::default();
    let mut ids = Vec::with_capacity(n);
    for i in 0..n {
        let e = world.spawn();
        let parent = Parent(if i % 10 == 0 { u32::MAX } else { (i / 10 * 10) as u32 });
        world.insert(e, transform(i));
        world.insert(e, Velocity(Vec3::X));
        world.insert(e, Health { hp: 100, max: 100 });
        world.insert(e, Team((i % 4) as u8));
        world.insert(e, parent);
        world.insert(e, World(Mat4::IDENTITY));
        world.insert(e, script());
        ids.push(e);
    }
    (world, ids)
}

fn bench(n: usize) {
    const RUNS: usize = 301;
    const DT: f32 = 1.0 / 30.0;
    println!("\n{n} entities, 7 components each (µs, median of {RUNS})");
    println!("{:<44}{:>10}{:>10}", "", "hecs", "own");

    let row = |name: &str, h: f64, o: f64| println!("{name:<44}{h:>10.1}{o:>10.1}");

    row(
        "Build the world (Play pressed)",
        time(RUNS, || {
            black_box(hecs_world(n));
        }),
        time(RUNS, || {
            black_box(own_world(n));
        }),
    );

    let (mut hw, hids) = hecs_world(n);
    let (mut ow, oids) = own_world(n);

    row(
        "Move: position += velocity",
        time(RUNS, || {
            for (t, v) in hw.query_mut::<(&mut Transform, &Velocity)>() {
                t.position += v.0 * DT;
            }
        }),
        time(RUNS, || {
            if let Some((ts, vs)) = ow.two::<Transform, Velocity>() {
                for (e, t) in ts.iter_mut() {
                    if let Some(v) = vs.get(e) {
                        t.position += v.0 * DT;
                    }
                }
            }
        }),
    );

    row(
        "Read three: health per team",
        time(RUNS, || {
            let mut sum = [0i64; 4];
            for (h, team, _) in hw.query::<(&Health, &Team, &Transform)>().iter() {
                sum[team.0 as usize] += i64::from(h.hp);
            }
            black_box(sum);
        }),
        time(RUNS, || {
            let mut sum = [0i64; 4];
            let (Some(hs), Some(teams), Some(ts)) = (
                ow.column::<Health>(),
                ow.column::<Team>(),
                ow.column::<Transform>(),
            ) else {
                return;
            };
            for (e, h) in hs.iter() {
                if let (Some(team), Some(_)) = (teams.get(e), ts.get(e)) {
                    sum[team.0 as usize] += i64::from(h.hp);
                }
            }
            black_box(sum);
        }),
    );

    row(
        "Ask another entity: get(Health) × n",
        time(RUNS, || {
            let mut sum = 0i64;
            for (i, &e) in hids.iter().enumerate() {
                let target = hids[(i * 31) % hids.len()];
                let _ = e;
                if let Ok(h) = hw.get::<&Health>(target) {
                    sum += i64::from(h.hp + h.max);
                }
            }
            black_box(sum);
        }),
        time(RUNS, || {
            let mut sum = 0i64;
            for (i, _) in oids.iter().enumerate() {
                let target = oids[(i * 31) % oids.len()];
                if let Some(h) = ow.get::<Health>(target) {
                    sum += i64::from(h.hp + h.max);
                }
            }
            black_box(sum);
        }),
    );

    row(
        "World matrices from parents",
        time(RUNS, || {
            // Parents come first in spawn order, so one pass is enough.
            let locals: Vec<(Mat4, Parent)> = hw
                .query::<(&Transform, &Parent)>()
                .iter()
                .map(|(t, p)| {
                    (
                        Mat4::from_scale_rotation_translation(t.scale, t.rotation, t.position),
                        *p,
                    )
                })
                .collect();
            let mut worlds: Vec<Mat4> = Vec::with_capacity(locals.len());
            for (local, p) in &locals {
                let parent = (p.0 != u32::MAX).then(|| worlds[p.0 as usize]);
                worlds.push(parent.map_or(*local, |m| m * *local));
            }
            for ((w,), m) in hw.query_mut::<(&mut World,)>().into_iter().zip(&worlds) {
                w.0 = *m;
            }
        }),
        time(RUNS, || {
            let (Some(ts), Some(ps)) = (ow.column::<Transform>(), ow.column::<Parent>()) else {
                return;
            };
            let mut worlds: Vec<Mat4> = Vec::with_capacity(ts.len());
            for (e, t) in ts.iter() {
                let local = Mat4::from_scale_rotation_translation(t.scale, t.rotation, t.position);
                let parent = ps
                    .get(e)
                    .filter(|p| p.0 != u32::MAX)
                    .map(|p| worlds[p.0 as usize]);
                worlds.push(parent.map_or(local, |m| m * local));
            }
            if let Some(ws) = ow.column_mut::<World>() {
                for ((_, w), m) in ws.iter_mut().zip(&worlds) {
                    w.0 = *m;
                }
            }
        }),
    );

    row(
        "Run-time (Lua) component: cooldown -= dt",
        time(RUNS, || {
            for (s,) in hw.query_mut::<(&mut Script,)>() {
                if let Some(c) = s.0.get_mut("cooldown") {
                    *c -= f64::from(DT);
                }
            }
        }),
        time(RUNS, || {
            if let Some(ss) = ow.column_mut::<Script>() {
                for (_, s) in ss.iter_mut() {
                    if let Some(c) = s.0.get_mut("cooldown") {
                        *c -= f64::from(DT);
                    }
                }
            }
        }),
    );

    // A tenth of the units die and as many are made, every tick.
    let churn = n / 10;
    let mut hids = hids;
    let mut oids = oids;
    let mut turn = 0usize;
    row(
        "Churn: a tenth despawned and respawned",
        time(RUNS, || {
            for k in 0..churn {
                let i = (turn * churn + k * 7) % hids.len();
                let _ = hw.despawn(hids[i]);
                hids[i] = hw.spawn((
                    transform(i),
                    Velocity(Vec3::X),
                    Health { hp: 100, max: 100 },
                    Team(0),
                    Parent(u32::MAX),
                    World(Mat4::IDENTITY),
                    script(),
                ));
            }
            turn += 1;
        }),
        time(RUNS, || {
            for k in 0..churn {
                let i = (turn * churn + k * 7) % oids.len();
                ow.despawn(oids[i]);
                let e = ow.spawn();
                ow.insert(e, transform(i));
                ow.insert(e, Velocity(Vec3::X));
                ow.insert(e, Health { hp: 100, max: 100 });
                ow.insert(e, Team(0));
                ow.insert(e, Parent(u32::MAX));
                ow.insert(e, World(Mat4::IDENTITY));
                ow.insert(e, script());
                oids[i] = e;
            }
            turn += 1;
        }),
    );
}

fn main() {
    bench(2_000);
    bench(20_000);
}
