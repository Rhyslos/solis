//! Helpers shared by the editor's whole-window tests: a test window, a
//! stand-in renderer that answers picks, and small areas to work on.

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
                if !fonts_ready(ui.ctx()) {
                    return;
                }
                let (workbench, picker) = state;
                let actions = workbench.show(ui, picker);
                picker.actions.extend(actions);
            },
            (Workbench::new(info()), Picker::default()),
        )
        .tap(|h| h.state_mut().0 = workbench)
}

/// Whether the editor's own fonts are in `ctx`, as the app loads them
/// before its first frame: the frame asks for the design's weights by
/// name. The first call asks for them and says no (they arrive with the
/// next frame), so a test window draws nothing in the frame it is built
/// in.
pub(crate) fn fonts_ready(ctx: &egui::Context) -> bool {
    let asked = egui::Id::new("the editor's fonts were asked for");
    let ready = ctx.data(|data| data.get_temp::<bool>(asked).is_some());
    if !ready {
        ctx.set_fonts(crate::fonts::fonts());
        ctx.data_mut(|data| data.insert_temp(asked, true));
    }
    ready
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

/// Opens the File tab and clicks the item of its column called `label`,
/// as a person does.
pub(crate) fn file_item<S>(h: &mut egui_kittest::Harness<'_, S>, label: &str) {
    use egui_kittest::kittest::Queryable;
    h.run();
    h.get_by_label("File tab").click();
    h.run();
    // The column's item: a page may have a button of the same name
    // (Recent projects' New project), always to the column's right.
    h.get_all_by_label(label)
        .min_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .expect("the File column has it")
        .click();
    h.run();
}

/// Draws the editor with `panel` in view (on its tab, in front of its
/// group or on its shelf tab) and the pointer resting over the middle of
/// it.
pub(crate) fn rest_over(
    mut h: egui_kittest::Harness<'static, State>,
    panel: Panel,
) -> egui_kittest::Harness<'static, State> {
    h.run();
    h.state_mut().0.show_panel(panel);
    h.run();
    let body = body_of(&h, panel);
    h.hover_at(body.center());
    h.run();
    h
}

/// Where `panel`'s body was drawn in the last frame. Fails if it wasn't.
pub(crate) fn body_of(h: &egui_kittest::Harness<'static, State>, panel: Panel) -> egui::Rect {
    h.state()
        .0
        .shell
        .drawn_at(panel)
        .unwrap_or_else(|| panic!("{panel:?} is in view"))
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

/// Plays `w`'s games unhurried (the review's TEST-02): game code's and
/// physics' time limits are kept on the real clock, so on a busy machine a
/// tick of one run is cut short and the game pauses, and a test comparing
/// it with another run fails though nothing is wrong. For tests that
/// compare runs (or count what a whole tick made) only: the others keep
/// the real limits, which stop code that never ends.
pub(crate) fn unhurried(w: &mut Workbench) {
    w.unhurried = crate::game_code::Unhurried::Everything;
}

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
    let mut w = Workbench::new(info());
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

/// A new editor with Home's lists in `layout` (decision 0109): for
/// tests of the design's own shelf (`PanelLayout::Shelf`), or of one
/// layout in particular.
pub(crate) fn in_layout(layout: crate::shell::PanelLayout) -> Workbench {
    let look = crate::interface_look::InterfaceLook {
        layout,
        ..Default::default()
    };
    Workbench::new(info()).with_interface_look(look)
}

/// A double-click at `at`: both clicks in one frame, since the test
/// window's frames are too far apart in time for two of them to be one.
pub(crate) fn double_click(h: &mut egui_kittest::Harness<'_, State>, at: Pos2) {
    h.hover_at(at);
    h.run();
    for pressed in [true, false, true, false] {
        h.input_mut().events.push(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
    h.run();
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

/// Clicks the × that closes `panel`'s file (on its document tab, or on
/// its shelf tab), as the mouse does.
pub(crate) fn click_the_x(h: &mut egui_kittest::Harness<'static, State>, panel: Panel) {
    use egui_kittest::kittest::Queryable;
    h.state_mut().0.show_panel(panel);
    h.run();
    h.run();
    // "Close Clips", or "Close Animator · walk" for a document's tab; a
    // LIBRARY TOOLS document is called by its file's name.
    let file = |path: &str| path.rsplit('/').next().unwrap_or(path).to_owned();
    let w = &h.state().0;
    let open = &w.open;
    // On the shelf, the × is on the shelf tab the panel is a column of.
    let shelf = match crate::shell::workspaces::home_of(w.ribbon_tab(), w.open_now(), panel) {
        Some(crate::shell::workspaces::Home::Shelf(shelf)) => Some(shelf.name().to_owned()),
        _ => None,
    };
    let title = match panel {
        Panel::Tileset => open.tilesets.tab().map(|t| file(&t.path)),
        Panel::AtlasCrop => open.atlas_editor.as_ref().map(|e| file(&e.atlas.picture)),
        Panel::Canvas => open.screen_doc.as_ref().map(|d| file(&d.path)),
        _ => shelf,
    }
    .unwrap_or_else(|| panel.title().to_owned());
    let name = format!("Close {title}");
    let x = h
        .get_all_by_label_contains(&name)
        .next()
        .unwrap_or_else(|| panic!("{name} is drawn"))
        .rect();
    click(h, x.center(), egui::Modifiers::NONE);
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
    let mut w = Workbench::new(info());
    let (asset, file) = lamp_post_file();
    w.set_assembly(asset, file);
    w.place_assembly(asset);
    let root = w.selection().unwrap();
    let lamp = area::assembly::derive(root, &[EntityId(reflect::Id::from_raw(2).unwrap())]);
    (w, asset, root, lamp)
}

/// Whether `panel` has a place now: a panel that comes with a file only
/// while its file is open (and the Map while a tile map is selected).
/// The canvas and Elements are always the UI tab's, so for them it is
/// whether a screen is open on them.
pub(crate) fn has_panel(w: &Workbench, panel: Panel) -> bool {
    match panel {
        Panel::Canvas | Panel::Elements | Panel::Pictures => w.open.screen_doc.is_some(),
        _ => crate::shell::workspaces::tab_for(panel, w.open_now()).is_some(),
    }
}

/// Gives Properties the most room it can have in the test window, for
/// tests that click far down it: where it is a column of the shelf (on
/// every tab but Home, and on Home in the shelf layout), the tallest
/// shelf; where it is a column of its own beside the groups (Home in a
/// side layout, decision 0109), the shortest shelf, so the column is
/// tall.
pub(crate) fn roomy_shelf(w: &mut Workbench) {
    use crate::shell::{PanelLayout, RibbonTab, sizes::Edge};
    for tab in RibbonTab::ALL {
        for layout in PanelLayout::ALL {
            let height = if tab == RibbonTab::Home && layout.is_side() {
                crate::tokens::size::SHELF_MIN_H
            } else {
                ROOMY_SHELF
            };
            w.shell.sizes.set(tab, layout, Edge::ShelfHeight, height);
        }
    }
}

/// The shelf's height in [`roomy_shelf`]: as tall as the test window lets
/// it be, the center keeping its least.
const ROOMY_SHELF: f32 = 600.0;

/// Whether `panel` was drawn in the last frame: in front of its group,
/// or on the shelf tab in front.
pub(crate) fn in_view(w: &Workbench, panel: Panel) -> bool {
    w.shell.drawn_at(panel).is_some()
}
