//! Helpers shared by the editor's whole-window tests: a test window, a
//! stand-in renderer that answers picks, and small areas to work on.

use crate::models::ModelInfo;
use crate::viewport::{ViewportRenderer, ViewportView};
use crate::viewport_tools::PickAnswer;
use crate::workbench::{AppInfo, Workbench};
use egui::Pos2;
use glam::Vec3;
use reflect::{AssetId, EntityId};

/// Draws nothing; remembers each pick asked for and answers the next
/// one with `answer`.
#[derive(Default)]
pub(crate) struct Picker {
    pub(crate) asked: Vec<[u32; 4]>,
    pub(crate) answer: Option<PickAnswer>,
    /// Every action the workbench returned.
    pub(crate) actions: Vec<crate::workbench::WorkbenchAction>,
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

pub(crate) type State = (Workbench, Picker);

pub(crate) fn harness(workbench: Workbench) -> egui_kittest::Harness<'static, State> {
    egui_kittest::Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_ui_state(
            |ui, state: &mut State| {
                let (workbench, picker) = state;
                let actions = workbench.show(ui, picker);
                picker.actions.extend(actions);
            },
            (Workbench::new(info(), None), Picker::default()),
        )
        .tap(|h| h.state_mut().0 = workbench)
}

pub(crate) trait Tap: Sized {
    fn tap(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }
}
impl<T> Tap for T {}

pub(crate) fn info() -> AppInfo {
    AppInfo {
        name: "Test Engine".into(),
        version: "9.9.9".into(),
    }
}

pub(crate) fn crate_model() -> ModelInfo {
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
pub(crate) fn with_a_crate() -> (Workbench, EntityId) {
    let mut w = Workbench::new(info(), None);
    let id = w.add_model(crate_model(), &[], 0.1, &[]);
    w.document.selection.clear();
    (w, id)
}

/// A point in the middle of the viewport, clear of its corner controls.
pub(crate) const IN_VIEW: Pos2 = egui::pos2(500.0, 300.0);

pub(crate) fn press(
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
pub(crate) fn click(
    h: &mut egui_kittest::Harness<'_, State>,
    at: Pos2,
    modifiers: egui::Modifiers,
) {
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
