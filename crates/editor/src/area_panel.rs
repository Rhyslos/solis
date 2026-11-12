//! The Area column: the area's entities as a tree (spec 197; the
//! approved additions, round 2, `05-area-column*.png`). Clicking one
//! selects it and shows it in Properties. The column's header holds the
//! search, Fold everything and a ⋯ menu; a right-click on a row opens its
//! menu; each row can be folded, and shows its hide and lock switches
//! while pointed at (or while they're on).

use crate::document::Document;
use crate::icons::{self, Icon};
use crate::menu_items::MenuCommand;
use crate::models::ModelInfo;
use crate::theme::{self, Accent};
use crate::tokens::{self, size, text};
use crate::widgets;
use area::EntityData;
use area::builtin::{Light, LightKind, Model};
use egui::{Color32, Rect, Ui};
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
    /// Whether the search field is open under the header.
    pub(crate) searching: bool,
    /// The rows as last listed, and what they were listed from.
    rows_kept: Option<KeptRows>,
}

/// The Area column's rows (each entity and how deep it is) and what they
/// were listed from: the area's tree (held, so it can't be another's), the
/// entities folded, what's searched for and, while searching, the area's
/// revision (names change without the tree). Listed again only when one
/// changes (the review, BUG-28: every frame listed the whole tree).
#[derive(Debug, Clone)]
struct KeptRows {
    tree: std::sync::Arc<area::Tree>,
    collapsed: HashSet<EntityId>,
    search: String,
    revision: Option<u64>,
    rows: std::sync::Arc<Vec<(EntityId, usize)>>,
}

impl AreaPanelState {
    /// The rows of `document`'s tree as this shows it, listed again only
    /// when what they're listed from changes.
    fn rows(&mut self, document: &Document) -> std::sync::Arc<Vec<(EntityId, usize)>> {
        let tree = document.tree();
        let revision = (!self.search.trim().is_empty()).then(|| document.revision());
        if let Some(kept) = &self.rows_kept
            && std::sync::Arc::ptr_eq(&kept.tree, &tree)
            && kept.revision == revision
            && kept.search == self.search
            && kept.collapsed == self.collapsed
        {
            return std::sync::Arc::clone(&kept.rows);
        }
        let rows = std::sync::Arc::new(tree_rows(document.area(), &tree, self));
        self.rows_kept = Some(KeptRows {
            tree,
            collapsed: self.collapsed.clone(),
            search: self.search.clone(),
            revision,
            rows: std::sync::Arc::clone(&rows),
        });
        rows
    }
}

impl AreaPanelState {
    /// Every entity hidden or locked, counting those under a hidden or
    /// locked one: (hidden, locked).
    pub(crate) fn in_effect(
        &self,
        document: &crate::document::Document,
    ) -> (HashSet<EntityId>, HashSet<EntityId>) {
        if self.hidden.is_empty() && self.locked.is_empty() {
            return Default::default();
        }
        let area = document.area();
        let mut hidden = HashSet::new();
        let mut locked = HashSet::new();
        for &id in document.tree().order() {
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

/// How faint a hidden entity's icon is.
const DIM: f32 = 0.4;

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

/// One row of the Area column, 24 tall: an icon, the name, a quiet note
/// after it, the accent-mixed fill when selected, and the hide and lock
/// switches at the right (shown while the row is pointed at, or while
/// they're on). Returns the row's response and which switch was clicked.
fn area_row(
    ui: &mut Ui,
    (icon, icon_color): (Icon, Color32),
    (name, note): (&str, &str),
    selected: bool,
    accent: Accent,
    toggles: Toggles,
) -> (egui::Response, Option<Toggle>) {
    let (row, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), size::row_h()),
        egui::Sense::click_and_drag(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, name)
    });
    // The fill reaches the column's edges, as the design's does.
    let full = row.with_min_x(ui.max_rect().left() - size::GAP);
    if selected {
        ui.painter()
            .rect_filled(full, 0.0, crate::tokens::select_row(accent));
    } else if response.hovered() {
        ui.painter().rect_filled(full, 0.0, tokens::row_hover());
    }
    let dim = toggles.shown.hidden;
    let icon_rect = Rect::from_center_size(
        egui::pos2(row.left() + size::ICON_SMALL / 2.0, row.center().y),
        egui::vec2(size::ROW_ICON, size::ROW_ICON),
    );
    icons::icon(
        ui.painter(),
        icon_rect,
        icon,
        if dim {
            icon_color.gamma_multiply(DIM)
        } else {
            icon_color
        },
    );
    let ink = if selected {
        tokens::text_emphasis()
    } else if dim {
        tokens::text_meta()
    } else {
        tokens::text_body()
    };
    let pointed = ui.rect_contains_pointer(row);
    let switches_w = 2.0 * (size::ROW_SWITCH + size::GAP);
    let words_room = Rect::from_min_max(
        egui::pos2(icon_rect.right() + size::GAP + 2.0, row.top()),
        egui::pos2(row.right() - switches_w, row.bottom()),
    );
    let drawn = widgets::words(
        ui,
        words_room,
        egui::Align2::LEFT_CENTER,
        name,
        text::row(),
        ink,
    );
    if !note.is_empty() {
        let after = words_room.with_min_x(drawn.right() + size::GAP);
        widgets::words(
            ui,
            after,
            egui::Align2::LEFT_CENTER,
            note,
            text::tile_kind(),
            tokens::text_meta(),
        );
    }
    let mut toggled = None;
    let mut x = row.right();
    let t = toggles;
    for (which, on, by_parent, icon_on, icon_off, label, hint) in [
        (
            Toggle::Hide,
            t.hidden,
            t.shown.hidden,
            Icon::Hidden,
            Icon::Eye,
            format!("Hide {name}"),
            "Hidden: not drawn in the editor (nor what's under it); the game still shows it",
        ),
        (
            Toggle::Lock,
            t.locked,
            t.shown.locked,
            Icon::Lock,
            Icon::Lock,
            format!("Lock {name}"),
            "Locked: clicks in the viewport pass through it (and what's under it)",
        ),
    ] {
        let spot = Rect::from_min_max(
            egui::pos2(x - size::ROW_SWITCH, row.top()),
            egui::pos2(x, row.bottom()),
        );
        x -= size::ROW_SWITCH + size::GAP;
        if !(pointed || on || by_parent) {
            continue;
        }
        let icon = if on || by_parent { icon_on } else { icon_off };
        if switch(ui, spot, (on, by_parent), icon, &label, hint) {
            toggled = Some(which);
        }
    }
    (response, toggled)
}

/// A small switch for hiding or locking, in `spot`: bright while on,
/// faint while on only because of a parent, quiet otherwise. Returns
/// whether it was clicked.
fn switch(
    ui: &mut Ui,
    spot: Rect,
    (on, by_parent): (bool, bool),
    icon: Icon,
    name: &str,
    hint: &str,
) -> bool {
    let response = ui.interact(spot, ui.id().with(name), egui::Sense::click());
    let color = if on {
        tokens::text_primary()
    } else if by_parent {
        tokens::text_faint()
    } else if response.hovered() {
        tokens::text_primary()
    } else {
        tokens::text_meta()
    };
    let square = Rect::from_center_size(spot.center(), egui::vec2(size::ROW_ICON, size::ROW_ICON));
    icons::icon(ui.painter(), square, icon, color);
    let response = response.on_hover_text(hint);
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, name));
    response.clicked()
}

/// An entity's icon and note, from its components. The icon's colour is
/// its kind's, as in the Library (decision 0060): a lamp shows its own
/// light's colour.
fn looks(entity: &EntityData, models: &BTreeMap<AssetId, ModelInfo>) -> (Icon, Color32, String) {
    if let Some(light) = entity.get::<Light>() {
        let [r, g, b] = light.color.map(|c| (c * 255.0).round() as u8);
        let color = if light.kind == LightKind::Sun {
            theme::kind::SUN
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
        return (Icon::Model, theme::kind::MODEL, note);
    }
    if let Some(sprite) = entity.get::<area::builtin::Sprite>() {
        let note = if sprite.sheet.is_some() {
            format!("frame {}", sprite.frame)
        } else {
            "no sheet".to_owned()
        };
        return (Icon::Sprite, theme::kind::SPRITE, note);
    }
    if entity.has("Camera") {
        return (Icon::Camera, theme::kind::CAMERA, "Camera".into());
    }
    if entity.has("Environment") {
        return (Icon::AreaFile, theme::kind::ENVIRONMENT, "sky, fog".into());
    }
    // The root of a placed assembly.
    if entity.origin.as_ref().is_some_and(|o| o.path.is_empty()) {
        return (Icon::Assembly, theme::kind::ASSEMBLY, String::new());
    }
    (Icon::Empty, theme::kind::OTHER, String::new())
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
    /// Place a model or assembly dragged from the Library under this
    /// entity.
    PlaceUnder {
        /// The asset.
        asset: AssetId,
        /// The entity it goes under.
        parent: EntityId,
    },
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

/// The Area column: every entity, as a tree. Click to select, Ctrl+click
/// to add or remove, Shift+click for a range, drag onto another to put it
/// under that one, or onto empty space to move it to the top level;
/// double-click to rename; right-click for the row's menu. Returns what
/// the user asked for.
pub(crate) fn area_tree(
    ui: &mut Ui,
    document: &Document,
    models: &BTreeMap<AssetId, ModelInfo>,
    accent: Accent,
    state: &mut AreaPanelState,
) -> Vec<AreaRequest> {
    let mut requests = Vec::new();
    if state.searching || !state.search.is_empty() {
        search_row(ui, state);
    }
    if let Some(id) = state.reveal {
        // Unfold the way to it, so its row is there to scroll to.
        for (parent, _) in document.area().ancestors(id) {
            state.collapsed.remove(&parent);
        }
    }
    let rows = state.rows(document);
    let tree = document.tree();
    let scroll_to = state
        .reveal
        .take()
        .and_then(|id| rows.iter().position(|(r, _)| *r == id));
    let (hidden, locked) = state.in_effect(document);
    let row_h = size::row_h();
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(
            size::GAP as i8,
            size::ROW_GAP as i8,
        ))
        .show(ui, |ui| {
            // Only the rows in view are drawn, so thousands of entities
            // cost no more than a screenful (CONTRIBUTING.md, rule 18).
            let mut scroll = egui::ScrollArea::vertical().auto_shrink(false);
            if let Some(index) = scroll_to {
                // Its row in the middle of the list.
                let at = index as f32 * row_h;
                scroll = scroll.vertical_scroll_offset((at - 4.0 * row_h).max(0.0));
            }
            let out = scroll.show_rows(ui, row_h, rows.len(), |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for &(id, depth) in &rows[range] {
                    let Some(entity) = document.area().get(id) else {
                        continue;
                    };
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.add_space(depth as f32 * size::TREE_LEVEL);
                        let parent = !tree.children(Some(id)).is_empty();
                        fold_button(ui, id, parent, state);
                        let shown = Shown {
                            hidden: hidden.contains(&id),
                            locked: locked.contains(&id),
                        };
                        entity_row(
                            ui,
                            (document, models),
                            accent,
                            (id, entity),
                            shown,
                            state,
                            &mut requests,
                        );
                    });
                }
            });
            if rows.is_empty() {
                let note = if state.search.is_empty() {
                    "Nothing here yet. Home \u{203a} Create adds an entity, a light or a camera."
                } else {
                    "Nothing has that name."
                };
                crate::panels::empty_note(ui, note);
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
        });
    requests
}

/// The search field under the header, while the search is open. Esc
/// empties and closes it.
fn search_row(ui: &mut Ui, state: &mut AreaPanelState) {
    let opening = state.searching && state.search.is_empty();
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(size::GAP as i8, size::GAP as i8))
        .show(ui, |ui| {
            let width = ui.available_width();
            let field = widgets::field::text_field(
                ui,
                "Search the area",
                &mut state.search,
                "Search by name",
                width,
            );
            if opening && !field.has_focus() && !field.lost_focus() {
                field.request_focus();
            }
            if field.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                state.search.clear();
                state.searching = false;
            }
        });
}

/// The tools in the Area column's header, from the right: the ⋯ menu,
/// Fold everything and the search. Returns a menu command picked.
pub(crate) fn header_tools(ui: &mut Ui, state: &mut AreaPanelState) -> Option<MenuCommand> {
    let mut picked = None;
    let more = widgets::button::icon(ui, Icon::More, "More of the Area column")
        .on_hover_text("Fold or unfold everything; show or unlock everything");
    egui::Popup::menu(&more).show(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if widgets::menu::item(ui, "Unfold everything", "").clicked() {
            picked = Some(MenuCommand::ExpandAll);
            ui.close();
        }
        if widgets::menu::item(ui, "Fold everything", "").clicked() {
            picked = Some(MenuCommand::CollapseAll);
            ui.close();
        }
        widgets::menu::line(ui);
        let any_hidden = !state.hidden.is_empty();
        let show = ui.add_enabled_ui(any_hidden, |ui| {
            widgets::menu::item(ui, "Show everything hidden", "")
        });
        if show
            .inner
            .on_disabled_hover_text("Nothing is hidden")
            .clicked()
        {
            state.hidden.clear();
            ui.close();
        }
        let any_locked = !state.locked.is_empty();
        let unlock = ui.add_enabled_ui(any_locked, |ui| {
            widgets::menu::item(ui, "Unlock everything", "")
        });
        if unlock
            .inner
            .on_disabled_hover_text("Nothing is locked")
            .clicked()
        {
            state.locked.clear();
            ui.close();
        }
    });
    if widgets::button::icon(ui, Icon::FoldAll, "Fold everything")
        .on_hover_text("Folds every entity with others under it")
        .clicked()
    {
        picked = Some(MenuCommand::CollapseAll);
    }
    let search = widgets::button::icon(ui, Icon::Search, "Search the area by name")
        .on_hover_text("Lists only the entities whose names have the words typed");
    if search.clicked() {
        state.searching = !state.searching;
        if !state.searching {
            state.search.clear();
        }
    }
    picked
}

/// A row's right-click menu (the approved additions, round 2,
/// `05-area-column-menu.png`): the commands the old Edit and Entity menus
/// had. The row right-clicked is selected, so each has something to act
/// on.
fn row_menu(ui: &mut Ui) -> Option<MenuCommand> {
    use MenuCommand as M;
    ui.spacing_mut().item_spacing.y = 0.0;
    let groups: [&[(&str, &str, MenuCommand)]; 4] = [
        &[
            ("Rename", "F2", M::Rename),
            ("Duplicate", "Ctrl+D", M::Duplicate),
            ("Delete", "Delete", M::Delete),
        ],
        &[
            ("Add entity", "", M::AddEntity),
            ("Add point light", "", M::AddPointLight),
            ("Add spotlight", "", M::AddSpotlight),
            ("Add camera", "", M::AddCamera),
        ],
        &[("Create assembly from selection", "", M::CreateAssembly)],
        &[
            ("Select all", "Ctrl+A", M::SelectAll),
            ("Select nothing", "Esc", M::SelectNone),
        ],
    ];
    let mut picked = None;
    for (n, group) in groups.iter().enumerate() {
        if n > 0 {
            widgets::menu::line(ui);
        }
        for &(words, key, command) in *group {
            if widgets::menu::item(ui, words, key).clicked() {
                picked = Some(command);
                ui.close();
            }
        }
    }
    picked
}

/// Every entity with how deep it sits, in the order the panel lists them:
/// depth-first, parents before children, leaving out what's folded away.
/// With a search, only names containing it are listed, each with its
/// parents (folded or not). A loop in a damaged area is cut where it comes
/// round again; entities caught in one come last.
fn tree_rows(
    area: &area::Area,
    tree: &area::Tree,
    state: &AreaPanelState,
) -> Vec<(EntityId, usize)> {
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
        .children(None)
        .iter()
        .rev()
        .map(|&id| (id, 0))
        .collect();
    while let Some((id, depth)) = todo.pop() {
        if !seen.insert(id) {
            continue;
        }
        if wanted.as_ref().is_some_and(|w| !w.contains(&id)) {
            continue;
        }
        rows.push((id, depth));
        let folded = wanted.is_none() && state.collapsed.contains(&id);
        if !folded {
            todo.extend(
                tree.children(Some(id))
                    .iter()
                    .rev()
                    .map(|&k| (k, depth + 1)),
            );
        }
    }
    if wanted.is_none() {
        // Entities caught in a loop: below nothing reachable, but listed.
        rows.extend(
            area.iter()
                .map(|(id, _)| id)
                .filter(|&id| !seen.contains(&id) && tree.in_loop(id))
                .map(|id| (id, 0)),
        );
    }
    rows
}

/// The ▸ or ▾ before a row with children, which folds them away or
/// shows them; room for one otherwise.
fn fold_button(ui: &mut Ui, id: EntityId, has_children: bool, state: &mut AreaPanelState) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(size::TREE_FOLD_W, size::row_h()),
        egui::Sense::click(),
    );
    if !has_children {
        return;
    }
    let folded = state.collapsed.contains(&id);
    let color = if response.hovered() {
        tokens::text_primary()
    } else {
        tokens::text_meta()
    };
    let icon = if folded {
        Icon::ChevronRight
    } else {
        Icon::ChevronDown
    };
    let mark = Rect::from_center_size(rect.center(), egui::vec2(size::FOLD_ICON, size::FOLD_ICON));
    icons::icon(ui.painter(), mark, icon, color);
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
    (document, models): (&Document, &BTreeMap<AssetId, ModelInfo>),
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
    let (icon, color, note) = looks(entity, models);
    let toggles = Toggles {
        hidden: state.hidden.contains(&id),
        locked: state.locked.contains(&id),
        shown,
    };
    let selected = document.selection.contains(id);
    let (response, toggled) = area_row(
        ui,
        (icon, color),
        (&entity.name, &note),
        selected,
        accent,
        toggles,
    );
    if response.secondary_clicked() && !selected {
        // The menu acts on the selection: the row pointed at becomes it.
        requests.push(AreaRequest::Select(id));
    }
    response.context_menu(|ui| {
        if let Some(command) = row_menu(ui) {
            requests.push(AreaRequest::Command(command));
        }
    });
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
    let from_library = response
        .dnd_hover_payload::<crate::library::LibraryDrag>()
        .filter(|d| d.0.kind.placeable());
    if let Some(dropped) = response.dnd_release_payload::<crate::library::LibraryDrag>()
        && let Some(asset) = dropped.0.id.filter(|_| dropped.0.kind.placeable())
    {
        requests.push(AreaRequest::PlaceUnder { asset, parent: id });
    } else if from_library.is_some() {
        ui.painter().rect_stroke(
            response.rect,
            3.0,
            egui::Stroke::new(1.5, accent.colors().light),
            egui::StrokeKind::Inside,
        );
    } else if let Some(dragged) = response.dnd_release_payload::<EntityId>() {
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
    if response.double_clicked() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use area::Origin;

    fn color_of(entity: &EntityData) -> Color32 {
        looks(entity, &BTreeMap::new()).1
    }

    #[test]
    fn icons_take_their_kind_s_colour_whatever_the_accent() {
        // Decision 0060: the same colour as the Library's stripe.
        assert_eq!(
            color_of(&EntityData::new("Ship").with(&Model::default())),
            theme::kind::MODEL
        );
        assert_eq!(
            color_of(&EntityData::new("Sun").with(&Light::sun())),
            theme::kind::SUN
        );
        let mut camera = EntityData::new("Camera");
        camera
            .components
            .insert("Camera".into(), reflect::ComponentData::default());
        assert_eq!(color_of(&camera), theme::kind::CAMERA);
        assert_eq!(color_of(&EntityData::new("Empty")), theme::kind::OTHER);
        let walker = EntityData::new("Walker").with(&area::builtin::Sprite::default());
        assert_eq!(color_of(&walker), theme::kind::SPRITE);
        assert_eq!(looks(&walker, &BTreeMap::new()).2, "no sheet");
    }

    #[test]
    fn a_lamp_shows_its_own_light_s_colour() {
        let mut red = Light::point();
        red.color = [1.0, 0.0, 0.0];
        assert_eq!(
            color_of(&EntityData::new("Lamp").with(&red)),
            Color32::from_rgb(255, 0, 0)
        );
    }

    #[test]
    fn a_placed_assembly_s_root_is_coloured_as_an_assembly() {
        let instance = EntityId::random();
        let mut root = EntityData::new("Fighter");
        root.origin = Some(Origin {
            assembly: AssetId::random(),
            instance,
            path: Vec::new(),
        });
        assert_eq!(color_of(&root), theme::kind::ASSEMBLY);
        // A plain entity inside it is not the assembly itself.
        let mut part = root.clone();
        part.origin.as_mut().unwrap().path = vec![EntityId::random()];
        assert_eq!(color_of(&part), theme::kind::OTHER);
    }
}

#[cfg(test)]
mod rows_tests {
    use super::*;
    use area::Edit;
    use std::sync::Arc;

    /// A document with a room holding a lamp, and a box.
    fn room() -> (Document, EntityId, EntityId) {
        let mut d = Document::default();
        let (room, lamp) = (EntityId::random(), EntityId::random());
        let mut lit = EntityData::new("Lamp");
        lit.parent = Some(room);
        d.edit(
            "Add",
            vec![
                Edit::Spawn {
                    entities: vec![(room, EntityData::new("Room")), (lamp, lit)],
                },
                Edit::Spawn {
                    entities: vec![(EntityId::random(), EntityData::new("Box"))],
                },
            ],
            None,
            None,
        )
        .unwrap();
        (d, room, lamp)
    }

    /// BUG-28: the Area column listed the whole tree again every frame.
    #[test]
    fn the_rows_are_listed_again_only_when_what_they_show_changes() {
        let (mut d, room, lamp) = room();
        let mut state = AreaPanelState::default();
        let rows = state.rows(&d);
        assert!(rows.iter().any(|&(id, depth)| id == lamp && depth == 1));
        for _ in 0..3 {
            assert!(Arc::ptr_eq(&rows, &state.rows(&d)), "kept");
        }
        // Renaming changes the area, not its tree: still kept.
        d.edit(
            "Rename",
            vec![Edit::Rename {
                entity: lamp,
                name: "Light".into(),
            }],
            None,
            None,
        )
        .unwrap();
        assert!(Arc::ptr_eq(&rows, &state.rows(&d)));
        // Folding the room: listed again, without the lamp.
        state.collapsed.insert(room);
        let folded = state.rows(&d);
        assert!(!folded.iter().any(|&(id, _)| id == lamp));
        assert!(Arc::ptr_eq(&folded, &state.rows(&d)));
        // Searching: what's found, and again when a name changes.
        state.search = "lig".into();
        let found = state.rows(&d);
        assert_eq!(
            found.iter().map(|&(id, _)| id).collect::<Vec<_>>(),
            [room, lamp]
        );
        assert!(Arc::ptr_eq(&found, &state.rows(&d)));
        d.edit(
            "Rename",
            vec![Edit::Rename {
                entity: lamp,
                name: "Lamp".into(),
            }],
            None,
            None,
        )
        .unwrap();
        assert!(state.rows(&d).is_empty(), "nothing called that now");
        // A new entity: listed again.
        state.search.clear();
        let before = state.rows(&d);
        d.edit(
            "Add",
            vec![Edit::Spawn {
                entities: vec![(EntityId::random(), EntityData::new("Crate"))],
            }],
            None,
            None,
        )
        .unwrap();
        assert_eq!(state.rows(&d).len(), before.len() + 1);
    }
}
