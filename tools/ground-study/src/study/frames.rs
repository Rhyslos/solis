//! A chunk made on the frame against one made on a worker: what the
//! frame pays, how late the mesh arrives, and whether a busy worker
//! slows the frame (this machine has two cores; the app keeps one for
//! the window and gives the job system the other).

use super::stats::{Spread, Table, ms, timed};
use glam::{IVec3, UVec3};
use ground_study::ground::{Ground, Samples};
use ground_study::mesh::Method;
use ground_study::shapes::{busy, fill};
use ground_study::store::Full;
use ground_study::voxel::{Dense, Voxel};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Instant;

/// Chunks next to each other on the surface: the first `n` of them.
fn neighbours(full: &Ground<Full>, n: usize) -> Vec<IVec3> {
    let mixed = full.mixed_keys();
    let first = mixed.get(mixed.len() / 2).copied().unwrap_or_default();
    let mut near: Vec<IVec3> = mixed
        .iter()
        .copied()
        .filter(|k| (*k - first).abs().max_element() <= 1)
        .collect();
    near.sort_by_key(|k| (*k - first).abs().element_sum());
    near.truncate(n);
    near
}

/// The two ways compared for 1, 4 and 8 chunks.
pub(crate) fn worker_or_frame(full: &Arc<Ground<Full>>) {
    let table = Table::start(
        "A dab's chunks made on the frame or on the worker (median of 20, ms)",
        &[
            "Way",
            "Chunks",
            "On the frame: the frame pays",
            "On the worker: the frame pays",
            "On the worker: the mesh is back after",
        ],
    );
    let Ok(pool) = rayon::ThreadPoolBuilder::new().num_threads(1).build() else {
        println!("No worker could be started.");
        return;
    };
    for method in [
        Method::SurfaceNets,
        Method::DualContouring,
        Method::GreedyCubes,
    ] {
        for n in [1, 4, 8] {
            let keys = neighbours(full, n);
            let mut on_frame = Vec::new();
            let mut hand_off = Vec::new();
            let mut back = Vec::new();
            let mut scratch = Dense::filled(Voxel::AIR);
            for _ in 0..20 {
                let mut samples = Samples::default();
                on_frame.push(
                    timed(|| {
                        for key in &keys {
                            full.gather(*key, &mut scratch, &mut samples);
                            std::hint::black_box(method.mesh(&samples));
                        }
                    })
                    .0,
                );
                let (tx, rx) = mpsc::channel();
                let start = Instant::now();
                let (paid, _) = timed(|| {
                    for key in &keys {
                        let mut samples = Samples::default();
                        full.gather(*key, &mut scratch, &mut samples);
                        let tx = tx.clone();
                        pool.spawn(move || {
                            let _ = tx.send(method.mesh(&samples));
                        });
                    }
                });
                for _ in &keys {
                    let _ = rx.recv();
                }
                hand_off.push(paid);
                back.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            table.row(&[
                method.name().into(),
                keys.len().to_string(),
                ms(Spread::of(&on_frame).median),
                ms(Spread::of(&hand_off).median),
                ms(Spread::of(&back).median),
            ]);
        }
    }
}

/// Work standing in for the rest of a frame: a few milliseconds of
/// arithmetic.
fn frame_work() -> f64 {
    let mut x = 1.0f64;
    for i in 0..600_000 {
        x = (x * 1.000_001 + f64::from(i % 7)).sqrt();
    }
    x
}

/// The frame's own work, alone and while one or two workers mesh.
pub(crate) fn interference() {
    let table = Table::start(
        "The rest of the frame while chunks are made (median of 40, ms)",
        &["While", "Frame's own work"],
    );
    let ground: Ground<Full> = Ground::build(UVec3::ONE, |key, dense| fill(key, dense, busy));
    let ground = Arc::new(ground);
    let measure = || -> f64 {
        let times: Vec<f64> = (0..40)
            .map(|_| timed(|| std::hint::black_box(frame_work())).0)
            .collect();
        Spread::of(&times).median
    };
    table.row(&["Nothing else".into(), ms(measure())]);
    for workers in [1, 2] {
        let busy_now = Arc::new(AtomicBool::new(true));
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let busy_now = Arc::clone(&busy_now);
                let ground = Arc::clone(&ground);
                std::thread::spawn(move || {
                    let mut scratch = Dense::filled(Voxel::AIR);
                    let mut samples = Samples::default();
                    while busy_now.load(Ordering::Relaxed) {
                        ground.gather(IVec3::ZERO, &mut scratch, &mut samples);
                        std::hint::black_box(Method::DualContouring.mesh(&samples));
                    }
                })
            })
            .collect();
        let while_busy = measure();
        busy_now.store(false, Ordering::Relaxed);
        for handle in handles {
            let _ = handle.join();
        }
        let label = if workers == 1 {
            "One worker meshing without pause (the app's job system on this machine)"
        } else {
            "Two workers meshing (both cores taken)"
        };
        table.row(&[label.into(), ms(while_busy)]);
    }
}
