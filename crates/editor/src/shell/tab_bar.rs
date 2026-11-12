//! The tab bar, along the top of the window (specification, "Tab bar"):
//! the logo in its light tile, the ribbon's tabs, the tabs that come with
//! an open file, the search box, the project and area, and the chevron
//! that collapses the ribbon.

use super::RibbonTab;
use crate::icons::{self, Icon};
use crate::interface_look;
use crate::panel::Panel;
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::widgets::{self, width_of, words};
use crate::workbench::Workbench;
use egui::{
    Align, Align2, CornerRadius, Layout, Rect, Response, Sense, Shape, Ui, UiBuilder, WidgetInfo,
    pos2, vec2,
};

/// The words in the search box.
pub(crate) const SEARCH_WORDS: &str = "Search or run a command";
/// The key the search box shows: today's key.
const SEARCH_KEY: &str = "Space";

/// The tabs in the tab bar's order, each as a list of one thing: a tab
/// that works, or the unfinished part a whole tab is. Passed through the
/// filter, so a hidden tab leaves no gap.
fn tabs() -> Vec<(RibbonTab, Vec<Entry<()>>)> {
    use RibbonTab as T;
    let built = |tab| (tab, vec![Entry::Built(())]);
    unbuilt::filter_groups(vec![
        built(T::File),
        built(T::Home),
        built(T::Library),
        (T::Shading, vec![Entry::Unbuilt(UnbuiltId::ShellTabShading)]),
        built(T::Animate),
        built(T::Ui),
        (T::Vfx, vec![Entry::Unbuilt(UnbuiltId::ShellTabVfx)]),
        built(T::Audio),
        built(T::Debug),
        built(T::Build),
    ])
}

/// A tab that is there only while a file of its kind is open: the words
/// before its name, its name, the tab it follows and the panel it brings
/// to the front.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextTab {
    /// The tab it comes after.
    pub(crate) after: RibbonTab,
    /// The small capitals before its name.
    pub(crate) prefix: &'static str,
    /// Its name.
    pub(crate) name: &'static str,
    /// The panel that shows the file.
    pub(crate) panel: Panel,
    /// The tab it brings to the front, for one whose screen is a page
    /// (the model viewer): it has no panel to show.
    pub(crate) opens: Option<RibbonTab>,
}

impl ContextTab {
    /// What a screen reader calls it, and how tests find it.
    pub(crate) fn label(self) -> String {
        format!("{} \u{b7} {}", self.prefix, self.name)
    }
}

impl Workbench {
    /// The tabs that come with what is open now.
    pub(crate) fn context_tabs(&self) -> Vec<ContextTab> {
        let open = &self.open;
        [
            (
                open.map_edits.path.is_some(),
                ContextTab {
                    after: RibbonTab::Home,
                    prefix: "Home tools",
                    name: "Tile map",
                    panel: Panel::Map,
                    opens: None,
                },
            ),
            (
                open.tilesets.tab().is_some(),
                ContextTab {
                    after: RibbonTab::Library,
                    prefix: "Library tools",
                    name: "Tileset",
                    panel: Panel::Tileset,
                    opens: None,
                },
            ),
            (
                open.atlas_editor.is_some(),
                ContextTab {
                    after: RibbonTab::Library,
                    prefix: "Library tools",
                    name: "Picture",
                    panel: Panel::AtlasCrop,
                    opens: None,
                },
            ),
            (
                self.shell.viewing_model.is_some() && !crate::unbuilt::hidden(),
                ContextTab {
                    after: RibbonTab::Library,
                    prefix: "Library tools",
                    name: "Model",
                    panel: Panel::Library,
                    opens: Some(RibbonTab::Model),
                },
            ),
        ]
        .into_iter()
        .filter_map(|(there, tab)| there.then_some(tab))
        .collect()
    }

    /// Whether `extra` is the tab in front: its ribbon is the one shown.
    pub(crate) fn context_in_front(&self, extra: ContextTab) -> bool {
        if let Some(tab) = extra.opens {
            return self.shell.tab == tab;
        }
        match extra.panel {
            Panel::Map => self.map_tools_in_front(),
            Panel::Tileset => self.shell.tab == RibbonTab::Tileset,
            Panel::AtlasCrop => self.shell.tab == RibbonTab::Picture,
            panel => self.in_front.contains(&panel),
        }
    }

    /// Draws the tab bar across the top of `ui`.
    pub(crate) fn tab_bar(&mut self, ui: &mut Ui) {
        egui::Panel::top("tab bar")
            .frame(
                egui::Frame::new()
                    .fill(tokens::app())
                    .inner_margin(egui::Margin::symmetric(size::TAB_BAR_PAD as i8, 0)),
            )
            .exact_size(size::TAB_BAR_H)
            .show_separator_line(false)
            .show(ui, |ui| {
                let bar = ui.max_rect();
                let left = self.logo_tile(ui, bar) + size::LOGO_GAP + size::TAB_BAR_GAP;
                let tabs_end = self.tabs(ui, bar, left);
                let right_start = self.right_end(ui, bar);
                let between = Rect::from_min_max(
                    pos2(tabs_end + size::TAB_BAR_GAP, bar.top()),
                    pos2(right_start - size::TAB_BAR_GAP, bar.bottom()),
                );
                self.search_box(ui, between);
            });
    }

    /// The logo in its light tile, at the bar's left. Returns where the
    /// tile ends.
    fn logo_tile(&self, ui: &Ui, bar: Rect) -> f32 {
        let tile = Rect::from_min_size(
            pos2(bar.left(), bar.center().y - size::LOGO_TILE / 2.0),
            vec2(size::LOGO_TILE, size::LOGO_TILE),
        );
        ui.painter().rect_filled(
            tile,
            CornerRadius::same(size::radius::CONTROL),
            tokens::PLAY_FILL,
        );
        // The logo itself is the app's picture (branding), never drawn
        // here.
        if let Some(logo) = self.logo {
            egui::Image::new((logo, tile.shrink(size::LOGO_INSET).size()))
                .paint_at(ui, tile.shrink(size::LOGO_INSET));
        }
        tile.right()
    }

    /// The ribbon's tabs, and the contextual ones after their parents,
    /// from `left`. Returns where they end.
    fn tabs(&mut self, ui: &mut Ui, bar: Rect, left: f32) -> f32 {
        let room = Rect::from_min_max(
            pos2(left, bar.bottom() - size::RIBBON_TAB_H),
            bar.right_bottom(),
        );
        let context = self.context_tabs();
        let mut clicked = None;
        let mut toggled = false;
        let mut bring = None;
        let mut strip = ui.new_child(
            UiBuilder::new()
                .max_rect(room)
                .layout(Layout::left_to_right(Align::Max)),
        );
        let ui = &mut strip;
        ui.spacing_mut().item_spacing.x = 0.0;
        // Until a project is open, Recent projects is where to be: the
        // other tabs are dimmed (they still carry on without one).
        let dim = self.open.project.is_none() && self.shell.tab == RibbonTab::File;
        for (tab, entries) in tabs() {
            // A tab whose tools tab is in front isn't drawn in front itself.
            let front = self.shell.tab == tab
                && !context
                    .iter()
                    .any(|extra| extra.after == tab && self.context_in_front(*extra));
            let response = match entries.first() {
                Some(Entry::Unbuilt(id)) => unbuilt_tab(ui, *id, front),
                _ => ribbon_tab(ui, tab, front, dim && tab != RibbonTab::File),
            };
            if response.double_clicked() && tab != RibbonTab::File {
                toggled = true;
            } else if response.clicked() {
                clicked = Some(tab);
            }
            for extra in context.iter().filter(|extra| extra.after == tab) {
                let shown = self.context_in_front(*extra);
                if context_tab(ui, *extra, shown).clicked() {
                    bring = Some(*extra);
                }
            }
        }
        let end = ui.min_rect().right();
        if let Some(tab) = clicked {
            self.tab_clicked(tab);
        }
        if toggled {
            self.set_ribbon_collapsed(!self.shell.collapsed);
        }
        if let Some(extra) = bring {
            match extra.opens {
                Some(tab) => self.show_tab(tab),
                None => self.show_panel(extra.panel),
            }
        }
        end
    }

    /// The bar's right end, laid out from the right: the chevron, the
    /// project and area, and Restart to update while a new build waits.
    /// Returns where it starts.
    fn right_end(&mut self, ui: &mut Ui, bar: Rect) -> f32 {
        let mut side = ui.new_child(
            UiBuilder::new()
                .max_rect(bar)
                .layout(Layout::right_to_left(Align::Center)),
        );
        let ui = &mut side;
        ui.spacing_mut().item_spacing.x = size::GAP;
        let collapsed = self.shell.collapsed;
        let (icon, name) = if collapsed {
            (Icon::ChevronDown, "Show the ribbon")
        } else {
            (Icon::ChevronUp, "Collapse the ribbon")
        };
        if widgets::button::icon(ui, icon, name).clicked() {
            self.set_ribbon_collapsed(!collapsed);
        }
        self.path(ui);
        ui.min_rect().left()
    }

    /// The project and the area: "Harbour / docks", with the unsaved dot
    /// after an area that has changes.
    fn path(&self, ui: &mut Ui) {
        let font = text::row();
        if self.open.project.is_none() && self.shell.tab == RibbonTab::File {
            let words_w = width_of(ui, NO_PROJECT, font.clone());
            let height = ui.available_height();
            let (rect, response) = ui.allocate_exact_size(vec2(words_w, height), Sense::hover());
            response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Label, true, NO_PROJECT));
            words(
                ui,
                rect,
                Align2::LEFT_CENTER,
                NO_PROJECT,
                font,
                tokens::text_meta(),
            );
            return;
        }
        let area = self.open.document.name();
        let project = self.open.project.as_ref().map(|p| p.name.as_str());
        let dot = if self.open.document.is_dirty() {
            size::GAP + size::STATUS_DOT
        } else {
            0.0
        };
        let slash = " / ";
        let project_w = project.map_or(0.0, |name| {
            width_of(ui, name, font.clone()) + width_of(ui, slash, font.clone())
        });
        let width = project_w + width_of(ui, &area, font.clone()) + dot;
        let height = ui.available_height();
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let label = match project {
            Some(project) => format!("{project} / {area}"),
            None => area.clone(),
        };
        response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Label, true, &label));
        let mut room = rect;
        if let Some(project) = project {
            let drawn = words(
                ui,
                room,
                Align2::LEFT_CENTER,
                project,
                font.clone(),
                tokens::text_label(),
            );
            room.min.x = drawn.right();
            let drawn = words(
                ui,
                room,
                Align2::LEFT_CENTER,
                slash,
                font.clone(),
                tokens::text_separator(),
            );
            room.min.x = drawn.right();
        }
        let drawn = words(
            ui,
            room,
            Align2::LEFT_CENTER,
            &area,
            font,
            tokens::text_primary(),
        );
        if dot > 0.0 {
            let middle = pos2(
                drawn.right() + size::GAP + size::STATUS_DOT / 2.0,
                rect.center().y,
            );
            ui.painter()
                .circle_filled(middle, size::STATUS_DOT / 2.0, tokens::WARNING);
            response.on_hover_text("The area has changes that aren't saved");
        }
    }

    /// The search box, in the middle of `between`: a click opens the
    /// search, as its key does.
    fn search_box(&mut self, ui: &mut Ui, between: Rect) {
        let whole = ui.ctx().content_rect().width();
        let width = size::SEARCH_W
            .min(whole * size::SEARCH_MAX_SHARE)
            .min(between.width());
        if width < size::SEARCH_MIN_W {
            return;
        }
        let rect = Rect::from_center_size(between.center(), vec2(width, size::SEARCH_H));
        let response = ui.interact(rect, ui.id().with("search box"), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, SEARCH_WORDS));
        let radius = size::radius::TAB;
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius), tokens::ribbon());
        let edge = if response.hovered() {
            tokens::strong_line()
        } else {
            tokens::line()
        };
        widgets::border(ui, rect, radius, edge);
        let inside = rect.shrink2(vec2(size::TAB_BAR_PAD, 0.0));
        let spot = Rect::from_center_size(
            pos2(inside.left() + size::SEARCH_ICON / 2.0, inside.center().y),
            vec2(size::SEARCH_ICON, size::SEARCH_ICON),
        );
        icons::icon(ui.painter(), spot, Icon::Search, tokens::text_meta());
        let key = words(
            ui,
            inside,
            Align2::RIGHT_CENTER,
            SEARCH_KEY,
            text::mono_small(),
            tokens::text_meta(),
        );
        let room = Rect::from_min_max(
            pos2(spot.right() + size::CARD_INNER_GAP, inside.top()),
            pos2(key.left() - size::CARD_INNER_GAP, inside.bottom()),
        );
        words(
            ui,
            room,
            Align2::LEFT_CENTER,
            SEARCH_WORDS,
            text::row(),
            tokens::text_meta(),
        );
        widgets::focus_ring(ui, &response, rect, radius);
        let response = response.on_hover_text(
            "Finds anything the editor can do, any file, entity or panel, and does it: press \
             Space with nothing being typed",
        );
        if response.clicked() {
            self.open_search();
        }
    }
}

/// The shape behind the tab in front: the ribbon's own colour with
/// rounded top corners, and the accent's line along its foot, so it
/// reads as the tab of the ribbon below it.
fn front_shape(rect: Rect) -> Shape {
    let round = CornerRadius {
        nw: size::radius::TAB,
        ne: size::radius::TAB,
        sw: 0,
        se: 0,
    };
    let foot = Rect::from_min_max(
        pos2(rect.left(), rect.bottom() - size::TAB_UNDERLINE),
        rect.right_bottom(),
    );
    Shape::Vec(vec![
        Shape::rect_filled(rect, round, tokens::ribbon()),
        Shape::rect_filled(foot, CornerRadius::ZERO, interface_look::accent_fill()),
    ])
}

/// What the bar's right end says while no project is open.
const NO_PROJECT: &str = "No project open";

/// One of the ribbon's tabs; `dim` while no project is open.
fn ribbon_tab(ui: &mut Ui, tab: RibbonTab, front: bool, dim: bool) -> Response {
    let font = if front {
        text::tab_active()
    } else {
        text::tab()
    };
    let width = width_of(ui, tab.name(), text::tab()) + 2.0 * size::RIBBON_TAB_PAD;
    let (rect, response) = ui.allocate_exact_size(vec2(width, size::RIBBON_TAB_H), Sense::click());
    // Called "Library tab", as the search calls it: the panels have
    // tabs of the same names.
    let name = format!("{} tab", tab.name());
    response.widget_info(|| WidgetInfo::selected(egui::WidgetType::Button, true, front, &name));
    if ui.is_rect_visible(rect) {
        let ink = if front {
            ui.painter().add(front_shape(rect));
            tokens::text_emphasis()
        } else if response.hovered() {
            tokens::text_primary()
        } else if dim {
            tokens::text_faint()
        } else {
            tokens::text_label()
        };
        // The words sit on the tab's 28 px above its underline.
        let room = rect.with_max_y(rect.bottom() - size::TAB_UNDERLINE);
        words(ui, room, Align2::CENTER_CENTER, tab.name(), font, ink);
        widgets::focus_ring(ui, &response, rect, size::radius::TAB);
    }
    response.on_hover_text(tab.help())
}

/// A tab whose screen isn't built: drawn by the one mechanism, on the
/// front tab's shape while it is the one in front.
fn unbuilt_tab(ui: &mut Ui, id: UnbuiltId, front: bool) -> Response {
    let behind = ui.painter().add(Shape::Noop);
    let response = unbuilt::tab(ui, id);
    if front {
        ui.painter().set(behind, front_shape(response.rect));
    }
    response
}

/// A tab that came with an open file: drawn as a tab in front, with a
/// faint line of the accent along its top and small capitals before its
/// name.
fn context_tab(ui: &mut Ui, tab: ContextTab, shown: bool) -> Response {
    let prefix = ui.painter().layout_job(text::caps(
        tab.prefix,
        text::CONTEXT_PREFIX,
        tokens::text_meta(),
    ));
    let name_w = width_of(ui, tab.name, text::tab_active());
    let width = 2.0 * size::RIBBON_TAB_PAD + prefix.size().x + size::GAP + name_w;
    let (rect, response) = ui.allocate_exact_size(vec2(width, size::RIBBON_TAB_H), Sense::click());
    let label = tab.label();
    response.widget_info(|| WidgetInfo::selected(egui::WidgetType::Button, true, shown, &label));
    if ui.is_rect_visible(rect) {
        let round = CornerRadius {
            nw: size::radius::TAB,
            ne: size::radius::TAB,
            sw: 0,
            se: 0,
        };
        ui.painter().rect_filled(rect, round, tokens::ribbon());
        let line = tokens::mix(
            interface_look::accent_fill(),
            tokens::ribbon(),
            tokens::share::CONTEXT_TAB,
        );
        tokens::hline(
            ui.painter(),
            rect.x_range().shrink(f32::from(size::radius::TAB)),
            rect.top(),
            line,
        );
        let left = rect.left() + size::RIBBON_TAB_PAD;
        let at = pos2(left, rect.center().y - prefix.size().y / 2.0);
        let prefix_w = prefix.size().x;
        ui.painter().galley(at, prefix, tokens::text_meta());
        let room = Rect::from_min_max(
            pos2(left + prefix_w + size::GAP, rect.top()),
            rect.right_bottom(),
        );
        words(
            ui,
            room,
            Align2::LEFT_CENTER,
            tab.name,
            text::tab_active(),
            tokens::text_emphasis(),
        );
        widgets::focus_ring(ui, &response, rect, size::radius::TAB);
    }
    response.on_hover_text(format!(
        "Brings the {} open to the front",
        tab.name.to_lowercase()
    ))
}
