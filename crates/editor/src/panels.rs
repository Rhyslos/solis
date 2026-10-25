//! Drawing each docked panel's contents, and the bits panels share: the
//! small menu row under a title, a quiet placeholder note, and the hint
//! shown while files are dragged over the window.

use crate::area_panel::{AreaPanelState, AreaRequest, area_tree};
use crate::console::{Console, ConsoleRequest, console};
use crate::document::Document;
use crate::icons::{self, Icon};
use crate::library::{LibraryItem, LibraryRequest, LibraryState, ProjectInfo, library};
use crate::menu_items::MenuState;
use crate::models::ModelInfo;
use crate::panel::Panel;
use crate::properties::{AssemblyPart, PropertyRequest, properties};
use crate::theme::{self, Accent};
use crate::viewport::{Viewport, ViewportRenderer};
use egui::{Align, Color32, Id, Layout, RichText, Ui, WidgetText};
use egui_dock::TabViewer;
use reflect::AssetId;
use std::collections::{BTreeMap, HashSet};

/// Where egui_dock starts a tab's text, from the tab's left edge.
pub(crate) const TAB_TEXT_INSET: f32 = 8.0;
/// Room left before a tab's name for its icon.
pub(crate) const TAB_ICON_SPACE: f32 = 18.0;

/// Draws each docked panel.
pub(crate) struct PanelViewer<'a> {
    pub(crate) area_state: &'a mut AreaPanelState,
    /// The components whose values were copied, by name.
    pub(crate) copied: &'a [String],
    pub(crate) menu: &'a MenuState,
    pub(crate) viewport: &'a mut Viewport,
    pub(crate) renderer: &'a mut dyn ViewportRenderer,
    pub(crate) console: &'a mut Console,
    pub(crate) console_requests: &'a mut Vec<ConsoleRequest>,
    pub(crate) problems: &'a [crate::problems::Problem],
    /// Mistakes in the game's code.
    pub(crate) code_problems: &'a [script::CodeProblem],
    /// What the Game panel shows, while the game runs.
    pub(crate) game: Option<&'a crate::game_panel::GameScene>,
    /// Whether the game is running.
    pub(crate) playing: bool,
    pub(crate) document: &'a Document,
    pub(crate) models: &'a BTreeMap<AssetId, ModelInfo>,
    pub(crate) area_requests: &'a mut Vec<AreaRequest>,
    pub(crate) project: Option<&'a ProjectInfo>,
    pub(crate) library: &'a [LibraryItem],
    pub(crate) folders: &'a [String],
    pub(crate) library_state: &'a mut LibraryState,
    pub(crate) library_requests: &'a mut Vec<LibraryRequest>,
    pub(crate) part: Option<&'a AssemblyPart>,
    pub(crate) requests: &'a mut Vec<PropertyRequest>,
    pub(crate) accent: Accent,
    pub(crate) in_front: &'a HashSet<Panel>,
    pub(crate) shown: &'a mut HashSet<Panel>,
}

impl TabViewer for PanelViewer<'_> {
    type Tab = Panel;

    fn id(&mut self, tab: &mut Panel) -> Id {
        Id::new(("panel", *tab))
    }

    fn title(&mut self, tab: &mut Panel) -> WidgetText {
        // Room before the name for the panel's icon, drawn in
        // `on_tab_button`. The colour comes from the dock style (bright in
        // front, quieter behind).
        let mut job = egui::text::LayoutJob::default();
        job.append(
            tab.title(),
            TAB_ICON_SPACE,
            egui::TextFormat {
                font_id: egui::FontId::proportional(theme::TEXT_SIZE),
                color: Color32::PLACEHOLDER,
                ..Default::default()
            },
        );
        let problems = self.problems.len() + self.code_problems.len();
        if *tab == Panel::Problems && problems > 0 {
            job.append(
                &problems.to_string(),
                6.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(theme::TEXT_SIZE),
                    color: theme::WARNING,
                    ..Default::default()
                },
            );
        }
        job.into()
    }

    fn on_tab_button(&mut self, tab: &mut Panel, response: &egui::Response) {
        // Name the tab for screen readers (and tests).
        let in_front = self.in_front.contains(tab);
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, in_front, tab.title())
        });
        let painter = response.ctx.layer_painter(response.layer_id);
        // The icon, in colour on the front tab and quiet behind it.
        let color = match (in_front, *tab) {
            (true, Panel::Library) => theme::FOLDER,
            (true, _) => self.accent.colors().light,
            (false, _) => theme::TEXT_WEAK,
        };
        let size = 12.0;
        icons::icon(
            &painter,
            egui::Rect::from_min_size(
                egui::pos2(
                    response.rect.left() + TAB_TEXT_INSET,
                    response.rect.center().y - size / 2.0,
                ),
                egui::vec2(size, size),
            ),
            tab.icon(),
            color,
        );
        // The tab in front is underlined in the accent colour.
        if in_front {
            painter.hline(
                response.rect.x_range(),
                response.rect.bottom() - 1.0,
                egui::Stroke::new(2.0, self.accent.colors().fill),
            );
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Panel) {
        self.shown.insert(*tab);
        if *tab == Panel::Viewport {
            let full = ui.max_rect();
            let mut view_ui = ui.new_child(egui::UiBuilder::new().max_rect(full));
            self.viewport.show(&mut view_ui, self.renderer, self.accent);
            if self.playing {
                playing_border(ui, full, self.accent);
            }
            ui.advance_cursor_after_rect(full);
            return;
        }
        match tab {
            Panel::Area => {
                let requests = area_tree(
                    ui,
                    self.document,
                    self.models,
                    self.accent,
                    self.area_state,
                    self.menu,
                );
                self.area_requests.extend(requests);
            }
            Panel::Layers => {
                menu_row(ui, &["Edit", "View"]);
                placeholder(
                    ui,
                    "Layers (groups of entities to show, hide or lock together) aren't built \
                     yet. Until then, hide and lock entities in the Area panel.",
                );
            }
            Panel::Properties => {
                let requests = properties(ui, self.document, self.part, self.models, self.copied);
                self.requests.extend(requests);
            }
            Panel::Library | Panel::Code => {
                let requests = library(
                    ui,
                    self.project,
                    self.library,
                    self.folders,
                    self.library_state,
                    self.accent,
                    *tab == Panel::Code,
                );
                self.library_requests.extend(requests);
            }
            Panel::Game => {
                let full = ui.max_rect();
                let mut game_ui = ui.new_child(egui::UiBuilder::new().max_rect(full));
                crate::game_panel::game_panel(
                    &mut game_ui,
                    self.game,
                    self.playing,
                    &self.viewport.camera,
                    self.renderer,
                );
                if self.playing {
                    playing_border(ui, full, self.accent);
                }
                ui.advance_cursor_after_rect(full);
            }
            Panel::Console => {
                if let Some(request) = console(ui, self.console, self.accent) {
                    self.console_requests.push(request);
                }
            }
            Panel::Problems => {
                use crate::problems::ProblemClick;
                match crate::problems::problems_panel(ui, self.code_problems, self.problems) {
                    Some(ProblemClick::Entity(entity)) => {
                        self.area_state.reveal = Some(entity);
                        self.area_requests.push(AreaRequest::Select(entity));
                    }
                    Some(ProblemClick::File(file)) => {
                        self.library_requests
                            .push(crate::library::LibraryRequest::Open(
                                crate::library::LibraryKind::Code,
                                file,
                            ))
                    }
                    None => {}
                }
            }
            Panel::Viewport => {}
        }
    }
}

/// The border around the viewport and the Game panel while the game
/// runs, so it's plain that changes made now are undone on Stop.
fn playing_border(ui: &Ui, rect: egui::Rect, accent: Accent) {
    ui.painter().rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(2.0, accent.colors().light),
        egui::StrokeKind::Inside,
    );
}

/// The small menu row under a panel's title: "≡ Edit View …".
pub(crate) fn menu_row(ui: &mut Ui, items: &[&str]) {
    theme::menu_row_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            ui.label("≡");
            for item in items {
                ui.label(*item);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                response.on_hover_text("Searching arrives with the panel's contents");
                icons::icon(ui.painter(), rect, Icon::Search, theme::TEXT_WEAK);
            });
        });
    });
}

/// Shown while files are dragged over the window: where they'll go.
pub(crate) fn drop_hint(ui: &Ui, accent: Accent) {
    let screen = ui.ctx().content_rect();
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        Id::new("drop hint"),
    ));
    painter.rect_filled(screen, 0.0, Color32::from_black_alpha(140));
    painter.rect_stroke(
        screen.shrink(6.0),
        6.0,
        egui::Stroke::new(2.0, accent.colors().light),
        egui::StrokeKind::Inside,
    );
    painter.text(
        screen.center(),
        egui::Align2::CENTER_CENTER,
        "Drop to import the model (.gltf or .glb)",
        egui::FontId::proportional(18.0),
        theme::TEXT_STRONG,
    );
}

/// A quiet note in a panel that has nothing to show yet.
pub(crate) fn placeholder(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .inner_margin(theme::panel_margin())
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(theme::TEXT_WEAK));
        });
}
