//! The whole editor window: the top bar and the docked panels
//! ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).
//!
//! The workbench never touches the window, the GPU or files itself: it
//! returns [`WorkbenchAction`]s for the app to carry out, and draws the
//! viewport through a [`ViewportRenderer`] it is handed each frame.

use crate::camera::Camera;
use crate::icons::{self, Icon};
use crate::layout::{self, Preset};
use crate::panel::Panel;
use crate::theme::{self, Accent};
use crate::viewport::{Overlays, Tool, Viewport, ViewportRenderer};
use egui::{Align, Color32, Id, Layout, RichText, Ui, WidgetText};
use egui_dock::{DockArea, DockState, TabViewer};
use std::collections::HashSet;

/// Facts about the program, shown in the window title and the About box.
/// The app passes these in, so this crate never names the program itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// The program's name, such as "Solis".
    pub name: String,
    /// The version, such as "0.0.1".
    pub version: String,
}

/// Something the app must do, asked for by the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkbenchAction {
    /// Close the window.
    Quit,
}

/// How serious a console message is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    /// Something happened.
    Info,
    /// Something works, but not quite as asked.
    Warning,
    /// Something failed.
    Error,
}

impl LogLevel {
    /// Every level, in the console's filter order.
    pub const ALL: [Self; 3] = [Self::Info, Self::Warning, Self::Error];

    fn icon(self) -> Icon {
        match self {
            Self::Info => Icon::Info,
            Self::Warning => Icon::Warning,
            Self::Error => Icon::Error,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warnings",
            Self::Error => "Errors",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::Info => theme::TEXT_CONSOLE,
            Self::Warning => theme::WARNING,
            Self::Error => theme::ERROR,
        }
    }
}

/// One console message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// When it was written, local time, such as "12:04:31".
    pub time: String,
    /// How serious it is.
    pub level: LogLevel,
    /// What it says.
    pub text: String,
}

/// The console's messages and which levels are shown.
#[derive(Debug, Clone)]
struct Console {
    lines: Vec<LogLine>,
    hidden: HashSet<LogLevel>,
    search: String,
}

/// The most messages the console keeps; older ones are dropped.
const CONSOLE_LIMIT: usize = 2_000;

impl Console {
    fn push(&mut self, level: LogLevel, text: String) {
        let time = chrono::Local::now().format("%H:%M:%S").to_string();
        self.lines.push(LogLine { time, level, text });
        if self.lines.len() > CONSOLE_LIMIT {
            let extra = self.lines.len() - CONSOLE_LIMIT;
            self.lines.drain(..extra);
        }
    }

    fn count(&self, level: LogLevel) -> usize {
        self.lines.iter().filter(|l| l.level == level).count()
    }

    fn shown(&self) -> impl Iterator<Item = &LogLine> {
        let search = self.search.to_lowercase();
        self.lines.iter().filter(move |l| {
            !self.hidden.contains(&l.level)
                && (search.is_empty() || l.text.to_lowercase().contains(&search))
        })
    }
}

/// The editor's state, kept between frames.
pub struct Workbench {
    info: AppInfo,
    dock: DockState<Panel>,
    accent: Accent,
    viewport: Viewport,
    console: Console,
    about_open: bool,
    /// The tabs that were in front last frame, to underline them in the
    /// accent colour.
    in_front: HashSet<Panel>,
}

impl Workbench {
    /// A workbench with a saved panel layout, if it is usable (otherwise the
    /// default one).
    pub fn new(info: AppInfo, saved_layout: Option<DockState<Panel>>) -> Self {
        Self {
            info,
            dock: layout::restore(saved_layout),
            accent: Accent::default(),
            viewport: Viewport::default(),
            console: Console {
                lines: Vec::new(),
                hidden: HashSet::new(),
                search: String::new(),
            },
            about_open: false,
            in_front: HashSet::new(),
        }
    }

    /// Uses `accent` (as saved last time).
    #[must_use]
    pub fn with_accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Uses `overlays` (as saved last time).
    #[must_use]
    pub fn with_overlays(mut self, overlays: Overlays) -> Self {
        self.viewport.overlays = overlays;
        self
    }

    /// The panel layout, for saving.
    pub fn layout(&self) -> &DockState<Panel> {
        &self.dock
    }

    /// The accent colour, for saving.
    pub fn accent(&self) -> Accent {
        self.accent
    }

    /// Uses `camera` (as saved last time), with anything unusable in it
    /// replaced (see [`Camera::sanitized`]).
    #[must_use]
    pub fn with_camera(mut self, camera: Camera) -> Self {
        self.viewport.camera = camera.sanitized();
        self
    }

    /// The viewport camera, for saving.
    pub fn camera(&self) -> Camera {
        self.viewport.camera
    }

    /// Which viewport overlays are shown, for saving.
    pub fn overlays(&self) -> Overlays {
        self.viewport.overlays
    }

    /// The chosen tool.
    pub fn tool(&self) -> Tool {
        self.viewport.tool
    }

    /// The console's messages, oldest first.
    pub fn log_lines(&self) -> &[LogLine] {
        &self.console.lines
    }

    /// Adds a message to the console.
    pub fn log(&mut self, level: LogLevel, text: impl Into<String>) {
        self.console.push(level, text.into());
    }

    /// Arranges the panels as `preset`.
    pub fn apply_preset(&mut self, preset: Preset) {
        self.dock = preset.layout();
    }

    /// The window title: the program, its version and the open scene.
    pub fn title(&self) -> String {
        format!("{} {} · Untitled scene", self.info.name, self.info.version)
    }

    /// Draws the editor filling `ui`, and returns what the app must do.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
    ) -> Vec<WorkbenchAction> {
        let mut actions = Vec::new();
        self.tool_keys(ui);
        egui::Panel::top("top_bar")
            .frame(theme::top_bar_frame())
            .exact_size(34.0)
            .show(ui, |ui| self.top_bar(ui, &mut actions));
        let style = theme::dock_style(ui.style().as_ref(), self.accent);
        let mut shown = HashSet::new();
        theme::dock_frame().show(ui, |ui| {
            DockArea::new(&mut self.dock)
                .style(style)
                .tab_context_menus(false)
                .show_close_buttons(false)
                .show_add_buttons(false)
                .show_leaf_close_all_buttons(false)
                .show_leaf_collapse_buttons(true)
                .show_inside(
                    ui,
                    &mut PanelViewer {
                        viewport: &mut self.viewport,
                        renderer,
                        console: &mut self.console,
                        accent: self.accent,
                        in_front: &self.in_front,
                        shown: &mut shown,
                    },
                );
        });
        self.in_front = shown;
        self.about(ui);
        actions
    }

    /// Q, W, R, S and T pick the tools, unless typing in a field.
    fn tool_keys(&mut self, ui: &Ui) {
        // While flying, W A S D, Q and E move the camera instead.
        if ui.ctx().egui_wants_keyboard_input() || self.viewport.is_flying() {
            return;
        }
        let pressed = ui.input(|i| {
            Tool::ALL
                .into_iter()
                .find(|tool| i.key_pressed(tool.key()) && i.modifiers.is_none())
        });
        if let Some(tool) = pressed {
            self.viewport.tool = tool;
        }
    }

    fn top_bar(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
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

    fn menus(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        ui.menu_button("File", |ui| {
            for item in ["New scene", "Open…", "Save"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Scenes arrive in Phase 2");
            }
            ui.separator();
            if ui.button("Quit").clicked() {
                actions.push(WorkbenchAction::Quit);
            }
        });
        ui.menu_button("Edit", |ui| {
            for item in ["Undo", "Redo"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Undo arrives with editing in Phase 2");
            }
        });
        ui.menu_button("View", |ui| {
            let overlays = &mut self.viewport.overlays;
            ui.checkbox(&mut overlays.tools, "Viewport tools");
            ui.checkbox(&mut overlays.snapping, "Grid, angle and camera speed");
            ui.checkbox(&mut overlays.stats, "Frame rate and entity count");
        });
        ui.menu_button("Entity", |ui| {
            for item in ["Add entity", "Add light", "Add camera"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Entities arrive in Phase 2");
            }
        });
        ui.menu_button("Build", |ui| {
            for item in ["Build", "Build & Play"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Building games arrives in Phase 10");
            }
        });
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
        ui.menu_button("Help", |ui| {
            if ui.button(format!("About {}", self.info.name)).clicked() {
                self.about_open = true;
                ui.close();
            }
        });
    }

    /// Play, Play with others, Stop, Build and Build & Play, laid out
    /// right to left.
    fn play_and_build(&mut self, ui: &mut Ui) {
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
                "Playing the scene arrives in Phase 5 (ROADMAP.md).",
            );
        }
    }

    fn about(&mut self, ui: &mut Ui) {
        let mut open = self.about_open;
        egui::Window::new(format!("About {}", self.info.name))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(&self.info.name);
                    ui.label(format!("Version {}", self.info.version));
                    ui.add_space(8.0);
                    ui.label("A free, open-source 3D game engine.");
                    ui.label("Free software under the MIT licence.");
                });
            });
        self.about_open = open;
    }
}

/// Size of the icon buttons in the top bar (Play, Stop).
const BAR_BUTTON: egui::Vec2 = egui::vec2(30.0, 26.0);

/// A top-bar button showing `icon` at `size` points, filled with `fill`
/// (or plain until pointed at), named `name` for hover text and screen
/// readers.
fn icon_button(
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
fn document_tabs(ui: &mut Ui, bar: egui::Rect, accent: Accent) {
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

/// Where egui_dock starts a tab's text, from the tab's left edge.
const TAB_TEXT_INSET: f32 = 8.0;
/// Room left before a tab's name for its icon.
const TAB_ICON_SPACE: f32 = 18.0;

/// Draws each docked panel.
struct PanelViewer<'a> {
    viewport: &'a mut Viewport,
    renderer: &'a mut dyn ViewportRenderer,
    console: &'a mut Console,
    accent: Accent,
    in_front: &'a HashSet<Panel>,
    shown: &'a mut HashSet<Panel>,
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
            ui.advance_cursor_after_rect(full);
            return;
        }
        match tab {
            Panel::Scene => {
                menu_row(ui, &["Edit", "View", "Entity"]);
                placeholder(ui, "No entities yet. They arrive in Phase 2.");
            }
            Panel::Layers => {
                menu_row(ui, &["Edit", "View"]);
                placeholder(ui, "Layers arrive with scenes in Phase 2.");
            }
            Panel::Properties => {
                menu_row(ui, &["Mode", "Edit", "Units"]);
                placeholder(ui, "Nothing selected.");
            }
            Panel::Library => {
                menu_row(ui, &["Import", "View"]);
                placeholder(
                    ui,
                    "The library arrives with the asset database in Phase 2.",
                );
            }
            Panel::Game => placeholder(ui, "Press Play to run the game here (Phase 5)."),
            Panel::Code => {
                menu_row(ui, &["File", "Edit", "View"]);
                placeholder(
                    ui,
                    "Writing game code in Lust, Rust or Lua arrives in Phase 7.",
                );
            }
            Panel::Console => console(ui, self.console, self.accent),
            Panel::Problems => {
                placeholder(ui, "No problems. Mistakes in code and scenes show here.");
            }
            Panel::Viewport => {}
        }
    }
}

/// The small menu row under a panel's title: "≡ Edit View …".
fn menu_row(ui: &mut Ui, items: &[&str]) {
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

/// A quiet note in a panel that has nothing to show yet.
fn placeholder(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .inner_margin(theme::panel_margin())
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(theme::TEXT_WEAK));
        });
}

/// The console: filters by level, a search box, and the messages with
/// their times.
fn console(ui: &mut Ui, console: &mut Console, accent: Accent) {
    theme::menu_row_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for level in LogLevel::ALL {
                let on = !console.hidden.contains(&level);
                if level_filter(ui, level, console.count(level), on, accent).clicked() {
                    if on {
                        console.hidden.insert(level);
                    } else {
                        console.hidden.remove(&level);
                    }
                }
            }
            ui.add_space(4.0);
            // Clear at the right edge, and the search box takes the width
            // that is left.
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(egui::Button::new("Clear").min_size(egui::vec2(0.0, 20.0)))
                    .clicked()
                {
                    console.lines.clear();
                }
                ui.add(
                    egui::TextEdit::singleline(&mut console.search)
                        .hint_text("Search")
                        .desired_width(ui.available_width().max(40.0)),
                );
            });
        });
    });
    // Only the rows in view are laid out, so a full console (2,000
    // messages) costs no more per frame than a short one.
    let shown: Vec<&LogLine> = console.shown().collect();
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .stick_to_bottom(true)
                .show_rows(ui, CONSOLE_ROW, shown.len(), |ui, rows| {
                    for line in &shown[rows] {
                        console_line(ui, line);
                    }
                });
        });
}

/// The height of one console row, as in the mockup.
const CONSOLE_ROW: f32 = 18.0;

/// One level's filter button: its icon and count, filled with the accent
/// while that level is shown.
fn level_filter(
    ui: &mut Ui,
    level: LogLevel,
    count: usize,
    on: bool,
    accent: Accent,
) -> egui::Response {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let (text_color, fill) = if on {
        (Color32::WHITE, accent.colors().fill)
    } else {
        (theme::TEXT_WEAK, theme::MENU_ROW)
    };
    let number = ui
        .painter()
        .layout_no_wrap(count.to_string(), font, text_color);
    let icon_size = 11.0;
    let size = egui::vec2(8.0 + icon_size + 5.0 + number.size().x + 8.0, 20.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let response = response.on_hover_text(format!(
        "{}: {} (click to {} them)",
        level.name(),
        count,
        if on { "hide" } else { "show" }
    ));
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, level.name())
    });
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, fill);
    if !on {
        painter.rect_stroke(
            rect,
            3.0,
            egui::Stroke::new(1.0, theme::BUTTON_BORDER),
            egui::StrokeKind::Inside,
        );
    }
    icons::icon(
        painter,
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 8.0, rect.center().y - icon_size / 2.0),
            egui::vec2(icon_size, icon_size),
        ),
        level.icon(),
        text_color,
    );
    painter.galley(
        egui::pos2(
            rect.left() + 8.0 + icon_size + 5.0,
            rect.center().y - number.size().y / 2.0,
        ),
        number,
        text_color,
    );
    response
}

/// One console message: the time, the level's icon and the text.
fn console_line(ui: &mut Ui, line: &LogLine) {
    let (row, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), CONSOLE_ROW),
        egui::Sense::hover(),
    );
    let mut row_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let ui = &mut row_ui;
    ui.label(
        RichText::new(&line.time)
            .monospace()
            .color(theme::TEXT_TIME),
    );
    let size = 11.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    icons::icon(ui.painter(), rect, line.level.icon(), line.level.color());
    // One row per message: a long one is cut short, and shows in full when
    // pointed at.
    ui.add(
        egui::Label::new(
            RichText::new(&line.text)
                .monospace()
                .color(line.level.color()),
        )
        .truncate(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewport::NoRenderer;
    use egui_kittest::kittest::Queryable;

    fn info() -> AppInfo {
        AppInfo {
            name: "Test Engine".into(),
            version: "9.9.9".into(),
        }
    }

    #[test]
    fn the_title_names_program_version_and_scene() {
        let w = Workbench::new(info(), None);
        assert_eq!(w.title(), "Test Engine 9.9.9 · Untitled scene");
    }

    #[test]
    fn a_new_workbench_uses_the_default_layout() {
        let w = Workbench::new(info(), None);
        assert!(layout::is_complete(w.layout()));
        assert_eq!(w.tool(), Tool::Select);
        assert_eq!(w.accent(), Accent::PetrolBlue);
    }

    #[test]
    fn the_console_keeps_a_limited_number_of_messages() {
        let mut w = Workbench::new(info(), None);
        for i in 0..(CONSOLE_LIMIT + 10) {
            w.log(LogLevel::Info, format!("line {i}"));
        }
        assert_eq!(w.log_lines().len(), CONSOLE_LIMIT);
        assert_eq!(w.log_lines()[0].text, "line 10");
    }

    #[test]
    fn the_console_filters_by_level_and_search() {
        let mut c = Console {
            lines: Vec::new(),
            hidden: HashSet::new(),
            search: String::new(),
        };
        c.push(LogLevel::Info, "Graphics: Vulkan".into());
        c.push(LogLevel::Warning, "pairs() order".into());
        c.push(LogLevel::Error, "Shader failed".into());
        assert_eq!(c.shown().count(), 3);
        c.hidden.insert(LogLevel::Info);
        assert_eq!(c.shown().count(), 2);
        c.search = "SHADER".into();
        assert_eq!(c.shown().count(), 1);
        assert_eq!(c.count(LogLevel::Warning), 1);
    }

    #[test]
    fn presets_rearrange_the_panels() {
        let mut w = Workbench::new(info(), None);
        w.apply_preset(Preset::SmallScreen);
        let path = w.layout().find_tab(&Panel::Console).unwrap();
        let group = w.layout()[path.surface][path.node].tabs().unwrap().to_vec();
        assert_eq!(
            group,
            [Panel::Library, Panel::Code, Panel::Console, Panel::Problems]
        );
    }

    /// The whole editor in a test window, with no GPU behind the viewport.
    fn harness() -> egui_kittest::Harness<'static, Workbench> {
        let mut renderer = NoRenderer {
            reason: "testing".into(),
        };
        let mut themed = false;
        egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui_state(
                move |ui, workbench: &mut Workbench| {
                    if !themed {
                        theme::apply(ui.ctx(), workbench.accent());
                        themed = true;
                    }
                    let _ = workbench.show(ui, &mut renderer);
                },
                Workbench::new(info(), None),
            )
    }

    #[test]
    fn the_editor_draws_every_panel_without_a_gpu() {
        let mut h = harness();
        h.run();
        for label in ["Viewport", "Scene", "Properties", "Console", "Library"] {
            let _ = h.get_by_label(label);
        }
        let _ = h.get_by_label("Untitled scene");
        let _ = h.get_by_label("No 3D view: testing");
    }

    #[test]
    fn tool_keys_pick_tools() {
        let mut h = harness();
        h.run();
        h.key_press(egui::Key::W);
        h.run();
        assert_eq!(h.state().tool(), Tool::Move);
        h.key_press(egui::Key::T);
        h.run();
        assert_eq!(h.state().tool(), Tool::Transform);
    }

    #[test]
    fn clicking_a_tool_button_picks_it() {
        let mut h = harness();
        h.run();
        h.get_by_label("Rotate (R)").click();
        h.run();
        assert_eq!(h.state().tool(), Tool::Rotate);
    }

    #[test]
    fn the_viewport_menu_hides_the_tool_bar() {
        let mut h = harness();
        h.run();
        h.get_by_role_and_label(egui::accesskit::Role::Button, "≡")
            .click();
        h.run();
        h.get_by_label("Tools").click();
        h.run();
        assert!(!h.state().overlays().tools);
        assert!(h.query_by_label("Move (W)").is_none());
    }

    #[test]
    fn play_says_when_it_arrives() {
        let mut h = harness();
        h.run();
        h.get_by_label("Play").click();
        h.run();
        let last = h.state().log_lines().last().map(|l| l.text.clone());
        assert_eq!(
            last.as_deref(),
            Some("Playing the scene arrives in Phase 5 (ROADMAP.md).")
        );
    }

    /// A point in the middle of the viewport, clear of its corner controls.
    const IN_VIEW: egui::Pos2 = egui::pos2(500.0, 300.0);

    fn button(
        h: &egui_kittest::Harness<'_, Workbench>,
        button: egui::PointerButton,
        pressed: bool,
        at: egui::Pos2,
    ) {
        h.event(egui::Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }

    #[test]
    fn holding_right_mouse_and_w_flies_forward_without_changing_tool() {
        let mut h = harness();
        h.hover_at(IN_VIEW);
        h.run();
        let start = h.state().camera();
        button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
        h.run();
        assert!(h.state().viewport.is_flying());
        h.key_down(egui::Key::W);
        h.run_steps(5);
        h.key_up(egui::Key::W);
        button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
        h.run();
        assert!(!h.state().viewport.is_flying());
        let moved = h.state().camera().position - start.position;
        assert!(moved.dot(start.forward()) > 0.0, "moved {moved}");
        assert_eq!(
            h.state().tool(),
            Tool::Select,
            "W flew instead of picking Move"
        );
    }

    #[test]
    fn right_dragging_looks_around() {
        let mut h = harness();
        h.hover_at(IN_VIEW);
        h.run();
        let start = h.state().camera();
        button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
        h.run();
        h.event(egui::Event::PointerMoved(IN_VIEW + egui::vec2(80.0, 0.0)));
        h.run();
        button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
        h.run();
        assert!(h.state().camera().heading > start.heading);
        assert_eq!(h.state().camera().position, start.position);
    }

    #[test]
    fn the_wheel_changes_speed_while_flying_and_moves_otherwise() {
        let wheel = |h: &egui_kittest::Harness<'_, Workbench>| {
            h.event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 1.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            });
        };
        let mut h = harness();
        h.hover_at(IN_VIEW);
        h.run();
        let start = h.state().camera();
        wheel(&h);
        h.run();
        let after_wheel = h.state().camera();
        assert!((after_wheel.position - start.position).dot(start.forward()) > 0.0);
        assert_eq!(after_wheel.speed, start.speed);

        button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
        h.run();
        wheel(&h);
        h.run();
        button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
        h.run();
        assert_eq!(h.state().camera().speed, start.speed + 1);
        assert_eq!(h.state().camera().position, after_wheel.position);
        let _ = h.get_by_label(&(start.speed + 1).to_string());
    }

    #[test]
    fn middle_dragging_slides_the_camera() {
        let mut h = harness();
        h.hover_at(IN_VIEW);
        h.run();
        let start = h.state().camera();
        button(&h, egui::PointerButton::Middle, true, IN_VIEW);
        h.run();
        for step in 1..=5 {
            h.event(egui::Event::PointerMoved(
                IN_VIEW + egui::vec2(step as f32 * 20.0, 0.0),
            ));
            h.run();
        }
        button(
            &h,
            egui::PointerButton::Middle,
            false,
            IN_VIEW + egui::vec2(100.0, 0.0),
        );
        h.run();
        let moved = h.state().camera().position - start.position;
        assert!(moved.dot(start.right()) < 0.0, "moved {moved}");
        assert_eq!(h.state().camera().heading, start.heading);
    }

    #[test]
    fn tool_keys_still_work_after_flying() {
        let mut h = harness();
        h.hover_at(IN_VIEW);
        h.run();
        button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
        h.run();
        button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
        h.run();
        h.key_press(egui::Key::R);
        h.run();
        assert_eq!(h.state().tool(), Tool::Rotate);
    }

    #[test]
    fn console_messages_carry_the_time() {
        let mut w = Workbench::new(info(), None);
        w.log(LogLevel::Info, "hello");
        let time = &w.log_lines()[0].time;
        let parts: Vec<_> = time.split(':').collect();
        assert_eq!(parts.len(), 3, "{time}");
        assert!(
            parts
                .iter()
                .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_digit()))
        );
    }

    #[test]
    fn a_level_filter_hides_and_shows_its_messages() {
        let mut h = harness();
        h.state_mut().log(LogLevel::Warning, "Shader slow to build");
        h.run();
        assert!(h.query_by_label("Shader slow to build").is_some());
        h.get_by_label("Warnings").click();
        h.run();
        assert!(h.query_by_label("Shader slow to build").is_none());
        h.get_by_label("Warnings").click();
        h.run();
        assert!(h.query_by_label("Shader slow to build").is_some());
    }

    #[test]
    fn every_panel_has_a_named_tab_and_the_bar_has_its_buttons() {
        let mut h = harness();
        h.run();
        for panel in Panel::ALL {
            let _ = h.get_by_label(panel.title());
        }
        for button in [
            "Play",
            "Play with two players",
            "Stop",
            "Build ▾",
            "Build & Play",
        ] {
            let _ = h.get_by_label(button);
        }
    }

    #[test]
    fn play_with_others_says_when_it_arrives() {
        let mut h = harness();
        h.run();
        h.get_by_label("Play with two players").click();
        h.run();
        let last = h.state().log_lines().last().map(|l| l.text.clone());
        assert!(last.is_some_and(|t| t.contains("Phase 8")));
    }

    /// Part of the scale check (`tools/scale/scale-check.sh`, decision
    /// 0011): how long the editor takes to lay out and draw one frame of
    /// its interface, with a full console. Not run with the normal tests.
    #[test]
    #[ignore = "scale check: run by tools/scale/scale-check.sh"]
    #[allow(clippy::print_stdout)]
    fn scale_check_editor_frame() {
        const TARGET_MS: f64 = 4.0;
        let mut h = harness();
        for i in 0..CONSOLE_LIMIT {
            h.state_mut().log(LogLevel::Info, format!("Message {i}"));
        }
        h.run();
        let frames = 200;
        let start = std::time::Instant::now();
        for _ in 0..frames {
            h.step();
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
        let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
        println!("Scale check: editor frame {ms:.2} ms (target {TARGET_MS} ms) {verdict}");
        assert!(ms <= TARGET_MS, "the editor takes {ms:.2} ms per frame");
    }
}
