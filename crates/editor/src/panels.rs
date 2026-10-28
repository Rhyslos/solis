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
    /// Whether the game has the mouse and keys.
    pub(crate) game_input: bool,
    /// The project's 2D settings, for a 2D camera's picture.
    pub(crate) pixels: area::pixels::PixelSettings,
    /// Set when the Game panel's picture was clicked.
    pub(crate) game_clicked: &'a mut bool,
    pub(crate) document: &'a Document,
    pub(crate) models: &'a BTreeMap<AssetId, ModelInfo>,
    pub(crate) area_requests: &'a mut Vec<AreaRequest>,
    pub(crate) project: Option<&'a ProjectInfo>,
    pub(crate) library: &'a [LibraryItem],
    pub(crate) folders: &'a [String],
    pub(crate) library_state: &'a mut LibraryState,
    pub(crate) library_requests: &'a mut Vec<LibraryRequest>,
    /// The sound files measured, and the one playing.
    pub(crate) sounds: crate::library::Sounds<'a>,
    /// The sound file Properties shows instead of the selection, if any.
    pub(crate) sound_shown: Option<&'a str>,
    /// The project's buses, and the bus Properties shows, if any.
    pub(crate) mixer: &'a crate::mixer_view::MixerState,
    pub(crate) mixer_requests: &'a mut Vec<crate::mixer_view::MixerRequest>,
    pub(crate) part: Option<&'a AssemblyPart>,
    /// While playing, the selected entity's game code values as they are
    /// now, by component.
    pub(crate) running: Option<&'a BTreeMap<String, reflect::ComponentData>>,
    pub(crate) requests: &'a mut Vec<PropertyRequest>,
    pub(crate) accent: Accent,
    pub(crate) in_front: &'a HashSet<Panel>,
    pub(crate) shown: &'a mut HashSet<Panel>,
    /// Where each frame's time goes.
    pub(crate) profiler: &'a crate::profiler::Profiler,
    /// The clips file open in the Clips and Frames tabs, if any (Phase 9,
    /// D3), and what happened to it this frame.
    pub(crate) frames: Option<crate::frames_view::FramesUi<'a>>,
    pub(crate) frames_out: &'a mut FramesOut,
}

/// What the Clips and Frames tabs did this frame.
#[derive(Debug, Default)]
pub(crate) struct FramesOut {
    /// The file changed: save it.
    pub(crate) changed: bool,
    /// A tab was closed: close the file.
    pub(crate) closed: bool,
    /// The pointer is over one of the tabs (Ctrl+Z, Delete and Space are
    /// theirs then).
    pub(crate) hovered: bool,
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
        // The clips file's name, and the clip chosen.
        let what = self.frames.as_ref().map(|f| match tab {
            Panel::Clips => crate::sprite_sheets::name_of(&f.editor.path),
            _ => f.editor.clip.clone(),
        });
        if tab.comes_with_a_file()
            && let Some(what) = what.filter(|w| !w.is_empty())
        {
            job.append(
                &format!("· {what}"),
                5.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(theme::TEXT_SIZE),
                    color: theme::TEXT_WEAK,
                    ..Default::default()
                },
            );
        }
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
                if let Some(path) = self.sound_shown {
                    if assets::aseprite::is_aseprite_file(std::path::Path::new(path)) {
                        if crate::sheet_view::sheet_properties(
                            ui,
                            path,
                            self.sounds.sheets.by_path.get(path),
                            self.sounds.sheet_pictures.get(path),
                        ) {
                            self.library_requests
                                .push(LibraryRequest::MakeClipsFile(path.to_owned()));
                        }
                        return;
                    }
                    if self.sounds.events.contains_key(path) || path.ends_with(".solsound") {
                        let requests = crate::sound_event_view::event_properties(
                            ui,
                            path,
                            self.sounds,
                            self.library,
                        );
                        self.library_requests.extend(requests);
                    } else {
                        let request = crate::sound_view::sound_properties(
                            ui,
                            path,
                            self.sounds.states.get(path),
                            self.sounds.preview,
                        );
                        self.library_requests.extend(request);
                    }
                    return;
                }
                if let Some((bus, _)) = self.mixer.shown {
                    let requests = crate::mixer_view::bus_properties(ui, self.mixer, bus);
                    self.mixer_requests.extend(requests);
                    return;
                }
                let requests = properties(
                    ui,
                    self.document,
                    self.part,
                    (self.models, self.sounds.sheets),
                    self.copied,
                    self.running,
                );
                self.requests.extend(requests);
            }
            Panel::Library | Panel::Code => {
                let requests = library(
                    ui,
                    self.project,
                    (self.library, self.folders),
                    self.library_state,
                    self.accent,
                    *tab == Panel::Code,
                    self.sounds,
                );
                self.library_requests.extend(requests);
            }
            Panel::Game => {
                let full = ui.max_rect();
                let mut game_ui = ui.new_child(egui::UiBuilder::new().max_rect(full));
                *self.game_clicked |= crate::game_panel::game_panel(
                    &mut game_ui,
                    self.game,
                    self.playing,
                    self.game_input,
                    &self.viewport.camera,
                    self.pixels,
                    &self.viewport.sheets,
                    self.renderer,
                );
                if self.playing {
                    playing_border(ui, full, self.accent);
                }
                if crate::mixer_view::game_mute_button(ui, full, self.mixer.listening.all_muted) {
                    self.mixer_requests
                        .push(crate::mixer_view::MixerRequest::MuteAll);
                }
                ui.advance_cursor_after_rect(full);
            }
            Panel::Console => {
                if let Some(request) = console(ui, self.console) {
                    self.console_requests.push(request);
                }
            }
            Panel::Mixer => {
                let requests = crate::mixer_view::mixer_panel(ui, self.mixer, self.playing);
                self.mixer_requests.extend(requests);
            }
            Panel::Profiler => {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| crate::profiler::profiler_panel(ui, self.profiler));
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
            Panel::Clips | Panel::Frames => {
                if ui.rect_contains_pointer(ui.max_rect()) {
                    self.frames_out.hovered = true;
                }
                let Some(frames) = &mut self.frames else {
                    placeholder(ui, "No clips file is open.");
                    return;
                };
                let changed = if *tab == Panel::Clips {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| crate::frames_view::clips_tab(ui, frames))
                        .inner
                } else {
                    crate::frames_view::frames_tab(ui, frames)
                };
                self.frames_out.changed |= changed;
            }
            Panel::Viewport => {}
        }
    }

    fn is_closeable(&self, tab: &Panel) -> bool {
        tab.comes_with_a_file()
    }

    fn on_close(&mut self, _tab: &mut Panel) -> egui_dock::widgets::tab_viewer::OnCloseResponse {
        // Either tab closes the file, and the other goes with it.
        self.frames_out.closed = true;
        egui_dock::widgets::tab_viewer::OnCloseResponse::Close
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
