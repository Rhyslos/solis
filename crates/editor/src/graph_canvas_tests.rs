//! The shared graph canvas, driven by a pointer: picking, moving boxes,
//! joining them from an edge, picking boxes with a box, right-click menus,
//! a menu's arrow ended by a click, Delete, panning, zooming and framing.

use super::*;
use egui::{Event, Modifiers, PointerButton};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

/// The canvas with two boxes ("A" at (0, 0) and "B" at (300, 0), each
/// 150 by 74) and an arrow from A to B, and what it reported.
#[derive(Default)]
struct Board {
    canvas: GraphCanvas,
    events: Vec<GraphEvent>,
    menus: Vec<Option<GraphTarget>>,
    aim: Option<u64>,
}

fn boxes() -> Vec<GraphBox> {
    let b = |id: u64, x: f32, title: &str| GraphBox {
        id,
        place: pos2(x, 0.0),
        size: vec2(150.0, 74.0),
        title: title.into(),
        line: "clip".into(),
        chip: None,
        picked: false,
        live: false,
        start: id == 1,
        movable: true,
        takes_arrows: true,
    };
    vec![b(1, 0.0, "A"), b(2, 300.0, "B")]
}

fn board() -> Harness<'static, Board> {
    let mut h = Harness::builder()
        .with_size(vec2(800.0, 500.0))
        .build_ui_state(
            |ui, board: &mut Board| {
                if let Some(from) = board.aim.take() {
                    board.canvas.aim_from(from);
                }
                let wires = [GraphWire {
                    id: 9,
                    from: 1,
                    to: 2,
                    label: "moving".into(),
                    picked: false,
                }];
                let mut menus = Vec::new();
                let events = graph_canvas(
                    ui,
                    &mut board.canvas,
                    &boxes(),
                    &wires,
                    Color32::GOLD,
                    &mut |_ui, target| menus.push(target),
                );
                board.menus.extend(menus);
                board.events.extend(events);
            },
            Board::default(),
        );
    // Where the graph's (0, 0) is: framed on the first frame, so set
    // plainly for the tests.
    h.run();
    h.state_mut().canvas.view = GraphView {
        offset: vec2(100.0, 100.0),
        zoom: 1.0,
    };
    h.state_mut().events.clear();
    h.run();
    h
}

/// A point of the graph on the screen (the canvas starts at the window's
/// corner, give or take egui's margin).
fn at(h: &Harness<'_, Board>, x: f32, y: f32) -> Pos2 {
    let canvas = h.get_by_label("Graph canvas").rect();
    let v = h.state().canvas.view;
    canvas.min + v.offset + vec2(x, y) * v.zoom
}

fn press(h: &mut Harness<'_, Board>, p: Pos2, button: PointerButton, pressed: bool) {
    h.event(Event::PointerButton {
        pos: p,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    });
    h.run();
}

fn drag(h: &mut Harness<'_, Board>, from: Pos2, to: Pos2) {
    h.hover_at(from);
    h.run();
    press(h, from, PointerButton::Primary, true);
    for i in 1..=6 {
        h.event(Event::PointerMoved(from + (to - from) * (i as f32 / 6.0)));
        h.run();
    }
    press(h, to, PointerButton::Primary, false);
    h.run();
}

fn take(h: &mut Harness<'_, Board>) -> Vec<GraphEvent> {
    std::mem::take(&mut h.state_mut().events)
}

#[test]
fn a_click_picks_a_box_an_arrow_or_nothing() {
    let mut h = board();
    h.get_by_label("State A").click();
    h.run();
    assert!(take(&mut h).contains(&GraphEvent::Pick(GraphTarget::Box(1), false)));
    h.get_by_label("Transition moving").click();
    h.run();
    assert!(take(&mut h).contains(&GraphEvent::Pick(GraphTarget::Wire(9), false)));
    let empty = at(&h, 200.0, 300.0);
    h.hover_at(empty);
    press(&mut h, empty, PointerButton::Primary, true);
    press(&mut h, empty, PointerButton::Primary, false);
    assert!(take(&mut h).contains(&GraphEvent::Clear));
}

#[test]
fn dragging_a_box_moves_it_and_dragging_its_edge_joins_it() {
    let mut h = board();
    let (from, to) = (at(&h, 75.0, 37.0), at(&h, 125.0, 77.0));
    drag(&mut h, from, to);
    let moved = take(&mut h)
        .into_iter()
        .find_map(|e| match e {
            GraphEvent::Moved(ids, by) => Some((ids, by)),
            _ => None,
        })
        .expect("moved");
    assert_eq!(moved.0, [1]);
    assert!((moved.1 - vec2(50.0, 40.0)).length() < 2.0, "{:?}", moved.1);
    // From A's right edge to B: an arrow.
    let (from, to) = (at(&h, 147.0, 37.0), at(&h, 375.0, 37.0));
    drag(&mut h, from, to);
    assert!(take(&mut h).contains(&GraphEvent::Join(1, 2)));
    // From the edge to empty space: nothing.
    let (from, to) = (at(&h, 147.0, 37.0), at(&h, 200.0, 300.0));
    drag(&mut h, from, to);
    assert!(
        !take(&mut h)
            .iter()
            .any(|e| matches!(e, GraphEvent::Join(..)))
    );
}

#[test]
fn a_picking_box_picks_what_it_touches() {
    let mut h = board();
    let (from, to) = (at(&h, -20.0, -20.0), at(&h, 320.0, 100.0));
    drag(&mut h, from, to);
    assert!(take(&mut h).contains(&GraphEvent::PickBoxes(vec![1, 2], false)));
}

#[test]
fn right_clicks_open_the_menu_for_what_s_under_the_pointer() {
    let mut h = board();
    let on_b = at(&h, 375.0, 37.0);
    h.hover_at(on_b);
    press(&mut h, on_b, PointerButton::Secondary, true);
    press(&mut h, on_b, PointerButton::Secondary, false);
    h.run();
    assert!(h.state().menus.contains(&Some(GraphTarget::Box(2))));
    let empty = at(&h, 200.0, 300.0);
    h.hover_at(empty);
    press(&mut h, empty, PointerButton::Secondary, true);
    press(&mut h, empty, PointerButton::Secondary, false);
    h.run();
    assert!(h.state().menus.contains(&None));
    let p = h.state().canvas.pointer;
    assert!((p - pos2(200.0, 300.0)).length() < 2.0, "{p:?}");
}

#[test]
fn an_arrow_from_a_menu_ends_at_the_next_click() {
    let mut h = board();
    h.state_mut().aim = Some(1);
    h.run();
    let on_b = at(&h, 375.0, 37.0);
    h.hover_at(on_b);
    press(&mut h, on_b, PointerButton::Primary, true);
    press(&mut h, on_b, PointerButton::Primary, false);
    assert!(take(&mut h).contains(&GraphEvent::Join(1, 2)));
    // Escape gives up.
    h.state_mut().aim = Some(1);
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    press(&mut h, on_b, PointerButton::Primary, true);
    press(&mut h, on_b, PointerButton::Primary, false);
    assert!(
        !take(&mut h)
            .iter()
            .any(|e| matches!(e, GraphEvent::Join(..)))
    );
}

#[test]
fn the_view_pans_zooms_and_frames_and_delete_is_reported() {
    let mut h = board();
    let middle = at(&h, 200.0, 300.0);
    h.hover_at(middle);
    h.run();
    // Middle-drag pans.
    press(&mut h, middle, PointerButton::Middle, true);
    h.event(Event::PointerMoved(middle + vec2(30.0, 10.0)));
    h.run();
    press(
        &mut h,
        middle + vec2(30.0, 10.0),
        PointerButton::Middle,
        false,
    );
    let v = h.state().canvas.view;
    assert!(
        (v.offset - vec2(130.0, 110.0)).length() < 1.0,
        "{:?}",
        v.offset
    );
    // The wheel zooms round the pointer: the graph point under it stays.
    let p = middle + vec2(30.0, 10.0);
    let canvas = h.get_by_label("Graph canvas").rect();
    let before = v.to_world(canvas.min, p);
    h.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, 120.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    });
    for _ in 0..30 {
        h.run();
    }
    let v = h.state().canvas.view;
    assert!(v.zoom > 1.05, "{}", v.zoom);
    assert!((v.to_world(canvas.min, p) - before).length() < 2.0);
    // F frames both boxes; Delete is reported.
    h.key_press(egui::Key::F);
    h.run();
    let v = h.state().canvas.view;
    let a = v.to_screen(canvas.min, pos2(0.0, 0.0));
    let b = v.to_screen(canvas.min, pos2(450.0, 74.0));
    assert!(canvas.contains(a) && canvas.contains(b));
    h.key_press(egui::Key::Delete);
    h.run();
    assert!(take(&mut h).contains(&GraphEvent::Delete));
}

#[test]
fn framing_nothing_or_a_point_is_safe() {
    let mut v = GraphView::default();
    v.frame(Rect::NOTHING, vec2(400.0, 300.0));
    assert_eq!(v, GraphView::default());
    v.frame(
        Rect::from_min_size(pos2(5.0, 5.0), Vec2::ZERO),
        vec2(400.0, 300.0),
    );
    assert_eq!(v, GraphView::default());
    let mut v = GraphView::default();
    v.zoom_at(Pos2::ZERO, pos2(10.0, 10.0), 1000.0);
    assert_eq!(v.zoom, ZOOM_RANGE.1);
    v.zoom_at(Pos2::ZERO, pos2(10.0, 10.0), 0.0);
    assert_eq!(v.zoom, ZOOM_RANGE.0);
    assert!(distance_to_segment(pos2(5.0, 5.0), pos2(0.0, 0.0), pos2(0.0, 0.0)) > 7.0);
}
