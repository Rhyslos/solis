//! The Area panel: the area's entities as a tree. Clicking one selects
//! it and shows it in Properties. Its menu row has the Edit and Entity
//! menus, folding the tree, and a search box; each row can be folded,
//! hidden and locked.

use crate::document::Document;
use crate::icons::{self, Icon};
use crate::menu_items::{self, MenuCommand, MenuState};
use crate::models::ModelInfo;
use crate::theme::{self, Accent};
use area::EntityData;
use area::builtin::{Light, LightKind, Model};
use egui::{Align, Color32, Layout, RichText, Ui};
use reflect::{AssetId, EntityId};
use std::collections::{BTreeMap, HashSet};

/// How the Area panel shows the area: what's searched for, folded, hidden
/// and locked. The editor's view only, like the selection: never saved in
/// the area and never an undo step (decision 0022).
#[derive(Debug, Clone, Default)]
pub(crate) struct AreaPanelState {
    /// Only names containing this are listed (with their parents).
    pub(crate) search: String,
    /// Entities whose children are folded away.
    pub(crate) collapsed: HashSet<EntityId>,
    /// Entities not drawn in the viewport (with everything under them).
    pub(crate) hidden: HashSet<EntityId>,
    /// Entities the viewport can't pick (with everything under them).
    pub(crate) locked: HashSet<EntityId>,
    /// An entity to scroll to and unfold the way to, once.
    pub(crate) reveal: Option<EntityId>,
}

impl AreaPanelState {
    /// Every entity hidden or locked, counting those under a hidden or
    /// locked one: (hidden, locked).
    pub(crate) fn in_effect(&self, area: &area::Area) -> (HashSet<EntityId>, HashSet<EntityId>) {
        if self.hidden.is_empty() && self.locked.is_empty() {
            return Default::default();
        }
        let mut hidden = HashSet::new();
        let mut locked = HashSet::new();
        for id in area.depth_first() {
            let parent = area.get(id).and_then(|e| e.parent);
            if self.hidden.contains(&id) || parent.is_some_and(|p| hidden.contains(&p)) {
                hidden.insert(id);
            }
            if self.locked.contains(&id) || parent.is_some_and(|p| locked.contains(&p)) {
                locked.insert(id);
            }
        }
        (hidden, locked)
    }
}

/// How far each level of the tree is indented.
const INDENT: f32 = 14.0;

/// An entity row's hide and lock switches: on for the entity itself, and
/// whether it's hidden or locked by a parent.
#[derive(Debug, Clone, Copy)]
struct Toggles {
    hidden: bool,
    locked: bool,
    shown: Shown,
}

/// Which switch was clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Toggle {
    Hide,
    Lock,
}

/// One row of the Area panel: an icon, a name, a quiet note on the
/// right, highlighted when selected, a × when `remove` names what it
/// removes, and hide and lock switches when `toggles` is given (shown
/// while the row is pointed at, or while they're on). Returns the row's
/// response, whether × was clicked, and which switch was.
#[allow(clippy::too_many_arguments)]
fn area_row(
    ui: &mut Ui,
    (icon, icon_color): (Icon, Color32),
    name: &str,
    note: &str,
    selected: bool,
    accent: Accent,
    remove: Option<&str>,
    toggles: Option<Toggles>,
) -> (egui::Response, bool, Option<Toggle>) {
    let (row, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::click_and_drag(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, name)
    });
    if selected {
        ui.painter()
            .rect_filled(row, 3.0, accent.colors().fill.gamma_multiply(0.45));
    } else if response.hovered() {
        ui.painter().rect_filled(row, 3.0, theme::HOVER);
    }
    let mut row_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row.shrink2(egui::vec2(4.0, 0.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    let ui = &mut row_ui;
    let dim = toggles.is_some_and(|t| t.shown.hidden);
    let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    icons::icon(
        ui.painter(),
        icon_rect,
        icon,
        if dim {
            icon_color.gamma_multiply(0.4)
        } else {
            icon_color
        },
    );
    ui.add_space(4.0);
    let label = if dim {
        RichText::new(name).color(theme::TEXT_WEAK)
    } else {
        RichText::new(name)
    };
    // Room on the right for the switches and ×, so a long name stops short.
    let right = 18.0 * 3.0 + 40.0;
    ui.allocate_ui_with_layout(
        egui::vec2((ui.available_width() - right).max(20.0), ROW_HEIGHT),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(egui::Label::new(label).truncate().selectable(false));
        },
    );
    let mut removed = false;
    let mut toggled = None;
    let pointed = ui.rect_contains_pointer(row);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if let Some(what) = remove {
            let button = ui
                .add(egui::Button::new(RichText::new("×").color(theme::TEXT_WEAK)).frame(false))
                .on_hover_text(format!("Remove {what} from the area"));
            button.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Remove {what}"))
            });
            removed = button.clicked();
        }
        if let Some(t) = toggles {
            if (pointed || t.locked || t.shown.locked)
                && switch(ui, t.locked, t.shown.locked, Icon::Lock, &format!("Lock {name}"), "Locked: clicks in the viewport pass through it (and what's under it)")
            {
                toggled = Some(Toggle::Lock);
            }
            if (pointed || t.hidden || t.shown.hidden)
                && switch(ui, t.hidden, t.shown.hidden, Icon::Hidden, &format!("Hide {name}"), "Hidden: not drawn in the editor (nor what's under it); the game still shows it")
            {
                toggled = Some(Toggle::Hide);
            }
        }
        ui.add(egui::Label::new(RichText::new(note).color(theme::TEXT_WEAK)).selectable(false));
    });
    (response, removed, toggled)
}

/// A small switch for hiding or locking: bright while on, faint while on
/// only because of a parent, weak otherwise. Returns whether it was
/// clicked.
fn switch(ui: &mut Ui, on: bool, by_parent: bool, icon: Icon, name: &str, hint: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::click());
    let color = if on {
        theme::TEXT_STRONG
    } else if by_parent {
        theme::TEXT_WEAK.gamma_multiply(0.6)
    } else if response.hovered() {
        theme::TEXT
    } else {
        theme::TEXT_WEAK
    };
    icons::icon(ui.painter(), rect.shrink(2.0), icon, color);
    let response = response.on_hover_text(hint);
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, name));
    response.clicked()
}

/// An entity's icon and note, from its components.
fn looks(
    entity: &EntityData,
    models: &BTreeMap<AssetId, ModelInfo>,
    accent: Accent,
) -> (Icon, Color32, String) {
    if let Some(light) = entity.get::<Light>() {
        let [r, g, b] = light.color.map(|c| (c * 255.0).round() as u8);
        let color = if light.kind == LightKind::Sun {
            theme::WARNING
        } else {
            Color32::from_rgb(r, g, b)
        };
        let note = match light.kind {
            LightKind::Sun => "",
            LightKind::Point => "Point light",
            LightKind::Spot => "Spotlight",
        };
        return (Icon::Light, color, note.to_string());
    }
    if let Some(model) = entity.get::<Model>() {
        let note = model
            .asset
            .and_then(|r| models.get(&r.asset))
            .map_or_else(String::new, |m| {
                format!("{} tris", crate::models::grouped(m.triangles))
            });
        return (Icon::Model, accent.colors().light, note);
    }
    if entity.has("Environment") {
        return (Icon::AreaFile, accent.colors().light, "sky, fog".into());
    }
    (Icon::Area, theme::TEXT_WEAK, String::new())
}

/// Something the user asked for in the Area panel. Like Properties, the
/// panel never changes the area or the selection itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AreaRequest {
    /// Select only this entity (a click).
    Select(EntityId),
    /// Add it to the selection, or take it out (Ctrl+click).
    Toggle(EntityId),
    /// Select every row from the last one picked to this one
    /// (Shift+click).
    SelectRange(EntityId),
    /// Delete this entity and everything below it.
    Delete(EntityId),
    /// Move an entity (and the rest of the selection, if it's selected)
    /// under another, or to the top level with `None` (drag and drop).
    Reparent {
        /// The entity dragged.
        entity: EntityId,
        /// Where it was dropped.
        onto: Option<EntityId>,
    },
    /// Give an entity a new name (typed after F2 or a double-click).
    Rename {
        /// The entity.
        entity: EntityId,
        /// Its new name.
        name: String,
    },
    /// Something picked from the panel's menus.
    Command(MenuCommand),
}

/// Where the name being typed is kept between frames.
fn rename_key() -> egui::Id {
    egui::Id::new("area-panel-rename")
}

/// Starts renaming `id` in the Area panel (F2), with `name` to edit.
pub(crate) fn begin_rename(ctx: &egui::Context, id: EntityId, name: &str) {
    ctx.data_mut(|d| d.insert_temp(rename_key(), (id, name.to_string(), true)));
}

/// The name being typed for an entity, if one is being renamed:
/// (entity, text, whether the box still needs the keyboard focus).
fn renaming(ctx: &egui::Context) -> Option<(EntityId, String, bool)> {
    ctx.data(|d| d.get_temp(rename_key()))
}

fn stop_renaming(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<(EntityId, String, bool)>(rename_key()));
}

/// The Area panel: every entity, as a tree. Click to select, Ctrl+click to
/// add or remove, Shift+click for a range, drag onto another to put it
/// under that one, or onto empty space to move it to the top level;
/// double-click to rename. Returns what the user asked for.
pub(crate) fn area_tree(
    ui: &mut Ui,
    document: &Document,
    models: &BTreeMap<AssetId, ModelInfo>,
    accent: Accent,
    state: &mut AreaPanelState,
    menu: &MenuState,
) -> Vec<AreaRequest> {
    let mut requests = Vec::new();
    if let Some(command) = menu_row(ui, state, menu) {
        requests.push(AreaRequest::Command(command));
    }
    if let Some(id) = state.reveal {
        // Unfold the way to it, so its row is there to scroll to.
        let mut parent = document.area().get(id).and_then(|e| e.parent);
        while let Some(p) = parent {
            state.collapsed.remove(&p);
            parent = document.area().get(p).and_then(|e| e.parent);
        }
    }
    let rows = tree_rows(document.area(), state);
    let scroll_to = state
        .reveal
        .take()
        .and_then(|id| rows.iter().position(|(r, _)| *r == id));
    let (hidden, locked) = state.in_effect(document.area());
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            let note = if models.is_empty() {
                "To see a model, use File > Import model, or drop a .gltf or .glb file onto \
                 the window. Entity > Add point light adds a lamp."
            } else {
                "Save the area with Ctrl+S; it's a text file Git can merge."
            };
            // Only the rows in view are drawn, so thousands of entities
            // cost no more than a screenful (CONTRIBUTING.md, rule 18).
            let mut scroll = egui::ScrollArea::vertical()
                .auto_shrink(false)
                .max_height((ui.available_height() - NOTE_HEIGHT).max(ROW_HEIGHT));
            if let Some(index) = scroll_to {
                // Its row in the middle of the list.
                let at = index as f32 * (ROW_HEIGHT + ROW_GAP);
                scroll = scroll.vertical_scroll_offset((at - 4.0 * ROW_HEIGHT).max(0.0));
            }
            let parents: HashSet<EntityId> = document
                .area()
                .iter()
                .filter_map(|(_, e)| e.parent)
                .collect();
            let out = scroll.show_rows(ui, ROW_HEIGHT, rows.len(), |ui, range| {
                ui.spacing_mut().item_spacing.y = ROW_GAP;
                for &(id, depth) in &rows[range] {
                    let Some(entity) = document.area().get(id) else {
                        continue;
                    };
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        ui.add_space(depth as f32 * INDENT);
                        fold_button(ui, id, parents.contains(&id), state);
                        let shown = Shown {
                            hidden: hidden.contains(&id),
                            locked: locked.contains(&id),
                        };
                        entity_row(
                            ui,
                            document,
                            models,
                            accent,
                            (id, entity),
                            shown,
                            state,
                            &mut requests,
                        );
                    });
                }
            });
            if rows.is_empty() && !state.search.is_empty() {
                ui.label(RichText::new("Nothing has that name.").color(theme::TEXT_WEAK));
            }
            // Dropping anywhere in the list but on a row moves to the top
            // level.
            let empty = ui.interact(
                out.inner_rect,
                ui.id().with("area-panel-empty"),
                egui::Sense::hover(),
            );
            if let Some(dragged) = empty.dnd_release_payload::<EntityId>() {
                requests.push(AreaRequest::Reparent {
                    entity: *dragged,
                    onto: None,
                });
            }
            ui.label(RichText::new(note).color(theme::TEXT_WEAK).size(11.0));
        });
    requests
}

/// Height of one row, and the gap between rows.
const ROW_HEIGHT: f32 = 21.0;
const ROW_GAP: f32 = 1.0;
/// Room kept under the list for its note.
const NOTE_HEIGHT: f32 = 40.0;

/// Every entity with how deep it sits, in the order the panel lists them:
/// depth-first, parents before children, leaving out what's folded away.
/// With a search, only names containing it are listed, each with its
/// parents (folded or not). A loop in a damaged area is cut where it comes
/// round again; entities caught in one come last.
fn tree_rows(area: &area::Area, state: &AreaPanelState) -> Vec<(EntityId, usize)> {
    let tree = area.hierarchy();
    let search = state.search.trim().to_lowercase();
    // With a search: the matches and everything above them.
    let wanted: Option<HashSet<EntityId>> = (!search.is_empty()).then(|| {
        let mut wanted = HashSet::new();
        for (id, entity) in area.iter() {
            if entity.name.to_lowercase().contains(&search) {
                let mut at = Some(id);
                while let Some(e) = at {
                    if !wanted.insert(e) {
                        break;
                    }
                    at = area.get(e).and_then(|d| d.parent);
                }
            }
        }
        wanted
    });
    let mut rows = Vec::with_capacity(area.len());
    let mut seen = HashSet::new();
    let mut todo: Vec<(EntityId, usize)> = tree
        .get(&None)
        .map(|top| top.iter().rev().map(|&id| (id, 0)).collect())
        .unwrap_or_default();
    while let Some((id, depth)) = todo.pop() {
        if !seen.insert(id) {
            continue;
        }
        if wanted.as_ref().is_some_and(|w| !w.contains(&id)) {
            continue;
        }
        rows.push((id, depth));
        let folded = wanted.is_none() && state.collapsed.contains(&id);
        if let Some(kids) = tree.get(&Some(id))
            && !folded
        {
            todo.extend(kids.iter().rev().map(|&k| (k, depth + 1)));
        }
    }
    if wanted.is_none() {
        // Entities caught in a loop: below nothing reachable, but listed.
        let reachable: HashSet<EntityId> = area.depth_first().into_iter().collect();
        rows.extend(
            area.iter()
                .map(|(id, _)| id)
                .filter(|id| !seen.contains(id) && !reachable.contains(id))
                .map(|id| (id, 0)),
        );
    }
    rows
}

/// The panel's menu row: ≡, the Edit, View and Entity menus, and the
/// search box. Returns the menu command picked.
fn menu_row(ui: &mut Ui, state: &mut AreaPanelState, menu: &MenuState) -> Option<MenuCommand> {
    let mut picked = None;
    theme::menu_row_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        egui::MenuBar::new().ui(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label("≡");
            if let Some(c) = ui
                .menu_button("Edit", |ui| menu_items::edit_items(ui, menu))
                .inner
                .flatten()
            {
                picked = Some(c);
            }
            if let Some(c) = ui
                .menu_button("View", |ui| {
                    let mut c = None;
                    if ui.button("Unfold everything").clicked() {
                        c = Some(MenuCommand::ExpandAll);
                        ui.close();
                    }
                    if ui.button("Fold everything").clicked() {
                        c = Some(MenuCommand::CollapseAll);
                        ui.close();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(
                            !state.hidden.is_empty(),
                            egui::Button::new("Show everything hidden"),
                        )
                        .clicked()
                    {
                        state.hidden.clear();
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            !state.locked.is_empty(),
                            egui::Button::new("Unlock everything"),
                        )
                        .clicked()
                    {
                        state.locked.clear();
                        ui.close();
                    }
                    c
                })
                .inner
                .flatten()
            {
                picked = Some(c);
            }
            if let Some(c) = ui
                .menu_button("Entity", |ui| menu_items::entity_items(ui, menu))
                .inner
                .flatten()
            {
                picked = Some(c);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let search = ui.add(
                    egui::TextEdit::singleline(&mut state.search)
                        .hint_text("Search")
                        .desired_width(ui.available_width().clamp(40.0, 160.0)),
                );
                search.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search the area")
                });
            });
        });
    });
    picked
}

/// The ▸ or ▾ before a row with children, which folds them away or
/// shows them; room for one otherwise.
fn fold_button(ui: &mut Ui, id: EntityId, has_children: bool, state: &mut AreaPanelState) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(12.0, ROW_HEIGHT), egui::Sense::click());
    if !has_children {
        return;
    }
    let folded = state.collapsed.contains(&id);
    let color = if response.hovered() {
        theme::TEXT_STRONG
    } else {
        theme::TEXT_WEAK
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        if folded { "▸" } else { "▾" },
        egui::FontId::proportional(11.0),
        color,
    );
    let name = if folded { "Unfold" } else { "Fold" };
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    if response.clicked() {
        if folded {
            state.collapsed.remove(&id);
        } else {
            state.collapsed.insert(id);
        }
    }
}

/// Whether a row's entity is hidden or locked (itself or by a parent).
#[derive(Debug, Clone, Copy, Default)]
struct Shown {
    hidden: bool,
    locked: bool,
}

/// One entity's row: its name (or the box to rename it), and what clicks,
/// drags and drops on it ask for.
#[allow(clippy::too_many_arguments)]
fn entity_row(
    ui: &mut Ui,
    document: &Document,
    models: &BTreeMap<AssetId, ModelInfo>,
    accent: Accent,
    (id, entity): (EntityId, &EntityData),
    shown: Shown,
    state: &mut AreaPanelState,
    requests: &mut Vec<AreaRequest>,
) {
    if let Some((renamed, mut text, focus)) = renaming(ui.ctx()).filter(|(r, ..)| *r == id) {
        let edit = ui.add(egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY));
        if focus {
            edit.request_focus();
        }
        let (enter, escape) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if escape {
            stop_renaming(ui.ctx());
        } else if edit.lost_focus() || enter {
            stop_renaming(ui.ctx());
            if text != entity.name {
                requests.push(AreaRequest::Rename {
                    entity: renamed,
                    name: text,
                });
            }
        } else {
            ui.ctx()
                .data_mut(|d| d.insert_temp(rename_key(), (renamed, text, false)));
        }
        return;
    }
    let (icon, color, note) = looks(entity, models, accent);
    let removable = entity.has("Model").then_some(entity.name.as_str());
    let toggles = Toggles {
        hidden: state.hidden.contains(&id),
        locked: state.locked.contains(&id),
        shown,
    };
    let (response, remove, toggled) = area_row(
        ui,
        (icon, color),
        &entity.name,
        &note,
        document.selection.contains(id),
        accent,
        removable,
        Some(toggles),
    );
    let switched = match toggled {
        Some(Toggle::Hide) => Some(&mut state.hidden),
        Some(Toggle::Lock) => Some(&mut state.locked),
        None => None,
    };
    if let Some(set) = switched
        && !set.remove(&id)
    {
        set.insert(id);
    }
    response.dnd_set_drag_payload(id);
    if let Some(dragged) = response.dnd_release_payload::<EntityId>() {
        if *dragged != id {
            requests.push(AreaRequest::Reparent {
                entity: *dragged,
                onto: Some(id),
            });
        }
    } else if response
        .dnd_hover_payload::<EntityId>()
        .is_some_and(|d| *d != id)
    {
        ui.painter().rect_stroke(
            response.rect,
            3.0,
            egui::Stroke::new(1.5, accent.colors().light),
            egui::StrokeKind::Inside,
        );
    }
    let modifiers = ui.input(|i| i.modifiers);
    if remove {
        requests.push(AreaRequest::Delete(id));
    } else if response.double_clicked() {
        begin_rename(ui.ctx(), id, &entity.name);
    } else if response.clicked() {
        requests.push(if modifiers.command {
            AreaRequest::Toggle(id)
        } else if modifiers.shift {
            AreaRequest::SelectRange(id)
        } else {
            AreaRequest::Select(id)
        });
    }
}
