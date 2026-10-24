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
    /// Every view drawn, oldest first.
    pub(crate) views: Vec<ViewportView>,
    waiting: bool,
}

impl ViewportRenderer for Picker {
    fn render(&mut self, view: &ViewportView) -> Result<egui::TextureId, String> {
        // Only the newest few are kept.
        if self.views.len() > 64 {
            self.views.remove(0);
        }
        self.views.push(view.clone());
        if let Some(rect) = view.pick {
            self.asked.push(rect);
            self.waiting = true;
        }
        Ok(egui::TextureId::default())
    }

    fn picked(&mut self, _image: usize) -> Option<PickAnswer> {
        if !self.waiting {
            return None;
        }
        self.waiting = false;
        // The answer to the question last asked.
        let rect = self.asked.last().copied().unwrap_or_default();
        Some(PickAnswer {
            rect,
            ..self.answer.clone().unwrap_or_default()
        })
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
        lods: Vec::new(),
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

/// A lamp-post assembly file: a post with a lamp under it.
pub(crate) fn lamp_post_file() -> (AssetId, std::sync::Arc<area::text::AreaFile>) {
    let id = |n| EntityId(reflect::Id::from_raw(n).unwrap());
    let mut area = area::Area::new();
    area.insert(
        id(1),
        area::EntityData::new("Lamp post").with(&area::builtin::Transform::default()),
    );
    let mut lamp = area::EntityData::new("Lamp").with(&area::builtin::Light::point());
    lamp.parent = Some(id(1));
    area.insert(id(2), lamp);
    let file = area::text::AreaFile {
        kind: area::text::FileKind::Assembly { root: id(1) },
        area,
        overrides: Default::default(),
    };
    (
        AssetId(reflect::Id::from_raw(900).unwrap()),
        std::sync::Arc::new(file),
    )
}

/// A workbench with the lamp post known and one copy placed.
pub(crate) fn with_a_lamp_post() -> (Workbench, AssetId, EntityId, EntityId) {
    let mut w = Workbench::new(info(), None);
    let (asset, file) = lamp_post_file();
    w.set_assembly(asset, file);
    w.place_assembly(asset);
    let root = w.selection().unwrap();
    let lamp = area::assembly::derive(root, &[EntityId(reflect::Id::from_raw(2).unwrap())]);
    (w, asset, root, lamp)
}
