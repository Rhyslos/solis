//! The scale check for editing a brush's insides (Tools 1a, B3; decision
//! 0108, "all fast"): a brush of 5,000 faces edited in an area of 10,000
//! brushes. The frame that starts editing, frames hovering over it,
//! a box over all its faces, frames dragging the gizmo on 1,000 picked
//! faces, and letting go (the check, the command, the brush made again
//! and uploaded), each within the editor's frame target. Frames are
//! timed several times and the middle one judged: this machine's vary. Run by
//! `tools/scale/scale-check.sh`.

#![allow(clippy::print_stdout)]

use crate::brushing::edit::{Part, PickHow};
use crate::gizmo::Change;
use crate::panel::Panel;
use crate::test_support::{State, harness, info, rest_over};
use area::builtin::{Brush, Transform};
use area::{Edit, EntityData};
use glam::Vec3;
use std::sync::Arc;
use std::time::Instant;

type H = egui_kittest::Harness<'static, State>;

/// The editor's frame target with a big area.
const FRAME_MS: f64 = 8.0;

fn ms_since(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn verdict(ms: f64) -> &'static str {
    if ms <= FRAME_MS { "ok" } else { "SLOW" }
}

/// The last frame's milliseconds in each of the profiler's stages.
fn stages(h: &H) -> String {
    let profiler = &h.state().0.profiler;
    let Some(last) = profiler.history().last() else {
        return String::new();
    };
    crate::profiler::Stage::ALL
        .iter()
        .zip(last.cpu)
        .filter(|(_, ms)| *ms > 0.05)
        .map(|(stage, ms)| format!("{} {ms:.1}", stage.name()))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The editor's own work in the last frame: the profiler's stages.
fn own_ms(h: &H) -> f64 {
    h.state()
        .0
        .profiler
        .history()
        .last()
        .map_or(0.0, crate::profiler::Sample::cpu_total)
}

fn median(mut times: Vec<f64>) -> f64 {
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

fn timed(h: &mut H) -> f64 {
    let start = Instant::now();
    h.step();
    ms_since(start)
}

/// 10,000 small boxes, each its own size, and a box of 5,046 faces (29
/// pieces a side, 20 m) standing among them at the middle, picked.
fn editor() -> (H, reflect::EntityId) {
    let mut spawns = Vec::new();
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
        let data = EntityData::new(&format!("Box_{i:05}"))
            .with(&Transform {
                position: Vec3::new((i % 100) as f32 * 3.0, (i / 100) as f32 * 3.0, 0.0),
                ..Transform::default()
            })
            .with(&brush);
        spawns.push(Edit::Spawn {
            entities: vec![(reflect::EntityId::random(), data)],
        });
    }
    let mut w = crate::Workbench::new(info());
    w.open.document.edit("Fill", spawns, None, None).unwrap();
    let big = mesh::Primitive::Cube(mesh::CubeParams {
        size: Vec3::splat(20.0),
        segments: [29, 29, 29],
    })
    .mesh();
    assert!(big.face_count() >= 5_000);
    let id = w.spawn_selected(
        EntityData::new("Big")
            .with(&Transform {
                position: Vec3::new(150.0, 150.0, 10.0),
                ..Transform::default()
            })
            .with(&Brush {
                mesh: Arc::new(big),
                ..Brush::default()
            }),
    );
    w.viewport.tool = crate::viewport::Tool::Move;
    w.viewport.camera.position = Vec3::new(130.0, 115.0, 45.0);
    w.viewport.camera.look_at(Vec3::new(150.0, 150.0, 10.0));
    let mut h = rest_over(harness(w), Panel::Viewport);
    h.run();
    (h, id)
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_editing_a_5000_face_brush_among_10000_brushes() {
    let (mut h, id) = editor();
    let entities = h.state().0.document().area().len();
    // The frame that starts editing (its picking geometry made once):
    // the first time the ribbon's and the card's words are laid out, and
    // then again as it is every time after.
    h.key_press(egui::Key::Tab);
    let first_ms = timed(&mut h);
    h.key_press(egui::Key::Tab);
    h.step();
    assert!(h.state().0.viewport.brushing.edit.is_none());
    // Five times each, the middle one kept: this machine's frames vary.
    let mut other = Vec::new();
    for _ in 0..5 {
        h.key_press(egui::Key::K);
        other.push(timed(&mut h));
    }
    let other_key_ms = median(other);
    let other_stages = stages(&h);
    // The frame that starts editing: the whole test frame, and the
    // editor's own work in it (the profiler's stages). The test window
    // also builds the accessibility tree every frame and lays out the
    // words that weren't on screen the frame before (the ribbon's and the
    // card's), which a frame that changes the screen pays wherever it is.
    let (mut starts, mut own) = (Vec::new(), Vec::new());
    for i in 0..5 {
        h.key_press(egui::Key::Tab);
        starts.push(timed(&mut h));
        own.push(own_ms(&h));
        if i < 4 {
            h.key_press(egui::Key::Tab);
            h.step();
        }
    }
    let start_ms = median(starts);
    let start_own_ms = median(own);
    let start_stages = stages(&h);
    // Its picking geometry, made alone.
    let geometry_ms = {
        let mesh = h.state().0.open.brushes.get(id).unwrap().mesh.clone();
        let start = Instant::now();
        let _ = crate::brushing::parts::PartCache::of(mesh, 0);
        ms_since(start)
    };
    assert!(h.state().0.viewport.brushing.edit.is_some());
    // Hovering: the pointer moving over the brush, frame after frame.
    let rect = h.state().0.viewport.rect;
    let mut hovers = Vec::new();
    for i in 0..20 {
        let at = rect.center() + egui::vec2(i as f32 * 4.0 - 40.0, i as f32 * 2.0 - 20.0);
        h.hover_at(at);
        hovers.push(timed(&mut h));
    }
    assert!(
        h.state()
            .0
            .viewport
            .brushing
            .edit
            .as_ref()
            .unwrap()
            .hovered
            .is_some()
    );
    let hover_most = hovers.iter().copied().fold(0.0, f64::max);
    let hover_ms = median(hovers);
    // What's under the pointer, found alone.
    let (find_ms, box_ms, faces) = {
        let w = &h.state().0;
        let edit = w.viewport.brushing.edit.as_ref().unwrap();
        let parts = edit.parts.clone().unwrap();
        let camera = w.viewport.camera;
        let size = glam::Vec2::new(rect.width(), rect.height());
        let project = camera.projector(size);
        let world = w.document().world_matrix(id);
        let mouse = size / 2.0;
        let start = Instant::now();
        let ray = camera.ray_through(mouse, size);
        let _ = parts.under(edit.mode, (world, ray), &project, mouse);
        let find_ms = ms_since(start);
        // A box over the whole view: every face.
        let start = Instant::now();
        let all = parts.in_box(edit.mode, world, &project, (glam::Vec2::ZERO, size));
        (find_ms, ms_since(start), all)
    };
    assert!(faces.len() >= 5_000, "{}", faces.len());
    h.state_mut().0.pick_parts(&faces, PickHow::Replace);
    let box_frame_ms = timed(&mut h);
    // 1,000 faces picked: the top's and a ring below it.
    let mesh = h.state().0.open.brushes.get(id).unwrap().mesh.clone();
    let top = mesh.bounds().unwrap().max.z;
    let picked: Vec<Part> = mesh
        .faces()
        .filter(|(_, corners)| {
            corners.iter().all(|v| {
                mesh.position(*v)
                    .is_some_and(|p| p.z > top - 2.5 * 20.0 / 29.0)
            })
        })
        .map(|(f, _)| Part::Face(f))
        .collect();
    assert!(picked.len() >= 1_000, "{}", picked.len());
    h.state_mut().0.pick_parts(&picked, PickHow::Replace);
    h.step();
    // Frames dragging the gizmo: the drag's change made and drawn (only
    // this brush's mesh, never the area's).
    let made = h.state().0.open.brushes.made();
    {
        let w = &mut h.state_mut().0;
        w.viewport
            .tools
            .events
            .push(crate::viewport_tools::ViewportEvent::GizmoStart);
        w.tool_events();
    }
    let mut drags = Vec::new();
    for i in 1..=20 {
        let start = Instant::now();
        let w = &mut h.state_mut().0;
        w.viewport
            .tools
            .events
            .push(crate::viewport_tools::ViewportEvent::GizmoChange {
                change: Change::Translate(Vec3::Z * i as f32 * 0.05),
                flip: false,
            });
        w.tool_events();
        h.step();
        drags.push(ms_since(start));
    }
    assert_eq!(
        h.state().0.open.brushes.made(),
        made,
        "nothing made while dragging"
    );
    let drag_most = drags.iter().copied().fold(0.0, f64::max);
    let drag_ms = median(drags);
    // Letting go: the frame the button comes up in (the check, the
    // command, the brush made again once from the mesh in hand), and the
    // next (it uploaded and drawn).
    let steps = h.state().0.document().history().undo_len();
    h.state_mut()
        .0
        .viewport
        .tools
        .events
        .push(crate::viewport_tools::ViewportEvent::GizmoEnd);
    let let_go_ms = timed(&mut h);
    let let_go_stages = stages(&h);
    let next_ms = timed(&mut h);
    assert_eq!(h.state().0.document().history().undo_len(), steps + 1);
    assert_eq!(h.state().0.open.brushes.made(), made + 1, "made once");
    println!(
        "Scale check: editing a brush of {} faces among {entities} entities: the frame that \
         starts it {start_ms:.2} ms, the editor's own work in it {start_own_ms:.2} ms {} (its picking geometry {geometry_ms:.2} ms; the first \
         time, with the words laid out, {first_ms:.2} ms; a frame with another key pressed \
         {other_key_ms:.2} ms, {other_stages}); hovering {hover_ms:.2} ms a frame ({hover_most:.2} at most) {} (finding the part \
         {find_ms:.3} ms); a box over all its faces {box_ms:.2} ms, its frame {box_frame_ms:.2} \
         ms {}; dragging {} faces {drag_ms:.2} ms a frame ({drag_most:.2} at most) {}; letting go {let_go_ms:.2} ms \
         and the next frame {next_ms:.2} ms {} (target {FRAME_MS} ms; the frames that start \
         and let go by stage: {start_stages}, {let_go_stages})",
        mesh.face_count(),
        verdict(start_own_ms),
        verdict(hover_ms),
        verdict(box_ms + box_frame_ms),
        picked.len(),
        verdict(drag_ms),
        verdict(let_go_ms.max(next_ms)),
    );
    for (what, ms) in [
        ("starting", start_own_ms),
        ("hovering", hover_ms),
        ("the box", box_ms + box_frame_ms),
        ("dragging", drag_ms),
        ("letting go", let_go_ms),
        ("the frame after letting go", next_ms),
    ] {
        assert!(ms <= FRAME_MS, "{what} takes {ms:.2} ms");
    }
}
