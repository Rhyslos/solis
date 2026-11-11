//! Drawing each docked panel's contents, and the bits panels share: the
//! small menu row under a title, a quiet placeholder note, and the hint
//! shown while files are dragged over the window.

use crate::area_panel::{AreaPanelState, AreaRequest, area_tree};
use crate::console::{Console, ConsoleRequest, console};
use crate::document::Document;
use crate::icons;
use crate::library::{LibraryItem, LibraryRequest, LibraryState, ProjectInfo, library};
use crate::menu_items::MenuState;
use crate::models::ModelInfo;
use crate::panel::Panel;
use crate::properties::{AssemblyPart, PropertyRequest, properties};
use crate::theme::{self, Accent};
use crate::viewport::{Viewport, ViewportRenderer};
use egui::{Color32, Id, RichText, Ui, WidgetText};
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
    /// A test screen shown over the game (`ui test`).
    pub(crate) ui_test: Option<std::sync::Arc<ui::Screen>>,
    /// The game's screens while it plays (decision 0087).
    pub(crate) game_screens: &'a [std::sync::Arc<ui::Screen>],
    /// Whether the game is running.
    pub(crate) playing: bool,
    /// Whether the game has the mouse and keys.
    pub(crate) game_input: bool,
    /// The project's 2D settings, for a 2D camera's picture.
    pub(crate) pixels: area::pixels::PixelSettings,
    /// Set when the Game panel's picture was clicked.
    pub(crate) game_out: &'a mut crate::game_panel::GamePanelOut,
    pub(crate) document: &'a Document,
    pub(crate) models: &'a BTreeMap<AssetId, ModelInfo>,
    pub(crate) area_requests: &'a mut Vec<AreaRequest>,
    pub(crate) project: Option<&'a ProjectInfo>,
    pub(crate) library: &'a [LibraryItem],
    pub(crate) folders: &'a [String],
    pub(crate) library_state: &'a mut LibraryState,
    pub(crate) library_requests: &'a mut Vec<LibraryRequest>,
    /// Set when the docked Library's Unpin was clicked.
    pub(crate) library_pin_clicked: &'a mut bool,
    /// What the Hints panel says (none: hints are off).
    pub(crate) hints: Option<&'a [String]>,
    /// The sound files measured, and the one playing.
    pub(crate) sounds: crate::library::Sounds<'a>,
    /// The sound file Properties shows instead of the selection, if any.
    pub(crate) sound_shown: Option<&'a str>,
    /// The text file shown, as the app last read it.
    pub(crate) text_file: Option<&'a (String, crate::text_view::TextRead)>,
    /// The project's buses, and the bus Properties shows, if any.
    pub(crate) mixer: &'a crate::mixer_view::MixerState,
    pub(crate) mixer_requests: &'a mut Vec<crate::mixer_view::MixerRequest>,
    pub(crate) part: Option<&'a AssemblyPart>,
    /// While playing, the selected entity's game code values as they are
    /// now, by component.
    pub(crate) running: Option<&'a BTreeMap<String, reflect::ComponentData>>,
    pub(crate) requests: &'a mut Vec<PropertyRequest>,
    /// The manual's address and where game code components are written,
    /// for Properties' **?** and **Open in Ledger** (decision 0085).
    pub(crate) help: crate::properties::Help<'a>,
    pub(crate) accent: Accent,
    pub(crate) in_front: &'a HashSet<Panel>,
    pub(crate) shown: &'a mut HashSet<Panel>,
    /// Where each frame's time goes.
    pub(crate) profiler: &'a crate::profiler::Profiler,
    /// The clips file open in the Clips and Frames tabs, if any (Phase 9,
    /// D3), and what happened to it this frame.
    pub(crate) frames: Option<crate::frames_view::FramesUi<'a>>,
    pub(crate) frames_out: &'a mut FramesOut,
    /// The animator file open in the Animator tab, if any (Phase 9, D3b).
    pub(crate) animator: Option<crate::animator_view::AnimatorUi<'a>>,
    /// The tileset open in the Tileset tab, if any (Phase 9, D5).
    pub(crate) tileset: Option<crate::tileset_view::TilesetUi<'a>>,
    /// The atlas open in the Atlas Crop tab, its picture, and why there's
    /// none (decision 0089).
    pub(crate) atlas: Option<(
        &'a mut crate::atlas_editor::AtlasEditor,
        Option<crate::atlas_view::AtlasPicture<'a>>,
        Option<String>,
    )>,
    /// The selected tile map, for the Map tab (Phase 9, D4).
    pub(crate) map: Option<crate::map_tab::MapUi<'a>>,
    /// The screen open on the canvas, the fonts it's measured with and the
    /// screens' pictures (decision 0089).
    pub(crate) screen: Option<ScreenUi<'a>>,
    pub(crate) map_requests: &'a mut Vec<crate::map_tab::MapRequest>,
}

/// The screen open on the canvas, for the Canvas, Elements and
/// Properties.
pub(crate) struct ScreenUi<'a> {
    pub(crate) doc: &'a mut crate::screen_doc::ScreenDoc,
    pub(crate) fonts: &'a mut ui::Fonts,
    pub(crate) pictures: Option<std::sync::Arc<ui::Pictures>>,
    /// The project's pictures, for the Image tool.
    pub(crate) picture_files: Vec<String>,
    /// What screens can be tied to and send.
    pub(crate) game: std::sync::Arc<crate::screen_ties::ScreenGame>,
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
    /// The animator file changed: save it.
    pub(crate) animator_changed: bool,
    /// The Animator tab was closed: close the file.
    pub(crate) animator_closed: bool,
    /// The pointer is over the Animator tab (Ctrl+Z and Delete are its).
    pub(crate) animator_hovered: bool,
    /// The pointer is over the Map tab (Ctrl+Z is its).
    pub(crate) map_hovered: bool,
    /// The tileset changed: save it.
    pub(crate) tileset_changed: bool,
    /// The Tileset tab was closed: close the file.
    pub(crate) tileset_closed: bool,
    /// The pointer is over the Tileset tab (Ctrl+Z is its).
    pub(crate) tileset_hovered: bool,
    /// Save this kind's template.
    pub(crate) tileset_template: Option<tiles::TileKind>,
    /// The atlas changed: save it.
    pub(crate) atlas_changed: bool,
    /// The Atlas Crop tab was closed: close the file.
    pub(crate) atlas_closed: bool,
    /// The pointer is over the Atlas Crop tab (Ctrl+Z is its).
    pub(crate) atlas_hovered: bool,
    /// What the Atlas Crop tab has to say in the Console.
    pub(crate) atlas_said: Option<String>,
    /// The screen on the canvas changed: save it.
    pub(crate) screen_changed: bool,
    /// The pointer is over the canvas or Elements (Ctrl+Z is theirs).
    pub(crate) screen_hovered: bool,
    /// Open the screen's file as code.
    pub(crate) screen_code: bool,
    /// What the canvas has to say in the Console.
    pub(crate) screen_said: Option<String>,
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
        let what = if *tab == Panel::Map {
            self.map.as_ref().map(|m| m.name.clone())
        } else if *tab == Panel::Tileset {
            self.tileset
                .as_ref()
                .map(|t| crate::sprite_sheets::name_of(&t.editor.path))
        } else if *tab == Panel::Animator {
            self.animator
                .as_ref()
                .map(|a| crate::sprite_sheets::name_of(&a.editor.path))
        } else {
            self.frames.as_ref().map(|f| match tab {
                Panel::Clips => crate::sprite_sheets::name_of(&f.editor.path),
                _ => f.editor.clip.clone(),
            })
        };
        if tab.comes_with_a_file()
            && let Some(what) = what.filter(|w| !w.is_empty())
        {
            job.append(
                &format!("· {what}"),
                5.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(theme::TEXT_SIZE),
                    color: theme::text_weak(),
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
            (false, _) => theme::text_weak(),
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
                placeholder(
                    ui,
                    "Layers (groups of entities to show, hide or lock together) aren't built \
                     yet. Until then, hide and lock entities in the Area panel.",
                );
            }
            Panel::Properties => {
                if (self.in_front.contains(&Panel::Canvas)
                    || self.in_front.contains(&Panel::Elements))
                    && let Some(screen) = &mut self.screen
                {
                    egui::ScrollArea::vertical()
                        .id_salt("screen properties")
                        .show(ui, |ui| {
                            egui::Frame::new().inner_margin(8.0).show(ui, |ui| {
                                self.frames_out.screen_changed |=
                                    crate::screen_props::screen_properties(
                                        ui,
                                        screen.doc,
                                        &screen.game,
                                    );
                            });
                        });
                    return;
                }
                if self.in_front.contains(&Panel::Tileset)
                    && let Some(tileset) = &mut self.tileset
                {
                    egui::ScrollArea::vertical()
                        .id_salt("tileset properties")
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Frame::new()
                                .inner_margin(theme::panel_margin())
                                .show(ui, |ui| {
                                    self.frames_out.tileset_changed |=
                                        crate::tileset_view::tileset_properties(ui, tileset);
                                });
                        });
                    return;
                }
                if self.in_front.contains(&Panel::Animator)
                    && let Some(animator) = &mut self.animator
                {
                    egui::ScrollArea::vertical()
                        .id_salt("animator properties")
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Frame::new()
                                .inner_margin(theme::panel_margin())
                                .show(ui, |ui| {
                                    self.frames_out.animator_changed |=
                                        crate::animator_view::animator_properties(ui, animator);
                                });
                        });
                    return;
                }
                if let Some(path) = self.sound_shown {
                    if self
                        .library
                        .iter()
                        .any(|i| i.path == path && i.kind == crate::library::LibraryKind::Text)
                    {
                        let read = self
                            .text_file
                            .filter(|(read_path, _)| read_path == path)
                            .map(|(_, read)| read);
                        let requests = crate::text_view::text_properties(ui, path, read);
                        self.library_requests.extend(requests);
                        return;
                    }
                    let sheets = self.sounds.sheets;
                    if let Some(crate::sprite_sheets::TilesetState::Read(set, notes)) =
                        sheets.tilesets.get(path)
                    {
                        egui::Frame::new()
                            .inner_margin(theme::panel_margin())
                            .show(ui, |ui| {
                                if let Some(changed) =
                                    crate::map_tab::tileset_properties(ui, path, set, notes)
                                {
                                    self.library_requests.push(LibraryRequest::SaveTileset(
                                        path.to_owned(),
                                        changed,
                                    ));
                                }
                            });
                        return;
                    }
                    if let Some(crate::sprite_sheets::MapFileState::Read(map, notes)) =
                        sheets.tile_maps.get(path)
                    {
                        let tilesets = sheets.listed_tiles(false);
                        egui::Frame::new()
                            .inner_margin(theme::panel_margin())
                            .show(ui, |ui| {
                                if let Some((tileset, size)) = crate::map_tab::map_file_properties(
                                    ui, path, map, &tilesets, notes,
                                ) {
                                    self.library_requests.push(LibraryRequest::ShapeTileMap(
                                        path.to_owned(),
                                        tileset,
                                        size,
                                    ));
                                }
                            });
                        return;
                    }
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
                    self.help,
                );
                self.requests.extend(requests);
            }
            Panel::Library | Panel::Code => {
                let code = *tab == Panel::Code;
                let requests = library(
                    ui,
                    self.project,
                    (self.library, self.folders),
                    self.library_state,
                    self.accent,
                    code,
                    self.sounds,
                    (
                        (!code).then_some(crate::library::Pin::Docked),
                        &mut *self.library_pin_clicked,
                    ),
                );
                self.library_requests.extend(requests);
            }
            Panel::Game => {
                let full = ui.max_rect();
                let mut game_ui = ui.new_child(egui::UiBuilder::new().max_rect(full));
                *self.game_out = crate::game_panel::game_panel(
                    &mut game_ui,
                    self.game,
                    self.playing,
                    self.game_input,
                    &self.viewport.camera,
                    self.pixels,
                    &self.viewport.sheets,
                    self.renderer,
                    self.ui_test
                        .iter()
                        .chain(self.game_screens)
                        .cloned()
                        .collect(),
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
            Panel::Hints => crate::hints::hints_panel(ui, self.hints),
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
            Panel::Map => {
                if ui.rect_contains_pointer(ui.max_rect()) {
                    self.frames_out.map_hovered = true;
                }
                let Some(map) = &self.map else {
                    placeholder(ui, "Select an entity with a Tile Map to see its map here.");
                    return;
                };
                egui::ScrollArea::vertical()
                    .id_salt("map tab")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        egui::Frame::new()
                            .inner_margin(theme::panel_margin())
                            .show(ui, |ui| {
                                let asked = crate::map_tab::map_tab(ui, map);
                                self.map_requests.extend(asked);
                            });
                    });
            }
            Panel::Tileset => {
                if ui.rect_contains_pointer(ui.max_rect()) {
                    self.frames_out.tileset_hovered = true;
                }
                let Some(tileset) = &mut self.tileset else {
                    placeholder(ui, "No tileset is open.");
                    return;
                };
                let out = crate::tileset_view::tileset_tab(ui, tileset);
                self.frames_out.tileset_changed |= out.changed;
                if out.save_template.is_some() {
                    self.frames_out.tileset_template = out.save_template;
                }
            }
            Panel::Canvas => {
                let Some(screen) = &mut self.screen else {
                    placeholder(
                        ui,
                        "No screen is open. Double-click a screen's file (in ui/) in the \
                         Library to draw it here.",
                    );
                    return;
                };
                let out = crate::canvas_view::canvas_tab(
                    ui,
                    crate::canvas_view::CanvasUi {
                        doc: screen.doc,
                        renderer: self.renderer,
                        fonts: screen.fonts,
                        pictures: screen.pictures.clone(),
                        accent: self.accent.colors().fill,
                        picture_files: &screen.picture_files,
                        readable: &screen.game.readable,
                    },
                );
                if out.said.is_some() {
                    self.frames_out.screen_said = out.said;
                }
                self.frames_out.screen_changed |= out.changed;
                self.frames_out.screen_hovered |= out.hovered;
                self.frames_out.screen_code |= out.open_code;
            }
            Panel::Elements => {
                let Some(screen) = &mut self.screen else {
                    placeholder(ui, "No screen is open.");
                    return;
                };
                let out = egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(6, 6))
                    .show(ui, |ui| {
                        crate::elements_view::elements_tab(
                            ui,
                            screen.doc,
                            self.accent.colors().fill,
                        )
                    })
                    .inner;
                self.frames_out.screen_changed |= out.changed;
                self.frames_out.screen_hovered |= out.hovered;
            }
            Panel::AtlasCrop => {
                if ui.rect_contains_pointer(ui.max_rect()) {
                    self.frames_out.atlas_hovered = true;
                }
                let Some((editor, picture, problem)) = &mut self.atlas else {
                    placeholder(ui, "No picture's crops are open.");
                    return;
                };
                let out = crate::atlas_view::atlas_tab(
                    ui,
                    editor,
                    picture.take(),
                    problem.as_deref(),
                    self.accent.colors().fill,
                );
                self.frames_out.atlas_changed |= out.changed;
                if out.said.is_some() {
                    self.frames_out.atlas_said = out.said;
                }
            }
            Panel::Animator => {
                if ui.rect_contains_pointer(ui.max_rect()) {
                    self.frames_out.animator_hovered = true;
                }
                let Some(animator) = &mut self.animator else {
                    placeholder(ui, "No animator is open.");
                    return;
                };
                self.frames_out.animator_changed |=
                    crate::animator_view::animator_tab(ui, animator);
            }
            Panel::Viewport => {}
        }
    }

    fn is_closeable(&self, tab: &Panel) -> bool {
        // The Map tab comes and goes with the selection, not a file.
        tab.comes_with_a_file() && *tab != Panel::Map
    }

    fn on_close(&mut self, tab: &mut Panel) -> egui_dock::widgets::tab_viewer::OnCloseResponse {
        if *tab == Panel::Animator {
            self.frames_out.animator_closed = true;
            return egui_dock::widgets::tab_viewer::OnCloseResponse::Close;
        }
        if *tab == Panel::Tileset {
            self.frames_out.tileset_closed = true;
            return egui_dock::widgets::tab_viewer::OnCloseResponse::Close;
        }
        if *tab == Panel::AtlasCrop {
            self.frames_out.atlas_closed = true;
            return egui_dock::widgets::tab_viewer::OnCloseResponse::Close;
        }
        // Either tab closes the clips file, and the other goes with it.
        self.frames_out.closed = true;
        egui_dock::widgets::tab_viewer::OnCloseResponse::Close
    }
}

/// The border around the viewport and the Game panel while the game
/// runs, so it's plain that the area goes back on Stop.
fn playing_border(ui: &Ui, rect: egui::Rect, accent: Accent) {
    ui.painter().rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(2.0, accent.colors().light),
        egui::StrokeKind::Inside,
    );
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
        theme::text_strong(),
    );
}

/// A quiet note in a panel that has nothing to show yet.
pub(crate) fn placeholder(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .inner_margin(theme::panel_margin())
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(theme::text_weak()));
        });
}
