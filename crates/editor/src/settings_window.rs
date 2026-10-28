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
    /// The editor's look: the accent colour and the panel layout.
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
    Layout,
    BaseResolution,
    PixelsPerUnit,
    TileSize,
    Scaling,
}

impl Setting {
    fn page(self) -> Page {
        match self {
            Self::Accent | Self::Layout => Page::Appearance,
            Self::BaseResolution | Self::PixelsPerUnit | Self::TileSize | Self::Scaling => {
                Page::TwoD
            }
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Accent => "Accent colour",
            Self::Layout => "Panel layout",
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
            Self::Layout => "Puts every panel back where it started.",
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

    const ALL: [Self; 6] = [
        Self::Accent,
        Self::Layout,
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
    ResetLayout,
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
        self.accent = accent;
        theme::set_accent(ctx, accent);
    }

    /// Uses the project's 2D settings (as read from its `pixels.solpix`,
    /// or the standard ones).
    pub fn set_pixels(&mut self, settings: PixelSettings) {
        self.pixels = settings.sanitized();
        self.viewport.pixels = self.pixels;
    }

    /// The project's 2D settings.
    pub fn pixels(&self) -> PixelSettings {
        self.pixels
    }

    /// Changes the project's 2D settings, saving them when `save` (the
    /// Settings window's 2D page).
    pub(crate) fn change_pixels(
        &mut self,
        settings: PixelSettings,
        save: bool,
        actions: &mut Vec<crate::WorkbenchAction>,
    ) {
        self.pixels = settings.sanitized();
        self.viewport.pixels = self.pixels;
        if save {
            if self.project.is_some() {
                actions.push(crate::WorkbenchAction::SavePixels(self.pixels));
            } else {
                self.log(
                    crate::LogLevel::Info,
                    "The 2D settings are kept with a project: open or make one to keep these \
                     changes.",
                );
            }
        }
    }

    /// Puts every panel back in the standard layout.
    pub fn reset_layout(&mut self) {
        self.dock = crate::layout::standard();
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
            accent: self.accent,
            pixels: self.pixels,
            project: self.project.is_some(),
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
            Some(Request::ResetLayout) => self.reset_layout(),
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
    accent: Accent,
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
                ui.label(RichText::new(heading).color(theme::TEXT_WEAK).size(11.0));
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
                ui.label(RichText::new(line).color(theme::TEXT_WEAK).size(11.0));
            }
            ui.add_space(4.0);
            if shown.is_empty() {
                ui.label(RichText::new("No setting matches that.").color(theme::TEXT_WEAK));
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
                "More settings join here as features arrive. The editor's are kept on this \
                 computer, a project's in the project."
                    .to_owned()
            };
            ui.label(RichText::new(footer).color(theme::TEXT_WEAK).size(11.0));
            ui.add_space(8.0);
            if words.is_empty()
                && ui
                    .button("Reset this page")
                    .on_hover_text("Puts this page's settings back as they were at first")
                    .clicked()
            {
                request = match window.page {
                    Page::Appearance => Some(Request::Accent(Accent::default())),
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
    let accent = now.accent;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(330.0);
            ui.label(setting.name());
            ui.label(
                RichText::new(setting.hint())
                    .color(theme::TEXT_WEAK)
                    .size(11.0),
            );
        });
        match setting {
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
                    ui.label(RichText::new("Your own").color(theme::TEXT_WEAK).size(11.0));
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
        Setting::Accent | Setting::Layout => {}
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

    #[test]
    fn choosing_an_accent_changes_the_editor_s_colour() {
        let mut h = opened();
        // The accent restyles the editor, whose headings need its fonts.
        h.ctx.set_fonts(theme::fonts());
        h.run();
        // The accent's drop-down, then a colour in it.
        h.get_by_role(egui::accesskit::Role::ComboBox).click();
        h.run();
        h.get_by_label("Plum").click();
        h.run();
        assert_eq!(h.state().0.accent(), Accent::Plum);
        // Reset this page puts the standard one back.
        h.get_by_label("Reset this page").click();
        h.run();
        assert_eq!(h.state().0.accent(), Accent::default());
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
