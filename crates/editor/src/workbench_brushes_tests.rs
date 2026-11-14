//! Making shape brushes in the whole editor (Tools 1a, B2), as a person
//! does it: B over the view, B again, Space, Esc; a click on a row or
//! elsewhere; the Shape button; drawing out with the mouse, given up with
//! Esc or a right-click; one undo step; names counting up; the tile map's
//! B; Made as; and the Problems list naming a damaged brush.

use super::*;
use crate::test_support::*;
use crate::workbench_tiles::tests::with_map;
use area::builtin::Brush;
use egui::{Key, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use reflect::{Component, Value};

type H = egui_kittest::Harness<'static, State>;

/// The editor with the pointer resting over the 3D view.
fn over_the_view() -> H {
    rest_over(harness(Workbench::new(info())), Panel::Viewport)
}

fn list_shape(h: &H) -> Option<Shape> {
    h.state().0.viewport.brushing.list.map(|l| l.chosen())
}

fn status(h: &H) -> Option<String> {
    h.state().0.status().brushing
}

#[test]
fn b_opens_the_list_each_b_moves_down_and_space_takes_the_shape() {
    let mut h = over_the_view();
    h.key_press(Key::B);
    h.run();
    assert_eq!(list_shape(&h), Some(Shape::Box), "open on the first");
    assert!(h.query_by_label("Ramp").is_some(), "the list is drawn");
    h.key_press(Key::B);
    h.run();
    assert_eq!(list_shape(&h), Some(Shape::Ramp));
    assert_eq!(status(&h).as_deref(), Some("Choosing a shape brush: Ramp"));
    // Round the end and back to the first.
    for _ in 0..Shape::ALL.len() {
        h.key_press(Key::B);
        h.run();
    }
    assert_eq!(list_shape(&h), Some(Shape::Ramp), "wrapped round");
    h.key_press(Key::Space);
    h.run();
    let w = &h.state().0;
    assert!(w.viewport.brushing.list.is_none());
    assert_eq!(
        w.viewport.brushing.draw.as_ref().map(|d| d.shape),
        Some(Shape::Ramp)
    );
    assert!(!w.search.open, "the list's Space comes before the search's");
    assert!(status(&h).unwrap().starts_with("Drawing a Ramp"));
    assert!(w.hints()[0].starts_with("Drawing out a brush"));
}

#[test]
fn esc_closes_the_list_leaving_the_selection_and_space_is_the_search_s_again() {
    let mut w = Workbench::new(info());
    let lamp = w.add_point_light();
    let mut h = rest_over(harness(w), Panel::Viewport);
    h.key_press(Key::B);
    h.run();
    assert!(list_shape(&h).is_some());
    assert!(h.state().0.hints()[0].starts_with("Shape brushes are blocks"));
    h.key_press(Key::Escape);
    h.run();
    let w = &h.state().0;
    assert!(!w.viewport.brushing.is_active());
    assert_eq!(w.selection(), Some(lamp), "Esc did one thing");
    h.key_press(Key::Space);
    h.run();
    assert!(h.state().0.search.open);
}

#[test]
fn b_away_from_the_view_or_while_typing_opens_nothing() {
    let mut h = rest_over(harness(Workbench::new(info())), Panel::Properties);
    h.key_press(Key::B);
    h.run();
    assert_eq!(list_shape(&h), None, "over Properties");
}

#[test]
fn with_a_tile_map_picked_b_stays_the_tile_brush() {
    let (mut w, map, _) = with_map();
    w.place_tile_map(map, None).unwrap();
    let mut h = rest_over(harness(w), Panel::Viewport);
    h.run();
    assert!(h.state().0.viewport.tile_overlay.paint.allowed);
    h.key_press(Key::B);
    h.run();
    let w = &h.state().0;
    assert_eq!(list_shape(&h), None);
    assert_eq!(
        w.viewport.tile_overlay.paint.tool,
        Some(crate::tile_paint::PaintTool::Brush)
    );
}

#[test]
fn a_click_on_a_row_takes_it_and_a_click_elsewhere_closes_the_list() {
    let mut h = over_the_view();
    h.key_press(Key::B);
    h.run();
    h.get_by_label("Stairs").click();
    h.run();
    let w = &h.state().0;
    assert!(w.viewport.brushing.list.is_none());
    assert_eq!(
        w.viewport.brushing.draw.as_ref().map(|d| d.shape),
        Some(Shape::Stairs)
    );
    // Again, and a click on the view away from it.
    h.state_mut().0.viewport.brushing = Default::default();
    h.key_press(Key::B);
    h.run();
    let far = h.state().0.viewport.rect.left_top() + egui::vec2(60.0, 120.0);
    click(&mut h, far, egui::Modifiers::NONE);
    let w = &h.state().0;
    assert!(!w.viewport.brushing.is_active(), "closed, nothing taken");
    assert_eq!(
        w.document().area().len(),
        Workbench::new(info()).document().area().len()
    );
}

#[test]
fn the_shape_button_opens_the_same_list_and_is_lit_while_it_is_open() {
    let mut h = harness(Workbench::new(info()));
    h.run();
    h.run();
    let button = h.get_by_label("Choose a shape brush").rect();
    click(&mut h, button.center(), egui::Modifiers::NONE);
    assert_eq!(list_shape(&h), Some(Shape::Box), "opened, still open");
    // B moves down it wherever the pointer is now.
    h.key_press(Key::B);
    h.run();
    assert_eq!(list_shape(&h), Some(Shape::Ramp));
    // Create › More has no Box brush any more.
    h.key_press(Key::Escape);
    h.run();
    h.get_by_label("More").click();
    h.run();
    assert!(h.query_by_label("Box brush").is_none());
}

/// Where world point `p` is on screen in the view in use.
fn on_screen(h: &H, p: Vec3) -> Pos2 {
    let w = &h.state().0;
    let rect = w.viewport.rect;
    let seen = w.viewport.camera;
    let at = seen
        .project(p, glam::Vec2::new(rect.width(), rect.height()))
        .expect("in view");
    rect.min + egui::vec2(at.x, at.y)
}

fn button(h: &H, pressed: bool, at: Pos2, button: egui::PointerButton) {
    h.event(egui::Event::PointerButton {
        pos: at,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    });
}

/// Takes `shape` from the list and drags its footprint from `a` to `b`
/// on the ground, letting go there.
fn draw_footprint(h: &mut H, shape: Shape, (a, b): (Vec3, Vec3)) {
    h.state_mut().0.take_brush(shape);
    h.run();
    let (from, to) = (on_screen(h, a), on_screen(h, b));
    h.hover_at(from);
    h.run();
    button(h, true, from, egui::PointerButton::Primary);
    h.run();
    for i in 1..=3 {
        h.hover_at(from + (to - from) * i as f32 / 3.0);
        h.run();
    }
    button(h, false, to, egui::PointerButton::Primary);
    h.run();
}

/// Moves up the screen by `up` points and clicks there.
fn pull_up_and_click(h: &mut H, from: Pos2, up: f32) {
    let top = from - egui::vec2(0.0, up);
    h.hover_at(top);
    h.run();
    button(h, true, top, egui::PointerButton::Primary);
    h.run();
    button(h, false, top, egui::PointerButton::Primary);
    h.run();
}

fn brushes(w: &Workbench) -> Vec<(String, Brush)> {
    let area = w.document().area();
    let mut found: Vec<(String, Brush)> = area
        .with::<Brush>()
        .map(|(id, b)| (area.get(id).unwrap().name.clone(), b))
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

#[test]
fn a_brush_drawn_out_is_one_undo_step_named_after_its_shape_and_picked() {
    let mut h = over_the_view();
    let corner = Vec3::new(4.0, 3.0, 0.0);
    draw_footprint(&mut h, Shape::Box, (Vec3::ZERO, corner));
    assert!(status(&h).unwrap().contains("height 0.00 m"));
    let undo_before = h.state().0.document().history().undo_len();
    let at = on_screen(&h, corner);
    pull_up_and_click(&mut h, at, 120.0);
    let w = &h.state().0;
    assert!(!w.viewport.brushing.is_active(), "done");
    let made = brushes(w);
    assert_eq!(made.len(), 1);
    let (name, brush) = &made[0];
    assert_eq!(name, "Box_01");
    assert_eq!(brush.shape, Shape::Box);
    let size = brush.mesh.bounds().unwrap().size();
    assert_eq!((size.x, size.y), (4.0, 3.0));
    assert!(
        size.z > 0.0 && size.z % 1.0 == 0.0,
        "on the grid's step: {size}"
    );
    let id = w.selection().expect("picked");
    assert_eq!(w.document().area().get(id).unwrap().name, "Box_01");
    let transform = w
        .document()
        .area()
        .get(id)
        .unwrap()
        .get::<area::builtin::Transform>()
        .unwrap();
    assert_eq!(transform.position, Vec3::new(2.0, 1.5, 0.0));
    assert_eq!(w.document().history().undo_len(), undo_before + 1);
    // A second: the name counts up. A Ramp starts its own count.
    draw_footprint(
        &mut h,
        Shape::Box,
        (Vec3::new(8.0, 0.0, 0.0), Vec3::new(10.0, 2.0, 0.0)),
    );
    let at = on_screen(&h, Vec3::new(10.0, 2.0, 0.0));
    pull_up_and_click(&mut h, at, 80.0);
    draw_footprint(
        &mut h,
        Shape::Ramp,
        (Vec3::new(-6.0, 0.0, 0.0), Vec3::new(-4.0, 4.0, 0.0)),
    );
    let at = on_screen(&h, Vec3::new(-4.0, 4.0, 0.0));
    pull_up_and_click(&mut h, at, 80.0);
    let names: Vec<String> = brushes(&h.state().0).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["Box_01", "Box_02", "Ramp_01"]);
    // Undo takes the last away whole.
    h.state_mut().0.undo();
    let names: Vec<String> = brushes(&h.state().0).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["Box_01", "Box_02"]);
}

#[test]
fn a_brush_drawn_on_top_of_another_stands_on_it() {
    let mut h = over_the_view();
    draw_footprint(&mut h, Shape::Box, (Vec3::ZERO, Vec3::new(4.0, 4.0, 0.0)));
    let at = on_screen(&h, Vec3::new(4.0, 4.0, 0.0));
    pull_up_and_click(&mut h, at, 30.0);
    let top = brushes(&h.state().0)[0].1.mesh.bounds().unwrap().max.z;
    assert!(top >= 1.0, "{top}");
    h.run();
    // Drawn from a point on its top.
    let (a, b) = (Vec3::new(1.0, 1.0, top), Vec3::new(3.0, 3.0, top));
    draw_footprint(&mut h, Shape::Box, (a, b));
    let at = on_screen(&h, b);
    // (Not so far up that the click lands on the steps over the view.)
    pull_up_and_click(&mut h, at, 30.0);
    let w = &h.state().0;
    let id = w.selection().unwrap();
    let placed = w
        .document()
        .area()
        .get(id)
        .unwrap()
        .get::<area::builtin::Transform>()
        .unwrap();
    assert_eq!(w.document().area().get(id).unwrap().name, "Box_02");
    assert!(
        (placed.position.z - top).abs() < 1e-4,
        "{} on {top}",
        placed.position
    );
}

#[test]
fn esc_or_a_right_click_gives_a_drawing_up_leaving_nothing() {
    for give_up in ["Esc", "right-click"] {
        let mut h = over_the_view();
        let before = h.state().0.document().history().undo_len();
        let corner = Vec3::new(3.0, 3.0, 0.0);
        draw_footprint(&mut h, Shape::Wedge, (Vec3::ZERO, corner));
        let at = on_screen(&h, corner) - egui::vec2(0.0, 50.0);
        h.hover_at(at);
        h.run();
        if give_up == "Esc" {
            h.key_press(Key::Escape);
        } else {
            button(&h, true, at, egui::PointerButton::Secondary);
            h.run();
            button(&h, false, at, egui::PointerButton::Secondary);
        }
        h.run();
        let w = &h.state().0;
        assert!(!w.viewport.brushing.is_active(), "{give_up}");
        assert!(brushes(w).is_empty(), "{give_up}");
        assert_eq!(w.document().history().undo_len(), before, "{give_up}");
        assert!(!w.viewport.is_flying(), "{give_up}: the camera stayed");
    }
}

#[test]
fn a_click_without_a_drag_makes_nothing_and_the_status_bar_says_why() {
    let mut h = over_the_view();
    h.state_mut().0.take_brush(Shape::Box);
    h.run();
    let spot = on_screen(&h, Vec3::new(1.0, 1.0, 0.0));
    click(&mut h, spot, egui::Modifiers::NONE);
    assert!(brushes(&h.state().0).is_empty());
    assert!(status(&h).unwrap().contains("the footprint had no size"));
}

#[test]
fn in_the_2d_view_letting_go_makes_the_brush() {
    let mut w = Workbench::new(info());
    w.viewport.camera = w.viewport.camera.as_kind(crate::camera::ViewKind::TwoD);
    let mut h = rest_over(harness(w), Panel::Viewport);
    h.state_mut().0.take_brush(Shape::Box);
    h.run();
    assert!(h.state().0.hints()[0].contains("one grid step deep"));
    // (Clear of the keys over the view's foot.)
    let (a, b) = (Vec3::new(-2.0, 0.0, 0.0), Vec3::new(1.0, 2.0, 0.0));
    let (from, to) = (on_screen(&h, a), on_screen(&h, b));
    h.hover_at(from);
    h.run();
    button(&h, true, from, egui::PointerButton::Primary);
    h.run();
    h.hover_at(to);
    h.run();
    button(&h, false, to, egui::PointerButton::Primary);
    h.run();
    let made = brushes(&h.state().0);
    assert_eq!(made.len(), 1, "no height step");
    let size = made[0].1.mesh.bounds().unwrap().size();
    let step = h.state().0.viewport.snapping.grid_step;
    assert_eq!(size, Vec3::new(3.0, 2.0, step));
}

#[test]
fn made_as_makes_the_mesh_again_at_the_brush_s_size_as_one_undo_step() {
    let mut h = over_the_view();
    draw_footprint(&mut h, Shape::Box, (Vec3::ZERO, Vec3::new(4.0, 2.0, 0.0)));
    let at = on_screen(&h, Vec3::new(4.0, 2.0, 0.0));
    pull_up_and_click(&mut h, at, 120.0);
    let w = &mut h.state_mut().0;
    let id = w.selection().unwrap();
    let before = brushes(w)[0].1.clone();
    let undo_before = w.document().history().undo_len();
    w.apply_property(
        crate::properties::PropertyRequest::SetField {
            entity: id,
            component: Brush::NAME.into(),
            field: "shape".into(),
            value: Value::Text("Cylinder".into()),
        },
        w.gesture + 1,
    );
    let after = brushes(w)[0].1.clone();
    assert_eq!(after.shape, Shape::Cylinder);
    assert_ne!(after.mesh, before.mesh, "made again");
    let (was, now) = (before.mesh.bounds().unwrap(), after.mesh.bounds().unwrap());
    assert!(was.min.distance(now.min) < 1e-4 && was.max.distance(now.max) < 1e-4);
    // 16 sides, two ends.
    assert_eq!(after.mesh.face_count(), 18);
    assert_eq!(w.document().history().undo_len(), undo_before + 1);
    w.undo();
    assert_eq!(brushes(w)[0].1, before, "one step back");
    // A name that is no shape sets the field alone, as any other.
    assert!(w.made_as_edits(id, &Value::Text("Teapot".into())).is_none());
}

#[test]
fn a_damaged_brush_is_named_in_the_problems_list_with_why() {
    let mut w = Workbench::new(info());
    let mut data = Brush::default().to_data();
    // A face naming a point that isn't there.
    data.set(
        "faces",
        Value::List(vec![
            Value::Int(3),
            Value::Int(0),
            Value::Int(1),
            Value::Int(99),
        ]),
    );
    let mut area = area::Area::new();
    let mut entity = area::EntityData::new("Broken");
    entity.components.insert(Brush::NAME.into(), data);
    area.spawn(entity);
    w.open_area(area, std::path::PathBuf::from("broken.solarea"));
    let mut h = harness(w);
    h.run();
    h.run();
    let problems = &h.state().0.open.problems.list;
    let found = problems
        .iter()
        .find(|p| p.kind == crate::problems::ProblemKind::DamagedBrush)
        .expect("named");
    assert!(
        found.text.starts_with("Broken's brush shows nothing"),
        "{}",
        found.text
    );
    assert!(
        found.text.contains("a face names a point that isn't there"),
        "{}",
        found.text
    );
    // Drawn again whole: the problem goes.
    let id = found.entity;
    let w = &mut h.state_mut().0;
    let gesture = w.gesture + 1;
    w.apply_property(
        crate::properties::PropertyRequest::SetField {
            entity: id,
            component: Brush::NAME.into(),
            field: "shape".into(),
            value: Value::Text("Box".into()),
        },
        gesture,
    );
    h.run();
    assert!(
        h.state()
            .0
            .open
            .problems
            .list
            .iter()
            .all(|p| p.kind != crate::problems::ProblemKind::DamagedBrush)
    );
}

#[test]
fn a_tab_without_the_view_closes_the_list_and_gives_the_drawing_up() {
    let mut h = over_the_view();
    h.state_mut().0.take_brush(Shape::Box);
    h.run();
    assert!(h.state().0.viewport.brushing.is_active());
    ribbon_tab(&mut h, crate::shell::RibbonTab::Library);
    h.run();
    let w = &h.state().0;
    assert!(!w.viewport.brushing.is_active());
    assert_eq!(w.status().brushing, None);
}
