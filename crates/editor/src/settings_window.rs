//! Edit > Settings…: the editor's settings in one window, a page per
//! subject with a search box ([decision 0061](../../../docs/decisions/0061-settings-window-and-no-layout-presets.md)).
//! Pages appear as they get settings: the editor's own (Appearance: the
//! accent colour and Reset layout), kept on this computer, and the open
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
enum Setting {
    Accent,
    Scale,
    Density,
    Layout,
    Hints,
    BaseResolution,
    PixelsPerUnit,
    TileSize,
    Scaling,
}

impl Setting {
    fn page(self) -> Page {
        match self {
            Self::Accent | Self::Scale | Self::Density | Self::Layout | Self::Hints => {
                Page::Appearance
            }
            Self::BaseResolution | Self::PixelsPerUnit | Self::TileSize | Self::Scaling => {
                Page::TwoD
            }
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Accent => "Accent colour",
            Self::Scale => "Interface scale",
            Self::Density => "Density",
            Self::Layout => "Panel layout",
            Self::Hints => "Hints",
            Self::BaseResolution => "Base resolution",
            Self::PixelsPerUnit => "Pixels per unit",
            Self::TileSize => "Tile size",
            Self::Scaling => "Scaling to the screen",
        }
    }

    fn hint(self) -> &'static str {
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

    const ALL: [Self; 9] = [
        Self::Accent,
        Self::Scale,
        Self::Density,
        Self::Layout,
        Self::Hints,
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
enum Request {
    Accent(Accent),
    /// How large the interface is drawn.
    Scale(theme::Scale),
    /// How tall rows and fields are.
    Density(theme::Density),
    ResetLayout,
    /// Hints on or off.
    Hints(bool),
    /// The Appearance page as it was at first: the standard accent, scale
    /// and density, and hints on.
    ResetAppearance,
    /// New 2D settings, and whether to save them now (not while a number
    /// is being dragged).
    Pixels(PixelSettings, bool),
}

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
    /// Opens Edit > Settings….
    pub(crate) fn open_settings(&mut self) {
        if self.settings_window.is_none() {
            self.settings_window = Some(SettingsWindow::default());
        }
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

    /// Puts every panel of the workspace in use back where it started.
    pub fn reset_layout(&mut self) {
        self.reset_workspace();
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
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            // Opens in the middle of the window, clear of the menus.
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .default_width(720.0)
            .default_height(420.0)
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
            Some(Request::ResetAppearance) => {
                // The switches that aren't on this page keep their choice.
                let standard = theme::InterfaceLook::default();
                self.choose_interface_look(
                    ctx,
                    theme::InterfaceLook {
                        accent: standard.accent,
                        scale: standard.scale,
                        density: standard.density,
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
struct Now {
    /// The accent, scale and density in use.
    look: theme::InterfaceLook,
    /// Whether the Hints panel shows hints.
    hints: bool,
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
        ui.vertical(|ui| {
            ui.set_width(170.0);
            ui.add(
                egui::TextEdit::singleline(&mut window.search)
                    .hint_text("Search settings")
                    .desired_width(170.0),
            );
            ui.add_space(6.0);
            for (heading, project) in [("Editor", false), ("This project", true)] {
                ui.label(RichText::new(heading).color(theme::text_weak()).size(11.0));
                for page in Page::ALL.into_iter().filter(|p| p.is_project() == project) {
                    let chosen = words.is_empty() && window.page == page;
                    if ui.selectable_label(chosen, page.name()).clicked() {
                        window.page = page;
                        window.search.clear();
                    }
                }
                ui.add_space(6.0);
            }
        });
        ui.separator();
        ui.vertical(|ui| {
            let heading = if words.is_empty() {
                window.page.name()
            } else {
                "Search results"
            };
            ui.heading(heading);
            if words.is_empty() && window.page == Page::TwoD {
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
            for setting in shown {
                if let Some(r) = row(ui, setting, now) {
                    request = Some(r);
                }
                ui.separator();
            }
            ui.add_space(8.0);
            let footer = if words.is_empty() && window.page == Page::TwoD {
                fits(now.pixels)
            } else {
                "The editor's settings are kept on this computer, a project's in the project."
                    .to_owned()
            };
            ui.label(RichText::new(footer).color(theme::text_weak()).size(11.0));
            ui.add_space(8.0);
            // The 2D page's settings are the project's: nothing to reset
            // without one.
            let can_reset = window.page != Page::TwoD || now.project;
            if words.is_empty()
                && ui
                    .add_enabled(can_reset, egui::Button::new("Reset this page"))
                    .on_hover_text("Puts this page's settings back as they were at first")
                    .on_disabled_hover_text("Open a project first: these settings are its own")
                    .clicked()
            {
                request = match window.page {
                    Page::Appearance => Some(Request::ResetAppearance),
                    Page::TwoD => now
                        .project
                        .then(|| Request::Pixels(PixelSettings::default(), true)),
                };
            }
        });
    });
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

/// One setting's row: its name and hint on the left, its control on the
/// right.
fn row(ui: &mut Ui, setting: Setting, now: &Now) -> Option<Request> {
    let mut request = None;
    let accent = now.look.accent;
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
        match setting {
            Setting::Hints => {
                let mut on = now.hints;
                if ui.checkbox(&mut on, "Show hints").changed() {
                    request = Some(Request::Hints(on));
                }
            }
            Setting::Accent => {
                egui::ComboBox::from_id_salt("settings-accent")
                    .selected_text(accent.name())
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for choice in Accent::ALL {
                            ui.horizontal(|ui| {
                                let (swatch, _) = ui.allocate_exact_size(
                                    egui::vec2(16.0, 10.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(swatch, 3.0, choice.colors().fill);
                                if ui
                                    .selectable_label(accent == choice, choice.name())
                                    .clicked()
                                {
                                    request = Some(Request::Accent(choice));
                                }
                            });
                        }
                    });
            }
            Setting::Scale => {
                egui::ComboBox::from_id_salt("settings-scale")
                    .selected_text(now.look.scale.name())
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for choice in theme::Scale::ALL {
                            if ui
                                .selectable_label(now.look.scale == choice, choice.name())
                                .clicked()
                            {
                                request = Some(Request::Scale(choice));
                            }
                        }
                    });
            }
            Setting::Density => {
                egui::ComboBox::from_id_salt("settings-density")
                    .selected_text(now.look.density.name())
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for choice in theme::Density::ALL {
                            if ui
                                .selectable_label(now.look.density == choice, choice.name())
                                .clicked()
                            {
                                request = Some(Request::Density(choice));
                            }
                        }
                    });
            }
            Setting::Layout => {
                if ui.button("Reset layout").clicked() {
                    request = Some(Request::ResetLayout);
                }
            }
            Setting::BaseResolution
            | Setting::PixelsPerUnit
            | Setting::TileSize
            | Setting::Scaling => {
                ui.add_enabled_ui(now.project, |ui| {
                    request = pixel_control(ui, setting, now.pixels);
                });
            }
        }
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
        Setting::Accent | Setting::Scale | Setting::Density | Setting::Layout | Setting::Hints => {}
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
mod tests {
    use super::*;
    use crate::panel::Panel;
    use crate::test_support::*;
    use egui_kittest::kittest::Queryable;

    /// Opens Edit > Settings… by clicking, as a person would.
    fn opened() -> egui_kittest::Harness<'static, State> {
        let mut h = harness(Workbench::new(info(), None));
        h.run();
        h.get_all_by_label("Edit").next().unwrap().click();
        h.run();
        h.get_by_label("Settings…").click();
        h.run();
        h
    }

    #[test]
    fn edit_settings_opens_the_window_on_appearance() {
        let h = opened();
        assert!(h.state().0.settings_window.is_some());
        assert!(h.query_by_label("Accent colour").is_some());
        assert!(h.query_by_label("Reset layout").is_some());
    }

    /// Opens the drop-down beside `setting` and picks `choice` in it.
    fn choose(h: &mut egui_kittest::Harness<'static, State>, nth: usize, choice: &str) {
        h.get_all_by_role(egui::accesskit::Role::ComboBox)
            .nth(nth)
            .unwrap()
            .click();
        h.run();
        h.get_by_label(choice).click();
        h.run();
    }

    #[test]
    fn choosing_an_accent_changes_the_editor_s_colour() {
        let mut h = opened();
        // The accent restyles the editor, whose headings need its fonts.
        h.ctx.set_fonts(theme::fonts());
        h.run();
        // The accent's drop-down is the page's first.
        choose(&mut h, 0, "Purple");
        assert_eq!(h.state().0.accent(), Accent::Purple);
        assert_eq!(h.state().0.interface_look().accent, Accent::Purple);
        let style = h.ctx.global_style();
        assert_eq!(
            style.visuals.selection.bg_fill,
            crate::tokens::select_row(Accent::Purple)
        );
        // Reset this page puts the standard one back.
        h.get_by_label("Reset this page").click();
        h.run();
        assert_eq!(h.state().0.accent(), Accent::default());
    }

    #[test]
    fn the_five_accents_are_offered_and_the_old_ones_are_gone() {
        let mut h = opened();
        h.ctx.set_fonts(theme::fonts());
        h.run();
        h.get_all_by_role(egui::accesskit::Role::ComboBox)
            .next()
            .unwrap()
            .click();
        h.run();
        for accent in Accent::ALL {
            assert!(h.query_all_by_label(accent.name()).next().is_some());
        }
        for gone in ["Plum", "Marine green", "Petrol blue", "Amber", "Gold"] {
            assert!(h.query_by_label(gone).is_none(), "{gone}");
        }
    }

    #[test]
    fn the_theme_and_rounded_panels_settings_are_gone() {
        let h = opened();
        for gone in ["Theme", "Rounded panels", "Use rounded panels", "Graphite"] {
            assert!(h.query_by_label(gone).is_none(), "{gone}");
        }
        assert!(!Setting::ALL.iter().any(|s| s.matches("graphite")));
        assert!(!Setting::ALL.iter().any(|s| s.matches("rounded")));
    }

    #[test]
    fn interface_scale_draws_everything_larger_and_reset_puts_it_back() {
        let mut h = opened();
        h.ctx.set_fonts(theme::fonts());
        h.run();
        assert!(h.query_by_label("Interface scale").is_some());
        let wide = h.ctx.content_rect().width();
        choose(&mut h, 1, "150%");
        h.run();
        assert_eq!(h.state().0.interface_look().scale.percent(), 150);
        assert_eq!(h.ctx.zoom_factor(), 1.5);
        // Each point is now a pixel and a half, so the same window holds
        // two thirds as many points across.
        assert_eq!(h.ctx.pixels_per_point(), 1.5);
        let narrow = h.ctx.content_rect().width();
        assert!((narrow * 1.5 - wide).abs() < 1.0, "{wide} then {narrow}");
        // The accent kept its choice.
        assert_eq!(h.state().0.accent(), Accent::default());
        h.get_by_label("Reset this page").click();
        h.run();
        h.run();
        assert_eq!(
            h.state().0.interface_look(),
            theme::InterfaceLook::default()
        );
        assert_eq!(h.ctx.zoom_factor(), 1.0);
    }

    #[test]
    fn density_makes_buttons_and_fields_taller_or_shorter() {
        let mut h = opened();
        h.ctx.set_fonts(theme::fonts());
        // The standard look first, as the app applies it when it starts.
        theme::restyle(&h.ctx, theme::InterfaceLook::default());
        h.run();
        assert!(h.query_by_label("Density").is_some());
        let height = |h: &egui_kittest::Harness<'static, State>| {
            h.get_by_label("Reset layout").rect().height()
        };
        assert_eq!(height(&h), 24.0);
        choose(&mut h, 2, "Compact");
        h.run();
        assert_eq!(
            h.state().0.interface_look().density,
            theme::Density::Compact
        );
        assert_eq!(height(&h), 22.0);
        choose(&mut h, 2, "Comfortable");
        h.run();
        assert_eq!(height(&h), 28.0);
        h.get_by_label("Reset this page").click();
        h.run();
        h.run();
        assert_eq!(height(&h), 24.0);
        theme::restyle(&h.ctx, theme::InterfaceLook::default());
    }

    #[test]
    fn reset_this_page_leaves_the_switches_of_other_pages_alone() {
        let mut w = Workbench::new(info(), None).with_interface_look(theme::InterfaceLook {
            accent: Accent::Teal,
            hide_unfinished: true,
            ribbon_labels: false,
            ..theme::InterfaceLook::default()
        });
        w.open_settings();
        let mut h = harness(w);
        h.ctx.set_fonts(theme::fonts());
        h.run();
        h.get_by_label("Reset this page").click();
        h.run();
        let look = h.state().0.interface_look();
        assert_eq!(look.accent, Accent::default());
        assert!(look.hide_unfinished);
        assert!(!look.ribbon_labels);
        theme::restyle(&h.ctx, theme::InterfaceLook::default());
    }

    #[test]
    fn reset_layout_puts_the_panels_back() {
        let mut h = opened();
        let console = h.state().0.dock.find_tab(&Panel::Console).unwrap();
        h.state_mut().0.dock.remove_tab(console);
        h.state_mut().0.dock.push_to_first_leaf(Panel::Console);
        h.get_all_by_label("Reset layout").next().unwrap().click();
        h.run();
        // Every panel is back in its standard tab group.
        let dock = &h.state().0.dock;
        let standard = crate::layout::standard();
        for panel in Panel::ALL {
            let group = |d: &egui_dock::DockState<Panel>| {
                let path = d.find_tab(&panel).unwrap();
                d[path.surface][path.node].tabs().unwrap().to_vec()
            };
            assert_eq!(group(dock), group(&standard), "{panel:?}");
        }
    }

    #[test]
    fn the_search_box_narrows_the_settings() {
        let mut w = Workbench::new(info(), None);
        w.open_settings();
        w.settings_window.as_mut().unwrap().search = "layout".into();
        let mut h = harness(w);
        h.run();
        assert!(h.query_by_label("Reset layout").is_some());
        assert!(h.query_by_label("Accent colour").is_none());
        h.state_mut().0.settings_window.as_mut().unwrap().search = "zebra".into();
        h.run();
        assert!(h.query_by_label("No setting matches that.").is_some());
    }

    #[test]
    fn the_window_menu_has_reset_layout_and_no_presets() {
        let mut h = harness(Workbench::new(info(), None));
        h.run();
        h.get_by_label("Window").click();
        h.run();
        assert!(h.query_by_label("Reset layout").is_some());
        for gone in ["Layouts", "Wide screen", "Small screen", "Accent colour"] {
            assert!(h.query_by_label(gone).is_none(), "{gone}");
        }
    }

    #[test]
    fn search_finds_settings_by_name_hint_or_page() {
        assert!(Setting::Accent.matches("accent"));
        assert!(Setting::Accent.matches("colour"));
        assert!(Setting::Layout.matches("panel"));
        assert!(Setting::Layout.matches("appearance"));
        assert!(Setting::Scale.matches("scale"));
        assert!(Setting::Scale.matches("larger"));
        assert!(Setting::Density.matches("compact"));
        // Every word must be found.
        assert!(Setting::Accent.matches("accent colour"));
        assert!(!Setting::Accent.matches("accent zebra"));
        assert!(Setting::ALL.iter().all(|s| s.matches("")));
    }

    #[test]
    fn every_setting_is_on_a_listed_page() {
        for setting in Setting::ALL {
            assert!(Page::ALL.contains(&setting.page()), "{}", setting.name());
        }
    }
}
