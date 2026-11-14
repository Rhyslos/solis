//! Drawing a brush out, frame by frame, as the viewport hands the mouse
//! over: snapping on and off, sizes of nothing, giving up, the 2D and side
//! views, the height's two ways, drawing on top of a brush, and the Ramp
//! and Stairs climbing from where the drag started.

use super::*;
use crate::area_view::AreaView;
use crate::brushes::{BrushModels, ViewBrush};
use area::builtin::Brush;
use glam::Mat4;

const SIZE: Vec2 = Vec2::new(800.0, 600.0);
const GRID: f32 = 0.5;

fn view<'a>(camera: &'a Camera, area: &'a AreaView) -> DrawView<'a> {
    DrawView {
        camera,
        size: SIZE,
        area,
        grid: GRID,
    }
}

/// Where `p` is on screen.
fn at(camera: &Camera, p: Vec3) -> Vec2 {
    camera.project(p, SIZE).expect("in front of the camera")
}

/// One frame's buttons.
#[derive(Clone, Copy)]
enum Mouse {
    Press,
    Hold,
    Release,
    Move,
    RightPress,
}

fn input(mouse: Vec2, what: Mouse, ctrl: bool) -> DrawInput {
    DrawInput {
        mouse: Some(mouse),
        pressed: matches!(what, Mouse::Press),
        down: matches!(what, Mouse::Press | Mouse::Hold),
        released: matches!(what, Mouse::Release),
        right_pressed: matches!(what, Mouse::RightPress),
        ctrl,
        on_view: true,
    }
}

/// Drags on the ground from `a` to `b` (world points) and lets go there.
fn drag(draw: &mut Draw, v: &DrawView, (a, b): (Vec3, Vec3), ctrl: bool) -> Outcome {
    let (from, to) = (at(v.camera, a), at(v.camera, b));
    assert_eq!(
        draw.update(input(from, Mouse::Press, ctrl), v),
        Outcome::Going
    );
    for i in 1..=4 {
        let m = from + (to - from) * i as f32 / 4.0;
        assert_eq!(draw.update(input(m, Mouse::Hold, ctrl), v), Outcome::Going);
    }
    draw.update(input(to, Mouse::Release, ctrl), v)
}

fn footprint(draw: &Draw) -> (Vec2, Vec2) {
    match draw.step {
        Step::Footprint { from, to, .. } | Step::Height { from, to, .. } => (from, to),
        Step::Waiting { .. } => panic!("no footprint: {:?}", draw.step),
    }
}

fn close(a: Vec2, b: Vec2) -> bool {
    a.distance(b) < 1e-3
}

#[test]
fn the_footprint_snaps_to_the_grid_and_ctrl_draws_freely() {
    let camera = Camera::default();
    let area = AreaView::default();
    let v = view(&camera, &area);
    let mut snapped = Draw::new(Shape::Box);
    drag(
        &mut snapped,
        &v,
        (Vec3::new(0.1, 0.1, 0.0), Vec3::new(2.2, 1.4, 0.0)),
        false,
    );
    let (from, to) = footprint(&snapped);
    assert!(close(from, Vec2::ZERO), "{from}");
    assert!(close(to, Vec2::new(2.0, 1.5)), "{to}");
    assert!(matches!(snapped.step, Step::Height { height: 0.0, .. }));
    let mut free = Draw::new(Shape::Box);
    drag(
        &mut free,
        &v,
        (Vec3::new(0.1, 0.1, 0.0), Vec3::new(2.2, 1.4, 0.0)),
        true,
    );
    let (from, to) = footprint(&free);
    assert!(from.distance(Vec2::new(0.1, 0.1)) < 0.01, "{from}");
    assert!(to.distance(Vec2::new(2.2, 1.4)) < 0.01, "{to}");
    assert!(snapped.status_words(GRID).contains("height 0.00 m"));
}

#[test]
fn the_height_follows_the_pointer_up_then_a_click_keeps_it() {
    let camera = Camera::default();
    let area = AreaView::default();
    let v = view(&camera, &area);
    let mut draw = Draw::new(Shape::Box);
    let corner = Vec3::new(4.0, 3.0, 0.0);
    drag(&mut draw, &v, (Vec3::ZERO, corner), false);
    // Up the screen from where the footprint was let go.
    let start = at(&camera, corner);
    let mut last = 0.0;
    for up in [40.0, 80.0, 160.0] {
        draw.update(input(start - Vec2::Y * up, Mouse::Move, false), &v);
        let Step::Height { height, .. } = draw.step else {
            panic!("on to the height");
        };
        assert!(height > last, "{height} after {last}");
        assert_eq!(height % GRID, 0.0, "{height} is on the grid");
        last = height;
    }
    // Ctrl: not on the grid.
    draw.update(input(start - Vec2::Y * 157.0, Mouse::Move, true), &v);
    let Step::Height { height, .. } = draw.step else {
        panic!()
    };
    assert_ne!(height % GRID, 0.0);
    let made = draw.update(input(start - Vec2::Y * 157.0, Mouse::Press, true), &v);
    let Outcome::Made(planned) = made else {
        panic!("a click keeps it: {made:?}")
    };
    assert_eq!(planned.shape, Shape::Box);
    assert_eq!(
        planned.position,
        Vec3::new(2.0, 1.5, 0.0),
        "the base's middle"
    );
    assert_eq!(planned.rotation, Quat::IDENTITY);
    assert_eq!(planned.settings.size.truncate(), Vec2::new(4.0, 3.0));
    assert!((planned.settings.size.z - height).abs() < 1e-6);
}

#[test]
fn the_standing_line_gives_the_height_the_pointer_is_at() {
    let camera = Camera::default();
    let middle = Vec3::new(2.0, 1.0, 0.0);
    for h in [0.0, 0.75, 3.0] {
        let ray = camera.ray_through(at(&camera, middle + Vec3::Z * h), SIZE);
        let up = up_the_line(middle, &ray).unwrap();
        assert!((up - h).abs() < 1e-3, "{up} for {h}");
    }
    // A ray running alongside it gives nothing, never a crash.
    let alongside = crate::screen::Ray {
        origin: Vec3::new(5.0, 5.0, 10.0),
        direction: Vec3::NEG_Z,
    };
    assert_eq!(up_the_line(middle, &alongside), None);
}

#[test]
fn looking_straight_down_the_height_follows_the_pointer_up_the_screen() {
    let camera = Camera::default().as_kind(ViewKind::Top);
    let area = AreaView::default();
    let v = view(&camera, &area);
    let mut draw = Draw::new(Shape::Box);
    let corner = Vec3::new(3.0, 2.0, 0.0);
    drag(&mut draw, &v, (Vec3::ZERO, corner), false);
    let Step::Height { lift, .. } = draw.step else {
        panic!("the Top view has a height step: {:?}", draw.step)
    };
    assert!(matches!(lift, Lift::Screen { .. }), "{lift:?}");
    let start = at(&camera, corner);
    // A metre on screen, up: a metre high.
    let per_metre = at(&camera, Vec3::X).x - at(&camera, Vec3::ZERO).x;
    draw.update(
        input(start - Vec2::Y * per_metre * 2.0, Mouse::Move, false),
        &v,
    );
    let Step::Height { height, .. } = draw.step else {
        panic!()
    };
    assert!((height - 2.0).abs() < 1e-3, "{height}");
    // Down past where it started: no height below the footprint.
    draw.update(input(start + Vec2::Y * 50.0, Mouse::Move, false), &v);
    assert!(matches!(draw.step, Step::Height { height: 0.0, .. }));
}

#[test]
fn a_click_without_a_drag_makes_nothing_and_says_why() {
    let camera = Camera::default();
    let area = AreaView::default();
    let v = view(&camera, &area);
    let mut draw = Draw::new(Shape::Ramp);
    let spot = at(&camera, Vec3::new(1.0, 1.0, 0.0));
    draw.update(input(spot, Mouse::Press, false), &v);
    assert!(draw.is_under_way());
    assert_eq!(
        draw.update(input(spot, Mouse::Release, false), &v),
        Outcome::Going
    );
    assert_eq!(
        draw.step,
        Step::Waiting {
            nothing: Some(Nothing::NoFootprint)
        }
    );
    assert!(draw.status_words(GRID).starts_with("Nothing made"));
    // A height of nothing: the footprint drawn, then a click at once.
    let corner = Vec3::new(2.0, 2.0, 0.0);
    drag(&mut draw, &v, (Vec3::ZERO, corner), false);
    let made = draw.update(input(at(&camera, corner), Mouse::Press, false), &v);
    assert_eq!(made, Outcome::Going, "nothing made");
    assert_eq!(
        draw.step,
        Step::Waiting {
            nothing: Some(Nothing::NoHeight)
        }
    );
    assert!(draw.status_words(GRID).contains("no height"));
    // Waiting again: the next drag draws.
    assert!(!draw.is_under_way());
}

#[test]
fn a_right_click_gives_up_once_under_way() {
    let camera = Camera::default();
    let area = AreaView::default();
    let v = view(&camera, &area);
    let mut draw = Draw::new(Shape::Box);
    let mouse = at(&camera, Vec3::ZERO);
    // Before the drag, the right button is the camera's.
    assert_eq!(
        draw.update(input(mouse, Mouse::RightPress, false), &v),
        Outcome::Going
    );
    draw.update(input(mouse, Mouse::Press, false), &v);
    assert_eq!(
        draw.update(input(mouse, Mouse::RightPress, false), &v),
        Outcome::GaveUp,
        "during the footprint"
    );
    let mut draw = Draw::new(Shape::Box);
    drag(&mut draw, &v, (Vec3::ZERO, Vec3::new(2.0, 2.0, 0.0)), false);
    assert_eq!(
        draw.update(input(mouse, Mouse::RightPress, false), &v),
        Outcome::GaveUp,
        "during the height"
    );
}

#[test]
fn in_the_2d_view_letting_go_makes_it_one_grid_step_deep() {
    let camera = Camera::default().as_kind(ViewKind::TwoD);
    let area = AreaView::default();
    let v = view(&camera, &area);
    let mut draw = Draw::new(Shape::Box);
    draw.update(input(SIZE / 2.0, Mouse::Move, false), &v);
    assert!(draw.in_plane(), "said before the drag");
    assert!(draw.status_words(GRID).contains("one grid step deep"));
    let made = drag(
        &mut draw,
        &v,
        (Vec3::new(-1.0, -1.0, 0.0), Vec3::new(2.0, 1.0, 0.0)),
        false,
    );
    let Outcome::Made(planned) = made else {
        panic!("made on letting go: {made:?}")
    };
    assert_eq!(planned.settings.size, Vec3::new(3.0, 2.0, GRID));
    assert_eq!(planned.position, Vec3::new(0.5, 0.0, 0.0));
}

#[test]
fn in_the_front_view_the_footprint_stands_upright() {
    let camera = Camera::default().as_kind(ViewKind::Front);
    let area = AreaView::default();
    let v = view(&camera, &area);
    let mut draw = Draw::new(Shape::Box);
    let made = drag(
        &mut draw,
        &v,
        (Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 3.0)),
        false,
    );
    let Outcome::Made(planned) = made else {
        panic!("{made:?}")
    };
    assert_eq!(planned.settings.size, Vec3::new(2.0, GRID, 3.0));
    assert_eq!(planned.position, Vec3::new(1.0, 0.0, 0.0));
}

/// A box brush 2 m on a side, 1.5 m tall, standing at `at`, as the
/// viewport has it.
fn area_with_a_box(at: Vec3) -> AreaView {
    let mut area = area::Area::new();
    let mesh = Shape::Box.build(&ShapeSettings {
        size: Vec3::new(2.0, 2.0, 1.5),
        ..ShapeSettings::default()
    });
    let brush = Brush {
        mesh: std::sync::Arc::new(mesh),
        ..Brush::default()
    };
    let id = area.spawn(area::EntityData::new("Box").with(&brush));
    let mut document = crate::document::Document::new(area);
    let mut models = BrushModels::default();
    models.refresh(&mut document);
    let mut view = AreaView::default();
    view.brushes.push(ViewBrush {
        entity: id,
        model: std::sync::Arc::clone(models.get(id).unwrap()),
        transform: Mat4::from_translation(at),
        pick: 1,
    });
    view
}

#[test]
fn a_drag_starting_on_a_brush_draws_on_its_top() {
    let camera = Camera::default();
    let area = area_with_a_box(Vec3::new(5.0, 5.0, 0.0));
    let v = view(&camera, &area);
    let mut draw = Draw::new(Shape::Box);
    // From a point on the box's top, to beyond it.
    drag(
        &mut draw,
        &v,
        (Vec3::new(5.0, 5.0, 1.5), Vec3::new(7.0, 6.0, 1.5)),
        false,
    );
    let Step::Height {
        plane, from, to, ..
    } = draw.step
    else {
        panic!("{:?}", draw.step)
    };
    assert_eq!(plane, Plane::Ground { base: 1.5 });
    assert!(close(from, Vec2::new(5.0, 5.0)) && close(to, Vec2::new(7.0, 6.0)));
    // Beside it, on the ground.
    let mut beside = Draw::new(Shape::Box);
    drag(
        &mut beside,
        &v,
        (Vec3::new(-3.0, -3.0, 0.0), Vec3::new(-1.0, -1.0, 0.0)),
        false,
    );
    assert!(matches!(
        beside.step,
        Step::Height {
            plane: Plane::Ground { base: 0.0 },
            ..
        }
    ));
    let planned = draw.planned_now(GRID).unwrap();
    assert_eq!(planned.position.z, 1.5, "it stands on the box");
}

#[test]
fn ramps_and_stairs_climb_along_the_longer_side_from_where_the_drag_started() {
    let ground = Plane::Ground { base: 0.0 };
    let climb_of = |shape, from: Vec2, to: Vec2| {
        let p = planned(shape, ground, (from, to), 1.0).unwrap();
        // Its own +Y, the way it climbs, in the world.
        (p.rotation * Vec3::Y, p.settings.size)
    };
    let (up, size) = climb_of(Shape::Ramp, Vec2::ZERO, Vec2::new(4.0, 2.0));
    assert!(up.distance(Vec3::X) < 1e-5, "{up}");
    assert_eq!(size.truncate(), Vec2::new(2.0, 4.0), "long along its climb");
    let (up, _) = climb_of(Shape::Ramp, Vec2::new(4.0, 0.0), Vec2::new(0.0, 2.0));
    assert!(up.distance(Vec3::NEG_X) < 1e-5, "{up}");
    let (up, _) = climb_of(Shape::Stairs, Vec2::new(0.0, 5.0), Vec2::new(1.0, 0.0));
    assert!(up.distance(Vec3::NEG_Y) < 1e-5, "{up}");
    let (up, size) = climb_of(Shape::Box, Vec2::new(4.0, 0.0), Vec2::new(0.0, 2.0));
    assert_eq!(up, Vec3::Y, "other shapes stand square");
    assert_eq!(size.truncate(), Vec2::new(4.0, 2.0));
}

#[test]
fn a_size_of_nothing_or_not_a_number_plans_nothing() {
    let ground = Plane::Ground { base: 0.0 };
    let square = (Vec2::ZERO, Vec2::ONE);
    assert!(planned(Shape::Box, ground, square, 1.0).is_some());
    assert!(planned(Shape::Box, ground, square, 0.0).is_none());
    assert!(planned(Shape::Box, ground, square, f32::NAN).is_none());
    assert!(planned(Shape::Box, ground, (Vec2::ZERO, Vec2::X), 1.0).is_none());
    // Too big is brought within the shapes' limits.
    let huge = planned(Shape::Box, ground, (Vec2::ZERO, Vec2::splat(1.0e7)), 1.0e9).unwrap();
    assert_eq!(huge.settings.size, Vec3::splat(mesh::shapes::MAX_SIZE));
}

#[test]
fn lengths_are_said_without_trailing_noughts() {
    assert_eq!(metres(4.0), "4");
    assert_eq!(metres(0.5), "0.5");
    assert_eq!(metres(1.25), "1.25");
    assert_eq!(metres(-0.0001), "0");
    assert_eq!(
        footprint_words(Vec2::ZERO, Vec2::new(-4.0, 3.0)),
        "4 \u{d7} 3 m"
    );
}
