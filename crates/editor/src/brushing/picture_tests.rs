//! The brush list open and a box being drawn out, drawn for real in the
//! sample project as the approved mock-ups show them
//! (`docs/design/tools-1/01-brush-list.png`, `02-brush-draw.png`), each
//! held to an approved picture of Solis's own.

use crate::picture_support::{Picture, drawn, editor_window, matches_approved, sample_project};
use egui::{Pos2, pos2};
use egui_kittest::Harness;
use glam::Vec3;

/// Where the pointer rests over the view, as in the mock-ups.
const POINTER: Pos2 = pos2(585.0, 345.0);

/// The sample project with nothing selected and the grid on at half a
/// metre, as the mock-ups draw it, in Home's standard layout.
fn editor() -> Option<Harness<'static, crate::workbench::Workbench>> {
    let mut w = sample_project();
    w.open.document.selection.clear();
    w.viewport.snapping.grid = true;
    w.viewport.snapping.grid_step = 0.5;
    let mut h = editor_window(w, crate::InterfaceLook::default(), [1440.0, 900.0])?;
    h.run();
    h.run();
    h.hover_at(POINTER);
    h.run();
    Some(h)
}

fn press(h: &Harness<'static, crate::workbench::Workbench>, pressed: bool, at: Pos2) {
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    });
}

/// B pressed twice over the view: the list by the pointer, Ramp
/// highlighted.
pub(crate) fn list_open() -> Option<Picture> {
    let mut h = editor()?;
    for _ in 0..2 {
        h.key_press(egui::Key::B);
        h.run();
    }
    assert_eq!(
        h.state().viewport.brushing.list.map(|l| l.chosen()),
        Some(mesh::Shape::Ramp)
    );
    Some(drawn(&mut h))
}

/// A box being drawn out: its footprint, 4 by 3 m, kept, and the height
/// being pulled up.
pub(crate) fn drawing_out() -> Option<Picture> {
    let mut h = editor()?;
    // Near enough to see the box as the mock-up does.
    let camera = &mut h.state_mut().viewport.camera;
    camera.position = Vec3::new(-4.0, -9.0, 6.0);
    camera.look_at(Vec3::new(0.0, 0.0, 0.5));
    h.state_mut().take_brush(mesh::Shape::Box);
    h.run();
    let on_screen = |h: &Harness<'static, crate::workbench::Workbench>, p: Vec3| {
        let w = h.state();
        let rect = w.viewport.rect;
        let at = w
            .viewport
            .camera
            .project(p, glam::Vec2::new(rect.width(), rect.height()))
            .expect("in view");
        rect.min + egui::vec2(at.x, at.y)
    };
    let ground = Vec3::ZERO;
    let (from, to) = (
        on_screen(&h, ground - Vec3::new(2.0, 1.5, 0.0)),
        on_screen(&h, ground + Vec3::new(2.0, 1.5, 0.0)),
    );
    h.hover_at(from);
    h.run();
    press(&h, true, from);
    h.run();
    for step in 1..=4 {
        h.hover_at(from + (to - from) * step as f32 / 4.0);
        h.run();
    }
    press(&h, false, to);
    h.run();
    h.hover_at(to - egui::vec2(0.0, 60.0));
    h.run();
    h.run();
    let draw = h.state().viewport.brushing.draw.clone();
    assert!(
        matches!(
            draw.as_ref().map(|d| d.step),
            Some(super::draw::Step::Height { height, .. }) if height > 0.0
        ),
        "{draw:?}"
    );
    Some(drawn(&mut h))
}

#[test]
fn the_brush_list_open_is_its_approved_picture() {
    if let Some(picture) = list_open() {
        matches_approved("brush-list", &picture);
    }
}

#[test]
fn drawing_a_brush_out_is_its_approved_picture() {
    if let Some(picture) = drawing_out() {
        matches_approved("brush-draw", &picture);
    }
}
