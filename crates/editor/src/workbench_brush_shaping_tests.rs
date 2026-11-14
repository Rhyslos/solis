//! Shaping a brush in the whole editor (Tools 1a, B4), as a person does
//! it: E, the mouse and a click; Shift + drag on the move gizmo; giving
//! up with Esc or a right-click; snapping on and off; one undo step each;
//! touching faces extruded as one; edges refused on a closed brush; an
//! inset's width; each split and delete, refused ones saying why; the
//! whole check on every action; the ribbon greying its buttons; Delete
//! outside editing; a brush read open or inside out; and a save and
//! reopen after each action.

use super::*;
use crate::brushing::edit::{Mode, Part, PickHow};
use crate::gizmo::Change;
use crate::panel::Panel;
use crate::test_support::*;
use crate::viewport::Tool;
use crate::viewport_tools::ViewportEvent;
use area::EntityData;
use area::builtin::Transform;
use egui::{Key, Modifiers, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use reflect::EntityId;

type H = egui_kittest::Harness<'static, State>;

/// An editor with one 2 m box brush standing at the middle of the
/// ground, picked, its insides edited with the pointer over the 3D
/// view, the grid's step half a metre.
fn editing() -> (H, EntityId) {
    let mut w = Workbench::new(info());
    let id = w.spawn_selected(
        EntityData::new("Box_03")
            .with(&Transform::default())
            .with(&Brush::default()),
    );
    w.viewport.snapping.grid_step = 0.5;
    let mut h = rest_over(harness(w), Panel::Viewport);
    h.key_press(Key::Tab);
    h.run();
    assert!(h.state().0.viewport.brushing.edit.is_some());
    (h, id)
}

fn edit(h: &H) -> &crate::brushing::edit::Editing {
    h.state()
        .0
        .viewport
        .brushing
        .edit
        .as_ref()
        .expect("editing")
}

fn status(h: &H) -> Option<String> {
    h.state().0.status().brushing
}

fn steps(h: &H) -> usize {
    h.state().0.document().history().undo_len()
}

/// The brush's mesh as the area holds it.
fn mesh_of(h: &H, id: EntityId) -> mesh::Mesh {
    let brush = h
        .state()
        .0
        .document()
        .area()
        .get(id)
        .unwrap()
        .get::<Brush>();
    (*brush.unwrap().mesh).clone()
}

/// Where world point `p` is on screen in the view in use.
fn on_screen(h: &H, p: Vec3) -> Pos2 {
    let w = &h.state().0;
    let rect = w.viewport.rect;
    let at = w
        .viewport
        .camera
        .project(p, glam::Vec2::new(rect.width(), rect.height()))
        .expect("in view");
    rect.min + egui::vec2(at.x, at.y)
}

/// The edited brush's face facing `normal`.
fn face(h: &H, normal: Vec3) -> mesh::FaceId {
    let mesh = edit(h).parts.as_ref().unwrap().mesh();
    mesh.faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).dot(normal) > 0.99)
        .unwrap()
}

/// Picks `parts` in `mode`.
fn pick(h: &mut H, mode: Mode, parts: &[Part]) {
    let w = &mut h.state_mut().0;
    w.brush_mode(mode);
    w.pick_parts(parts, PickHow::Replace);
    h.run();
}

/// The top face picked.
fn top_picked(h: &mut H) -> mesh::FaceId {
    let top = face(h, Vec3::Z);
    pick(h, Mode::Faces, &[Part::Face(top)]);
    top
}

/// Moves the pointer to `p` (a world point) and draws a frame.
fn point_at(h: &mut H, p: Vec3, modifiers: Modifiers) {
    let at = on_screen(h, p);
    h.event(egui::Event::ModifiersChanged(modifiers));
    h.hover_at(at);
    h.run();
}

/// The pull under way set to `metres`, as the mouse would (no frame is
/// drawn after: the mouse would set it again).
fn pull_to(h: &mut H, metres: f32) {
    let w = &mut h.state_mut().0;
    let pull = w
        .viewport
        .brushing
        .edit
        .as_mut()
        .and_then(|e| e.pull.as_mut())
        .expect("a pull under way");
    pull.set(metres);
    w.pulled(crate::brushing::pull::PullOutcome::Going(true));
}

/// The pull under way kept, as a click does.
fn keep(h: &mut H) {
    h.state_mut()
        .0
        .pulled(crate::brushing::pull::PullOutcome::Keep);
    h.run();
}

/// The area written and read back: the brush's mesh as reopened.
fn reopened(h: &H, id: EntityId) -> mesh::Mesh {
    let w = &h.state().0;
    let registry = area::builtin::registry().unwrap();
    let text = area::text::write(w.document().area(), &registry, &|_| None);
    let read = area::text::read(&text, &registry).unwrap();
    let brush = read.get(id).unwrap().get::<Brush>().unwrap();
    assert!(!brush.damaged);
    (*brush.mesh).clone()
}

/// Whether the edited brush's mesh is whole.
fn whole(mesh: &mesh::Mesh) -> bool {
    let all: Vec<mesh::VertexId> = mesh.vertices().map(|(v, _)| v).collect();
    mesh::check_whole(mesh, &all).is_ok()
}

#[test]
fn e_then_the_mouse_then_a_click_extrudes_the_top_as_one_undo_step() {
    let (mut h, id) = editing();
    top_picked(&mut h);
    let before = steps(&h);
    // The pointer where the pull begins, then E.
    point_at(&mut h, Vec3::new(0.0, 0.0, 2.0), Modifiers::NONE);
    h.key_press(Key::E);
    h.run();
    assert!(edit(&h).pull.is_some(), "E starts it");
    assert_eq!(
        status(&h).as_deref(),
        Some("Extruding 1 face: 0.00 m \u{b7} Click to keep \u{b7} Esc or right-click to give up")
    );
    point_at(&mut h, Vec3::new(0.0, 0.0, 2.0), Modifiers::NONE);
    point_at(&mut h, Vec3::new(0.0, 0.0, 3.1), Modifiers::NONE);
    let metres = edit(&h).pull.as_ref().unwrap().metres;
    assert_eq!(metres, 1.0, "snapped to the grid's half metre");
    assert!(
        edit(&h)
            .top_words()
            .starts_with("Extruding 1 face \u{b7} 1.00 m")
    );
    let at = on_screen(&h, Vec3::new(0.0, 0.0, 3.1));
    click(&mut h, at, Modifiers::NONE);
    let mesh = mesh_of(&h, id);
    assert_eq!(mesh.face_count(), 10);
    assert!((mesh.bounds().unwrap().max.z - 3.0).abs() < 1e-4);
    assert_eq!(steps(&h), before + 1, "one undo step");
    assert_eq!(
        h.state().0.document().history().undo_label(),
        Some("Extrude 1 face")
    );
    assert!(edit(&h).pull.is_none());
    assert_eq!(edit(&h).picked.len(), 1, "the moved face stays picked");
    assert!(whole(&mesh));
    assert_eq!(reopened(&h, id), mesh, "a save and reopen");
    // Undo takes it back whole, editing on.
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run();
    assert_eq!(mesh_of(&h, id).face_count(), 6);
    assert!(h.state().0.viewport.brushing.edit.is_some());
}

#[test]
fn ctrl_pulls_without_snapping() {
    let (mut h, _) = editing();
    top_picked(&mut h);
    point_at(&mut h, Vec3::new(0.0, 0.0, 2.0), Modifiers::COMMAND);
    h.key_press(Key::E);
    h.run();
    point_at(&mut h, Vec3::new(0.0, 0.0, 2.8), Modifiers::COMMAND);
    let metres = edit(&h).pull.as_ref().unwrap().metres;
    assert!((metres - 0.8).abs() < 0.05, "{metres}");
    assert!(edit(&h).pull.as_ref().unwrap().free);
}

#[test]
fn esc_or_a_right_click_gives_up_leaving_nothing() {
    for right_click in [false, true] {
        let (mut h, id) = editing();
        top_picked(&mut h);
        let before = steps(&h);
        h.key_press(Key::E);
        h.run();
        point_at(&mut h, Vec3::new(0.0, 0.0, 2.0), Modifiers::NONE);
        point_at(&mut h, Vec3::new(0.0, 0.0, 3.6), Modifiers::NONE);
        if right_click {
            let at = on_screen(&h, Vec3::new(0.0, 0.0, 3.6));
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: Modifiers::NONE,
            });
            h.run();
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: Modifiers::NONE,
            });
        } else {
            h.key_press(Key::Escape);
        }
        h.run();
        assert!(edit(&h).pull.is_none(), "given up");
        assert_eq!(mesh_of(&h, id).face_count(), 6);
        assert_eq!(steps(&h), before, "no step");
        assert!(!edit(&h).picked.is_empty(), "the pick stays");
        assert!(
            h.state().0.viewport.brushing.edit.is_some(),
            "editing goes on"
        );
    }
}

#[test]
fn shift_and_a_drag_on_the_move_gizmo_extrudes_as_it_moves() {
    let (mut h, id) = editing();
    top_picked(&mut h);
    h.state_mut().0.viewport.tool = Tool::Move;
    let before = steps(&h);
    let w = &mut h.state_mut().0;
    w.viewport.tools.events = vec![
        ViewportEvent::GizmoStart { extrude: true },
        ViewportEvent::GizmoChange {
            change: Change::Translate(Vec3::new(0.5, 0.0, 1.5)),
            flip: false,
        },
        ViewportEvent::GizmoEnd,
    ];
    w.tool_events();
    h.run();
    let mesh = mesh_of(&h, id);
    assert_eq!(mesh.face_count(), 10, "extruded");
    assert!((mesh.bounds().unwrap().max.z - 3.5).abs() < 1e-4);
    assert!(
        (mesh.bounds().unwrap().max.x - 1.5).abs() < 1e-4,
        "and moved"
    );
    assert_eq!(steps(&h), before + 1);
    assert_eq!(
        h.state().0.document().history().undo_label(),
        Some("Extrude 1 face")
    );
    assert_eq!(reopened(&h, id), mesh);
    // A drag of nothing makes nothing.
    let w = &mut h.state_mut().0;
    w.viewport.tools.events = vec![
        ViewportEvent::GizmoStart { extrude: true },
        ViewportEvent::GizmoEnd,
    ];
    w.tool_events();
    h.run();
    assert_eq!(mesh_of(&h, id).face_count(), 10);
    // Points mode: Shift + drag only moves.
    let corner = Part::Point(mesh.vertices().next().unwrap().0);
    pick(&mut h, Mode::Points, &[corner]);
    let w = &mut h.state_mut().0;
    w.viewport.tools.events = vec![
        ViewportEvent::GizmoStart { extrude: true },
        ViewportEvent::GizmoChange {
            change: Change::Translate(Vec3::Z * 0.25),
            flip: false,
        },
        ViewportEvent::GizmoEnd,
    ];
    w.tool_events();
    h.run();
    assert_eq!(mesh_of(&h, id).face_count(), 10);
}

#[test]
fn shift_on_the_gizmo_is_extruding_and_not_fine_moves() {
    use crate::viewport_tools::ToolInput;
    let (mut h, _) = editing();
    top_picked(&mut h);
    h.state_mut().0.viewport.tool = Tool::Move;
    h.run();
    let w = &mut h.state_mut().0;
    let rect = w.viewport.rect;
    let size = glam::Vec2::new(rect.width(), rect.height());
    let camera = w.viewport.camera;
    let gizmo = w.viewport.tools.gizmo.expect("on the face");
    let (_, shape) = gizmo
        .shapes(&camera, size, [false; 3])
        .into_iter()
        .find(|(handle, _)| *handle == crate::gizmo::Handle::Axis(2))
        .unwrap();
    let crate::gizmo::Shape::Line { a, b, .. } = shape else {
        panic!("a line")
    };
    let grab = (a + b) / 2.0;
    let press = ToolInput {
        mouse: Some(grab),
        pressed: true,
        down: true,
        shift: true,
        on_viewport: true,
        ..ToolInput::default()
    };
    let snapping = w.viewport.snapping;
    w.viewport.tools.events.clear();
    w.viewport
        .tools
        .interact(press, &camera, &w.viewport.area, &snapping, size, 1.0);
    assert_eq!(
        w.viewport.tools.events,
        [ViewportEvent::GizmoStart { extrude: true }]
    );
}

#[test]
fn touching_faces_go_out_together_as_one_piece() {
    let (mut h, id) = editing();
    let (top, side) = (face(&h, Vec3::Z), face(&h, Vec3::X));
    pick(&mut h, Mode::Faces, &[Part::Face(top), Part::Face(side)]);
    run(&mut h, "Extrude");
    pull_to(&mut h, 1.0);
    keep(&mut h);
    let mesh = mesh_of(&h, id);
    // One piece: a wall along each of the six sides of its border (two
    // pieces would make eight).
    assert_eq!(mesh.face_count(), 12);
    assert!(whole(&mesh));
    assert_eq!(reopened(&h, id), mesh);
}

#[test]
fn edges_of_a_closed_brush_cant_be_extruded_and_the_status_bar_says_why() {
    let (mut h, id) = editing();
    let edge = edit(&h).parts.as_ref().unwrap().mesh().edges()[0];
    pick(&mut h, Mode::Edges, &[Part::Edge(edge)]);
    h.key_press(Key::E);
    h.run();
    assert!(edit(&h).pull.is_none());
    assert!(
        status(&h)
            .unwrap()
            .starts_with("Only an edge on an open side")
    );
    assert_eq!(mesh_of(&h, id).face_count(), 6);
}

#[test]
fn an_inset_is_as_wide_as_the_mouse_says_as_one_undo_step() {
    let (mut h, id) = editing();
    let top = top_picked(&mut h);
    let before = steps(&h);
    run(&mut h, "Inset");
    assert!(status(&h).unwrap().starts_with("Insetting 1 face"));
    pull_to(&mut h, 0.5);
    keep(&mut h);
    let mesh = mesh_of(&h, id);
    assert_eq!(mesh.face_count(), 10);
    let corners = mesh.face(top).unwrap();
    let size = mesh::Bounds::around(corners.iter().map(|v| mesh.position(*v).unwrap()))
        .unwrap()
        .size();
    assert!((size.x - 1.0).abs() < 1e-4 && (size.y - 1.0).abs() < 1e-4);
    assert_eq!(steps(&h), before + 1);
    assert_eq!(edit(&h).picked, [Part::Face(top)].into(), "the inner face");
    assert_eq!(reopened(&h, id), mesh);
    // Wider than the face: refused, put back.
    run(&mut h, "Inset");
    pull_to(&mut h, 1.5);
    keep(&mut h);
    assert_eq!(mesh_of(&h, id).face_count(), 10);
    assert!(status(&h).unwrap().starts_with("Put back"));
}

#[test]
fn each_kind_of_split_cuts_and_a_refused_one_says_why() {
    let (mut h, id) = editing();
    // Faces: the top cut across its middle.
    top_picked(&mut h);
    run(&mut h, "Split");
    let mesh = mesh_of(&h, id);
    assert_eq!((mesh.face_count(), mesh.vertex_count()), (7, 10));
    assert_eq!(edit(&h).picked.len(), 2, "both halves picked");
    assert_eq!(reopened(&h, id), mesh);
    // Edges: a point in each picked edge's middle.
    let edge = mesh.edges()[0];
    pick(&mut h, Mode::Edges, &[Part::Edge(edge)]);
    run(&mut h, "Split");
    assert_eq!(mesh_of(&h, id).vertex_count(), 11);
    // Points: two across a face cut it between them.
    let mesh = mesh_of(&h, id);
    let bottom = mesh
        .faces()
        .find(|(f, _)| mesh.face_normal(*f).z < -0.99)
        .unwrap()
        .1
        .to_vec();
    let (a, b) = (bottom[0], bottom[2]);
    let faces = mesh.face_count();
    pick(&mut h, Mode::Points, &[Part::Point(a), Part::Point(b)]);
    run(&mut h, "Split");
    assert_eq!(mesh_of(&h, id).face_count(), faces + 1);
    assert!(whole(&mesh_of(&h, id)));
    // Two points on no one face: nothing, and why.
    let steps_now = steps(&h);
    let mesh = mesh_of(&h, id);
    let top_corner = mesh
        .vertices()
        .max_by(|x, y| (x.1.z - x.1.x).total_cmp(&(y.1.z - y.1.x)))
        .unwrap()
        .0;
    let far = mesh
        .vertices()
        .max_by(|x, y| {
            let p = mesh.position(top_corner).unwrap();
            x.1.distance(p).total_cmp(&y.1.distance(p))
        })
        .unwrap()
        .0;
    pick(
        &mut h,
        Mode::Points,
        &[Part::Point(top_corner), Part::Point(far)],
    );
    h.state_mut()
        .0
        .shape_brush(crate::shell::ShellCommand::BrushSplit);
    h.run();
    assert_eq!(status(&h).as_deref(), Some(mesh::split::NOT_ON_ONE_FACE));
    assert_eq!(steps(&h), steps_now);
}

#[test]
fn each_kind_of_delete_merges_and_holes_are_refused() {
    let (mut h, id) = editing();
    // Faces: a lone face is refused; the two halves of a split merge.
    let top = top_picked(&mut h);
    h.key_press(Key::Delete);
    h.run();
    assert_eq!(status(&h).as_deref(), Some(mesh::merge::LONE_FACE));
    assert_eq!(mesh_of(&h, id).face_count(), 6);
    run(&mut h, "Split");
    assert_eq!(edit(&h).picked.len(), 2);
    let before = steps(&h);
    h.key_press(Key::Delete);
    h.run();
    let mesh = mesh_of(&h, id);
    assert_eq!(
        (mesh.face_count(), mesh.vertex_count()),
        (6, 8),
        "merged back"
    );
    assert_eq!(steps(&h), before + 1);
    assert_eq!(edit(&h).picked, [Part::Face(top)].into());
    assert_eq!(reopened(&h, id), mesh);
    // Edges: the faces on either side merge.
    let edge = mesh.edges()[0];
    pick(&mut h, Mode::Edges, &[Part::Edge(edge)]);
    h.key_press(Key::Backspace);
    h.run();
    let mesh = mesh_of(&h, id);
    assert_eq!(mesh.face_count(), 5);
    assert!(whole(&mesh));
    assert_eq!(reopened(&h, id), mesh);
    // Points: the faces around one merge, until fewer than four would be
    // left, which is refused.
    let mut refused = false;
    for _ in 0..4 {
        let mesh = mesh_of(&h, id);
        let corner = mesh.vertices().next().unwrap().0;
        pick(&mut h, Mode::Points, &[Part::Point(corner)]);
        run(&mut h, "Delete the picked parts");
        let after = mesh_of(&h, id);
        if after == mesh {
            refused = true;
            assert!(status(&h).unwrap().starts_with("Put back") || status(&h).is_some());
            break;
        }
        assert!(after.face_count() >= 4 && whole(&after));
        assert_eq!(reopened(&h, id), after);
    }
    assert!(refused, "a brush keeps four faces");
}

#[test]
fn an_extrude_that_would_pass_through_the_brush_is_refused() {
    let (mut h, id) = editing();
    top_picked(&mut h);
    let before = steps(&h);
    run(&mut h, "Extrude");
    pull_to(&mut h, -2.5);
    keep(&mut h);
    assert_eq!(mesh_of(&h, id).face_count(), 6, "put back");
    assert_eq!(steps(&h), before);
    assert!(status(&h).unwrap().starts_with("Put back: it would"));
    // Clicking with nothing pulled keeps nothing, and says so.
    run(&mut h, "Extrude");
    keep(&mut h);
    assert_eq!(steps(&h), before);
    assert!(status(&h).unwrap().starts_with("Nothing extruded"));
}

#[test]
fn the_ribbon_greys_each_shaping_button_with_why() {
    use crate::shell::ShellCommand as C;
    let (mut h, _) = editing();
    let why = |h: &H, c: C| c.why_not(&h.state().0);
    assert_eq!(
        why(&h, C::BrushExtrude).as_deref(),
        Some("Pick faces to extrude first")
    );
    assert_eq!(
        why(&h, C::BrushInset).as_deref(),
        Some("Pick faces to inset first")
    );
    assert_eq!(
        why(&h, C::BrushSplit).as_deref(),
        Some("Pick part of the brush first")
    );
    assert_eq!(
        why(&h, C::BrushDelete).as_deref(),
        Some("Pick part of the brush first")
    );
    let corner = edit(&h)
        .parts
        .as_ref()
        .unwrap()
        .mesh()
        .vertices()
        .next()
        .unwrap()
        .0;
    pick(&mut h, Mode::Points, &[Part::Point(corner)]);
    assert!(
        why(&h, C::BrushExtrude)
            .unwrap()
            .starts_with("Points can't be extruded")
    );
    assert!(
        why(&h, C::BrushInset)
            .unwrap()
            .starts_with("Only faces can be inset")
    );
    assert_eq!(
        why(&h, C::BrushSplit).as_deref(),
        Some(mesh::split::NOT_ON_ONE_FACE)
    );
    assert_eq!(why(&h, C::BrushDelete), None);
    top_picked(&mut h);
    for c in [C::BrushExtrude, C::BrushInset, C::BrushSplit] {
        assert_eq!(why(&h, c), None, "{c:?}");
    }
    assert_eq!(
        why(&h, C::BrushDelete).as_deref(),
        Some(mesh::merge::LONE_FACE)
    );
    let side = face(&h, Vec3::X);
    let top = face(&h, Vec3::Z);
    pick(&mut h, Mode::Faces, &[Part::Face(top), Part::Face(side)]);
    assert_eq!(why(&h, C::BrushDelete), None, "two that touch merge");
    // Two points next to each other are joined already.
    let mesh = edit(&h).parts.as_ref().unwrap().mesh().clone();
    let (a, b) = mesh.edges()[0].ends();
    pick(&mut h, Mode::Points, &[Part::Point(a), Part::Point(b)]);
    assert_eq!(
        why(&h, C::BrushSplit).as_deref(),
        Some(mesh::split::ALREADY_JOINED)
    );
    top_picked(&mut h);
    run(&mut h, "Extrude");
    assert_eq!(
        why(&h, C::BrushSplit).as_deref(),
        Some(crate::brushing::shaping::PULLING)
    );
    assert_eq!(
        why(&h, C::BrushExtrude),
        None,
        "lit while in use, not greyed"
    );
    // On the ribbon: the four buttons, built, Extrude lit with its key.
    for name in ["Extrude", "Inset", "Split", "Delete the picked parts"] {
        assert!(h.query_by_label(name).is_some(), "{name}");
    }
    assert_eq!(crate::shell::ShellCommand::BrushExtrude.keys(), "E");
    // Not editing: each says to start editing.
    let w = Workbench::new(info());
    assert_eq!(
        C::BrushExtrude.why_not(&w).as_deref(),
        Some("Pick one brush and press Tab first")
    );
}

#[test]
fn delete_outside_editing_still_deletes_the_entity() {
    let mut w = Workbench::new(info());
    let id = w.spawn_selected(EntityData::new("Box_03").with(&Brush::default()));
    let mut h = rest_over(harness(w), Panel::Viewport);
    h.key_press(Key::Delete);
    h.run();
    assert!(!h.state().0.document().area().contains(id));
}

/// A box's lists with its first face left out (`turn`: turned round
/// instead).
fn box_data(drop_first: bool, turn: bool) -> reflect::ComponentData {
    let mut data = reflect::Component::to_data(&Brush::default());
    let Some(reflect::Value::List(faces)) = data.get("faces").cloned() else {
        panic!("faces")
    };
    let count = match faces[0] {
        reflect::Value::Int(n) => n as usize,
        _ => panic!("a count"),
    };
    let mut rest = faces[1 + count..].to_vec();
    if turn {
        let mut first = faces[1..1 + count].to_vec();
        first.reverse();
        let mut front = vec![faces[0].clone()];
        front.extend(first);
        front.extend(rest);
        rest = front;
    } else if !drop_first {
        rest = faces;
    }
    data.set("faces", reflect::Value::List(rest));
    data
}

#[test]
fn a_brush_read_open_or_inside_out_is_mended_and_one_that_cant_be_is_a_problem() {
    for data in [box_data(true, false), box_data(false, true)] {
        let mut w = Workbench::new(info());
        let mut entity = EntityData::new("Mended");
        entity
            .components
            .insert(<Brush as reflect::Component>::NAME.into(), data);
        let id = w.spawn_selected(entity);
        let mut h = harness(w);
        h.run();
        let w = &h.state().0;
        let model = w.open.brushes.get(id).unwrap();
        assert_eq!(model.mesh.face_count(), 6);
        assert!(!model.unwhole);
        assert!(whole(&model.mesh));
    }
    // Two neighbouring faces left out: can't be mended.
    let mut data = box_data(true, false);
    let Some(reflect::Value::List(faces)) = data.get("faces").cloned() else {
        panic!()
    };
    let mesh = Brush::default().mesh;
    let first = mesh.faces().next().unwrap().1.to_vec();
    let mut kept = Vec::new();
    let mut at = 0;
    let mut dropped = false;
    while at < faces.len() {
        let reflect::Value::Int(n) = faces[at] else {
            panic!()
        };
        let corners = &faces[at + 1..at + 1 + n as usize];
        let shares = corners.iter().any(
            |c| matches!(c, reflect::Value::Int(i) if first.iter().any(|v| v.get() as i64 == *i)),
        );
        if shares && !dropped {
            dropped = true;
        } else {
            kept.extend_from_slice(&faces[at..at + 1 + n as usize]);
        }
        at += 1 + n as usize;
    }
    data.set("faces", reflect::Value::List(kept));
    let mut w = Workbench::new(info());
    let mut entity = EntityData::new("Open");
    entity
        .components
        .insert(<Brush as reflect::Component>::NAME.into(), data);
    let id = w.spawn_selected(entity);
    let mut h = harness(w);
    h.run();
    h.run();
    let w = &h.state().0;
    assert!(w.open.brushes.get(id).unwrap().unwhole);
    let problem = w
        .open
        .problems
        .list
        .iter()
        .find(|p| p.kind == crate::problems::ProblemKind::OpenBrush)
        .expect("in Problems");
    assert!(problem.text.contains("has a hole"), "{}", problem.text);
}
