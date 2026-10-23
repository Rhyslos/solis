//! The viewport's tools in the whole editor, drawn with egui_kittest and
//! a stand-in renderer that answers picks as told.

use super::*;
use crate::gizmo::Handle;
use crate::models::ModelInfo;
use crate::viewport::{ViewportRenderer, ViewportView};
use crate::viewport_tools::PickAnswer;
use crate::workbench::AppInfo;
use egui::Pos2;
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use reflect::{AssetId, EntityId};

/// Draws nothing; remembers each pick asked for and answers the next
/// one with `answer`.
#[derive(Default)]
struct Picker {
    asked: Vec<[u32; 4]>,
    answer: Option<PickAnswer>,
    waiting: bool,
}

impl ViewportRenderer for Picker {
    fn render(&mut self, view: &ViewportView) -> Result<egui::TextureId, String> {
        if let Some(rect) = view.pick {
            self.asked.push(rect);
            self.waiting = true;
        }
        Ok(egui::TextureId::default())
    }

    fn picked(&mut self) -> Option<PickAnswer> {
        if !self.waiting {
            return None;
        }
        self.waiting = false;
        Some(self.answer.clone().unwrap_or_default())
    }
}

type State = (Workbench, Picker);

fn harness(workbench: Workbench) -> egui_kittest::Harness<'static, State> {
    egui_kittest::Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_ui_state(
            |ui, state: &mut State| {
                let (workbench, picker) = state;
                let _ = workbench.show(ui, picker);
            },
            (Workbench::new(info(), None), Picker::default()),
        )
        .tap(|h| h.state_mut().0 = workbench)
}

trait Tap: Sized {
    fn tap(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }
}
impl<T> Tap for T {}

fn info() -> AppInfo {
    AppInfo {
        name: "Test Engine".into(),
        version: "9.9.9".into(),
    }
}

fn crate_model() -> ModelInfo {
    ModelInfo {
        asset: AssetId::random(),
        name: "Crate".into(),
        triangles: 12,
        min: Vec3::ZERO,
        max: Vec3::ONE,
        nodes: Vec::new(),
    }
}

/// A workbench with a crate placed at the origin, nothing selected.
fn with_a_crate() -> (Workbench, EntityId) {
    let mut w = Workbench::new(info(), None);
    let id = w.add_model(crate_model(), &[], 0.1, &[]);
    w.document.selection.clear();
    (w, id)
}

/// A point in the middle of the viewport, clear of its corner controls.
const IN_VIEW: Pos2 = egui::pos2(500.0, 300.0);

fn press(
    h: &egui_kittest::Harness<'_, State>,
    pressed: bool,
    at: Pos2,
    modifiers: egui::Modifiers,
) {
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    });
}

/// A click at `at`, with `modifiers` (Ctrl, say) held throughout.
fn click(h: &mut egui_kittest::Harness<'_, State>, at: Pos2, modifiers: egui::Modifiers) {
    h.event(egui::Event::ModifiersChanged(modifiers));
    h.hover_at(at);
    h.run();
    press(h, true, at, modifiers);
    h.run();
    press(h, false, at, modifiers);
    h.run();
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run();
}

#[test]
fn a_click_asks_the_renderer_and_selects_what_it_found() {
    let (w, id) = with_a_crate();
    let mut h = harness(w);
    h.state_mut().1.answer = Some(PickAnswer {
        middle: Some(1),
        ..PickAnswer::default()
    });
    click(&mut h, IN_VIEW, egui::Modifiers::NONE);
    h.run();
    let asked = h.state().1.asked.clone();
    assert_eq!(asked.len(), 1, "one pick");
    assert_eq!([asked[0][2], asked[0][3]], [1, 1], "of one pixel");
    assert_eq!(h.state().0.selection(), Some(id));
    assert!(
        h.state().0.document.history().undo_len() <= 1,
        "not an undo step"
    );

    // A click on nothing selects nothing.
    h.state_mut().1.answer = Some(PickAnswer::default());
    click(&mut h, IN_VIEW, egui::Modifiers::NONE);
    h.run();
    assert_eq!(h.state().0.selection(), None);
}

#[test]
fn ctrl_click_adds_and_takes_away() {
    let (mut w, id) = with_a_crate();
    let other = w.add_model(crate_model(), &[], 0.1, &[]);
    w.document.selection.select_only(other);
    let mut h = harness(w);
    // The crate placed first is pick number 1 or 2; find which.
    h.run();
    let n = h
        .state()
        .0
        .viewport
        .area
        .pickable
        .iter()
        .position(|&e| e == id)
        .unwrap() as u32
        + 1;
    h.state_mut().1.answer = Some(PickAnswer {
        middle: Some(n),
        ..PickAnswer::default()
    });
    click(&mut h, IN_VIEW, egui::Modifiers::COMMAND);
    h.run();
    let s = &h.state().0.document.selection;
    assert!(s.contains(id) && s.contains(other));
    click(&mut h, IN_VIEW, egui::Modifiers::COMMAND);
    h.run();
    let s = &h.state().0.document.selection;
    assert!(!s.contains(id) && s.contains(other));
}

#[test]
fn dragging_on_empty_space_draws_a_box_that_selects_what_is_inside() {
    let (mut w, id) = with_a_crate();
    let other = w.add_model(crate_model(), &[], 0.1, &[]);
    w.document.selection.clear();
    let mut h = harness(w);
    h.state_mut().1.answer = Some(PickAnswer {
        found: vec![1, 2],
        ..PickAnswer::default()
    });
    h.hover_at(IN_VIEW);
    h.run();
    press(&h, true, IN_VIEW, egui::Modifiers::NONE);
    h.run();
    let to = IN_VIEW + egui::vec2(120.0, 80.0);
    h.hover_at(to);
    h.run();
    press(&h, false, to, egui::Modifiers::NONE);
    h.run();
    h.run();
    let asked = h.state().1.asked.clone();
    assert_eq!(asked.len(), 1);
    assert!(asked[0][2] >= 119 && asked[0][3] >= 79, "{:?}", asked[0]);
    let s = &h.state().0.document.selection;
    assert!(s.contains(id) && s.contains(other));
}

/// Where the move gizmo's X arrow is on screen, most of the way along.
fn x_arrow(h: &egui_kittest::Harness<'_, State>) -> Pos2 {
    let w = &h.state().0;
    let gizmo = w.viewport.tools.gizmo.expect("a gizmo is shown");
    let rect = w.viewport.rect;
    let size = glam::Vec2::new(rect.width(), rect.height());
    let (_, shape) = gizmo
        .shapes(&w.viewport.camera, size, [false; 3])
        .into_iter()
        .find(|(handle, _)| *handle == Handle::Axis(0))
        .expect("an X arrow");
    let crate::gizmo::Shape::Line { a, b, .. } = shape else {
        panic!("an arrow is a line")
    };
    let p = a + (b - a) * 0.8;
    rect.min + egui::vec2(p.x, p.y)
}

/// A crate selected, with the Move tool, its gizmo grabbed by the X arrow
/// and dragged sideways; the button still held.
fn dragging_the_x_arrow() -> (egui_kittest::Harness<'static, State>, EntityId, Pos2) {
    let (mut w, id) = with_a_crate();
    w.document.selection.select_only(id);
    w.viewport.tool = Tool::Move;
    let mut h = harness(w);
    h.run();
    let grab = x_arrow(&h);
    h.hover_at(grab);
    h.run();
    press(&h, true, grab, egui::Modifiers::NONE);
    h.run();
    let to = grab + egui::vec2(60.0, 0.0);
    h.hover_at(to);
    h.run();
    (h, id, to)
}

fn position(w: &Workbench, id: EntityId) -> Vec3 {
    w.document
        .area()
        .get(id)
        .and_then(|e| e.get::<Transform>())
        .unwrap()
        .position
}

#[test]
fn dragging_an_arrow_moves_along_it_as_one_undo_step() {
    let (mut h, id, to) = dragging_the_x_arrow();
    let steps_before = h.state().0.document.history().undo_len();
    h.hover_at(to + egui::vec2(30.0, 0.0));
    h.run();
    press(&h, false, to + egui::vec2(30.0, 0.0), egui::Modifiers::NONE);
    h.run();
    let moved = position(&h.state().0, id);
    assert!(moved.x.abs() > 0.1, "moved along X: {moved}");
    assert!(
        moved.y.abs() < 1e-4 && moved.z.abs() < 1e-4,
        "only along X: {moved}"
    );
    let history = h.state().0.document.history();
    assert_eq!(history.undo_label(), Some("Move"));
    assert!(
        history.undo_len() <= steps_before + 1,
        "one step for the drag"
    );
    h.state_mut().0.undo();
    assert_eq!(position(&h.state().0, id), Vec3::ZERO);
}

#[test]
fn esc_or_a_right_click_cancels_a_drag_and_leaves_no_step() {
    for cancel in ["esc", "right"] {
        let (mut h, id, to) = dragging_the_x_arrow();
        assert_ne!(position(&h.state().0, id), Vec3::ZERO, "moving");
        let label_before = h
            .state()
            .0
            .document
            .history()
            .undo_label()
            .map(String::from);
        assert_eq!(label_before.as_deref(), Some("Move"));
        if cancel == "esc" {
            h.key_press(egui::Key::Escape);
        } else {
            h.event(egui::Event::PointerButton {
                pos: to,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run();
        assert_eq!(position(&h.state().0, id), Vec3::ZERO, "{cancel}: back");
        assert_ne!(
            h.state().0.document.history().undo_label(),
            Some("Move"),
            "{cancel}: no step left"
        );
        assert!(!h.state().0.viewport.is_flying(), "{cancel}: not flying");
        assert_eq!(
            h.state().0.selection(),
            Some(id),
            "{cancel}: still selected"
        );
    }
}

#[test]
fn snapping_moves_onto_the_grid() {
    let (mut h, id, to) = dragging_the_x_arrow();
    h.state_mut().0.viewport.snapping.grid = true;
    h.hover_at(to + egui::vec2(40.0, 0.0));
    h.run();
    let x = position(&h.state().0, id).x;
    assert_eq!(x, x.round(), "on a whole metre: {x}");
    press(&h, false, to, egui::Modifiers::NONE);
    h.run();
}

#[test]
fn esc_selects_nothing_and_ctrl_a_selects_everything() {
    let (w, id) = with_a_crate();
    let mut h = harness(w);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.run();
    let count = h.state().0.document.area().len();
    assert_eq!(h.state().0.document.selection.iter().count(), count);
    assert!(h.state().0.document.selection.contains(id));
    h.key_press(egui::Key::Escape);
    h.run();
    assert_eq!(h.state().0.selection(), None);
}

#[test]
fn x_y_z_lock_axes_and_l_switches_to_object_axes() {
    let (w, _) = with_a_crate();
    let mut h = harness(w);
    h.run();
    h.key_press(egui::Key::X);
    h.key_press(egui::Key::Z);
    h.key_press(egui::Key::L);
    h.run();
    let s = h.state().0.snapping();
    assert_eq!(s.locked, [true, false, true]);
    assert!(s.object_axes);
    h.key_press(egui::Key::X);
    h.run();
    assert_eq!(h.state().0.snapping().locked, [false, false, true]);
    // Ctrl+Z still undoes, not locks.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(h.state().0.snapping().locked, [false, false, true]);
}

#[test]
fn the_corner_buttons_switch_snapping_and_locks() {
    let (w, _) = with_a_crate();
    let mut h = harness(w);
    h.run();
    h.get_by_label("Grid 1 m").click();
    h.run();
    h.get_by_label("Lock Y").click();
    h.run();
    let s = h.state().0.snapping();
    assert!(s.grid);
    assert_eq!(s.locked, [false, true, false]);
}

#[test]
fn the_select_tool_shows_no_gizmo_and_others_do_on_a_selection() {
    let (mut w, id) = with_a_crate();
    let mut h = harness({
        w.document.selection.select_only(id);
        w
    });
    h.run();
    assert!(h.state().0.viewport.tools.gizmo.is_none(), "Select: none");
    h.key_press(egui::Key::W);
    h.run();
    h.run();
    let g = h.state().0.viewport.tools.gizmo.unwrap();
    assert_eq!(g.origin, Vec3::ZERO);
    assert_eq!(h.state().0.viewport.tools.outlines.len(), 1);
    h.key_press(egui::Key::Escape);
    h.run();
    h.run();
    assert!(
        h.state().0.viewport.tools.gizmo.is_none(),
        "nothing selected"
    );
}

#[test]
fn clicking_a_corner_button_never_clicks_the_viewport_behind_it() {
    let (mut w, id) = with_a_crate();
    w.document.selection.select_only(id);
    let mut h = harness(w);
    h.run();
    h.get_by_label("Move (W)").click();
    h.run();
    h.run();
    assert_eq!(h.state().0.tool(), Tool::Move);
    assert_eq!(h.state().0.selection(), Some(id), "still selected");
    assert!(h.state().1.asked.is_empty(), "no pick asked for");
    h.get_by_label("Grid 1 m").click();
    h.run();
    h.run();
    assert_eq!(h.state().0.selection(), Some(id), "still selected");
}
