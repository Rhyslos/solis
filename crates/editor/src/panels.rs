//! Drawing each panel's contents (the shell decides where), and the bits panels share: the
//! small menu row under a title, a quiet note for a panel with nothing
//! to show, and the hint shown while files are dragged over the window.

use crate::area_panel::{AreaPanelState, AreaRequest, area_tree};
use crate::console::{Console, ConsoleRequest, console};
use crate::document::Document;
use crate::key_owner::KeyOwner;
use crate::library::{LibraryItem, LibraryRequest, LibraryState, ProjectInfo, library};
use crate::models::ModelInfo;
use crate::panel::Panel;
use crate::properties::{AssemblyPart, PropertyRequest, properties};
use crate::theme::{self, Accent};
use crate::viewport::{Viewport, ViewportRenderer};
use egui::{Color32, Id, RichText, Ui};
use reflect::AssetId;
use std::collections::{BTreeMap, HashSet};

/// Draws each panel, wherever the shell puts it.
pub(crate) struct PanelViewer<'a> {
    pub(crate) area_state: &'a mut AreaPanelState,
    /// The components whose values were copied, by name.
    pub(crate) copied: &'a [String],
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
    /// What the Hints card says (none: hints are off).
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
    /// Who the keys belong to this frame: a tab acts on keys only when
    /// it's the one named.
    pub(crate) keys: KeyOwner,
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

/// What the tabs that come with a file did this frame.
#[derive(Debug, Default)]
pub(crate) struct FramesOut {
    /// The panel the pointer is over, if it's over one: who the keys
    /// belong to next frame is worked out from it.
    pub(crate) pointer_over: Option<Panel>,
    /// The clips file changed: save it.
    pub(crate) changed: bool,
    /// The Clips or Frames tab was closed: close the file.
    pub(crate) closed: bool,
    /// The animator file changed: save it.
    pub(crate) animator_changed: bool,
    /// The Animator tab was closed: close the file.
    pub(crate) animator_closed: bool,
    /// The tileset changed: save it.
    pub(crate) tileset_changed: bool,
    /// The Tileset tab was closed: close the file.
    pub(crate) tileset_closed: bool,
    /// Save this kind's template.
    pub(crate) tileset_template: Option<tiles::TileKind>,
    /// The tileset whose terrains Properties shows (its path): undo and
    /// redo over Properties are that tileset's.
    pub(crate) properties_tileset: Option<String>,
    /// The atlas changed: save it.
    pub(crate) atlas_changed: bool,
    /// The Atlas Crop tab was closed: close the file.
    pub(crate) atlas_closed: bool,
    /// What the Atlas Crop tab has to say in the Console.
    pub(crate) atlas_said: Option<String>,
    /// The screen on the canvas changed: save it.
    pub(crate) screen_changed: bool,
    /// Open the screen's file as code.
    pub(crate) screen_code: bool,
    /// What the canvas has to say in the Console.
    pub(crate) screen_said: Option<String>,
    /// The Canvas or Elements tab was closed: close the screen.
    pub(crate) screen_closed: bool,
}

impl PanelViewer<'_> {
    /// The words on `panel`'s document tab: the area's name on the
    /// viewport's, and a file's name after the panel's where it shows one
    /// ("Animator \u{b7} walk").
    pub(crate) fn tab_title(&self, panel: Panel) -> String {
        if panel == Panel::Viewport {
            return self.document.name();
        }
        let what = match panel {
            Panel::Map => self.map.as_ref().map(|m| m.name.clone()),
            Panel::Tileset => self
                .tileset
                .as_ref()
                .map(|t| crate::sprite_sheets::name_of(&t.editor.path)),
            Panel::Animator => self
                .animator
                .as_ref()
                .map(|a| crate::sprite_sheets::name_of(&a.editor.path)),
            Panel::AtlasCrop => self
                .atlas
                .as_ref()
                .map(|(editor, ..)| crate::sprite_sheets::name_of(&editor.path)),
            Panel::Canvas => self
                .screen
                .as_ref()
                .map(|s| crate::sprite_sheets::name_of(&s.doc.path)),
            _ => None,
        };
        match what.filter(|w| !w.is_empty()) {
            Some(what) => format!("{} \u{b7} {what}", panel.title()),
            None => panel.title().to_owned(),
        }
    }

    /// Whether `panel` shows a file of its own that its × closes now.
    pub(crate) fn closes_a_file(&self, panel: Panel) -> bool {
        match panel {
            Panel::Clips | Panel::Frames => self.frames.is_some(),
            Panel::Animator => self.animator.is_some(),
            Panel::Tileset => self.tileset.is_some(),
            Panel::AtlasCrop => self.atlas.is_some(),
            Panel::Canvas | Panel::Elements => self.screen.is_some(),
            Panel::Viewport
            | Panel::Game
            | Panel::Area
            | Panel::Properties
            | Panel::Library
            | Panel::Code
            | Panel::Console
            | Panel::Problems
            | Panel::Profiler
            | Panel::Mixer
            | Panel::Map => false,
        }
    }

    /// Closes the file `panel` shows (its ×). Every panel is named, with
    /// no arm for "the rest": a new one doesn't build until it's decided
    /// what its × closes.
    pub(crate) fn close(&mut self, panel: Panel) {
        match panel {
            // Either tab closes the clips file, and the other goes with it.
            Panel::Clips | Panel::Frames => self.frames_out.closed = true,
            Panel::Animator => self.frames_out.animator_closed = true,
            Panel::Tileset => self.frames_out.tileset_closed = true,
            Panel::AtlasCrop => self.frames_out.atlas_closed = true,
            Panel::Canvas | Panel::Elements => self.frames_out.screen_closed = true,
            // These have no × (`closes_a_file`): nothing closes them.
            Panel::Viewport
            | Panel::Game
            | Panel::Area
            | Panel::Properties
            | Panel::Library
            | Panel::Code
            | Panel::Console
            | Panel::Problems
            | Panel::Profiler
            | Panel::Mixer
            | Panel::Map => {}
        }
    }

    /// How many problems there are, for the Problems tab's name.
    pub(crate) fn problem_count(&self) -> usize {
        self.problems.len() + self.code_problems.len()
    }

    /// The count a shelf column's header shows for `panel`, if it has
    /// one: the area's entities over the Area column. The area keeps its
    /// count, so nothing is walked.
    pub(crate) fn column_count(&self, panel: Panel) -> Option<String> {
        (panel == Panel::Area).then(|| self.document.area().len().to_string())
    }

    /// The tools `panel` puts in its column's header: the Area column's
    /// search, Fold everything and ⋯.
    pub(crate) fn column_tools(&mut self, ui: &mut Ui, panel: Panel) {
        if panel == Panel::Area
            && let Some(command) = crate::area_panel::header_tools(ui, self.area_state)
        {
            self.area_requests.push(AreaRequest::Command(command));
        }
    }

    /// Draws `panel`'s contents filling `ui`.
    pub(crate) fn draw_panel(&mut self, ui: &mut Ui, panel: Panel) {
        let tab = &panel;
        self.shown.insert(*tab);
        // Noted here for every panel alike, and nowhere else.
        if ui.rect_contains_pointer(ui.max_rect()) {
            self.frames_out.pointer_over = Some(*tab);
        }
        if *tab == Panel::Viewport {
            let full = ui.max_rect();
            let mut view_ui = ui.new_child(egui::UiBuilder::new().max_rect(full));
            self.viewport.show(&mut view_ui, self.renderer, self.accent);
            ui.advance_cursor_after_rect(full);
            return;
        }
        match tab {
            Panel::Area => {
                let requests =
                    area_tree(ui, self.document, self.models, self.accent, self.area_state);
                self.area_requests.extend(requests);
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
                    self.frames_out.properties_tileset = Some(tileset.editor.path.clone());
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
                        self.frames_out.properties_tileset = Some(path.to_owned());
                        egui::Frame::new()
                            .inner_margin(theme::panel_margin())
                            .show(ui, |ui| {
                                let asked =
                                    crate::map_tab::tileset_properties(ui, path, set, notes);
                                self.library_requests.extend(asked.into_iter().map(|edit| {
                                    LibraryRequest::EditTileset(path.to_owned(), edit)
                                }));
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
                    (self.help, self.hints),
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
                let Some(frames) = &mut self.frames else {
                    empty_note(ui, "No clips file is open.");
                    return;
                };
                let changed = if *tab == Panel::Clips {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| crate::frames_view::clips_tab(ui, frames))
                        .inner
                } else {
                    crate::frames_view::frames_tab(ui, frames, self.keys == KeyOwner::Frames)
                };
                self.frames_out.changed |= changed;
            }
            Panel::Map => {
                let Some(map) = &self.map else {
                    empty_note(ui, "Select an entity with a Tile Map to see its map here.");
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
                let Some(tileset) = &mut self.tileset else {
                    empty_note(ui, "No tileset is open.");
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
                    empty_note(
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
                        keys: self.keys == KeyOwner::Canvas,
                    },
                );
                if out.said.is_some() {
                    self.frames_out.screen_said = out.said;
                }
                self.frames_out.screen_changed |= out.changed;
                self.frames_out.screen_code |= out.open_code;
            }
            Panel::Elements => {
                let Some(screen) = &mut self.screen else {
                    empty_note(ui, "No screen is open.");
                    return;
                };
                let out = egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(6, 6))
                    .show(ui, |ui| {
                        crate::elements_view::elements_tab(
                            ui,
                            screen.doc,
                            self.accent.colors().fill,
                            self.keys == KeyOwner::Elements,
                        )
                    })
                    .inner;
                self.frames_out.screen_changed |= out.changed;
            }
            Panel::AtlasCrop => {
                let Some((editor, picture, problem)) = &mut self.atlas else {
                    empty_note(ui, "No picture's crops are open.");
                    return;
                };
                let out = crate::atlas_view::atlas_tab(
                    ui,
                    editor,
                    picture.take(),
                    problem.as_deref(),
                    self.accent.colors().fill,
                    self.keys == KeyOwner::Atlas,
                );
                self.frames_out.atlas_changed |= out.changed;
                if out.said.is_some() {
                    self.frames_out.atlas_said = out.said;
                }
            }
            Panel::Animator => {
                let Some(animator) = &mut self.animator else {
                    empty_note(ui, "No animator is open.");
                    return;
                };
                self.frames_out.animator_changed |= crate::animator_view::animator_tab(
                    ui,
                    animator,
                    self.keys == KeyOwner::Animator,
                );
            }
            Panel::Viewport => {}
        }
    }
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

/// A quiet note in a panel that has nothing to show right now ("No
/// tileset is open."). Something the editor can't do yet is another
/// matter: that is drawn by `crate::unbuilt`, and only there.
pub(crate) fn empty_note(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .inner_margin(theme::panel_margin())
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(theme::text_weak()));
        });
}
