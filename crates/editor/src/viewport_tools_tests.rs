use super::*;
use crate::area_view::ViewLight;
use area::builtin::LightKind;

const SIZE: Vec2 = Vec2::new(800.0, 600.0);

fn camera() -> Camera {
    let mut camera = Camera {
        position: Vec3::new(0.0, -10.0, 5.0),
        ..Camera::default()
    };
    camera.look_at(Vec3::ZERO);
    camera
}

fn lamp(entity: EntityId, position: Vec3) -> ViewLight {
    ViewLight {
        entity,
        kind: LightKind::Point,
        position,
        direction: Vec3::NEG_Z,
        color: [1.0; 3],
        strength: 100.0,
        range: None,
        inner: 0.0,
        outer: 0.0,
        shadows: false,
    }
}

fn input(mouse: Vec2) -> ToolInput {
    ToolInput {
        mouse: Some(mouse),
        on_viewport: true,
        ..ToolInput::default()
    }
}

/// A press and release at `at`.
fn click(tools: &mut Tools, at: Vec2, area: &AreaView, ctrl: bool) {
    let snapping = Snapping::default();
    let camera = camera();
    let pressed = ToolInput {
        pressed: true,
        down: true,
        ctrl,
        ..input(at)
    };
    tools.interact(pressed, &camera, area, &snapping, SIZE, 1.0);
    let released = ToolInput {
        released: true,
        ctrl,
        ..input(at)
    };
    tools.interact(released, &camera, area, &snapping, SIZE, 1.0);
}

#[test]
fn a_click_on_a_lamps_marker_picks_it_without_asking_the_renderer() {
    let id = EntityId::random();
    let area = AreaView {
        lamps: vec![lamp(id, Vec3::ZERO)],
        ..AreaView::default()
    };
    let mut tools = Tools::default();
    let at = camera().project(Vec3::ZERO, SIZE).unwrap() + Vec2::new(4.0, 0.0);
    click(&mut tools, at, &area, true);
    assert_eq!(
        tools.events,
        [ViewportEvent::Click {
            hit: Some(id),
            mode: PickMode::Toggle
        }]
    );
    assert_eq!(tools.rect_to_send(), None, "no pick needed");
}

#[test]
fn a_click_elsewhere_asks_about_one_pixel_and_reads_the_answer() {
    let id = EntityId::random();
    let area = AreaView {
        pickable: vec![id],
        ..AreaView::default()
    };
    let mut tools = Tools::default();
    click(&mut tools, Vec2::new(100.4, 50.9), &area, false);
    assert!(tools.events.is_empty());
    assert_eq!(tools.rect_to_send(), Some([100, 50, 1, 1]));
    assert_eq!(tools.rect_to_send(), None, "sent once");
    tools.receive(None);
    assert!(tools.events.is_empty(), "still waiting");
    tools.receive(Some(PickAnswer {
        middle: Some(1),
        ..PickAnswer::default()
    }));
    assert_eq!(
        tools.events,
        [ViewportEvent::Click {
            hit: Some(id),
            mode: PickMode::Replace
        }]
    );
    assert!(tools.pick.is_none());
}

#[test]
fn a_pick_number_that_was_never_handed_out_finds_nothing() {
    let area = AreaView::default();
    let mut tools = Tools::default();
    click(&mut tools, Vec2::new(10.0, 10.0), &area, false);
    tools.rect_to_send();
    tools.receive(Some(PickAnswer {
        middle: Some(99),
        ..PickAnswer::default()
    }));
    assert_eq!(
        tools.events,
        [ViewportEvent::Click {
            hit: None,
            mode: PickMode::Replace
        }]
    );
}

#[test]
fn a_renderer_that_never_answers_is_given_up_on() {
    let area = AreaView::default();
    let mut tools = Tools::default();
    click(&mut tools, Vec2::new(10.0, 10.0), &area, false);
    tools.rect_to_send();
    for _ in 0..=PICK_PATIENCE {
        tools.receive(None);
    }
    assert!(tools.pick.is_none());
    assert!(tools.events.is_empty());
}

#[test]
fn a_second_pick_waits_for_the_first() {
    let area = AreaView {
        pickable: vec![EntityId::random()],
        ..AreaView::default()
    };
    let mut tools = Tools::default();
    click(&mut tools, Vec2::new(10.0, 10.0), &area, false);
    assert!(tools.rect_to_send().is_some());
    click(&mut tools, Vec2::new(20.0, 20.0), &area, false);
    assert_eq!(tools.rect_to_send(), None, "the first is in flight");
    tools.receive(Some(PickAnswer::default()));
    assert_eq!(
        tools.rect_to_send(),
        Some([20, 20, 1, 1]),
        "then the second"
    );
}

#[test]
fn a_box_asks_about_its_rectangle_and_adds_lamps_inside_it() {
    let lamp_id = EntityId::random();
    let model_id = EntityId::random();
    let area = AreaView {
        lamps: vec![lamp(lamp_id, Vec3::ZERO)],
        pickable: vec![model_id],
        ..AreaView::default()
    };
    let centre = camera().project(Vec3::ZERO, SIZE).unwrap();
    let mut tools = Tools::default();
    let snapping = Snapping::default();
    let from = centre - Vec2::splat(50.0);
    let to = centre + Vec2::splat(50.0);
    tools.interact(
        ToolInput {
            pressed: true,
            down: true,
            ..input(from)
        },
        &camera(),
        &area,
        &snapping,
        SIZE,
        2.0,
    );
    tools.interact(
        ToolInput {
            down: true,
            ..input(to)
        },
        &camera(),
        &area,
        &snapping,
        SIZE,
        2.0,
    );
    assert!(matches!(tools.pressing, Pressing::Box { .. }));
    tools.interact(
        ToolInput {
            released: true,
            ..input(to)
        },
        &camera(),
        &area,
        &snapping,
        SIZE,
        2.0,
    );
    let rect = tools.rect_to_send().unwrap();
    assert_eq!(
        [rect[2], rect[3]],
        [200, 200],
        "in pixels, at 2 pixels a point"
    );
    tools.receive(Some(PickAnswer {
        found: vec![1],
        ..PickAnswer::default()
    }));
    let [ViewportEvent::Box { hits, mode }] = tools.events.as_slice() else {
        panic!("{:?}", tools.events)
    };
    assert_eq!(*mode, PickMode::Replace);
    assert!(hits.contains(&model_id) && hits.contains(&lamp_id));
}

#[test]
fn esc_cancels_a_box_but_a_click_still_counts() {
    let mut tools = Tools {
        pressing: Pressing::Box { from: Vec2::ZERO },
        ..Tools::default()
    };
    assert!(tools.cancel());
    assert_eq!(tools.pressing, Pressing::Nothing);
    assert!(!tools.cancel(), "nothing left to cancel");
}

#[test]
fn the_readout_says_how_far_and_how_much() {
    assert_eq!(
        readout(Change::Translate(Vec3::new(1.5, 0.0, -0.25))),
        "X +1.50 m  Z -0.25 m"
    );
    assert_eq!(readout(Change::Translate(Vec3::ZERO)), "0 m");
    assert_eq!(
        readout(Change::Rotate {
            axis: Vec3::Z,
            angle: 15f32.to_radians()
        }),
        "+15.0°"
    );
    assert_eq!(readout(Change::Scale(Vec3::splat(1.3))), "× 1.30");
    assert_eq!(
        readout(Change::Scale(Vec3::new(2.0, 1.0, 1.0))),
        "× 2.00  1.00  1.00"
    );
    assert_eq!(grid_text(0.25), "25 cm");
    assert_eq!(grid_text(1.0), "1 m");
}
