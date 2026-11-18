//! The bottom panel (decision 0118, board 05): small tabs of one panel
//! (on Home the Library, the Console and Problems, each with its count)
//! under the centre only. Open, a 30 px bar (the tabs at the left; the
//! tools of the tab in front and ▾ at the right) over the tab in front,
//! its top edge dragged to any height. Folded, one 30 px line naming the
//! tabs with their counts, the newest warning or error faintly beside
//! them, and "Click a name to open it". A name opens its tab at its last
//! height; ▾, a click on the tab in front, or Ctrl + Space folds it.
//!
//! The command line is the Console's own, so it shows only on the
//! Console tab. While the Library is placing, its tab shows how placed
//! things land beside the files, the line between them dragged.

use super::layouts::{Border, Measure, PanelLayout};
use super::sizes::Edge;
use super::workspaces::{BottomTab, Column, Slot};
use super::{RibbonTab, Shell};
use crate::console::LogLevel;
use crate::icons::{self, Icon};
use crate::panel::Panel;
use crate::panels::PanelViewer;
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::widgets;
use egui::{Align, Color32, Layout, Rect, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2};

/// What the bottom panel holds this frame.
pub(crate) struct BottomNow {
    /// Its tabs, through the unfinished parts' filter (the Library left
    /// out where the layout puts it in a column).
    pub(crate) tabs: Vec<Entry<BottomTab>>,
    /// The slots hidden on this tab, which its bar offers to show again.
    pub(crate) hidden: Vec<(Slot, Column)>,
    /// The layout in use, for the placing column's edge.
    pub(crate) layout: PanelLayout,
}

impl BottomNow {
    /// Its tabs that work.
    fn built(&self) -> Vec<BottomTab> {
        self.tabs
            .iter()
            .filter_map(|e| match e {
                Entry::Built(t) => Some(*t),
                Entry::Unbuilt(_) => None,
            })
            .collect()
    }
}

/// What was clicked on the bar.
#[derive(Default)]
struct BarAsks {
    /// A tab's name.
    tab: Option<BottomTab>,
    /// ▾ or ▴: fold, or open the tab in front.
    fold: Option<bool>,
    /// A hidden slot to show again.
    show: Option<Slot>,
    /// The Library's Place switch.
    place: bool,
}

/// Draws the bottom panel into `rect` (bar and content).
pub(crate) fn bottom(
    ui: &mut Ui,
    rect: Rect,
    shell: &mut Shell,
    now: &BottomNow,
    viewer: &mut PanelViewer<'_>,
) {
    let tab = shell.tab;
    let built = now.built();
    let arrangement = shell.arrangement(tab);
    let front = arrangement.bottom_tab_of(&built);
    let folded = arrangement.bottom_folded;
    let bar = rect.with_max_y(rect.top() + size::BOTTOM_BAR_H);
    bar_background(ui, bar);
    let asks = if folded {
        folded_line(ui, bar, now, front, viewer)
    } else {
        open_bar(ui, bar, (tab, now), front, viewer)
    };
    let mut front = front;
    if let Some(chosen) = asks.tab {
        let arrangement = shell.arrangement_mut(tab);
        if !folded && Some(chosen) == front {
            arrangement.bottom_folded = true;
        } else {
            arrangement.bottom_tab = chosen;
            arrangement.bottom_folded = false;
            front = Some(chosen);
        }
        if chosen != BottomTab::Library {
            arrangement.placing = false;
        }
    }
    if let Some(fold) = asks.fold {
        shell.arrangement_mut(tab).bottom_folded = fold;
    }
    if asks.place {
        let arrangement = shell.arrangement_mut(tab);
        arrangement.placing = !arrangement.placing;
    }
    if let Some(slot) = asks.show {
        shell.arrangement_mut(tab).hidden.set(slot, false);
    }
    let content = rect.with_min_y(bar.bottom());
    if folded || content.height() < 1.0 {
        return;
    }
    if let Some(front) = front {
        tab_content(ui, content, front, shell, now.layout, viewer);
    }
}

/// The bar's fill and its lines above and below.
fn bar_background(ui: &Ui, bar: Rect) {
    let painter = ui.painter();
    painter.rect_filled(bar, 0.0, tokens::shelf_bar());
    let ppp = ui.ctx().pixels_per_point();
    tokens::hline(painter, bar.x_range(), bar.top(), tokens::line());
    tokens::hline(
        painter,
        bar.x_range(),
        bar.bottom() - tokens::hairline(ppp),
        tokens::line(),
    );
}

/// A child of `ui` in `rect`, laid out from the left, centred.
fn row_in(ui: &mut Ui, rect: Rect, salt: &str) -> Ui {
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center))
            .id_salt(("bottom bar", salt)),
    );
    row.spacing_mut().item_spacing.x = size::GAP;
    row
}

/// The folded line: ▴, the tabs' names with their counts, the newest
/// warning or error faintly, and "Click a name to open it".
fn folded_line(
    ui: &mut Ui,
    bar: Rect,
    now: &BottomNow,
    front: Option<BottomTab>,
    viewer: &mut PanelViewer<'_>,
) -> BarAsks {
    let mut asks = BarAsks::default();
    let mut row = row_in(ui, bar.shrink2(vec2(size::GAP, 0.0)), "folded");
    let opener = match front {
        Some(front) => format!("Open {}", front.name()),
        None => "Open the bottom panel".to_owned(),
    };
    if widgets::button::icon(&mut row, Icon::ChevronUp, &opener)
        .on_hover_text("Opens the tab in front at its last height")
        .clicked()
    {
        asks.fold = Some(false);
    }
    tab_names(&mut row, now, None, viewer, &mut asks);
    row.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = size::GAP;
        show_buttons(ui, now, &mut asks);
        quiet(
            ui,
            "Click a name to open it",
            text::ui(),
            tokens::text_meta(),
        );
        if let Some(line) = viewer.console.newest_notice() {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                newest_notice(ui, line);
            });
        }
    });
    asks
}

/// The newest warning or error, faintly: its level's word, then its
/// words cut to the room there is.
fn newest_notice(ui: &mut Ui, line: &crate::console::LogLine) {
    let colour = line.level.color().gamma_multiply(0.75);
    let faint = tokens::text_meta();
    let (sep, _) = ui.allocate_exact_size(vec2(1.0, size::ICON_SMALL), Sense::hover());
    ui.painter().rect_filled(sep, 0.0, tokens::line());
    ui.add(
        egui::Label::new(
            egui::RichText::new(line.level.word())
                .font(text::mono_bold())
                .color(colour),
        )
        .selectable(false),
    );
    ui.add(
        egui::Label::new(
            egui::RichText::new(&line.text)
                .font(text::mono_small())
                .color(faint),
        )
        .truncate()
        .selectable(false),
    )
    .on_hover_text(&line.text);
}

/// The open bar: the tabs with icons and counts, then at the right ▾,
/// any hidden slot to show again, and the front tab's tools.
fn open_bar(
    ui: &mut Ui,
    bar: Rect,
    (tab, now): (RibbonTab, &BottomNow),
    front: Option<BottomTab>,
    viewer: &mut PanelViewer<'_>,
) -> BarAsks {
    let mut asks = BarAsks::default();
    let mut row = row_in(ui, bar.shrink2(vec2(size::GAP, 0.0)), "open");
    tab_names(&mut row, now, Some(front), viewer, &mut asks);
    // The tools take only the room the tabs leave: on a narrow centre
    // they're cut at the left rather than drawn over the tabs.
    let room = Rect::from_min_max(
        pos2(row.cursor().left() + size::GAP, bar.top()),
        pos2(bar.right() - size::GAP, bar.bottom()),
    );
    let mut right = ui.new_child(
        UiBuilder::new()
            .max_rect(room)
            .layout(Layout::right_to_left(Align::Center))
            .id_salt(("bottom bar", "tools")),
    );
    right.set_clip_rect(room.intersect(ui.clip_rect()));
    {
        let ui = &mut right;
        ui.spacing_mut().item_spacing.x = size::GAP;
        if widgets::button::icon(ui, Icon::ChevronDown, "Fold the bottom panel")
            .on_hover_text("Folds it to one line; a name there opens it again (Ctrl+Space)")
            .clicked()
        {
            asks.fold = Some(true);
        }
        show_buttons(ui, now, &mut asks);
        asks.place = tools(ui, tab, front, viewer);
    }
    asks
}

/// Each tab's name (with its icon while `open` holds the front one) and
/// count; a clicked name goes into `asks`.
fn tab_names(
    ui: &mut Ui,
    now: &BottomNow,
    open: Option<Option<BottomTab>>,
    viewer: &mut PanelViewer<'_>,
    asks: &mut BarAsks,
) {
    for entry in &now.tabs {
        match entry {
            Entry::Built(bottom) => {
                let front = open.is_some_and(|f| f == Some(*bottom));
                if bottom_tab(ui, *bottom, front, open.is_some(), viewer).clicked() {
                    asks.tab = Some(*bottom);
                }
                if open.is_some() && bottom.closes_a_file() && close_mark(ui, *bottom) {
                    viewer.close(bottom.panel());
                }
            }
            Entry::Unbuilt(id) => drop(unbuilt::bottom_tab(ui, *id)),
        }
    }
}

/// What a tab's count shows: marks and numbers, each in its colour.
fn counts(tab: BottomTab, viewer: &PanelViewer<'_>) -> Vec<(Icon, Color32, usize)> {
    match tab {
        BottomTab::Console => [
            (LogLevel::Warning, Icon::WarningOutline, tokens::WARNING),
            (LogLevel::Error, Icon::Error, tokens::ERROR),
        ]
        .into_iter()
        .map(|(level, icon, colour)| (icon, colour, viewer.console.count(level)))
        .filter(|(_, _, n)| *n > 0)
        .collect(),
        BottomTab::Problems => {
            let n = viewer.problem_count();
            let colour = if viewer.code_problems.is_empty() {
                tokens::WARNING
            } else {
                tokens::ERROR
            };
            if n > 0 {
                vec![(Icon::Record, colour, n)]
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

/// One tab of the bar: its icon (on the open bar), its name and its
/// counts; the one in front filled.
fn bottom_tab(
    ui: &mut Ui,
    tab: BottomTab,
    front: bool,
    with_icon: bool,
    viewer: &PanelViewer<'_>,
) -> egui::Response {
    let font = if front {
        text::tab_active()
    } else {
        text::tab()
    };
    let name =
        ui.painter()
            .layout_no_wrap(tab.name().to_owned(), font.clone(), tokens::text_label());
    let marks = counts(tab, viewer);
    let numbers: Vec<_> = marks
        .iter()
        .map(|(_, colour, n)| {
            ui.painter()
                .layout_no_wrap(n.to_string(), text::ui(), *colour)
        })
        .collect();
    let mark_w: f32 = numbers
        .iter()
        .map(|g| size::GAP + size::ICON_SMALL + 2.0 + g.size().x)
        .sum();
    let icon_w = if with_icon {
        size::ICON_SMALL + size::GAP
    } else {
        0.0
    };
    let width = 2.0 * size::ROW_PAD + icon_w + name.size().x + mark_w;
    let height = size::BOTTOM_BAR_H - 6.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    response
        .widget_info(|| WidgetInfo::selected(egui::WidgetType::Button, true, front, tab.name()));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let ink = if front {
        tokens::text_emphasis()
    } else if response.hovered() {
        tokens::text_primary()
    } else {
        tokens::text_label()
    };
    if front {
        ui.painter()
            .rect_filled(rect, size::radius::TAB, tokens::line());
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, size::radius::TAB, tokens::row_hover());
    }
    let mut x = rect.left() + size::ROW_PAD;
    let middle = rect.center().y;
    let spot = |x: f32| {
        Rect::from_min_size(
            pos2(x, middle - size::ICON_SMALL / 2.0),
            vec2(size::ICON_SMALL, size::ICON_SMALL),
        )
    };
    if with_icon {
        icons::icon(ui.painter(), spot(x), tab.icon(), ink);
        x += size::ICON_SMALL + size::GAP;
    }
    ui.painter()
        .galley(pos2(x, middle - name.size().y / 2.0), name.clone(), ink);
    x += name.size().x;
    for ((icon, colour, n), number) in marks.iter().zip(numbers) {
        x += size::GAP;
        // Each count is named for screen readers (and tests): "1 warning".
        let mark = Rect::from_x_y_ranges(
            x..=x + size::ICON_SMALL + 2.0 + number.size().x,
            rect.y_range(),
        );
        let named = ui.interact(mark, ui.id().with((tab, *icon)), Sense::hover());
        named.widget_info(|| {
            WidgetInfo::labeled(egui::WidgetType::Label, true, count_words(*icon, *n))
        });
        if *icon == Icon::Record {
            ui.painter().circle_filled(
                pos2(x + size::ICON_SMALL / 2.0, middle),
                size::STATUS_DOT / 2.0,
                *colour,
            );
        } else {
            icons::icon(ui.painter(), spot(x), *icon, *colour);
        }
        x += size::ICON_SMALL + 2.0;
        ui.painter().galley(
            pos2(x, middle - number.size().y / 2.0),
            number.clone(),
            *colour,
        );
        x += number.size().x;
    }
    widgets::focus_ring(ui, &response, rect, size::radius::TAB);
    response.on_hover_text(if front {
        "Click again to fold the panel to one line"
    } else {
        "Opens this tab"
    })
}

/// A count in words, for screen readers and tests: "1 warning", "2
/// errors", "3 problems".
fn count_words(icon: Icon, n: usize) -> String {
    let what = match (icon, n) {
        (Icon::WarningOutline, 1) => "warning",
        (Icon::WarningOutline, _) => "warnings",
        (Icon::Error, 1) => "error",
        (Icon::Error, _) => "errors",
        (_, 1) => "problem",
        _ => "problems",
    };
    format!("{n} {what}")
}

/// The × after a tab that shows a file of its own. Returns whether it
/// was clicked.
fn close_mark(ui: &mut Ui, tab: BottomTab) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(vec2(size::ICON_SMALL, size::ICON_SMALL), Sense::click());
    let name = format!("Close {}", tab.name());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
    let ink = if response.hovered() {
        tokens::text_emphasis()
    } else {
        tokens::text_meta()
    };
    icons::icon(ui.painter(), rect.shrink(2.0), Icon::Close, ink);
    response.on_hover_text("Closes the clips file").clicked()
}

/// A small button for each hidden slot, which shows it again (laid out
/// from the right).
fn show_buttons(ui: &mut Ui, now: &BottomNow, asks: &mut BarAsks) {
    for (slot, column) in now.hidden.iter().rev() {
        let icon = column.panels.first().map_or(Icon::Eye, |p| p.icon());
        let name = format!("Show {}", column.title);
        if widgets::button::icon(ui, icon, &name)
            .on_hover_text(format!("{} is hidden: this shows it again", column.title))
            .clicked()
        {
            asks.show = Some(*slot);
        }
    }
}

/// The tools at the bar's right, laid out from the right: the Library's
/// Place switch on Home; the Console's filters, search, Clear and Copy
/// while it's in front; the Animate tab's unfinished animation transport;
/// the Library tab's count of files. Returns whether Place was clicked.
fn tools(
    ui: &mut Ui,
    tab: RibbonTab,
    front: Option<BottomTab>,
    viewer: &mut PanelViewer<'_>,
) -> bool {
    let mut place = false;
    if front == Some(BottomTab::Library) && viewer.placing {
        // What to do, where the design has its hint (spec 264).
        quiet(
            ui,
            "Drag into the level, or double-click to place where the camera looks",
            text::ui(),
            tokens::text_meta(),
        );
    }
    if front == Some(BottomTab::Library) && tab == RibbonTab::Home {
        place = place_switch(ui, viewer.placing);
    }
    // The browser's tools, as on the Library tab (board 03).
    if front == Some(BottomTab::Library)
        && !viewer.placing
        && let Some(project) = viewer.project
    {
        let name = project.name.clone();
        crate::library::tools(ui, viewer.library_state, &name);
    }
    if tab == RibbonTab::Animate {
        let parts = vec![
            Entry::Unbuilt(UnbuiltId::HomeShelfFrameCounter),
            Entry::Unbuilt(UnbuiltId::HomeShelfTransport),
        ];
        for entry in unbuilt::filter::<()>(parts) {
            match entry {
                Entry::Unbuilt(UnbuiltId::HomeShelfTransport) => {
                    for icon in [Icon::NextKey, Icon::Play, Icon::PreviousKey] {
                        unbuilt::icon_button(ui, UnbuiltId::HomeShelfTransport, icon);
                    }
                }
                Entry::Unbuilt(part) => drop(unbuilt::status_item(ui, part)),
                Entry::Built(()) => {}
            }
        }
    }
    if front == Some(BottomTab::Console) {
        crate::console::console_tools(ui, viewer.console);
    }
    place
}

/// The Library's Place switch on Home: lit while placing (the ribbon in
/// its build form, how placed things land beside the files). Returns
/// whether it was clicked.
pub(super) fn place_switch(ui: &mut Ui, placing: bool) -> bool {
    widgets::chip::chip(ui, widgets::chip::ChipKind::Filter, "Place", placing, None)
        .on_hover_text(if placing {
            "Placing: drag files into the level. Click to stop"
        } else {
            "Places files into the level: the ribbon shows how, and how they land"
        })
        .clicked()
}

/// The front tab's panel in `content`; the Library's with how placed
/// things land beside it while placing.
fn tab_content(
    ui: &mut Ui,
    content: Rect,
    front: BottomTab,
    shell: &mut Shell,
    layout: PanelLayout,
    viewer: &mut PanelViewer<'_>,
) {
    ui.painter().rect_filled(content, 0.0, tokens::panel());
    if front == BottomTab::Library && viewer.placing {
        let tab = shell.tab;
        let room = (content.width() - size::COLUMN_MIN_W - 1.0).max(size::COLUMN_MIN_W);
        let width = shell.size(tab, layout, Edge::Placement).min(room).round();
        let place = content.with_max_x(content.left() + width);
        let files = content.with_min_x(place.right() + 1.0);
        tokens::vline(
            ui.painter(),
            place.right(),
            content.y_range(),
            tokens::line(),
        );
        draw(ui, place, Panel::Placement, shell, viewer);
        draw(ui, files, Panel::Assets, shell, viewer);
        let border = Border {
            edge: Edge::Placement,
            line: Rect::from_x_y_ranges(place.right()..=files.left(), content.y_range()),
            measure: Measure::FromLeft {
                x: content.left(),
                span: 1.0,
            },
            range: size::COLUMN_MIN_W..=room,
            between: ("Placement", "Library"),
            grip: false,
        };
        super::columns::drag_borders(ui, &[border], shell, layout);
        return;
    }
    draw(ui, content, front.panel(), shell, viewer);
}

/// Draws `panel` in `rect`, and notes where.
pub(super) fn draw(
    ui: &mut Ui,
    rect: Rect,
    panel: Panel,
    shell: &mut Shell,
    viewer: &mut PanelViewer<'_>,
) {
    let mut panel_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .id_salt(("frame part", panel)),
    );
    panel_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
    viewer.draw_panel(&mut panel_ui, panel);
    shell.drawn.push((panel, rect));
}

/// Quiet words that can't be selected.
fn quiet(ui: &mut Ui, words: &str, font: egui::FontId, colour: Color32) {
    ui.add(egui::Label::new(egui::RichText::new(words).font(font).color(colour)).selectable(false));
}
