//! File > Preferences: the editor's settings in one window, a page per
//! subject with a search box ([decision 0061](../../../docs/decisions/0061-settings-window-and-no-layout-presets.md)).
//! Pages appear as they get settings: the editor's own (Appearance: the
//! accent colour, scale, density, Reset layout, hints and Hide unfinished
//! parts, drawn with the design's own controls by
//! `settings_appearance.rs`), kept on this computer, and the open
//! project's (2D, Phase 9 D1: the base resolution, pixels per unit, tile
//! size and scaling), kept in the project's `pixels.solpix` so they travel
//! with it. None of them is part of the area, so undo leaves them alone.

use crate::theme::{self, Accent};
use crate::workbench::Workbench;
use area::pixels::{PixelSettings, Scaling};
use egui::{RichText, Ui};

/// A page of the Settings window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Page {
    /// The editor's look: the theme, rounded panels, the accent colour and
    /// the panel layout.
    #[default]
    Appearance,
    /// The project's 2D settings.
    TwoD,
}

impl Page {
    /// Every page, in the order the list shows them.
    const ALL: [Self; 2] = [Self::Appearance, Self::TwoD];

    fn name(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::TwoD => "2D",
        }
    }

    /// Whether the page's settings belong to the project (kept in it)
    /// rather than the editor (kept on this computer).
    fn is_project(self) -> bool {
        self == Self::TwoD
    }
}

/// One setting: its name and its hint, which the search box looks in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Setting {
    Accent,
    Scale,
    Density,
    Layout,
    Hints,
    HideUnfinished,
    BaseResolution,
    PixelsPerUnit,
    TileSize,
    Scaling,
}

impl Setting {
    fn page(self) -> Page {
        match self {
            Self::Accent
            | Self::Scale
            | Self::Density
            | Self::Layout
            | Self::Hints
            | Self::HideUnfinished => Page::Appearance,
            Self::BaseResolution | Self::PixelsPerUnit | Self::TileSize | Self::Scaling => {
                Page::TwoD
            }
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Accent => "Accent colour",
            Self::Scale => "Interface scale",
            Self::Density => "Density",
            Self::Layout => "Panel layout",
            Self::Hints => "Hints",
            Self::HideUnfinished => "Hide unfinished parts",
            Self::BaseResolution => "Base resolution",
            Self::PixelsPerUnit => "Pixels per unit",
            Self::TileSize => "Tile size",
            Self::Scaling => "Scaling to the screen",
        }
    }

    pub(crate) fn hint(self) -> &'static str {
        match self {
            Self::Accent => {
                "Marks what's chosen and selected. Play, Stop and warnings keep their own colours."
            }
            Self::Scale => {
                "How large everything in the editor is drawn. Larger is easier to read on a \
                 small or very sharp screen; lines stay crisp at every step."
            }
            Self::Density => {
                "How tall buttons, fields and drop-downs are. Compact fits more on screen; \
                 Comfortable gives larger things to aim at."
            }
            Self::Layout => "Puts this workspace's panels back where they started.",
            Self::Hints => {
                "The Hints panel says what's selected and what you can do with it. Turn it off \
                 once you know your way around."
            }
            Self::HideUnfinished => {
                "Some parts of the editor are still to come: they are shown in grey with a \
                 small square mark. Turn this on to hide them all."
            }
            Self::BaseResolution => {
                "The size the game is drawn at, in pixels, before it's scaled up to fill the \
                 screen."
            }
            Self::PixelsPerUnit => {
                "How many pixels make one unit (one metre) of the world. Usually the tile size."
            }
            Self::TileSize => "The grid new tile maps and the 2D view use, in pixels.",
            Self::Scaling => {
                "Whole steps keeps every pixel the same size, with bars at the edges if need be. \
                 Fill the screen scales to the nearest whole step, then smooths the last bit."
            }
        }
    }

    const ALL: [Self; 10] = [
        Self::Accent,
        Self::Scale,
        Self::Density,
        Self::Layout,
        Self::Hints,
        Self::HideUnfinished,
        Self::BaseResolution,
        Self::PixelsPerUnit,
        Self::TileSize,
        Self::Scaling,
    ];

    /// Whether the search `words` (already lower case) find this setting.
    fn matches(self, words: &str) -> bool {
        let text = format!("{} {} {}", self.page().name(), self.name(), self.hint()).to_lowercase();
        words.split_whitespace().all(|word| text.contains(word))
    }
}

/// The Settings window's own state while it's open.
#[derive(Debug, Clone, Default)]
pub(crate) struct SettingsWindow {
    /// The page shown.
    pub(crate) page: Page,
    /// What's typed in the search box.
    pub(crate) search: String,
}

/// What a click in the window asks for, carried out once it's drawn.
pub(crate) enum Request {
    Accent(Accent),
    /// How large the interface is drawn.
    Scale(theme::Scale),
    /// How tall rows and fields are.
    Density(theme::Density),
    ResetLayout,
    /// Hints on or off.
    Hints(bool),
    /// Unfinished parts hidden or shown.
    HideUnfinished(bool),
    /// The Appearance page as it was at first: the standard accent, scale
    /// and density, hints on, and unfinished parts shown.
    ResetAppearance,
    /// New 2D settings, and whether to save them now (not while a number
    /// is being dragged).
    Pixels(PixelSettings, bool),
}

/// The Settings window's size when it first opens: room for the
/// Appearance page's rows, each with its control beside its words.
const WINDOW_SIZE: egui::Vec2 = egui::vec2(920.0, 740.0);
/// The room left round the Settings window at the least.
const WINDOW_MARGIN: f32 = 24.0;
/// The width of the list of pages at the window's left.
const PAGES_W: f32 = 170.0;

/// Common base resolutions for a pixel-art game, offered in the menu.
pub(crate) const BASE_SIZES: [[u32; 2]; 6] = [
    [320, 180],
    [384, 216],
    [480, 270],
    [640, 360],
    [960, 540],
    [1280, 720],
];

/// The screens the Settings window says a base resolution scales to.
const SCREENS: [(&str, [u32; 2]); 3] = [
    ("1080p", [1920, 1080]),
    ("1440p", [2560, 1440]),
    ("4K", [3840, 2160]),
];

impl Workbench {
    /// Opens File > Preferences.
    pub(crate) fn open_settings(&mut self) {
        if self.settings_window.is_none() {
            self.settings_window = Some(SettingsWindow::default());
        }
    }

    /// Opens the project's own settings: the same window, on its 2D
    /// page.
    pub(crate) fn open_project_settings(&mut self) {
        self.settings_window = Some(SettingsWindow {
            page: Page::TwoD,
            ..self.settings_window.take().unwrap_or_default()
        });
    }

    /// Sets the accent colour for the whole editor (remembered next time).
    pub(crate) fn choose_accent(&mut self, ctx: &egui::Context, accent: Accent) {
        self.choose_interface_look(
            ctx,
            theme::InterfaceLook {
                accent,
                ..self.interface
            },
        );
    }

    /// Sets the accent, scale and density for the whole editor (remembered
    /// next time).
    pub(crate) fn choose_interface_look(
        &mut self,
        ctx: &egui::Context,
        look: theme::InterfaceLook,
    ) {
        self.accent = look.accent;
        self.interface = look;
        theme::restyle(ctx, look);
    }

    /// Uses the project's 2D settings (as read from its `pixels.solpix`,
    /// or the standard ones).
    pub fn set_pixels(&mut self, settings: PixelSettings) {
        self.open.pixels = settings.sanitized();
        self.viewport.pixels = self.open.pixels;
    }

    /// The project's 2D settings.
    pub fn pixels(&self) -> PixelSettings {
        self.open.pixels
    }

    /// Changes the project's 2D settings, saving them when `save` (the
    /// Settings window's 2D page).
    pub(crate) fn change_pixels(
        &mut self,
        settings: PixelSettings,
        save: bool,
        actions: &mut Vec<crate::WorkbenchAction>,
    ) {
        self.open.pixels = settings.sanitized();
        self.viewport.pixels = self.open.pixels;
        if save {
            if self.open.project.is_some() {
                actions.push(crate::WorkbenchAction::SavePixels(self.open.pixels));
            } else {
                self.log(
                    crate::LogLevel::Info,
                    "The 2D settings are kept with a project: open or make one to keep these \
                     changes.",
                );
            }
        }
    }

    /// Hides every unfinished part of the editor, or shows them again
    /// (remembered next time).
    pub(crate) fn hide_unfinished(&mut self, ctx: &egui::Context, hide: bool) {
        self.choose_interface_look(
            ctx,
            theme::InterfaceLook {
                hide_unfinished: hide,
                ..self.interface_look()
            },
        );
    }

    /// Puts the middle of the tab in front back as the design draws it:
    /// its shelf tab, the shelf's height and fold, its split.
    pub fn reset_layout(&mut self) {
        let tab = self.shell.tab;
        self.shell.arrangements.remove(&tab);
    }

    /// Draws the Settings window, if it's open.
    pub(crate) fn settings(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Vec<crate::WorkbenchAction>,
    ) {
        let Some(mut window) = self.settings_window.take() else {
            return;
        };
        let mut open = true;
        let mut request = None;
        let now = Now {
            look: self.interface_look(),
            hints: self.hints,
            pixels: self.open.pixels,
            project: self.open.project.is_some(),
        };
        // A size of its own, never its page's: a setting that makes the
        // page longer or shorter (Density, Hide unfinished parts) must not
        // move the window from under the pointer. It is never larger than
        // the editor's window, whatever the interface scale.
        let room = ctx.content_rect().size() - egui::Vec2::splat(WINDOW_MARGIN);
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            // Opens in the middle of the window, clear of the menus.
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .default_size(WINDOW_SIZE.min(room))
            .max_size(room)
            .show(ctx, |ui| {
                request = contents(ui, &mut window, &now);
            });
        match request {
            Some(Request::Accent(accent)) => self.choose_accent(ctx, accent),
            Some(Request::Scale(scale)) => {
                self.choose_interface_look(ctx, theme::InterfaceLook { scale, ..now.look });
            }
            Some(Request::Density(density)) => {
                self.choose_interface_look(
                    ctx,
                    theme::InterfaceLook {
                        density,
                        ..now.look
                    },
                );
            }
            Some(Request::ResetLayout) => self.reset_layout(),
            Some(Request::Hints(on)) => self.hints = on,
            Some(Request::HideUnfinished(on)) => self.hide_unfinished(ctx, on),
            Some(Request::ResetAppearance) => {
                // The switches that aren't on this page keep their choice.
                let standard = theme::InterfaceLook::default();
                self.choose_interface_look(
                    ctx,
                    theme::InterfaceLook {
                        accent: standard.accent,
                        scale: standard.scale,
                        density: standard.density,
                        hide_unfinished: standard.hide_unfinished,
                        ..now.look
                    },
                );
                self.hints = true;
            }
            Some(Request::Pixels(settings, save)) => self.change_pixels(settings, save, actions),
            None => {}
        }
        if open {
            self.settings_window = Some(window);
        }
    }
}

/// The settings as they are, for the window to show.
pub(crate) struct Now {
    /// The accent, scale and density in use, and whether unfinished
    /// parts are hidden.
    pub(crate) look: theme::InterfaceLook,
    /// Whether the Hints panel shows hints.
    pub(crate) hints: bool,
    pixels: PixelSettings,
    /// Whether a project is open (its settings can be changed).
    project: bool,
}

/// The list of pages on the left, the settings on the right, and Reset
/// this page at the bottom.
fn contents(ui: &mut Ui, window: &mut SettingsWindow, now: &Now) -> Option<Request> {
    let mut request = None;
    let words = window.search.trim().to_lowercase();
    let found: Vec<Setting> = Setting::ALL
        .into_iter()
        .filter(|s| s.matches(&words))
        .collect();
    // While searching, every page's matches show; otherwise the chosen
    // page's settings.
    let shown: Vec<Setting> = if words.is_empty() {
        found
            .into_iter()
            .filter(|s| s.page() == window.page)
            .collect()
    } else {
        found
    };
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| pages_list(ui, window, words.is_empty()));
        ui.separator();
        // The page scrolls inside the window, which keeps its own size.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Downwards, whatever the row of panes it stands in.
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    request = page_body(ui, window.page, &words, shown, now);
                });
            });
    });
    request
}

/// The search box and the list of pages, at the window's left.
fn pages_list(ui: &mut Ui, window: &mut SettingsWindow, browsing: bool) {
    ui.set_width(PAGES_W);
    ui.add(
        egui::TextEdit::singleline(&mut window.search)
            .hint_text("Search settings")
            .desired_width(PAGES_W),
    );
    ui.add_space(6.0);
    for (heading, project) in [("Editor", false), ("This project", true)] {
        ui.label(RichText::new(heading).color(theme::text_weak()).size(11.0));
        for page in Page::ALL.into_iter().filter(|p| p.is_project() == project) {
            let chosen = browsing && window.page == page;
            if ui.selectable_label(chosen, page.name()).clicked() {
                window.page = page;
                window.search.clear();
            }
        }
        ui.add_space(6.0);
    }
}

/// A page's settings (or, while searching, the settings found), with
/// the line under them and Reset this page.
fn page_body(
    ui: &mut Ui,
    page: Page,
    words: &str,
    shown: Vec<Setting>,
    now: &Now,
) -> Option<Request> {
    let mut request = None;
    let browsing = words.is_empty();
    ui.heading(if browsing {
        page.name()
    } else {
        "Search results"
    });
    if browsing && page == Page::TwoD {
        let line = if now.project {
            "For this project's 2D games: kept in the project, so everyone working on it \
             shares them."
        } else {
            "These belong to a project: open or make one to change them."
        };
        ui.label(RichText::new(line).color(theme::text_weak()).size(11.0));
    }
    ui.add_space(4.0);
    if shown.is_empty() {
        ui.label(RichText::new("No setting matches that.").color(theme::text_weak()));
    }
    if browsing && page == Page::Appearance {
        // The page as the design lays it out, in sections.
        request = crate::settings_appearance::page(ui, now);
    } else {
        for setting in shown {
            // A setting found by searching is drawn as on its page.
            let asked = if setting.page() == Page::Appearance {
                crate::settings_appearance::row(ui, setting, now)
            } else {
                let asked = row(ui, setting, now);
                ui.separator();
                asked
            };
            if asked.is_some() {
                request = asked;
            }
        }
    }
    ui.add_space(8.0);
    let footer = if browsing && page == Page::TwoD {
        fits(now.pixels)
    } else {
        "The editor's settings are kept on this computer, a project's in the project.".to_owned()
    };
    ui.label(RichText::new(footer).color(theme::text_weak()).size(11.0));
    ui.add_space(8.0);
    // The 2D page's settings are the project's: nothing to reset without
    // one.
    let can_reset = page != Page::TwoD || now.project;
    let reset = browsing
        && ui
            .add_enabled_ui(can_reset, |ui| {
                crate::widgets::button::Button::plain("Reset this page").show(ui)
            })
            .inner
            .on_hover_text("Puts this page's settings back as they were at first")
            .on_disabled_hover_text("Open a project first: these settings are its own")
            .clicked();
    if reset {
        request = match page {
            Page::Appearance => Some(Request::ResetAppearance),
            Page::TwoD => now
                .project
                .then(|| Request::Pixels(PixelSettings::default(), true)),
        };
    }
    request
}

/// How the base resolution scales to common screens, for under the 2D
/// page: "480 × 270 scales evenly to 1080p (×4), 1440p (×5.33: bars) and
/// 4K (×8)."
pub(crate) fn fits(s: PixelSettings) -> String {
    let [w, h] = s.base;
    let parts: Vec<String> = SCREENS
        .iter()
        .map(|(name, [sw, sh])| {
            let exact = (*sw as f32 / w as f32).min(*sh as f32 / h as f32);
            let whole = (sw / w).min(sh / h);
            if whole >= 1 && whole * w == *sw && whole * h == *sh {
                format!("{name} (×{whole})")
            } else {
                let how = match s.scaling {
                    Scaling::WholeSteps if whole >= 1 => format!("×{whole} with bars"),
                    _ => format!("×{exact:.2}, smoothed"),
                };
                format!("{name} ({how})")
            }
        })
        .collect();
    format!("{w} × {h} on {}, {} and {}.", parts[0], parts[1], parts[2])
}

/// The base resolution's menu text: "480 × 270".
fn size_text([w, h]: [u32; 2]) -> String {
    format!("{w} × {h}")
}

/// One 2D setting's row: its name and hint on the left, its control on
/// the right. (The Appearance page's rows are `settings_appearance.rs`'s.)
fn row(ui: &mut Ui, setting: Setting, now: &Now) -> Option<Request> {
    let mut request = None;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(330.0);
            ui.label(setting.name());
            ui.label(
                RichText::new(setting.hint())
                    .color(theme::text_weak())
                    .size(11.0),
            );
        });
        ui.add_enabled_ui(now.project, |ui| {
            request = pixel_control(ui, setting, now.pixels);
        });
    });
    request
}

/// A 2D setting's control; a change gives the new settings, saved unless
/// a number is still being dragged.
fn pixel_control(ui: &mut Ui, setting: Setting, s: PixelSettings) -> Option<Request> {
    use std::cell::Cell;
    let mut new = s;
    let changed = Cell::new(false);
    let dragging = Cell::new(false);
    let stopped = Cell::new(false);
    let number = |ui: &mut Ui, value: &mut u32, range: (u32, u32), label: &str| {
        let r = ui
            .add(
                egui::DragValue::new(value)
                    .range(range.0..=range.1)
                    .speed(0.25),
            )
            .on_hover_text(label);
        changed.set(changed.get() | r.changed());
        dragging.set(dragging.get() | (r.dragged() && !r.drag_stopped()));
        stopped.set(stopped.get() | r.drag_stopped());
    };
    match setting {
        Setting::BaseResolution => {
            egui::ComboBox::from_id_salt("settings-base")
                .selected_text(size_text(s.base))
                .width(150.0)
                .show_ui(ui, |ui| {
                    for size in BASE_SIZES {
                        if ui
                            .selectable_label(s.base == size, size_text(size))
                            .clicked()
                        {
                            new.base = size;
                            changed.set(true);
                        }
                    }
                    ui.separator();
                    ui.label(
                        RichText::new("Your own")
                            .color(theme::text_weak())
                            .size(11.0),
                    );
                    ui.horizontal(|ui| {
                        number(
                            ui,
                            &mut new.base[0],
                            area::pixels::BASE_RANGE,
                            "Across, in pixels",
                        );
                        ui.label("×");
                        number(
                            ui,
                            &mut new.base[1],
                            area::pixels::BASE_RANGE,
                            "Up, in pixels",
                        );
                    });
                });
        }
        Setting::PixelsPerUnit => {
            number(
                ui,
                &mut new.pixels_per_unit,
                area::pixels::PIXELS_PER_UNIT_RANGE,
                "Pixels to a metre",
            );
        }
        Setting::TileSize => {
            ui.horizontal(|ui| {
                number(
                    ui,
                    &mut new.tile[0],
                    area::pixels::TILE_RANGE,
                    "Across, in pixels",
                );
                ui.label("×");
                number(
                    ui,
                    &mut new.tile[1],
                    area::pixels::TILE_RANGE,
                    "Up, in pixels",
                );
            });
        }
        Setting::Scaling => {
            egui::ComboBox::from_id_salt("settings-scaling")
                .selected_text(s.scaling.label())
                .width(150.0)
                .show_ui(ui, |ui| {
                    for scaling in Scaling::ALL {
                        if ui
                            .selectable_label(s.scaling == scaling, scaling.label())
                            .clicked()
                        {
                            new.scaling = scaling;
                            changed.set(true);
                        }
                    }
                });
        }
        Setting::Accent
        | Setting::Scale
        | Setting::Density
        | Setting::Layout
        | Setting::Hints
        | Setting::HideUnfinished => {}
    }
    if changed.get() && new != s {
        // Saved once the drag ends, not every step of it.
        Some(Request::Pixels(new, !dragging.get()))
    } else if stopped.get() {
        Some(Request::Pixels(s, true))
    } else {
        None
    }
}

#[cfg(test)]
#[path = "settings_window_tests.rs"]
mod tests;
