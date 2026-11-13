//! Scale checks for brushes (Tools 1a, B1; CONTRIBUTING.md, rule 18): an
//! area of 10,000 box brushes opened, shown, edited and played, and a
//! brush of 5,000 faces made again after a change. Run by
//! `tools/scale/scale-check.sh`.

#![allow(clippy::print_stdout)]

use super::*;
use crate::document::Document;
use area::builtin::{CharacterBody, Transform};
use area::{Edit, EntityData};
use std::time::Instant;

/// The editor's frame target with a big area (as the 10,000-entity
/// check's).
const FRAME_MS: f64 = 8.0;

fn ms_since(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn verdict(ms: f64, target: f64) -> &'static str {
    if ms <= target { "ok" } else { "SLOW" }
}

/// 10,000 box brushes on a grid, each its own size (so none share a
/// model: the most work), and a character standing on the first.
fn ten_thousand_boxes() -> Area {
    let mut area = Area::new();
    for i in 0..10_000 {
        let size = Vec3::new(
            1.0 + (i % 100) as f32 * 0.01,
            1.0 + (i / 100) as f32 * 0.01,
            1.0,
        );
        let brush = Brush {
            mesh: Arc::new(mesh::Shape::Box.build(&mesh::ShapeSettings {
                size,
                ..mesh::Shape::Box.standard()
            })),
            ..Brush::default()
        };
        area.spawn(
            EntityData::new(&format!("Box_{i:05}"))
                .with(&Transform {
                    position: Vec3::new((i % 100) as f32 * 3.0, (i / 100) as f32 * 3.0, 0.0),
                    ..Transform::default()
                })
                .with(&brush),
        );
    }
    area.spawn(
        EntityData::new("Walker")
            .with(&Transform {
                position: Vec3::new(0.0, 0.0, 1.5),
                ..Transform::default()
            })
            .with(&CharacterBody::default()),
    );
    area
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_an_area_of_10000_box_brushes() {
    let registry = area::builtin::registry().unwrap();
    let text = area::text::write(&ten_thousand_boxes(), &registry, &|_| None);
    // Opening: reading the file, and cutting every brush into triangles.
    let start = Instant::now();
    let area = area::text::read(&text, &registry).unwrap();
    let read_ms = ms_since(start);
    let mut w = crate::Workbench::new(crate::test_support::info());
    let ids: Vec<(EntityId, EntityData)> = area.iter().map(|(id, e)| (id, e.clone())).collect();
    w.open
        .document
        .edit("Fill", vec![Edit::Spawn { entities: ids }], None, None)
        .unwrap();
    let start = Instant::now();
    let document = &w.open.document;
    w.open.brushes.refresh(document.area(), document.brushes());
    let cut_ms = ms_since(start);
    assert_eq!(w.open.brushes.models(), 10_000);
    println!(
        "Scale check: 10,000 box brushes opened: read {read_ms:.0} ms ({} KB), cut into \
         triangles {cut_ms:.0} ms",
        text.len() / 1024
    );
    // An editor frame with them all shown, nothing changing.
    let mut h = crate::test_support::harness(w);
    h.run();
    let frames = 20;
    let start = Instant::now();
    for _ in 0..frames {
        h.step();
    }
    let frame_ms = ms_since(start) / f64::from(frames);
    println!(
        "Scale check: editor frame with 10,000 box brushes {frame_ms:.2} ms (target \
         {FRAME_MS} ms) {}",
        verdict(frame_ms, FRAME_MS)
    );
    // One brush changed: every brush is looked at again, one is made.
    let (w, _) = h.state_mut();
    let first = w.document().area().with::<CharacterBody>().count();
    assert_eq!(first, 1);
    let some = w.open.brushes.iter().next().map(|(id, _)| id).unwrap();
    w.open
        .document
        .edit(
            "Not solid",
            vec![Edit::SetField {
                entity: some,
                component: Brush::NAME.into(),
                field: "solid".into(),
                value: Some(Value::Bool(false)),
            }],
            None,
            None,
        )
        .unwrap();
    let made = w.open.brushes.made();
    let start = Instant::now();
    let document = &w.open.document;
    w.open.brushes.refresh(document.area(), document.brushes());
    let changed_ms = ms_since(start);
    assert_eq!(
        w.open.brushes.made(),
        made + 1,
        "only the changed one is made"
    );
    println!(
        "Scale check: one of 10,000 brushes changed, looked at again {changed_ms:.2} ms \
         (target {FRAME_MS} ms) {}",
        verdict(changed_ms, FRAME_MS)
    );
    // Play: the world built (every brush solid), then ticks of physics
    // with a character walking over them.
    let start = Instant::now();
    let mut world = world::World::from_area(w.document().area());
    let play_ms = ms_since(start);
    let walker = w
        .document()
        .area()
        .with::<CharacterBody>()
        .next()
        .map(|(id, _)| id)
        .unwrap();
    let ticks = 120;
    let start = Instant::now();
    for _ in 0..ticks {
        world.move_character(walker, Vec3::new(3.0, 0.0, -0.5) / 60.0);
        world.step_physics();
    }
    let tick_ms = ms_since(start) / f64::from(ticks);
    const TICK_MS: f64 = 4.0;
    println!(
        "Scale check: Play with 10,000 solid brushes: the world built {play_ms:.0} ms; a tick \
         of physics, a character walking on them {tick_ms:.2} ms (target {TICK_MS} ms) {}",
        verdict(tick_ms, TICK_MS)
    );
    assert!(
        frame_ms <= FRAME_MS,
        "the editor takes {frame_ms:.2} ms a frame"
    );
    assert!(
        changed_ms <= FRAME_MS,
        "a brush change takes {changed_ms:.2} ms"
    );
    assert!(tick_ms <= TICK_MS, "a tick takes {tick_ms:.2} ms");
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_a_5000_face_brush_made_again_after_a_change() {
    // A box cut into 29 × 29 pieces a side: 5,046 faces.
    let cube = mesh::Primitive::Cube(mesh::CubeParams {
        size: Vec3::splat(100.0),
        segments: [29, 29, 29],
    })
    .mesh();
    let faces = cube.face_count();
    assert!(faces >= 5_000);
    let brush = Brush {
        mesh: Arc::new(cube),
        ..Brush::default()
    };
    let mut area = Area::new();
    let id = area.spawn(EntityData::new("Big").with(&brush));
    let mut document = Document::new(area);
    let mut models = BrushModels::default();
    models.refresh(document.area(), document.brushes());
    // The change: every point moved a little (as a tool would), written
    // into the area as one edit.
    let mut moved = (*brush.mesh).clone();
    let points: Vec<_> = moved.vertices().map(|(v, _)| v).collect();
    mesh::edit::move_points(&mut moved, &points, Vec3::Z * 0.5);
    let start = Instant::now();
    let data = Brush {
        mesh: Arc::new(moved),
        ..brush
    }
    .to_data();
    let written_ms = ms_since(start);
    document
        .edit(
            "Move points",
            vec![Edit::SetField {
                entity: id,
                component: Brush::NAME.into(),
                field: "points".into(),
                value: data.get("points").cloned(),
            }],
            None,
            None,
        )
        .unwrap();
    let start = Instant::now();
    models.refresh(document.area(), document.brushes());
    let made_ms = ms_since(start);
    assert_eq!(models.get(id).unwrap().triangles, faces * 2);
    println!(
        "Scale check: a brush of {faces} faces changed: its values written {written_ms:.2} \
         ms, read and cut into triangles again {made_ms:.2} ms (target {FRAME_MS} ms) {}",
        verdict(written_ms + made_ms, FRAME_MS)
    );
    assert!(written_ms + made_ms <= FRAME_MS);
}
