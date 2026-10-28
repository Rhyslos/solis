//! Edit > Settings…: the editor's settings in one window, a page per
//! subject with a search box ([decision 0061](../../../docs/decisions/0061-settings-window-and-no-layout-presets.md)).
//! Pages appear as they get settings; for now there is Appearance (the
//! accent colour and Reset layout). Settings here are the editor's, kept on
//! this computer: they aren't part of the area, so undo leaves them alone.

use crate::theme::{self, Accent};
use crate::workbench::Workbench;
use egui::{RichText, Ui};

/// A page of the Settings window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Page {
    /// The editor's look: the accent colour and the panel layout.
    #[default]
    Appearance,
}

impl Page {
    /// Every page, in the order the list shows them.
    const ALL: [Self; 1] = [Self::Appearance];

    fn name(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
        }
    }
}

/// One setting: its name and its hint, which the search box looks in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Setting {
    Accent,
    Layout,
}

impl Setting {
    fn page(self) -> Page {
        match self {
            Self::Accent | Self::Layout => Page::Appearance,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Accent => "Accent colour",
            Self::Layout => "Panel layout",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::Accent => {
                "Marks what's chosen and selected. Play, Stop and warnings keep their own colours."
            }
            Self::Layout => "Puts every panel back where it started.",
        }
    }

    const ALL: [Self; 2] = [Self::Accent, Self::Layout];

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
}

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

    /// Puts every panel back in the standard layout.
    pub fn reset_layout(&mut self) {
        self.dock = crate::layout::standard();
    }

    /// Draws the Settings window, if it's open.
    pub(crate) fn settings(&mut self, ctx: &egui::Context) {
        let Some(mut window) = self.settings_window.take() else {
            return;
        };
        let mut open = true;
        let mut request = None;
        let accent = self.accent;
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            // Opens in the middle of the window, clear of the menus.
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .default_width(720.0)
            .default_height(420.0)
            .show(ctx, |ui| {
                request = contents(ui, &mut window, accent);
            });
        match request {
            Some(Request::Accent(accent)) => self.choose_accent(ctx, accent),
            Some(Request::ResetLayout) => self.reset_layout(),
            None => {}
        }
        if open {
            self.settings_window = Some(window);
        }
    }
}

/// The list of pages on the left, the settings on the right, and Reset
/// this page at the bottom.
fn contents(ui: &mut Ui, window: &mut SettingsWindow, accent: Accent) -> Option<Request> {
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
            ui.label(RichText::new("Editor").color(theme::TEXT_WEAK).size(11.0));
            for page in Page::ALL {
                let chosen = words.is_empty() && window.page == page;
                if ui.selectable_label(chosen, page.name()).clicked() {
                    window.page = page;
                    window.search.clear();
                }
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
            ui.add_space(4.0);
            if shown.is_empty() {
                ui.label(RichText::new("No setting matches that.").color(theme::TEXT_WEAK));
            }
            for setting in shown {
                if let Some(r) = row(ui, setting, accent) {
                    request = Some(r);
                }
                ui.separator();
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "More settings join here as features arrive. They're kept on this computer.",
                )
                .color(theme::TEXT_WEAK)
                .size(11.0),
            );
            ui.add_space(8.0);
            if words.is_empty()
                && ui
                    .button("Reset this page")
                    .on_hover_text("Puts this page's settings back as they were at first")
                    .clicked()
            {
                request = Some(match window.page {
                    Page::Appearance => Request::Accent(Accent::default()),
                });
            }
        });
    });
    request
}

/// One setting's row: its name and hint on the left, its control on the
/// right.
fn row(ui: &mut Ui, setting: Setting, accent: Accent) -> Option<Request> {
    let mut request = None;
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
        }
    });
    request
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
