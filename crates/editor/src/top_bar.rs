//! The top bar: the menus, the open documents as tabs, and Play and Build
//! ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).

use crate::console::LogLevel;
use crate::icons::{self, Icon};
use crate::layout::Preset;
use crate::theme::{self, Accent};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Color32, Layout, RichText, Ui};

impl Workbench {
    pub(crate) fn top_bar(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        // The whole bar, so the scene tab can fill its full height.
        let bar = ui.max_rect();
        // The right-hand buttons go first, so the menu bar (which takes all
        // the width it is given) only gets what is left.
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            self.play_and_build(ui);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    // The mockup's roomier menu titles.
                    ui.spacing_mut().button_padding = egui::vec2(7.0, 4.0);
                    ui.spacing_mut().item_spacing.x = 2.0;
                    self.menus(ui, actions);
                    ui.add_space(8.0);
                    document_tabs(ui, bar, self.accent);
                });
            });
        });
    }

    pub(crate) fn menus(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        self.file_menu(ui, actions);
        self.edit_menu(ui);
        self.view_menu(ui);
        self.entity_menu(ui);
        self.build_menu(ui);
        self.window_menu(ui);
        self.help_menu(ui, actions);
    }

    /// The File menu.
    fn file_menu(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        ui.menu_button("File", |ui| {
            for item in ["New scene", "Open…", "Save"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Scenes arrive in Phase 2");
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
                actions.push(WorkbenchAction::Quit);
            }
        });
    }

    /// The Edit menu.
    fn edit_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("Edit", |ui| {
            for item in ["Undo", "Redo"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Undo arrives with editing in Phase 2");
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
        ui.menu_button("Entity", |ui| {
            if ui
                .button("Add entity")
                .on_hover_text("An empty entity, at the point the camera looks at")
                .clicked()
            {
                self.add_entity();
                ui.close();
            }
            ui.separator();
            if ui
                .button("Add point light")
                .on_hover_text("A bulb shining every way, placed above where the camera looks")
                .clicked()
            {
                self.add_point_light();
                ui.close();
            }
            if ui
                .button("Add spotlight")
                .on_hover_text("A cone of light at the camera, shining where it looks")
                .clicked()
            {
                self.add_spotlight();
                ui.close();
            }
            ui.separator();
            ui.add_enabled(false, egui::Button::new("Add camera"))
                .on_disabled_hover_text("Cameras arrive with play mode in Phase 3");
        });
    }

    /// The Build menu.
    fn build_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("Build", |ui| {
            for item in ["Build", "Build & Play"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Building games arrives in Phase 10");
            }
        });
    }

    /// The Window menu.
    fn window_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("Window", |ui| {
            ui.menu_button("Layouts", |ui| {
                for preset in Preset::ALL {
                    if ui.button(preset.name()).clicked() {
                        self.apply_preset(preset);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Reset layout").clicked() {
                    self.apply_preset(Preset::Default);
                    ui.close();
                }
            });
            ui.menu_button("Accent colour", |ui| {
                for accent in Accent::ALL {
                    if ui
                        .add(egui::Button::new(accent.name()).selected(self.accent == accent))
                        .clicked()
                    {
                        self.accent = accent;
                        theme::set_accent(ui.ctx(), accent);
                        ui.close();
                    }
                }
            });
        });
    }

    /// The Help menu.
    fn help_menu(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        ui.menu_button("Help", |ui| {
            if ui
                .button("Add the sample scene")
                .on_hover_text(
                    "A small scene for trying the looks: a checker cube, rows of plastic and \
                     metal spheres, and a grooved wall with a glowing stripe",
                )
                .clicked()
            {
                actions.push(WorkbenchAction::AddSampleScene);
                ui.close();
            }
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
        let fill = self.accent.colors().fill;
        ui.spacing_mut().item_spacing.x = 4.0;
        let build_and_play = egui::Button::new(RichText::new("Build & Play").color(Color32::WHITE))
            .fill(fill)
            .stroke(egui::Stroke::NONE)
            .min_size(egui::vec2(0.0, BAR_BUTTON.y));
        if ui.add(build_and_play).clicked() {
            self.log(
                LogLevel::Info,
                "Building games arrives in Phase 10 (ROADMAP.md).",
            );
        }
        let build = egui::Button::new("Build ▾").min_size(egui::vec2(0.0, BAR_BUTTON.y));
        if ui.add(build).clicked() {
            self.log(
                LogLevel::Info,
                "Building games arrives in Phase 10 (ROADMAP.md).",
            );
        }
        ui.add_space(4.0);
        if icon_button(ui, Icon::Stop, 11.0, None, "Stop").clicked() {
            self.log(LogLevel::Info, "Nothing is running.");
        }
        if icon_button(ui, Icon::PlayTogether, 15.0, None, "Play with two players").clicked() {
            self.log(
                LogLevel::Info,
                "Playing with others arrives with netcode in Phase 8 (ROADMAP.md).",
            );
        }
        if icon_button(ui, Icon::Play, 13.0, Some(fill), "Play").clicked() {
            self.log(
                LogLevel::Info,
                "Playing the scene arrives in Phase 3 (ROADMAP.md).",
            );
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

/// The open documents as tabs, as in the mockup: the front one on a
/// lighter strip with the accent underline, its icon, its name and ×.
/// For now there is only the one scene; opening more arrives in Phase 2.
pub(crate) fn document_tabs(ui: &mut Ui, bar: egui::Rect, accent: Accent) {
    let name = "Untitled scene";
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text = ui
        .painter()
        .layout_no_wrap(name.to_string(), font.clone(), theme::TEXT_STRONG);
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
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, true, name));
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, theme::MENU_ROW);
    painter.hline(
        rect.x_range(),
        rect.bottom() - 1.0,
        egui::Stroke::new(2.0, accent.colors().fill),
    );
    let mut x = rect.left() + pad;
    icons::icon(
        painter,
        egui::Rect::from_min_size(
            egui::pos2(x, rect.center().y - icon_size / 2.0),
            egui::vec2(icon_size, icon_size),
        ),
        Icon::SceneFile,
        accent.colors().light,
    );
    x += icon_size + gap;
    let text_width = text.size().x;
    painter.galley(
        egui::pos2(x, rect.center().y - text.size().y / 2.0),
        text,
        theme::TEXT_STRONG,
    );
    x += text_width + gap;
    painter.galley(
        egui::pos2(x, rect.center().y - close.size().y / 2.0),
        close,
        theme::TEXT_WEAK,
    );
    let new_tab = ui.add_enabled(
        false,
        egui::Button::new(RichText::new("+").color(theme::TEXT_WEAK))
            .frame(false)
            .min_size(egui::vec2(28.0, 0.0)),
    );
    new_tab.on_disabled_hover_text("Opening more scenes, prefabs and code arrives in Phase 2");
}
