//! Editing a brush's insides in the whole editor (Tools 1a, B3), as a
//! person does it: Tab in and out; 1, 2 and 3; clicks, Ctrl + clicks,
//! boxes and clearing; the gizmo on what's picked, snapping on and off;
//! one undo step each, and an undo past the start ending it; a change
//! that would break the brush put back; Delete never deleting the brush; Made as after
//! edits; a size typed; the counts; the 2D view; a save and reopen; and
//! the picked brush's outline.

use super::*;
use crate::brushing::edit::{Mode, Part, PickHow};
use crate::gizmo::Change;
use crate::panel::Panel;
use crate::test_support::*;
use crate::viewport::Tool;
use crate::viewport_tools::{Outline, ViewportEvent};
use area::EntityData;
use area::builtin::Transform;
use egui::{Key, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use mesh::FaceId;

type H = egui_kittest::Harness<'static, State>;

/// An editor with one 2 m box brush, Box_03, standing at the middle of
/// the ground and picked, and a lamp beside it.
fn with_a_box() -> (Workbench, EntityId) {
    let mut w = Workbench::new(info());
    w.add_point_light();
    let id = w.spawn_selected(
        EntityData::new("Box_03")
            .with(&Transform::default())
            .with(&Brush::default()),
    );
    (w, id)
}

/// The box's editor drawn with the pointer over the 3D view, and Tab
/// pressed.
fn editing() -> (H, EntityId) {
    let (w, id) = with_a_box();
    let mut h = rest_over(harness(w), Panel::Viewport);
    h.key_press(Key::Tab);
    h.run();
    assert!(
        h.state().0.viewport.brushing.edit.is_some(),
        "Tab starts it"
    );
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

/// The box's face whose normal is `normal`.
fn face(h: &H, normal: Vec3) -> FaceId {
    let id = edit(h).entity;
    let mesh = &h.state().0.open.brushes.get(id).unwrap().mesh;
    mesh.faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).dot(normal) > 0.99)
        .unwrap()
}

/// The box's mesh now.
fn mesh_of(w: &Workbench, id: EntityId) -> mesh::Mesh {
    (*w.document()
        .area()
        .get(id)
        .unwrap()
        .get::<Brush>()
        .unwrap()
        .mesh)
        .clone()
}

/// The side of the box facing the camera most, as its normal.
fn side_facing(h: &H) -> Vec3 {
    let eye = h.state().0.viewport.camera.position;
    [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y]
        .into_iter()
        .max_by(|a, b| a.dot(eye).total_cmp(&b.dot(eye)))
        .unwrap()
}

/// The top face picked, and the Move tool's gizmo on it.
fn top_picked(h: &mut H) -> FaceId {
    let top = face(h, Vec3::Z);
    let over_top = on_screen(h, Vec3::new(0.0, 0.0, 2.0));
    click(h, over_top, egui::Modifiers::NONE);
    assert_eq!(edit(h).picked, [Part::Face(top)].into());
    h.state_mut().0.viewport.tool = Tool::Move;
    h.run();
    top
}

/// A drag of the gizmo as the viewport reports it: begun, changed to
/// `change` (`ctrl`: snapping turned around), let go.
fn gizmo_drag(h: &mut H, change: Change, ctrl: bool) {
    let w = &mut h.state_mut().0;
    w.viewport.tools.events = vec![
        ViewportEvent::GizmoStart { extrude: false },
        ViewportEvent::GizmoChange { change, flip: ctrl },
        ViewportEvent::GizmoEnd,
    ];
    w.tool_events();
    h.run();
}

#[test]
fn tab_starts_editing_with_the_brush_tab_in_front_and_tab_again_ends_it() {
    let (mut h, _) = editing();
    let w = &h.state().0;
    assert!(w.brush_tools_in_front());
    assert_eq!(edit(&h).mode, Mode::Faces);
    assert_eq!(
        status(&h).as_deref(),
        Some("Editing Box_03 \u{b7} Faces \u{b7} nothing picked")
    );
    assert!(h.query_by_label("Home tools \u{b7} Brush").is_some());
    assert!(
        h.query_by_label("Stop editing the brush's insides")
            .is_some(),
        "the ribbon's Edit group"
    );
    assert!(h.query_by_label("Extrude").is_some(), "the Shape group");
    h.key_press(Key::Tab);
    h.run();
    let w = &h.state().0;
    assert!(w.viewport.brushing.edit.is_none());
    assert!(!w.brush_tools_in_front());
    assert_eq!(w.ribbon_tab(), crate::shell::RibbonTab::Home);
    assert!(h.query_by_label("Home tools \u{b7} Brush").is_none());
    assert_eq!(status(&h), None);
    // Done on the ribbon ends it too.
    h.key_press(Key::Tab);
    h.run();
    h.get_by_label("Stop editing the brush's insides").click();
    h.run();
    assert!(h.state().0.viewport.brushing.edit.is_none());
}

#[test]
fn tab_with_no_brush_or_two_things_picked_does_nothing() {
    let (mut w, _) = with_a_box();
    w.select_all();
    let mut h = rest_over(harness(w), Panel::Viewport);
    h.key_press(Key::Tab);
    h.run();
    assert!(h.state().0.viewport.brushing.edit.is_none(), "two picked");
    let mut w = Workbench::new(info());
    w.add_point_light();
    let mut h = rest_over(harness(w), Panel::Viewport);
    h.key_press(Key::Tab);
    h.run();
    assert!(h.state().0.viewport.brushing.edit.is_none(), "a lamp");
}

#[test]
fn with_a_tile_map_picked_tab_stays_the_map_s_meaning() {
    let (mut w, map, _) = crate::workbench_tiles::tests::with_map();
    w.place_tile_map(map, None).unwrap();
    let mut h = rest_over(harness(w), Panel::Viewport);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    let w = &h.state().0;
    assert!(w.viewport.tile_overlay.paint.meaning, "the map's Tab");
    assert!(w.viewport.brushing.edit.is_none());
}

#[test]
fn one_two_and_three_choose_points_edges_and_faces_keeping_the_pick() {
    let (mut h, _) = editing();
    let top = top_picked(&mut h);
    h.key_press(Key::Num1);
    h.run();
    assert_eq!(edit(&h).mode, Mode::Points);
    assert_eq!(edit(&h).picked.len(), 4, "the top's corners");
    assert!(
        status(&h)
            .unwrap()
            .ends_with("Points \u{b7} 4 points picked")
    );
    h.key_press(Key::Num2);
    h.run();
    assert_eq!(edit(&h).mode, Mode::Edges);
    assert_eq!(edit(&h).picked.len(), 4, "the top's sides");
    h.key_press(Key::Num3);
    h.run();
    assert_eq!(edit(&h).picked, [Part::Face(top)].into());
    // Not editing, 1, 2 and 3 do nothing to the brush.
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::Num1);
    h.run();
    assert!(h.state().0.viewport.brushing.edit.is_none());
}

#[test]
fn a_click_picks_ctrl_click_adds_or_takes_away_a_box_picks_and_empty_space_clears() {
    let (mut h, _) = editing();
    let top = face(&h, Vec3::Z);
    let side_normal = side_facing(&h);
    let side = face(&h, side_normal);
    // Lit before the click.
    let over_top = on_screen(&h, Vec3::new(0.0, 0.0, 2.0));
    h.hover_at(over_top);
    h.run();
    assert_eq!(edit(&h).hovered, Some(Part::Face(top)));
    click(&mut h, over_top, egui::Modifiers::NONE);
    assert_eq!(edit(&h).picked, [Part::Face(top)].into());
    let over_side = on_screen(&h, side_normal + Vec3::Z);
    click(&mut h, over_side, egui::Modifiers::COMMAND);
    assert_eq!(edit(&h).picked, [Part::Face(top), Part::Face(side)].into());
    click(&mut h, over_top, egui::Modifiers::COMMAND);
    assert_eq!(edit(&h).picked, [Part::Face(side)].into());
    // A click on empty space clears the pick, and picks no other entity.
    let rect = h.state().0.viewport.rect;
    let empty = rect.left_top() + egui::vec2(60.0, rect.height() - 90.0);
    click(&mut h, empty, egui::Modifiers::NONE);
    assert!(edit(&h).picked.is_empty());
    let w = &h.state().0;
    assert_eq!(w.selection(), Some(edit(&h).entity), "still the brush");
    // A box round the whole brush picks all six faces, through it.
    let (a, b) = (
        on_screen(&h, Vec3::new(-1.0, -1.0, 0.0)),
        on_screen(&h, Vec3::new(1.0, 1.0, 2.0)),
    );
    let corners = [a, b, on_screen(&h, Vec3::new(1.0, -1.0, 0.0))];
    let lo = corners.iter().fold(a, |m, p| m.min(*p)) - egui::vec2(40.0, 40.0);
    let hi = corners.iter().fold(b, |m, p| m.max(*p)) + egui::vec2(40.0, 40.0);
    drag_for_frames(&mut h, lo, (hi - lo) / 4.0, 4);
    assert_eq!(edit(&h).picked.len(), 6);
    assert!(status(&h).unwrap().ends_with("6 faces picked"));
}

#[test]
fn a_real_drag_on_the_move_gizmo_raises_the_top_as_one_undo_step() {
    let (mut h, id) = editing();
    top_picked(&mut h);
    let steps = h.state().0.document().history().undo_len();
    let w = &h.state().0;
    let gizmo = w.viewport.tools.gizmo.expect("a gizmo on the face");
    assert!(
        (gizmo.origin - Vec3::new(0.0, 0.0, 2.0)).length() < 1e-4,
        "its middle"
    );
    let rect = w.viewport.rect;
    let size = glam::Vec2::new(rect.width(), rect.height());
    let (_, shape) = gizmo
        .shapes(&w.viewport.camera, size, [false; 3])
        .into_iter()
        .find(|(handle, _)| *handle == crate::gizmo::Handle::Axis(2))
        .expect("the Z arrow");
    let crate::gizmo::Shape::Line { a, b, .. } = shape else {
        panic!("a line");
    };
    let grab = rect.min + egui::vec2((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    drag_for_frames(&mut h, grab, egui::vec2(0.0, -10.0), 4);
    let w = &h.state().0;
    let height = mesh_of(w, id).bounds().unwrap().size().z;
    assert!(height > 2.0, "raised: {height}");
    assert_eq!(w.document().history().undo_len(), steps + 1, "one step");
    assert_eq!(w.document().history().undo_label(), Some("Move 1 face"));
    assert!(edit(&h).drag.is_none() && edit(&h).preview.is_none());
}

#[test]
fn each_gizmo_works_on_points_edges_and_faces_with_one_undo_step_each() {
    let (mut h, id) = editing();
    top_picked(&mut h);
    let steps = |h: &H| h.state().0.document().history().undo_len();
    let before = steps(&h);
    gizmo_drag(&mut h, Change::Translate(Vec3::Z), false);
    assert_eq!(mesh_of(&h.state().0, id).bounds().unwrap().size().z, 3.0);
    assert_eq!(steps(&h), before + 1);
    gizmo_drag(&mut h, Change::Scale(Vec3::new(2.0, 1.0, 1.0)), false);
    assert_eq!(mesh_of(&h.state().0, id).bounds().unwrap().size().x, 4.0);
    assert_eq!(steps(&h), before + 2);
    h.key_press(Key::Num2);
    h.run();
    gizmo_drag(
        &mut h,
        Change::Rotate {
            axis: Vec3::Z,
            angle: 0.25,
        },
        false,
    );
    assert_eq!(steps(&h), before + 3);
    assert!(
        h.state()
            .0
            .document()
            .history()
            .undo_label()
            .unwrap()
            .starts_with("Rotate 4 edges")
    );
    h.key_press(Key::Num1);
    h.run();
    gizmo_drag(&mut h, Change::Translate(Vec3::X * 0.5), false);
    assert_eq!(steps(&h), before + 4);
    // Undo and redo go through the brush's steps, still editing.
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::Z);
    h.run();
    assert_eq!(steps(&h), before + 3);
    assert!(h.state().0.viewport.brushing.edit.is_some());
}

#[test]
fn snapping_follows_the_grid_and_ctrl_turns_it_off_for_one_drag() {
    let (mut h, id) = editing();
    top_picked(&mut h);
    let snapping = &mut h.state_mut().0.viewport.snapping;
    snapping.grid = true;
    snapping.grid_step = 0.5;
    gizmo_drag(&mut h, Change::Translate(Vec3::Z * 0.3), false);
    assert_eq!(mesh_of(&h.state().0, id).bounds().unwrap().max.z, 2.5);
    gizmo_drag(&mut h, Change::Translate(Vec3::Z * 0.3), true);
    let top = mesh_of(&h.state().0, id).bounds().unwrap().max.z;
    assert!((top - 2.8).abs() < 1e-4, "{top}");
}

#[test]
fn local_axes_are_the_brush_s_own() {
    let (mut h, id) = editing();
    let turn = glam::Quat::from_rotation_z(0.5);
    let w = &mut h.state_mut().0;
    let edits = crate::commands::component_edits(
        id,
        w.open
            .document
            .area()
            .get(id)
            .unwrap()
            .components
            .get(Transform::NAME),
        &Transform {
            rotation: turn,
            ..Transform::default()
        },
    );
    w.command("Turn", edits, None, None);
    w.viewport.snapping.object_axes = true;
    h.run();
    top_picked(&mut h);
    let gizmo = h.state().0.viewport.tools.gizmo.unwrap();
    assert!((gizmo.axes[0] - turn * Vec3::X).length() < 1e-4);
}

#[test]
fn an_undo_back_past_the_start_of_editing_ends_it() {
    let (mut h, id) = editing();
    top_picked(&mut h);
    gizmo_drag(&mut h, Change::Translate(Vec3::Z), false);
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::Z);
    h.run();
    assert!(
        h.state().0.viewport.brushing.edit.is_some(),
        "back to the start"
    );
    assert_eq!(mesh_of(&h.state().0, id).bounds().unwrap().size().z, 2.0);
    // The step before editing began: the brush made.
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::Z);
    h.run();
    assert!(h.state().0.viewport.brushing.edit.is_none());
}

#[test]
fn picking_something_else_in_the_area_list_ends_editing_and_picks_it() {
    let (mut h, _) = editing();
    let lamp = h
        .state()
        .0
        .document()
        .area()
        .iter()
        .find(|(_, e)| e.name.starts_with("Point light"))
        .map(|(id, _)| id)
        .unwrap();
    h.state_mut().0.select(lamp);
    h.run();
    let w = &h.state().0;
    assert!(w.viewport.brushing.edit.is_none());
    assert_eq!(w.selection(), Some(lamp));
}

#[test]
fn a_change_that_would_break_the_brush_snaps_back_and_says_why() {
    let (mut h, id) = editing();
    top_picked(&mut h);
    let steps = h.state().0.document().history().undo_len();
    gizmo_drag(&mut h, Change::Translate(-Vec3::Z * 3.0), false);
    let w = &h.state().0;
    assert_eq!(mesh_of(w, id).bounds().unwrap().size().z, 2.0, "put back");
    assert_eq!(w.document().history().undo_len(), steps, "no step");
    assert_eq!(
        status(&h).as_deref(),
        Some("Put back: it would fold a face over itself")
    );
    assert!(edit(&h).preview.is_none());
}

#[test]
fn a_bent_face_is_allowed() {
    let (mut h, id) = editing();
    top_picked(&mut h);
    h.key_press(Key::Num1);
    h.run();
    let corner = *edit(&h).picked.iter().next().unwrap();
    h.state_mut().0.pick_parts(&[corner], PickHow::Replace);
    gizmo_drag(&mut h, Change::Translate(Vec3::Z * 0.7), false);
    let Part::Point(v) = corner else { panic!() };
    assert!((mesh_of(&h.state().0, id).position(v).unwrap().z - 2.7).abs() < 1e-4);
}

#[test]
fn delete_and_backspace_while_editing_never_delete_the_brush() {
    // They delete the picked parts (B4): a lone face is refused, as it
    // would leave a hole.
    let (mut h, id) = editing();
    top_picked(&mut h);
    for key in [Key::Delete, Key::Backspace] {
        h.key_press(key);
        h.run();
        let w = &h.state().0;
        assert!(w.document().area().contains(id));
        assert_eq!(status(&h).as_deref(), Some(mesh::merge::LONE_FACE));
    }
    // The search's Delete neither.
    run(&mut h, "Delete");
    assert!(h.state().0.document().area().contains(id));
}

#[test]
fn made_as_after_edits_replaces_them_and_says_so_and_undo_brings_them_back() {
    let (mut h, id) = editing();
    top_picked(&mut h);
    // The top slid sideways: no longer a plain box.
    gizmo_drag(&mut h, Change::Translate(Vec3::X * 0.5), false);
    let edited = mesh_of(&h.state().0, id);
    let w = &mut h.state_mut().0;
    let gesture = w.gesture;
    w.apply_property(
        crate::properties::PropertyRequest::SetField {
            entity: id,
            component: Brush::NAME.into(),
            field: "shape".into(),
            value: reflect::Value::Text("Ramp".into()),
        },
        gesture,
    );
    h.run();
    let w = &h.state().0;
    let brush = w.document().area().get(id).unwrap().get::<Brush>().unwrap();
    assert_eq!(brush.shape, mesh::Shape::Ramp, "Made as names the shape");
    assert_eq!(
        brush.mesh.bounds().unwrap().size(),
        Vec3::new(2.5, 2.0, 2.0),
        "at its size"
    );
    assert!(status(&h).unwrap().contains("replaced the edits"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::Z);
    h.run();
    assert_eq!(mesh_of(&h.state().0, id), edited);
    assert!(h.state().0.viewport.brushing.edit.is_some());
}

#[test]
fn a_size_typed_stretches_the_brush_about_its_middle_as_one_step() {
    let (mut h, id) = editing();
    let w = &mut h.state_mut().0;
    let steps = w.document().history().undo_len();
    w.open.document.close_step();
    w.apply_property(
        crate::properties::PropertyRequest::BrushSize {
            entity: id,
            size: Vec3::new(4.0, 2.5, 3.0),
        },
        77,
    );
    h.run();
    let w = &h.state().0;
    let bounds = mesh_of(w, id).bounds().unwrap();
    assert_eq!(bounds.size(), Vec3::new(4.0, 2.5, 3.0));
    assert_eq!(bounds.centre(), Vec3::new(0.0, 0.0, 1.0));
    assert_eq!(w.document().history().undo_len(), steps + 1);
    let facts = w.brush_facts().unwrap();
    assert_eq!(facts.size, Vec3::new(4.0, 2.5, 3.0));
}

#[test]
fn the_brush_card_counts_points_edges_and_faces_and_says_what_is_picked() {
    let (mut h, _) = editing();
    let facts = h.state().0.brush_facts().unwrap();
    assert_eq!(facts.counts, [8, 12, 6]);
    assert_eq!(facts.editing, Some((Mode::Faces, "nothing".to_owned())));
    top_picked(&mut h);
    let facts = h.state().0.brush_facts().unwrap();
    assert_eq!(facts.editing, Some((Mode::Faces, "1 face".to_owned())));
    h.state_mut().0.show_panel(Panel::Properties);
    h.run();
    assert!(h.query_by_label_contains("Faces mode").is_some());
    assert!(h.query_by_label("Size").is_some());
}

#[test]
fn the_2d_view_edits_too_in_its_plane() {
    let (mut h, id) = editing();
    let w = &mut h.state_mut().0;
    w.viewport.camera = w.viewport.camera.as_kind(crate::camera::ViewKind::TwoD);
    h.run();
    h.run();
    let top = face(&h, Vec3::Z);
    let over_top = on_screen(&h, Vec3::new(0.0, 0.0, 2.0));
    click(&mut h, over_top, egui::Modifiers::NONE);
    assert_eq!(edit(&h).picked, [Part::Face(top)].into());
    h.state_mut().0.viewport.snapping.pixel_snap = crate::transforming::PixelSnap::Off;
    gizmo_drag(&mut h, Change::Translate(Vec3::X * 0.5), false);
    assert_eq!(mesh_of(&h.state().0, id).bounds().unwrap().max.x, 1.5);
}

#[test]
fn a_save_and_reopen_after_edits_gives_the_same_brush() {
    let (mut h, id) = editing();
    top_picked(&mut h);
    gizmo_drag(&mut h, Change::Translate(Vec3::new(0.3, 0.0, 1.0)), false);
    let w = &h.state().0;
    let registry = area::builtin::registry().unwrap();
    let text = area::text::write(w.document().area(), &registry, &|_| None);
    let read = area::text::read(&text, &registry).unwrap();
    let again = read.get(id).unwrap().get::<Brush>().unwrap();
    let now = w.document().area().get(id).unwrap().get::<Brush>().unwrap();
    assert_eq!(*again.mesh, *now.mesh);
    assert!(!again.damaged);
}

#[test]
fn a_picked_brush_is_outlined_by_its_own_edges() {
    let (w, id) = with_a_box();
    let mut h = harness(w);
    h.run();
    let outlines = h.state().0.viewport.tools.outlines.clone();
    let lines = outlines
        .iter()
        .find_map(|o| match o {
            Outline::Lines { lines, .. } => Some(lines.len()),
            _ => None,
        })
        .expect("its edges");
    assert_eq!(lines, 12);
    assert!(!outlines.iter().any(|o| matches!(o, Outline::Point(_))));
    // While its insides are edited it draws its own edges instead.
    h.state_mut().0.toggle_brush_editing();
    h.run();
    let w = &h.state().0;
    assert!(
        w.viewport
            .brushing
            .edit
            .as_ref()
            .is_some_and(|e| e.entity == id)
    );
    assert!(w.viewport.tools.outlines.is_empty());
}

#[test]
fn the_other_entities_are_dimmed_while_editing() {
    let (h, id) = editing();
    let w = &h.state().0;
    let pick = w
        .viewport
        .area
        .brushes
        .iter()
        .find(|b| b.entity == id)
        .unwrap()
        .pick;
    let view = w.viewport.view([800, 600], 1.0);
    assert_eq!(view.dim_all_but, Some(pick));
}

#[test]
fn the_brush_tab_s_keys_and_names_do_the_same_and_each_is_on_its_ribbon() {
    use crate::shell::ShellCommand as C;
    // Tab, and the command's name, start editing.
    let (w, _) = with_a_box();
    let mut by_name = rest_over(harness(w), Panel::Viewport);
    run(&mut by_name, "Edit the brush's insides");
    assert!(by_name.state().0.viewport.brushing.edit.is_some());
    let (mut by_key, _) = editing();
    for mode in Mode::ALL {
        by_key.key_press(mode.key());
        by_key.run();
        let name = C::BrushMode(mode).name(&by_name.state().0);
        run(&mut by_name, &name);
        assert_eq!(edit(&by_key).mode, mode);
        assert_eq!(edit(&by_name).mode, mode);
        // Each mode is a button on HOME TOOLS · Brush, lit while in use.
        by_name.get_by_label(&name).click();
        by_name.run();
        assert_eq!(edit(&by_name).mode, mode);
    }
    // Solid is a switch on the ribbon, one undo step.
    let id = edit(&by_name).entity;
    // On the ribbon, above the Brush card's own Solid.
    let ribbon_s = by_name
        .query_all_by_label("Solid")
        .map(|n| n.rect())
        .min_by(|a, b| a.top().total_cmp(&b.top()))
        .unwrap();
    click(&mut by_name, ribbon_s.center(), egui::Modifiers::NONE);
    by_name.run();
    let w = &by_name.state().0;
    assert!(
        !w.document()
            .area()
            .get(id)
            .unwrap()
            .get::<Brush>()
            .unwrap()
            .solid
    );
    // Snapping's grid is there too.
    by_name
        .query_by_label_contains("Grid \u{b7}")
        .unwrap()
        .click();
    by_name.run();
    assert!(by_name.state().0.viewport.snapping.grid);
    // Not editing, the brush's commands say why not.
    let w = Workbench::new(info());
    assert_eq!(
        C::EditBrushInsides.why_not(&w).as_deref(),
        Some("Pick one shape brush first")
    );
    assert!(C::BrushMode(Mode::Points).why_not(&w).is_some());
}
