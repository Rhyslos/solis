//! Helpers shared by the editor's whole-window tests: a test window, a
//! stand-in renderer that answers picks, and small areas to work on.

use crate::layout;
use crate::models::ModelInfo;
use crate::panel::Panel;
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

/// The saves of the editor's own files among `actions` whose file `is`
/// picks, oldest first: each one's path and text.
pub(crate) fn saved_files(
    actions: &[crate::workbench::WorkbenchAction],
    is: fn(&crate::OwnFile) -> bool,
) -> Vec<(String, String)> {
    actions
        .iter()
        .filter_map(|a| match a {
            crate::workbench::WorkbenchAction::SaveFile { path, file, text } if is(file) => {
                Some((path.to_string_lossy().replace('\\', "/"), text.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Does the app's part of saving for it: every save of the editor's own
/// files waiting in `w`'s outbox is taken out, kept as the file in
/// `store` (the app's files) and answered as saved. Gives each one's
/// path and text, oldest first.
pub(crate) fn save_as_the_app(
    w: &mut Workbench,
    store: &mut crate::sprite_sheets::SpriteSheets,
) -> Vec<(String, String)> {
    let mut saved = Vec::new();
    let waiting = std::mem::take(&mut w.open.outbox);
    for action in waiting {
        match action {
            crate::workbench::WorkbenchAction::SaveFile { path, file, text } => {
                let path = path.to_string_lossy().replace('\\', "/");
                store.put(&path, file);
                w.file_saved(&path);
                saved.push((path, text));
            }
            other => w.open.outbox.push(other),
        }
    }
    saved
}

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
        // The editor's own fonts, as the app loads them before its first
        // frame: the design's controls ask for their weights by name.
        .tap(|h| h.ctx.set_fonts(crate::fonts::fonts()))
        .tap(|h| h.state_mut().0 = workbench)
}

/// Does the editor's command called `name` (as the search lists it), as
/// choosing it there, on the ribbon or on a File page does, and gives
/// what it asks the app for. Fails if no command has that name now, or
/// if it can't be done now.
pub(crate) fn do_command(
    workbench: &mut Workbench,
    ctx: &egui::Context,
    name: &str,
) -> Vec<crate::workbench::WorkbenchAction> {
    use crate::shell::ShellCommand;
    let command = ShellCommand::named(name, workbench)
        .unwrap_or_else(|| panic!("no command is called {name:?} now"));
    assert_eq!(
        command.why_not(workbench),
        None,
        "{name:?} can't be done now"
    );
    let mut actions = Vec::new();
    command.run(workbench, ctx, &mut actions);
    actions
}

/// [`do_command`] in the test window, drawn before and after.
pub(crate) fn run(h: &mut egui_kittest::Harness<'_, State>, name: &str) {
    h.run();
    let ctx = h.ctx.clone();
    let (workbench, picker) = h.state_mut();
    let actions = do_command(workbench, &ctx, name);
    picker.actions.extend(actions);
    h.run();
}

/// Brings `tab` to the front, as a click on it in the tab bar does.
pub(crate) fn ribbon_tab(h: &mut egui_kittest::Harness<'_, State>, tab: crate::shell::RibbonTab) {
    h.state_mut().0.show_tab(tab);
    h.run();
}

/// Draws the editor with `panel` in front and the pointer resting over
/// the middle of it.
pub(crate) fn rest_over(
    mut h: egui_kittest::Harness<'static, State>,
    panel: Panel,
) -> egui_kittest::Harness<'static, State> {
    h.run();
    layout::bring_to_front(&mut h.state_mut().0.dock, panel);
    h.run();
    let path = h.state().0.dock.find_tab(&panel).expect("the tab is open");
    let body = h.state().0.dock[path.surface][path.node]
        .get_leaf()
        .expect("a tab group")
        .viewport;
    h.hover_at(body.center());
    h.run();
    h
}

/// Every rectangle the last frame drew: where, its fill and its border's
/// colour.
pub(crate) fn rects_drawn<S>(
    h: &egui_kittest::Harness<'_, S>,
) -> Vec<(egui::Rect, egui::Color32, egui::Color32)> {
    fn gather(shape: &egui::Shape, found: &mut Vec<(egui::Rect, egui::Color32, egui::Color32)>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| gather(s, found)),
            egui::Shape::Rect(r) => found.push((r.rect, r.fill, r.stroke.color)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &h.output().shapes {
        gather(&clipped.shape, &mut found);
    }
    found
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
    w.open.document.selection.clear();
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

/// Clicks the × on `panel`'s tab, as the mouse does.
pub(crate) fn click_the_x(h: &mut egui_kittest::Harness<'static, State>, panel: Panel) {
    use egui_kittest::kittest::Queryable;
    h.run();
    h.run();
    // The tab is the thing of that name above its panel's body.
    let path = h.state().0.dock.find_tab(&panel).expect("the tab is open");
    let body = h.state().0.dock[path.surface][path.node]
        .get_leaf()
        .expect("a tab group")
        .viewport;
    let tab = h
        .get_all_by_label(panel.title())
        .map(|n| n.rect())
        .find(|r| r.bottom() <= body.top() + 1.0 && body.x_range().contains(r.center().x))
        .expect("the tab is drawn");
    // The × sits at the tab's right end.
    click(
        h,
        egui::pos2(tab.right() - 11.0, tab.center().y),
        egui::Modifiers::NONE,
    );
    h.run();
}

/// The number field on the row of the first thing labelled `label`, to
/// its right.
pub(crate) fn number_field_beside<S>(h: &egui_kittest::Harness<'_, S>, label: &str) -> egui::Rect {
    use egui_kittest::kittest::Queryable;
    let label = h
        .get_all_by_label(label)
        .next()
        .expect("the label is drawn")
        .rect();
    h.get_all_by_role(egui::accesskit::Role::SpinButton)
        .map(|n| n.rect())
        .filter(|r| r.y_range().contains(label.center().y) && r.left() >= label.right() - 1.0)
        .min_by(|a, b| a.left().total_cmp(&b.left()))
        .expect("a number field beside the label")
}

/// Presses the left button at `from`, moves by `by` a frame for `frames`
/// frames, and lets go there: a drag as the mouse makes it.
pub(crate) fn drag_for_frames<S>(
    h: &mut egui_kittest::Harness<'_, S>,
    from: Pos2,
    by: egui::Vec2,
    frames: usize,
) {
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    for frame in 1..=frames {
        h.hover_at(from + by * frame as f32);
        h.run();
    }
    h.event(egui::Event::PointerButton {
        pos: from + by * frames as f32,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
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
