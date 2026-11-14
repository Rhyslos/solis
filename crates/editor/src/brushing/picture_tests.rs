//! The brush list open, a box being drawn out, a box's insides being
//! edited in faces and in points mode, and its face extruded and inset
//! (B4), drawn for real in the sample project as the approved mock-ups
//! show them (`docs/design/tools-1/01-brush-list.png`, `02-brush-draw.png`,
//! `03-brush-edit-faces.png`, `04-brush-extrude.png`), each held to an
//! approved picture of Solis's own.

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

type H = Harness<'static, crate::workbench::Workbench>;

/// Box_03, 4 × 2.5 × 3 m, standing at `AT`, its insides edited with the
/// Move tool, seen as the mock-ups 3 and 4 see it.
fn box_edited() -> Option<H> {
    use area::builtin::{Brush, Transform};
    let mut h = editor()?;
    let w = h.state_mut();
    let size = Vec3::new(4.0, 2.5, 3.0);
    let brush = Brush {
        mesh: std::sync::Arc::new(
            mesh::Shape::Box.build(&super::settings_for(mesh::Shape::Box, size)),
        ),
        ..Brush::default()
    };
    w.spawn_selected(
        area::EntityData::new("Box_03")
            .with(&Transform {
                position: AT,
                ..Transform::default()
            })
            .with(&brush),
    );
    w.viewport.tool = crate::viewport::Tool::Move;
    let camera = &mut w.viewport.camera;
    camera.position = AT + Vec3::new(-4.5, -7.0, 5.5);
    camera.look_at(AT + Vec3::new(0.0, 0.0, 1.2));
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    Some(h)
}

/// Where the box stands.
const AT: Vec3 = Vec3::new(2.0, 0.0, 0.0);

/// Where world point `p` is on screen.
fn on_screen(h: &H, p: Vec3) -> Pos2 {
    let w = h.state();
    let rect = w.viewport.rect;
    let at = w
        .viewport
        .camera
        .project(p, glam::Vec2::new(rect.width(), rect.height()))
        .expect("in view");
    rect.min + egui::vec2(at.x, at.y)
}

/// The edited box's face facing `normal`, picked.
fn pick_face(h: &mut H, normal: Vec3) {
    use super::edit::{Part, PickHow};
    let w = h.state_mut();
    let edit = w.viewport.brushing.edit.as_ref().expect("editing");
    let mesh = edit.parts.as_ref().expect("its parts").mesh();
    let face = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).dot(normal) > 0.99)
        .expect("a face");
    w.pick_parts(&[Part::Face(face)], PickHow::Replace);
    h.run();
}

/// Box_03 being edited as mock-up 3 draws it: its side facing the
/// camera picked in faces mode, the Move gizmo on it; or, in points
/// mode, that face's corners.
pub(crate) fn editing(points: bool) -> Option<Picture> {
    let mut h = box_edited()?;
    let front = on_screen(&h, AT + Vec3::new(0.0, -1.25, 1.5));
    h.hover_at(front);
    h.run();
    press(&h, true, front);
    h.run();
    press(&h, false, front);
    h.run();
    if points {
        h.key_press(egui::Key::Num1);
        h.run();
    }
    h.hover_at(POINTER);
    h.run();
    h.run();
    let edit = h.state().viewport.brushing.edit.as_ref().expect("editing");
    assert_eq!(edit.picked.len(), if points { 4 } else { 1 });
    Some(drawn(&mut h))
}

/// Box_03's end face being extruded 2 m with E, as mock-up 4 draws it:
/// the new part following the mouse, its distance beside it and at the
/// top of the view, the keys at the foot and the status bar.
pub(crate) fn extruding() -> Option<Picture> {
    let mut h = box_edited()?;
    pick_face(&mut h, Vec3::X);
    let middle = AT + Vec3::new(2.0, 0.0, 1.5);
    h.hover_at(on_screen(&h, middle));
    h.run();
    h.key_press(egui::Key::E);
    h.run();
    h.hover_at(on_screen(&h, middle + Vec3::X * 2.0));
    h.run();
    h.run();
    let edit = h.state().viewport.brushing.edit.as_ref().expect("editing");
    let pull = edit.pull.as_ref().expect("extruding");
    assert_eq!(pull.metres, 2.0);
    Some(drawn(&mut h))
}

/// Box_03's front face being inset by half a metre.
pub(crate) fn insetting() -> Option<Picture> {
    let mut h = box_edited()?;
    pick_face(&mut h, -Vec3::Y);
    let middle = AT + Vec3::new(0.0, -1.25, 1.5);
    h.hover_at(on_screen(&h, middle));
    h.run();
    h.state_mut()
        .shape_brush(crate::shell::ShellCommand::BrushInset);
    h.run();
    let right = h.state().viewport.camera.right();
    h.hover_at(on_screen(&h, middle + right * 0.5));
    h.run();
    h.run();
    let edit = h.state().viewport.brushing.edit.as_ref().expect("editing");
    let pull = edit.pull.as_ref().expect("insetting");
    assert_eq!(pull.metres, 0.5);
    Some(drawn(&mut h))
}

#[test]
fn extruding_a_brush_s_face_is_its_approved_picture() {
    if let Some(picture) = extruding() {
        matches_approved("brush-extrude", &picture);
    }
}

#[test]
fn insetting_a_brush_s_face_is_its_approved_picture() {
    if let Some(picture) = insetting() {
        matches_approved("brush-inset", &picture);
    }
}

#[test]
fn editing_a_brush_s_faces_is_its_approved_picture() {
    if let Some(picture) = editing(false) {
        matches_approved("brush-edit-faces", &picture);
    }
}

#[test]
fn editing_a_brush_s_points_is_its_approved_picture() {
    if let Some(picture) = editing(true) {
        matches_approved("brush-edit-points", &picture);
    }
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
