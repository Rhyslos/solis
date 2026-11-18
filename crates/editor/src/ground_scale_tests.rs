//! The ground's scale cases (Tools 1b, G1; decision 0116), run by
//! `tools/scale/scale-check.sh`: opening the 1 km² ground (the target)
//! and the 4 km² one (stress), with the editor's frames timed while its
//! chunks arrive from the workers: the slowest frame, and how long until
//! every chunk has arrived.

use super::*;
use crate::panel::Panel;
use crate::test_support::{harness, rest_over};
use std::time::Instant;

/// The editor's frame target, in milliseconds.
const FRAME_MS: f64 = 8.0;

fn open_ground(side: u32) {
    let asset = AssetId(reflect::Id::from_raw(4242).unwrap());
    let settings = Settings {
        size: [side; 2],
        ..Settings::default()
    };
    let made = Instant::now();
    let ground = Arc::new(
        ground::Start::Gentle {
            height: 32.0,
            seed: 1,
        }
        .ground(settings.extent()),
    );
    let made_ms = made.elapsed().as_secs_f64() * 1000.0;
    let mut w = crate::Workbench::new(crate::test_support::info());
    let entity = level::EntityData::new("Ground").with(&level::builtin::Ground {
        ground: Some(reflect::AssetRef { asset, part: None }),
        solid: true,
    });
    w.open
        .document
        .edit(
            "Ground",
            vec![level::Edit::Spawn {
                entities: vec![(reflect::EntityId::random(), entity)],
            }],
            None,
            None,
        )
        .unwrap();
    let mut h = rest_over(harness(w), Panel::Viewport);
    h.run();
    let opened = Instant::now();
    h.state_mut().0.open.ground.asked = Some(asset);
    h.state_mut().0.ground_loaded(
        asset,
        Ok(LoadedGround {
            asset,
            path: "Ground.ground.lst".into(),
            settings,
            ground,
        }),
    );
    let chunks = h.state().0.open.ground.left();
    let (mut slowest, mut frames, mut arrived) = (0.0_f64, 0, 0usize);
    while h.state().0.ground_busy() && opened.elapsed().as_secs() < 120 {
        let start = Instant::now();
        h.step();
        slowest = slowest.max(start.elapsed().as_secs_f64() * 1000.0);
        frames += 1;
        arrived += h.state_mut().0.take_ground_meshes().fresh.len();
    }
    let all_ms = opened.elapsed().as_secs_f64() * 1000.0;
    println!(
        "Ground {0} × {0} m: made in {made_ms:.0} ms (a worker, before opening); {chunks} \
         chunks meshed and made solid in {all_ms:.0} ms over {frames} frames; slowest frame \
         {slowest:.1} ms (target {FRAME_MS} ms) {}",
        side,
        if slowest <= FRAME_MS { "ok" } else { "SLOW" }
    );
    assert_eq!(arrived, chunks);
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_opening_the_1_km_ground() {
    open_ground(1024);
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_opening_the_4_km2_ground() {
    open_ground(2048);
}
