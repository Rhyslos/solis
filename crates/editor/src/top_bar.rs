//! The top bar: the menus, the open documents as tabs, and Play and Build
//! ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).

use crate::console::LogLevel;
use crate::icons::{self, Icon};
use crate::panel::Panel;
use crate::play::PlayState;
use crate::theme::{self, Accent};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Color32, Layout, RichText, Ui};

impl Workbench {
    pub(crate) fn top_bar(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        // The whole bar, so the area tab can fill its full height.
        let bar = ui.max_rect();
        // The right-hand buttons go first, so the menu bar (which takes all
        // the width it is given) only gets what is left.
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            self.play_and_build(ui);
            self.restart_button(ui, actions);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    // The mockup's roomier menu titles.
                    ui.spacing_mut().button_padding = egui::vec2(7.0, 4.0);
                    ui.spacing_mut().item_spacing.x = 2.0;
                    self.menus(ui, actions);
                    ui.add_space(8.0);
                    let mut name = self.document.name();
                    if self.document.is_dirty() {
                        name.push_str(" •");
                    }
                    // The animator open is a document too (Phase 9, D3b):
                    // its tab is in front while the Animator panel is.
                    let animator = self.animator_editor.as_ref().map(|e| {
                        let file = e.path.rsplit_once('/').map_or(e.path.as_str(), |(_, n)| n);
                        (file.to_owned(), self.in_front.contains(&Panel::Animator))
                    });
                    let area_front = animator.as_ref().is_none_or(|(_, front)| !front);
                    let (area_clicked, _) = document_tab(
                        ui,
                        bar,
                        self.accent,
                        Icon::AreaFile,
                        &name,
                        area_front,
                        false,
                    );
                    if area_clicked && !area_front {
                        crate::layout::bring_to_front(&mut self.dock, Panel::Viewport);
                    }
                    if let Some((file, front)) = animator {
                        let (clicked, closed) =
                            document_tab(ui, bar, self.accent, Icon::Animator, &file, front, true);
                        if closed {
                            self.close_animator();
                        } else if clicked {
                            crate::layout::bring_to_front(&mut self.dock, Panel::Animator);
                        }
                    }
                    new_document_button(ui);
                });
            });
        });
    }

    pub(crate) fn menus(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        self.file_menu(ui, actions);
        self.edit_menu(ui);
        self.view_menu(ui);
        self.entity_menu(ui);
        self.play_menu(ui, actions);
        self.build_menu(ui);
        self.window_menu(ui);
        self.help_menu(ui, actions);
    }

    /// The File menu.
    fn file_menu(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        ui.menu_button("File", |ui| {
            if ui
                .button("New project…")
                .on_hover_text("Makes a folder into a project: its assets, settings and areas")
                .clicked()
            {
                self.request(WorkbenchAction::NewProject, actions);
                ui.close();
            }
            if ui.button("Open project…").clicked() {
                self.request(WorkbenchAction::OpenProject, actions);
                ui.close();
            }
            let mut pending = Vec::new();
            ui.add_enabled_ui(!self.recent.is_empty(), |ui| {
                ui.menu_button("Recent projects", |ui| {
                    for path in &self.recent {
                        let name = path.file_name().map_or_else(
                            || path.display().to_string(),
                            |n| n.to_string_lossy().into_owned(),
                        );
                        if ui
                            .button(name)
                            .on_hover_text(path.display().to_string())
                            .clicked()
                        {
                            pending.push(WorkbenchAction::OpenRecent(path.clone()));
                            ui.close();
                        }
                    }
                });
            });
            for action in pending {
                self.request(action, actions);
            }
            ui.separator();
            for (item, keys, action) in [
                ("New area", "Ctrl+N", WorkbenchAction::NewArea),
                ("Open area…", "Ctrl+O", WorkbenchAction::OpenArea),
                ("Save area", "Ctrl+S", WorkbenchAction::SaveArea),
                ("Save area as…", "Ctrl+Shift+S", WorkbenchAction::SaveAreaAs),
            ] {
                if ui
                    .add(egui::Button::new(item).shortcut_text(keys))
                    .clicked()
                {
                    self.request(action, actions);
                    ui.close();
                }
            }
            ui.separator();
            if ui
                .button("Import model…")
                .on_hover_text(
                    "A glTF model (.gltf or .glb). You can also drop one onto the window.",
                )
                .clicked()
            {
                actions.push(WorkbenchAction::ImportModel);
                ui.close();
            }
            ui.separator();
            if ui.button("Quit").clicked() {
                self.request(WorkbenchAction::Quit, actions);
            }
        });
    }

    /// The Edit menu.
    fn edit_menu(&mut self, ui: &mut Ui) {
        let state = crate::menu_items::MenuState::of(self);
        let mut input = false;
        let mut settings = false;
        let picked = ui
            .menu_button("Edit", |ui| {
                let picked = crate::menu_items::edit_items(ui, &state);
                ui.separator();
                if ui
                    .button("Input actions…")
                    .on_hover_text(
                        "The game's actions (Move, Jump…) and the keys and buttons for each",
                    )
                    .clicked()
                {
                    input = true;
                    ui.close();
                }
                ui.separator();
                if ui
                    .button("Settings…")
                    .on_hover_text(
                        "The editor's settings: its colour, its layout, and more to come",
                    )
                    .clicked()
                {
                    settings = true;
                    ui.close();
                }
                picked
            })
            .inner
            .flatten();
        if let Some(command) = picked {
            self.run(command, ui.ctx());
        }
        if settings {
            self.open_settings();
        }
        if input {
            self.open_input_window();
        }
    }

    /// The Play menu: the buttons at the top right, and recording a
    /// play's input to play it back.
    fn play_menu(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        let state = self.play_state();
        ui.menu_button("Play", |ui| {
            if ui.button("Play").clicked() {
                self.play();
                ui.close();
            }
            if ui
                .add_enabled(state != PlayState::Editing, egui::Button::new("Pause"))
                .clicked()
            {
                self.pause();
                ui.close();
            }
            if ui
                .add_enabled(
                    state == PlayState::Paused,
                    egui::Button::new("Step one tick"),
                )
                .clicked()
            {
                self.step();
                ui.close();
            }
            if ui
                .add_enabled(state != PlayState::Editing, egui::Button::new("Stop"))
                .clicked()
            {
                self.stop();
                ui.close();
            }
            ui.separator();
            if ui
                .button("Load a save…")
                .on_hover_text(
                    "Plays from a save game in the project's saves folder, exactly as it was saved",
                )
                .clicked()
            {
                self.request(WorkbenchAction::OpenSave, actions);
                ui.close();
            }
            ui.separator();
            if ui
                .add_enabled(
                    state == PlayState::Editing,
                    egui::Button::new("Play and record inputs"),
                )
                .on_hover_text(
                    "Plays, keeping every tick's input; Stop saves it in the project's recordings \
                     folder, to play back exactly",
                )
                .clicked()
            {
                self.play_and_record();
                ui.close();
            }
            if ui
                .button("Replay a recording…")
                .on_hover_text(
                    "Plays a recording's input back from Play, and says whether the game came out \
                     exactly the same",
                )
                .clicked()
            {
                self.request(WorkbenchAction::OpenRecording, actions);
                ui.close();
            }
        });
    }

    /// The View menu.
    fn view_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("View", |ui| {
            let overlays = &mut self.viewport.overlays;
            ui.checkbox(&mut overlays.tools, "Viewport tools");
            ui.checkbox(&mut overlays.snapping, "Grid, angle and camera speed");
            ui.checkbox(&mut overlays.stats, "Frame rate and entity count");
        });
    }

    /// The Entity menu.
    fn entity_menu(&mut self, ui: &mut Ui) {
        let state = crate::menu_items::MenuState::of(self);
        let picked = ui
            .menu_button("Entity", |ui| crate::menu_items::entity_items(ui, &state))
            .inner
            .flatten();
        if let Some(command) = picked {
            self.run(command, ui.ctx());
        }
    }

    /// The Build menu.
    fn build_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("Build", |ui| {
            for item in ["Build", "Build & Play"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Exporting games arrives in Phase 11");
            }
        });
    }

    /// The Window menu. Panels dock freely, so there are no layout
    /// presets; Reset layout is here as well as in Settings, for when the
    /// layout is too tangled to find your way (decision 0061).
    fn window_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("Window", |ui| {
            if ui
                .button("Reset layout")
                .on_hover_text("Puts every panel back where it started")
                .clicked()
            {
                self.reset_layout();
                ui.close();
            }
        });
    }

    /// The Help menu.
    fn help_menu(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        ui.menu_button("Help", |ui| {
            if ui
                .button("Add the sample area")
                .on_hover_text(
                    "A small area for trying the looks: a checker cube, rows of plastic and \
                     metal spheres, and a grooved wall with a glowing stripe",
                )
                .clicked()
            {
                actions.push(WorkbenchAction::AddSampleArea);
                ui.close();
            }
            if ui
                .button("Make the sample game…")
                .on_hover_text(
                    "A small first game, as a new project in a folder you choose: a turret \
                     shooting at a drone, written in Lust, Lua and Rust that talk through events",
                )
                .clicked()
            {
                actions.push(WorkbenchAction::MakeSampleGame);
                ui.close();
            }
            if ui
                .button("Make the benchmark…")
                .on_hover_text(
                    "Two fleets of ships firing lasers, as a new project in a folder you choose: \
                     play its areas to see how many this computer keeps at 60 frames a second",
                )
                .clicked()
            {
                actions.push(WorkbenchAction::MakeBenchmark);
                ui.close();
            }
            if ui
                .button("Make the playground…")
                .on_hover_text(
                    "A level to walk around in, as a new project in a folder you choose: a \
                     character with the third-person camera, steps, ramps, crates to push, a \
                     moving platform, and quick saves on F6 and F7",
                )
                .clicked()
            {
                actions.push(WorkbenchAction::MakePlayground);
                ui.close();
            }
            ui.separator();
            self.updates_menu(ui, actions);
            ui.separator();
            if ui.button(format!("About {}", self.info.name)).clicked() {
                self.about_open = true;
                ui.close();
            }
        });
    }

    /// Play, Play with others, Stop, Build and Build & Play, laid out
    /// right to left.
    pub(crate) fn play_and_build(&mut self, ui: &mut Ui) {
        // Play and Build & Play are blue whatever the accent (decision
        // 0060); Pause, lit while paused, takes the accent.
        let fill = self.accent.colors().fill;
        ui.spacing_mut().item_spacing.x = 4.0;
        let build_and_play = egui::Button::new(RichText::new("Build & Play").color(Color32::WHITE))
            .fill(theme::GO)
            .stroke(egui::Stroke::NONE)
            .min_size(egui::vec2(0.0, BAR_BUTTON.y));
        if ui.add(build_and_play).clicked() {
            self.log(
                LogLevel::Info,
                "Exporting games arrives in Phase 11 (ROADMAP.md).",
            );
        }
        let build = egui::Button::new("Build ▾").min_size(egui::vec2(0.0, BAR_BUTTON.y));
        if ui.add(build).clicked() {
            self.log(
                LogLevel::Info,
                "Exporting games arrives in Phase 11 (ROADMAP.md).",
            );
        }
        ui.add_space(4.0);
        // Right to left: Stop, Play with two players, Step, Pause, Play.
        let state = self.play_state();
        let stop_fill = (state != PlayState::Editing).then_some(theme::ERROR);
        if icon_button(ui, Icon::Stop, 11.0, stop_fill, "Stop").clicked() {
            self.stop();
        }
        if icon_button(ui, Icon::PlayTogether, 15.0, None, "Play with two players").clicked() {
            self.log(
                LogLevel::Info,
                "Playing with others arrives with netcode in Phase 17 (ROADMAP.md).",
            );
        }
        // Step runs one tick, while paused.
        let step = ui.add_enabled_ui(state == PlayState::Paused, |ui| {
            icon_button(ui, Icon::Step, 12.0, None, "Step one tick")
        });
        if step.inner.clicked() {
            self.step();
        }
        let pause_fill = (state == PlayState::Paused).then_some(fill);
        if icon_button(ui, Icon::Pause, 12.0, pause_fill, "Pause").clicked() {
            self.pause();
        }
        let play = icon_button(ui, Icon::Play, 13.0, Some(theme::GO), "Play");
        if state == PlayState::Playing {
            // Lit while the game runs.
            ui.painter().rect_stroke(
                play.rect,
                4.0,
                egui::Stroke::new(2.0, Color32::WHITE),
                egui::StrokeKind::Inside,
            );
        }
        if play.clicked() {
            self.play();
        }
    }
}

/// Size of the icon buttons in the top bar (Play, Stop).
pub(crate) const BAR_BUTTON: egui::Vec2 = egui::vec2(30.0, 26.0);

/// A top-bar button showing `icon` at `size` points, filled with `fill`
/// (or plain until pointed at), named `name` for hover text and screen
/// readers.
pub(crate) fn icon_button(
    ui: &mut Ui,
    icon: Icon,
    size: f32,
    fill: Option<Color32>,
    name: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(BAR_BUTTON, egui::Sense::click());
    let response = response.on_hover_text(name);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    let background = match fill {
        Some(fill) if response.hovered() => fill.gamma_multiply(1.15),
        Some(fill) => fill,
        None if response.hovered() => theme::HOVER,
        None => Color32::TRANSPARENT,
    };
    ui.painter().rect_filled(rect, 4.0, background);
    let color = if fill.is_some() {
        Color32::WHITE
    } else {
        theme::TEXT_CONSOLE
    };
    icons::icon(
        ui.painter(),
        egui::Rect::from_center_size(rect.center(), egui::vec2(size, size)),
        icon,
        color,
    );
    response
}

/// One open document's tab, as in the mockup: the front one on a lighter
/// strip with the accent underline, its icon, its name and ×. A dot after
/// the name means unsaved changes. Gives back whether the tab was clicked,
/// and whether its × was (only when `closes`).
pub(crate) fn document_tab(
    ui: &mut Ui,
    bar: egui::Rect,
    accent: Accent,
    icon: Icon,
    name: &str,
    front: bool,
    closes: bool,
) -> (bool, bool) {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let strong = if front {
        theme::TEXT_STRONG
    } else {
        theme::TEXT_WEAK
    };
    let text = ui
        .painter()
        .layout_no_wrap(name.to_string(), font.clone(), strong);
    let close = ui
        .painter()
        .layout_no_wrap("×".to_string(), font, theme::TEXT_WEAK);
    let (pad, icon_size, gap) = (12.0, 12.0, 7.0);
    let width = pad + icon_size + gap + text.size().x + gap + close.size().x + pad;
    let (row, response) = ui.allocate_exact_size(
        egui::vec2(width, ui.available_height().min(bar.height())),
        egui::Sense::click(),
    );
    // The tab reaches from the top of the bar to its bottom edge, as in the
    // mockup.
    let rect = egui::Rect::from_x_y_ranges(row.x_range(), bar.y_range());
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, front, name));
    let painter = ui.painter();
    if front {
        painter.rect_filled(rect, 0.0, theme::MENU_ROW);
        painter.hline(
            rect.x_range(),
            rect.bottom() - 1.0,
            egui::Stroke::new(2.0, accent.colors().fill),
        );
    }
    let mut x = rect.left() + pad;
    icons::icon(
        painter,
        egui::Rect::from_min_size(
            egui::pos2(x, rect.center().y - icon_size / 2.0),
            egui::vec2(icon_size, icon_size),
        ),
        icon,
        if front {
            accent.colors().light
        } else {
            theme::TEXT_WEAK
        },
    );
    x += icon_size + gap;
    let text_width = text.size().x;
    painter.galley(
        egui::pos2(x, rect.center().y - text.size().y / 2.0),
        text,
        strong,
    );
    x += text_width + gap;
    let close_rect = egui::Rect::from_min_size(
        egui::pos2(x - 3.0, rect.center().y - close.size().y / 2.0 - 2.0),
        close.size() + egui::vec2(6.0, 4.0),
    );
    painter.galley(
        egui::pos2(x, rect.center().y - close.size().y / 2.0),
        close,
        theme::TEXT_WEAK,
    );
    let mut closed = false;
    if closes {
        let r = ui.interact(
            close_rect,
            ui.id().with(("close document", name)),
            egui::Sense::click(),
        );
        r.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Close {name}"))
        });
        closed = r.clicked();
    }
    (response.clicked() && !closed, closed)
}

/// The + after the document tabs (more kinds of document arrive later).
fn new_document_button(ui: &mut Ui) {
    let new_tab = ui.add_enabled(
        false,
        egui::Button::new(RichText::new("+").color(theme::TEXT_WEAK))
            .frame(false)
            .min_size(egui::vec2(28.0, 0.0)),
    );
    new_tab.on_disabled_hover_text("Opening more areas, assemblies and code arrives in Phase 2");
}
