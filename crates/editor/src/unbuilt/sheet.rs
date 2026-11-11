//! The placeholder sheet: one page with unfinished parts among working
//! ones, in every shape the frame has, for the tests to walk and take
//! pictures of. Nothing in the editor shows it.
//!
//! It holds the four cases the project lead approved the look on (a
//! ribbon with unbuilt buttons among real ones, a shelf bar with an
//! unbuilt tab, a setting row, and a whole page's card), laid out as a
//! small editor so each is on the surface the design draws it on. The
//! working parts are stand-ins drawn here: the real ribbon, shelf and
//! File page arrive with their milestones.
//!
//! Every list on it is made of [`Entry::Built`] and [`Entry::Unbuilt`]
//! and goes through [`super::filter`], so the sheet also shows what
//! "Hide unfinished parts" does: the unbuilt entries go, a group left
//! empty goes with them, and what is left closes up.

use super::{Entry, RowKind, UnbuiltId, filter, filter_groups};
use crate::icons::{self, Icon};
use crate::tokens::{self, size, text};
use crate::widgets::{self, rows, toggle, width_of, words};
use egui::{Align, Align2, Layout, Rect, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2};

/// The sheet's size, in points.
pub(crate) const SIZE: [f32; 2] = [1340.0, 600.0];
/// Where the File menu column ends and the settings begin.
pub(crate) const MENU_W: f32 = 180.0;
/// Where the settings end and the right side begins.
pub(crate) const RIGHT_X: f32 = 900.0;
/// Where the page ends and the shelf column under it begins.
pub(crate) const PAGE_FOOT: f32 = 340.0;

/// A working large ribbon button: its icon and its name.
type Big = (Icon, &'static str);

/// The sheet's ribbon: each group's label and what is in it. An unbuilt
/// entry is a large button or a small row, as [`row_kind_of`] says.
fn ribbon_groups() -> Vec<(&'static str, Vec<Entry<Big>>)> {
    use Entry::{Built, Unbuilt};
    vec![
        (
            "Create",
            vec![
                Built((Icon::Model, "Model")),
                Built((Icon::Light, "Light")),
                Unbuilt(UnbuiltId::PlacePaintScatter),
            ],
        ),
        (
            "Lighting",
            vec![
                Unbuilt(UnbuiltId::HomeLightingBake),
                Unbuilt(UnbuiltId::HomeLightingQuality),
                Unbuilt(UnbuiltId::HomeLightingLightmapView),
            ],
        ),
        (
            "Project",
            vec![Built((Icon::Save, "Save")), Built((Icon::Build, "Build"))],
        ),
        (
            "Play",
            vec![
                Built((Icon::Play, "Play")),
                Unbuilt(UnbuiltId::ShellPlaySimulate),
            ],
        ),
    ]
}

/// The kind of small ribbon row an unbuilt part of the sheet is, if it
/// is a row and not a large button.
fn row_kind_of(id: UnbuiltId) -> Option<RowKind> {
    match id {
        UnbuiltId::HomeLightingQuality => Some(RowKind::List),
        UnbuiltId::HomeLightingLightmapView => Some(RowKind::Check),
        _ => None,
    }
}

/// The icon an unbuilt part of the sheet is drawn with.
fn icon_of(id: UnbuiltId) -> Icon {
    match id {
        UnbuiltId::PlacePaintScatter => Icon::Assembly,
        UnbuiltId::HomeLightingBake => Icon::Sun,
        UnbuiltId::ShellPlaySimulate => Icon::Simulate,
        UnbuiltId::HomeToolPose => Icon::PoseBones,
        UnbuiltId::HomeToolMeasure => Icon::Measure,
        UnbuiltId::FilePlugins => Icon::Plugin,
        UnbuiltId::HomeShelfTransport => Icon::NextKey,
        _ => Icon::Material,
    }
}

/// A stand-in for a working control: `label` in a box of `size`, found
/// by its name like the real one will be.
fn stand_in(ui: &mut Ui, wanted: egui::Vec2, label: &str) -> Rect {
    let (rect, response) = ui.allocate_exact_size(wanted, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    rect
}

/// A working large ribbon button, as the design draws one.
fn big(ui: &mut Ui, (icon, label): Big) {
    let [width, height] = size::RIBBON_BIG;
    let rect = stand_in(ui, vec2(width, height), label);
    let stack = size::RIBBON_BIG_ICON + size::RIBBON_BIG_GAP + text::RIBBON_LABEL_SIZE + 2.0;
    let spot = Rect::from_min_size(
        pos2(
            rect.center().x - size::RIBBON_BIG_ICON / 2.0,
            rect.center().y - stack / 2.0,
        ),
        vec2(size::RIBBON_BIG_ICON, size::RIBBON_BIG_ICON),
    );
    icons::icon(ui.painter(), spot, icon, tokens::text_body());
    let room = Rect::from_min_max(
        pos2(rect.left(), spot.bottom() + size::RIBBON_BIG_GAP),
        rect.right_bottom(),
    );
    let font = text::ribbon_label();
    words(
        ui,
        room,
        Align2::CENTER_TOP,
        label,
        font,
        tokens::text_body(),
    );
}

/// A working tab `height` tall with `pad` at each side; `front` lifts it.
fn tab(ui: &mut Ui, label: &str, height: f32, pad: f32, front: Option<egui::Color32>) {
    let font = if front.is_some() {
        text::tab_active()
    } else {
        text::tab()
    };
    let width = width_of(ui, label, font.clone()) + 2.0 * pad;
    let rect = stand_in(ui, vec2(width, height), label);
    let ink = match front {
        Some(fill) => {
            ui.painter().rect_filled(rect, size::radius::TAB, fill);
            tokens::text_emphasis()
        }
        None => tokens::text_label(),
    };
    words(ui, rect, Align2::CENTER_CENTER, label, font, ink);
}

/// A `Ui` over `rect`, laid out along a row.
fn strip(ui: &mut Ui, rect: Rect, pad: f32) -> Ui {
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink2(vec2(pad, 0.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.spacing_mut().item_spacing = vec2(0.0, 0.0);
    child
}

/// A `Ui` over `rect`, laid out downwards.
fn stack(ui: &mut Ui, rect: Rect, pad: f32) -> Ui {
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink(pad))
            .layout(Layout::top_down(Align::Min)),
    );
    child.spacing_mut().item_spacing = vec2(0.0, 0.0);
    child
}

/// What a click on the sheet asked for.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Asked {
    /// The page's "Hide unfinished parts" link was clicked.
    pub(crate) hide: bool,
}

/// Draws the whole sheet from the window's top left corner.
pub(crate) fn show(ui: &mut Ui) -> Asked {
    let whole = Rect::from_min_size(pos2(0.0, 0.0), vec2(SIZE[0], SIZE[1]));
    let (tabs, rest) = whole.split_top_bottom_at_y(size::TAB_BAR_H);
    let (ribbon, rest) = rest.split_top_bottom_at_y(tabs.bottom() + size::RIBBON_H);
    let (rest, status) = rest.split_top_bottom_at_y(whole.bottom() - size::STATUS_BAR_H);
    let (body, shelf) = rest.split_top_bottom_at_y(rest.bottom() - size::SHELF_BAR_H);
    let (menu, body) = body.split_left_right_at_x(MENU_W);
    let (settings, right) = body.split_left_right_at_x(RIGHT_X);
    let (page, column) = right.split_top_bottom_at_y(PAGE_FOOT);
    let painter = ui.painter().clone();
    painter.rect_filled(tabs, 0.0, tokens::app());
    painter.rect_filled(ribbon, 0.0, tokens::ribbon());
    painter.rect_filled(menu, 0.0, tokens::ribbon());
    painter.rect_filled(settings, 0.0, tokens::shelf_bar());
    painter.rect_filled(column, 0.0, tokens::panel());
    painter.rect_filled(shelf, 0.0, tokens::shelf_bar());
    painter.rect_filled(status, 0.0, tokens::code());
    tab_bar(&mut strip(ui, tabs, size::TAB_BAR_PAD));
    // The buttons' band: the ribbon less its padding and the group label.
    let [top, _, _] = size::RIBBON_GROUP_PAD;
    let buttons = Rect::from_min_size(
        pos2(ribbon.left(), ribbon.top() + top),
        vec2(ribbon.width(), size::RIBBON_BIG[1]),
    );
    let mut band = strip(ui, buttons, 0.0);
    ribbon_strip(&mut band, ribbon);
    file_menu(&mut stack(ui, menu, size::TAB_BAR_PAD));
    settings_page(&mut stack(ui, settings, size::FILE_PAGE_PAD[0]));
    let asked = Asked {
        hide: unbuilt_page(ui, page),
    };
    shelf_column(ui, column);
    shelf_bar(&mut strip(ui, shelf, size::TAB_BAR_PAD));
    status_bar(&mut strip(ui, status, size::ROW_PAD));
    asked
}

fn tab_bar(ui: &mut Ui) {
    use Entry::{Built, Unbuilt};
    let tabs = filter(vec![
        Built(("Home", true)),
        Built(("Library", false)),
        Unbuilt(UnbuiltId::ShellTabShading),
        Built(("Animate", false)),
        Unbuilt(UnbuiltId::ShellTabVfx),
    ]);
    for entry in tabs {
        match entry {
            Built((label, front)) => {
                let fill = front.then(tokens::ribbon);
                tab(ui, label, size::RIBBON_TAB_H, size::RIBBON_TAB_PAD, fill);
            }
            Unbuilt(id) => drop(super::tab(ui, id)),
        }
    }
}

fn ribbon_strip(ui: &mut Ui, band: Rect) {
    for (label, entries) in filter_groups(ribbon_groups()) {
        let [_, side, foot] = size::RIBBON_GROUP_PAD;
        let started = ui.cursor().left();
        ui.add_space(side);
        ui.spacing_mut().item_spacing.x = size::RIBBON_GAP;
        let rows: Vec<(UnbuiltId, RowKind)> = entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Unbuilt(id) => row_kind_of(*id).map(|kind| (*id, kind)),
                Entry::Built(_) => None,
            })
            .collect();
        for entry in entries {
            match entry {
                Entry::Built(button) => big(ui, button),
                Entry::Unbuilt(id) if row_kind_of(id).is_none() => {
                    super::ribbon_big(ui, id, icon_of(id));
                }
                Entry::Unbuilt(_) => {}
            }
        }
        if !rows.is_empty() {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                for (id, kind) in rows {
                    super::ribbon_row(ui, id, kind);
                }
            });
        }
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(side - size::RIBBON_GAP);
        // The group's label under its buttons, and the line at its right.
        let ended = ui.cursor().left();
        let label_h = text::RIBBON_LABEL_SIZE + 2.0;
        let caps = text::caps(label, text::GROUP_LABEL, tokens::text_meta());
        let galley = ui.painter().layout_job(caps);
        let at = pos2(
            (started + ended) / 2.0 - galley.size().x / 2.0,
            band.bottom() - foot - label_h,
        );
        ui.painter().galley(at, galley, tokens::text_meta());
        tokens::vline(
            ui.painter(),
            ended - size::LINE,
            band.y_range(),
            tokens::line(),
        );
    }
}

fn file_menu(ui: &mut Ui) {
    use Entry::{Built, Unbuilt};
    let items = filter(vec![
        Built((Icon::Gear, "Project settings")),
        Built((Icon::Sliders, "Preferences")),
        Unbuilt(UnbuiltId::FilePlugins),
    ]);
    for entry in items {
        match entry {
            Built((icon, label)) => {
                let wanted = vec2(ui.available_width(), size::FILE_MENU_ITEM_H);
                let rect = stand_in(ui, wanted, label);
                let spot = Rect::from_min_size(
                    pos2(
                        rect.left() + size::FILE_MENU_ITEM_PAD,
                        rect.center().y - size::TOOL_ICON / 2.0,
                    ),
                    vec2(size::TOOL_ICON, size::TOOL_ICON),
                );
                icons::icon(ui.painter(), spot, icon, tokens::text_label());
                let room = Rect::from_min_max(
                    pos2(spot.right() + size::FILE_MENU_ITEM_GAP, rect.top()),
                    rect.right_bottom(),
                );
                let font = text::menu_item();
                words(
                    ui,
                    room,
                    Align2::LEFT_CENTER,
                    label,
                    font,
                    tokens::text_label(),
                );
            }
            Unbuilt(id) => drop(super::menu_item(ui, id, icon_of(id))),
        }
    }
    // The toolbox's buttons, in a row here.
    ui.add_space(size::SECTION_GAP);
    ui.horizontal(|ui| {
        let tools = filter(vec![
            Built(Icon::Select),
            Built(Icon::Move),
            Unbuilt(UnbuiltId::HomeToolPose),
            Unbuilt(UnbuiltId::HomeToolMeasure),
        ]);
        for entry in tools {
            match entry {
                Built(icon) => {
                    let side = size::TOOL_BUTTON;
                    let rect = stand_in(ui, vec2(side, side), icon.name());
                    let spot = Rect::from_center_size(
                        rect.center(),
                        vec2(size::TOOL_ICON, size::TOOL_ICON),
                    );
                    icons::icon(ui.painter(), spot, icon, tokens::text_label());
                }
                Unbuilt(id) => drop(super::tool(ui, id, icon_of(id))),
            }
        }
    });
}

fn settings_page(ui: &mut Ui) {
    use Entry::{Built, Unbuilt};
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = size::SUB_TAB_GAP;
        let tabs = filter(vec![
            Built("General"),
            Built("Appearance"),
            Unbuilt(UnbuiltId::PrefsTabInput),
        ]);
        for entry in tabs {
            match entry {
                Built(label) => tab(ui, label, size::SUB_TAB_H, size::SUB_TAB_PAD, None),
                Unbuilt(id) => drop(super::sub_tab(ui, id)),
            }
        }
    });
    ui.add_space(size::PAGE_BLOCK_GAP);
    rows::section_caps(ui, "Layout");
    let settings = filter(vec![
        Built("Show ribbon labels"),
        Unbuilt(UnbuiltId::PrefsCodeSplit),
    ]);
    for entry in settings {
        match entry {
            Built(name) => {
                let hint = "Turn off to show icons only.";
                let mut on = true;
                rows::setting_row(ui, name, hint, |ui| toggle::toggle(ui, &mut on, name));
            }
            Unbuilt(id) => drop(super::setting_row(ui, id)),
        }
    }
    ui.add_space(size::SECTION_GAP);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = size::CARD_GAP;
        for entry in filter(vec![Built("Harbour"), Unbuilt(UnbuiltId::PlaceChipRecent)]) {
            match entry {
                Built(label) => {
                    let kind = widgets::chip::ChipKind::Quiet;
                    widgets::chip::chip(ui, kind, label, true, None);
                }
                Unbuilt(id) => drop(super::chip(ui, id)),
            }
        }
        for entry in filter(vec![Entry::<()>::Unbuilt(UnbuiltId::RecentTemplateAll)]) {
            if let Unbuilt(id) = entry {
                super::link(ui, id);
            }
        }
    });
}

fn unbuilt_page(ui: &mut Ui, rect: Rect) -> bool {
    let pages = filter(vec![Entry::<()>::Unbuilt(UnbuiltId::ShadingGraph)]);
    let mut inside = ui.new_child(UiBuilder::new().max_rect(rect));
    inside.set_clip_rect(rect);
    match pages.first() {
        Some(Entry::Unbuilt(id)) => super::page(&mut inside, *id, Icon::Material).hide,
        _ => {
            // With unfinished parts hidden the page is gone, and what is
            // real has the room.
            inside.painter().rect_filled(rect, 0.0, tokens::viewport());
            false
        }
    }
}

fn shelf_column(ui: &mut Ui, rect: Rect) {
    let (left, right) = rect.split_left_right_at_x(rect.center().x);
    let mut fields = stack(ui, left, size::ROW_PAD);
    fields.spacing_mut().item_spacing.y = size::CARD_INNER_GAP;
    rows::key_value(&mut fields, "Align to", "Grid");
    for entry in filter(vec![
        Entry::<()>::Unbuilt(UnbuiltId::PlaceRotation),
        Entry::Unbuilt(UnbuiltId::ProjVersionControl),
    ]) {
        match entry {
            Entry::Unbuilt(UnbuiltId::PlaceRotation) => {
                drop(super::field_row(&mut fields, UnbuiltId::PlaceRotation))
            }
            Entry::Unbuilt(id) => drop(super::card(&mut fields, id)),
            Entry::Built(()) => {}
        }
    }
    let columns = filter(vec![Entry::<()>::Unbuilt(UnbuiltId::LibShelfImportQueue)]);
    if let Some(Entry::Unbuilt(id)) = columns.first() {
        tokens::vline(ui.painter(), right.left(), right.y_range(), tokens::line());
        let mut inside = ui.new_child(UiBuilder::new().max_rect(right));
        super::column(&mut inside, *id);
    }
}

fn shelf_bar(ui: &mut Ui) {
    use Entry::{Built, Unbuilt};
    let tabs = filter(vec![
        Built(("Entity", true)),
        Unbuilt(UnbuiltId::HomeShelfAnimation),
        Built(("Console", false)),
    ]);
    ui.spacing_mut().item_spacing.x = size::TAB_BAR_GAP;
    for entry in tabs {
        match entry {
            Built((label, front)) => {
                let fill = front.then(tokens::line);
                tab(ui, label, size::shelf_tab_h(), size::ROW_PAD, fill);
            }
            Unbuilt(id) => drop(super::shelf_tab(ui, id)),
        }
    }
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        for entry in filter(vec![Entry::<()>::Unbuilt(UnbuiltId::HomeShelfTransport)]) {
            if let Unbuilt(id) = entry {
                super::icon_button(ui, id, icon_of(id));
            }
        }
    });
}

fn status_bar(ui: &mut Ui) {
    use Entry::{Built, Unbuilt};
    ui.spacing_mut().item_spacing.x = size::STATUS_GAP;
    for entry in filter(vec![Built("Ready"), Unbuilt(UnbuiltId::ShellStatusBranch)]) {
        match entry {
            Built(label) => {
                let font = text::ribbon_small();
                let width = width_of(ui, label, font.clone());
                let rect = stand_in(ui, vec2(width, size::STATUS_BAR_H), label);
                words(
                    ui,
                    rect,
                    Align2::LEFT_CENTER,
                    label,
                    font,
                    tokens::text_label(),
                );
            }
            Unbuilt(id) => drop(super::status_item(ui, id)),
        }
    }
    // What a click on an unfinished part says, at the right.
    if let Some(said) = super::said(ui.ctx()) {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(said)
                        .font(text::ribbon_small())
                        .color(tokens::text_label()),
                )
                .selectable(false),
            );
        });
    }
}

#[cfg(test)]
#[path = "sheet_tests.rs"]
mod tests;
