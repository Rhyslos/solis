//! One frame of the editor (`Workbench::show`): it begins (keys, the
//! game's ticks, the top), what the panels show is prepared, the panels
//! are drawn through one `PanelViewer`, and what they asked for is
//! carried out.

use super::{GameKey, ViewKey, Workbench, WorkbenchAction};
use crate::area_view::AreaView;
use crate::made_from::MadeFrom;
use crate::panel::Panel;
use crate::panels::{PanelViewer, drop_hint};
use crate::viewport::ViewportRenderer;
use egui::Ui;
use std::collections::{BTreeMap, HashSet};

/// What the panels ask for while they're drawn, carried out after
/// (`Workbench::apply_panel_asks`).
#[derive(Default)]
struct PanelAsks {
    /// The Area column's.
    area: Vec<crate::area_panel::AreaRequest>,
    /// The Library's.
    library: Vec<crate::library::LibraryRequest>,
    /// The Mixer's.
    mixer: Vec<crate::mixer_view::MixerRequest>,
    /// The Console's.
    console: Vec<crate::console::ConsoleRequest>,
    /// Properties'.
    properties: Vec<crate::properties::PropertyRequest>,
    /// The Map tab's.
    map: Vec<crate::map_tab::MapRequest>,
    /// The file tabs' (clips, frames, screen, atlas, tileset, animator).
    frames: crate::panels::FramesOut,
    /// The Game panel's.
    game: crate::game_panel::GamePanelOut,
}

impl Workbench {
    /// Draws the editor filling `ui`, and returns what the app must do:
    /// the frame begins (keys, the game's ticks, the top), what the
    /// panels show is prepared, they're drawn, and what they asked for is
    /// carried out.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
    ) -> Vec<WorkbenchAction> {
        let (mut actions, page) = self.begin_frame(ui);
        let sheets = std::sync::Arc::clone(&self.viewport.sheets);
        let mut shown = HashSet::new();
        let mut asks = PanelAsks::default();
        let picked_before = self.open.library_state.selected.clone();
        self.drop_stale_sound();
        let part = self.assembly_part();
        let (code_problems, copied, game) = self.prepare_frame();
        let running = self.running_values();
        // Only worked out while a screen is open: what it can be tied to
        // once per build of the game's code, the pictures only while the
        // Image tool is in use.
        let (picture_files, screen_game) = self.screen_ties();
        let playing = self.open.playing.is_some();
        let mut renderer = crate::profiler::Timed {
            inner: renderer,
            spent: std::time::Duration::ZERO,
        };
        self.tileset_texture(ui.ctx());
        self.atlas_texture_now(ui.ctx());
        let live = self.live_animator();
        // Which entities use each code file's components, for the Code
        // panel and the Library (decision 0085): only worked out while
        // either is in view, and then only when the area changed.
        let code_uses =
            if self.in_front.contains(&Panel::Code) || self.in_front.contains(&Panel::Library) {
                self.code_uses_now()
            } else {
                std::sync::Arc::default()
            };
        let open_now = self.open_now();
        let layout = self.interface.layout;
        let map_shown = self.map_shown();
        let map_ms = (ui.input(|i| i.time).max(0.0) * 1000.0) as u64;
        let map_name = self
            .open
            .map_edits
            .path
            .as_deref()
            .map(crate::sprite_sheets::name_of)
            .unwrap_or_default();
        let map = map_shown
            .as_ref()
            .map(|(map, tileset)| crate::map_tab::MapUi {
                name: map_name.clone(),
                map,
                tileset: tileset.as_deref(),
                hovered: self.viewport.tile_overlay.hovered,
                hidden: &self.open.map_edits.hidden,
                playing,
                paint: crate::map_tab::PaintShown {
                    code: self.viewport.tile_overlay.paint.code,
                    exact: self.viewport.tile_overlay.paint.exact,
                    piece: self.viewport.tile_overlay.paint.piece,
                    stamp: self
                        .viewport
                        .tile_overlay
                        .paint
                        .stamp
                        .as_ref()
                        .map(|s| s.size),
                    layer: self.viewport.tile_overlay.paint.layer,
                    picture: tileset
                        .as_ref()
                        .and_then(|t| t.picture)
                        .and_then(|p| sheets.tile_art(p))
                        .and_then(|art| {
                            let (texture, atlas) = crate::workbench_tileset::texture_of(
                                &self.open.tile_textures,
                                art,
                            )?;
                            Some(crate::tileset_view::Picture {
                                texture,
                                atlas,
                                size: art.size,
                                durations: &art.durations,
                                art,
                            })
                        }),
                    ms: map_ms,
                    accent: self.accent.colors().fill,
                },
            });
        // The hints for what's selected (decision 0085), while they show.
        let hints =
            (self.hints && self.in_front.contains(&Panel::Properties)).then(|| self.hints());
        let hints_off = !self.hints;
        let atlas_problem = self.atlas_picture().1;
        let frames = self.open.frames_editor.as_mut().map(|editor| {
            crate::workbench_frames::frames_ui(
                editor,
                &sheets,
                &self.open.sheet_pictures,
                &self.open.cue_sounds,
            )
        });
        let tile_ms = (ui.input(|i| i.time).max(0.0) * 1000.0) as u64;
        let tileset = self.open.tilesets.tab_mut().map(|editor| {
            crate::workbench_tileset::tileset_ui(
                editor,
                &sheets,
                &self.open.tile_textures,
                self.open.pixels.tile,
                tile_ms,
                self.accent.colors().fill,
            )
        });
        let screen_pictures = self.open.screen_pictures.pictures.clone();
        let screen_fonts = self
            .screen_fonts
            .get_or_insert_with(crate::game_screens::editor_fonts);
        let screen = self
            .open
            .screen_doc
            .as_mut()
            .map(|doc| crate::panels::ScreenUi {
                doc,
                fonts: screen_fonts,
                pictures: screen_pictures,
                picture_files,
                game: screen_game,
            });
        let atlas = self.open.atlas_editor.as_mut().map(|editor| {
            let picture = self.open.atlas_texture.as_ref().and_then(|(art, texture)| {
                (!art.frames.is_empty()).then_some(crate::atlas_view::AtlasPicture {
                    texture,
                    size: art.size,
                })
            });
            (editor, picture, atlas_problem)
        });
        let animator = self.open.animator_editor.as_mut().map(|editor| {
            crate::workbench_animator::animator_ui(
                editor,
                &sheets,
                &self.open.sheet_pictures,
                live,
                self.accent.colors().fill,
            )
        });
        let mut viewer = PanelViewer {
            area_state: &mut self.open.area_panel,
            copied: &copied,
            viewport: &mut self.viewport,
            renderer: &mut renderer,
            console: &mut self.console,
            console_requests: &mut asks.console,
            problems: &self.open.problems.list,
            code_problems: &code_problems,
            game: game.as_ref().map(|(_, scene)| scene),
            ui_test: self.ui_test.clone(),
            game_screens: &self.open.game_screens.pictures,
            playing,
            game_input: self.game_input,
            pixels: self.open.pixels,
            game_out: &mut asks.game,
            document: &self.open.document,
            models: &self.open.models,
            area_requests: &mut asks.area,
            project: self.open.project.as_ref(),
            library: &self.open.library,
            folders: &self.open.folders,
            library_state: &mut self.open.library_state,
            library_requests: &mut asks.library,
            hints: if hints_off { None } else { hints.as_deref() },
            sounds: crate::library::Sounds {
                states: &self.open.sounds,
                events: &self.open.sound_events,
                preview: self.open.preview.as_ref(),
                mixer: &self.open.mixer.mixer,
                sheets: &sheets,
                sheet_pictures: &self.open.sheet_pictures,
                code_uses: &code_uses,
            },
            mixer: &self.open.mixer,
            mixer_requests: &mut asks.mixer,
            sound_shown: self
                .open
                .sound_shown
                .as_ref()
                .map(|(path, _)| path.as_str()),
            text_file: self.open.text_file.as_ref(),
            part: part.as_ref(),
            running: self
                .open
                .playing
                .as_ref()
                .map(|p| crate::properties::WhilePlaying {
                    values: running.as_ref(),
                    yours: &p.yours,
                }),
            requests: &mut asks.properties,
            help: crate::properties::Help {
                places: self.open.program.as_ref().map(|p| &p.places),
            },
            accent: self.accent,
            in_front: &self.in_front,
            shown: &mut shown,
            profiler: &self.profiler,
            frames,
            frames_out: &mut asks.frames,
            animator,
            tileset,
            atlas,
            map,
            map_requests: &mut asks.map,
            screen,
            keys: self.keys,
        };
        let doc_asks = if page {
            self.shell.drawn.clear();
            Default::default()
        } else {
            crate::shell::middle(ui, &mut self.shell, open_now, layout, &mut viewer)
        };
        self.in_front = shown;
        // The Tileset tab's animated tiles move on by themselves, while it
        // is drawn (the second review pass: any tileset open on any tab
        // redrew the whole editor 25 times a second).
        if self.in_front.contains(&Panel::Tileset)
            && self
                .open
                .tilesets
                .tab()
                .is_some_and(|editor| sheets.tileset_animates(&editor.file))
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(40));
        }
        self.pointer_over = asks.frames.pointer_over;
        self.doc_asks(doc_asks, ui.ctx(), &mut actions);
        self.apply_panel_asks(ui, asks, &picked_before, &mut actions);
        // Kept for the next frame, which may show the same.
        self.open.game_scene = game;
        self.end_frame_windows(ui, &mut actions);
        self.profiler
            .add(crate::profiler::Stage::Draw, renderer.spent);
        let (components, entities) = match &mut self.open.playing {
            Some(playing) => (
                playing.game.as_mut().map_or(0, script::Game::len),
                playing.world.len(),
            ),
            None => (0, self.open.document.area().len()),
        };
        let drawn = renderer.inner.take_report();
        self.profiler.end(drawn, components, entities);
        actions
    }

    /// The frame begins: the profiler, the look, sounds' time, the File
    /// page's outbox, the preview hour and pictures, who has the keys and
    /// what they do, the game's ticks, the status bar and the top (tab bar
    /// and ribbon). What the app must do so far, and whether a File page
    /// or an unfinished screen stands where the panels would be.
    fn begin_frame(&mut self, ui: &mut Ui) -> (Vec<WorkbenchAction>, bool) {
        self.profiler.begin();
        // The look the editor holds is the one in use: the sizes that
        // follow the density read it there, and so does everything that
        // draws an unfinished part or leaves it out.
        crate::interface_look::use_look(self.interface_look());
        self.settle_tab();
        self.profiler.add(
            crate::profiler::Stage::Sound,
            std::mem::take(&mut self.sound_spent),
        );
        let mut actions = std::mem::take(&mut self.open.outbox);
        // The Environment's Preview time (decision 0092), set in
        // Properties: the viewport shows the area at that hour.
        self.viewport.preview_hour = self.preview_hour(ui.ctx());
        self.refresh_sheet_pictures(ui.ctx());
        self.viewport.idle = !self.wants_frames(ui.ctx());
        if ui.input(|i| i.pointer.any_pressed()) {
            self.gesture += 1;
        }
        // While the game has the mouse and keys, they're the game's: the
        // editor's own keys are off (decision 0037).
        self.feed_input(ui.ctx());
        // Who has the keys this frame is decided once, here: each handler
        // below, and each tab drawn after, acts only on what is its own.
        self.keys = self.key_owner(ui.ctx());
        self.tool_keys(ui);
        self.paint_keys(ui);
        self.tool_extra_keys(ui);
        self.undo_keys(ui);
        self.editing_keys(ui, &mut actions);
        self.file_keys(ui, &mut actions);
        self.console_key(ui);
        self.shelf_keys(ui);
        self.search_key(ui);
        let seconds = ui.input(|i| i.stable_dt);
        self.run_game(seconds);
        self.status_bar(ui, &mut actions);
        // The tab bar and the ribbon; a File page or an unfinished
        // screen stands where the panels would be.
        let page = self.top(ui, &mut actions);
        self.follow_selection();
        self.play_frames(ui.input(|i| f64::from(i.stable_dt)));
        self.play_animator(ui.input(|i| f64::from(i.stable_dt)));
        (actions, page)
    }

    /// What the panels show, worked out before they're drawn: the area as
    /// the viewport shows it (again only when something it shows
    /// changed), the tile overlay, the tools, problems and screens. Gives
    /// the code's problems, the names copied and the Game panel's scene
    /// (worked out only while it's in front).
    fn prepare_frame(
        &mut self,
    ) -> (
        Vec<script::CodeProblem>,
        Vec<String>,
        Option<(GameKey, crate::game_panel::GameScene)>,
    ) {
        let preparing = std::time::Instant::now();
        // Worked out again only when something it shows changed: while
        // playing, the area usually doesn't. What's hidden or locked
        // (with everything inside it) is only worked out then too (the
        // phase review, speed): the key holds what was picked, which is
        // small.
        let key = ViewKey {
            revision: self.open.document.revision(),
            sources: self.sources,
            models: self.open.models.len(),
            hidden: self.open.area_panel.hidden.clone(),
            locked: self.open.area_panel.locked.clone(),
        };
        if self.open.editor_view.as_ref() != Some(&key) {
            let in_effect = self.open.area_panel.in_effect(&self.open.document);
            let (hidden, locked) = &*in_effect;
            let world = self.open.document.world_matrices();
            self.viewport.area = AreaView::of_placed(
                self.open.document.area(),
                &self.open.document.tree(),
                &world,
                &self.open.models,
                hidden,
                locked,
            );
            self.open.brushes.refresh(&mut self.open.document);
            self.viewport.area.add_brushes(
                self.open.document.area(),
                &self.open.brushes,
                &world,
                (hidden, locked),
            );
            self.open.editor_view = Some(key);
        }
        self.viewport.entities = self.open.document.area().len();
        self.viewport.placing = self.placing();
        self.set_tile_overlay();
        self.prepare_tools();
        self.find_problems();
        self.refresh_screen_pictures();
        self.refresh_screen();
        let code_problems = self.all_code_problems().into_owned();
        let copied: Vec<String> = self
            .open
            .copied
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        // Worked out only while the Game panel is in front.
        let game = if self.in_front.contains(&crate::panel::Panel::Game) {
            self.game_scene_now()
        } else {
            None
        };
        self.profiler
            .add(crate::profiler::Stage::Prepare, preparing.elapsed());
        (code_problems, copied, game)
    }

    /// While playing, the selected entity's game code values as they are
    /// now, for Properties (in front) to show instead of the values it
    /// started with.
    fn running_values(&mut self) -> Option<BTreeMap<String, reflect::ComponentData>> {
        match (
            &mut self.open.playing,
            self.open.document.selection.primary(),
        ) {
            (Some(playing), Some(id))
                if self.in_front.contains(&crate::panel::Panel::Properties) =>
            {
                let names: Vec<String> = self
                    .open
                    .document
                    .area()
                    .get(id)
                    .map(|e| e.components.keys().cloned().collect())
                    .unwrap_or_default();
                playing.game.as_mut().map(|game| {
                    names
                        .into_iter()
                        .filter_map(|name| game.values(id, &name).map(|v| (name, v)))
                        .collect::<BTreeMap<String, reflect::ComponentData>>()
                })
            }
            _ => None,
        }
    }

    /// While a screen is open: the pictures it can use (only while the
    /// Image tool is in use) and what it can be tied to, worked out once
    /// per build of the game's code.
    fn screen_ties(&mut self) -> (Vec<String>, std::sync::Arc<crate::screen_ties::ScreenGame>) {
        match &self.open.screen_doc {
            Some(doc) => {
                let program = self.open.program.as_ref();
                let game = match &self.open.screen_game {
                    Some((built, game)) if MadeFrom::same(built.as_ref(), program) => {
                        std::sync::Arc::clone(game)
                    }
                    _ => {
                        let game = std::sync::Arc::new(crate::screen_ties::screen_game(
                            program.map(|p| &**p),
                            &self.open.document.registry,
                        ));
                        let built = program.map(MadeFrom::new);
                        self.open.screen_game = Some((built, std::sync::Arc::clone(&game)));
                        game
                    }
                };
                let files: Vec<String> = if doc.tool == crate::canvas_tools::CanvasTool::Image {
                    self.open
                        .library
                        .iter()
                        .filter(|i| {
                            matches!(
                                i.kind,
                                crate::library::LibraryKind::Texture
                                    | crate::library::LibraryKind::SpriteSheet
                            )
                        })
                        .map(|i| i.path.clone())
                        .collect()
                } else {
                    Vec::new()
                };
                (files, game)
            }
            None => (Vec::new(), std::sync::Arc::default()),
        }
    }

    /// Carries out what the panels asked for while they were drawn, in
    /// order: the Build page, the file tabs, the map, the tile overlay and
    /// painting, the Game panel, the tools, Properties, the Library, the
    /// Mixer, the sound picked, the Console and the Area column.
    fn apply_panel_asks(
        &mut self,
        ui: &mut Ui,
        asks: PanelAsks,
        picked_before: &Option<String>,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        if let Some(rect) = self.shell.drawn_at(Panel::Export) {
            let mut asked = Vec::new();
            self.build_page(ui, rect, &mut asked);
            for action in asked {
                self.request(action, actions);
            }
        }
        self.frames_tabs_done(asks.frames);
        self.map_requests_done(asks.map);
        self.tile_overlay_done();
        self.paint_frame();
        self.game_panel_done(ui.ctx(), asks.game);
        self.tool_events();
        for request in asks.properties {
            self.apply_property(request, self.gesture);
        }
        actions.append(&mut self.open.outbox);
        for request in asks.library {
            self.library_request(request, actions);
        }
        for request in asks.mixer {
            if request == crate::mixer_view::MixerRequest::HideUnfinished {
                self.hide_unfinished(ui.ctx(), true);
            } else {
                self.mixer_request(request, actions);
            }
        }
        self.follow_sound_selection(picked_before);
        for request in asks.console {
            self.console_request(request, ui.ctx(), actions);
        }
        for request in asks.area {
            match request {
                crate::area_panel::AreaRequest::Command(command) => self.run(command, ui.ctx()),
                other => self.apply_area_request(other),
            }
        }
    }

    /// The frame's last windows and questions: the drop hint, Keep or Undo
    /// all, what waited for it, the prompts and the update question; then
    /// what must wait while playing is held back.
    fn end_frame_windows(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        if self.files_hovering {
            drop_hint(ui, self.accent);
        }
        self.play_changes_window(ui);
        // What waited for Keep or Undo all (a quit, say) goes on now.
        actions.append(&mut self.open.outbox);
        self.prompts(ui, actions);
        self.update_question(ui.ctx(), actions);
        self.hold_while_playing(actions);
    }
}
