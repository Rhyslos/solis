//! The page Preferences and Project settings share (Screens 8 and 12):
//! the heading with the search across every sub-tab, the sub-tabs, and
//! the settings in their sections as setting rows, with a column of
//! cards beside them. A setting whose value isn't its standard one has
//! a dot before its name and Reset after it (Project settings), and the
//! heading counts them.

use super::settings::{Setting, Tab, found};
use super::{Page, page_head, plain, section, with_side};
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry};
use crate::widgets::{self, field, rows, sub_tab::sub_tab};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Rect, Ui, pos2, vec2};

impl Workbench {
    /// Shows Preferences (`project` false) or Project settings on `tab`.
    pub(crate) fn show_settings(&mut self, tab: Tab) {
        let page = if tab.is_project() {
            self.file_pages.project_tab = tab;
            Page::ProjectSettings
        } else {
            self.file_pages.prefs_tab = tab;
            Page::Preferences
        };
        self.show_file_page(page);
    }

    /// Draws Preferences or Project settings.
    pub(super) fn settings_page(
        &mut self,
        ui: &mut Ui,
        project: bool,
        asked: &mut Vec<WorkbenchAction>,
    ) {
        let (title, subtitle) = if project {
            let name = self
                .open
                .project
                .as_ref()
                .map_or("the project", |p| p.name.as_str());
            (
                "Project settings",
                format!(
                    "Shared with everyone on {name}. Saved in the project's folder, in small text \
                     files Git can merge."
                ),
            )
        } else {
            (
                "Preferences",
                "Saved for you on this computer. Project settings are shared with everyone on \
                 the project."
                    .to_string(),
            )
        };
        let changed = if project {
            self.changed_settings().len()
        } else {
            0
        };
        let accent = self.accent;
        page_head(ui, title, &subtitle, |ui| {
            let hint = if project {
                "Search project settings"
            } else {
                "Search all settings"
            };
            field::search_field(
                ui,
                hint,
                &mut self.file_pages.settings_search,
                hint,
                size::PAGE_SEARCH_W,
            );
            if changed > 0 {
                changed_pill(ui, changed, accent);
            }
        });
        ui.add_space(size::PAGE_BLOCK_GAP);
        self.sub_tabs(ui, project);
        ui.add_space(size::SECTION_GAP);
        let words = self.file_pages.settings_search.trim().to_lowercase();
        let tab = if project {
            self.file_pages.project_tab
        } else {
            self.file_pages.prefs_tab
        };
        // The input page and the shortcuts are tables with tools of their
        // own; the rest scroll as rows.
        if words.is_empty() && tab == Tab::Input {
            self.input_page(ui);
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if !words.is_empty() {
                    let shown = found(project, &words);
                    if shown.is_empty() {
                        plain(
                            ui,
                            "No setting matches that.",
                            text::ui(),
                            tokens::text_meta(),
                        );
                    }
                    let width = ui.available_width().min(size::SETTINGS_MAX_W);
                    ui.allocate_ui(vec2(width, 0.0), |ui| {
                        for setting in shown {
                            self.setting(ui, setting, asked);
                        }
                    });
                    return;
                }
                if tab == Tab::Shortcuts {
                    self.shortcuts(ui);
                    return;
                }
                with_side(
                    ui,
                    &mut (self, asked),
                    |ui, (w, asked)| w.sections(ui, tab, asked),
                    |ui, (w, asked)| w.side(ui, tab, asked),
                );
            });
    }

    /// The row of sub-tabs, a hairline under it.
    fn sub_tabs(&mut self, ui: &mut Ui, project: bool) {
        let tabs = if project {
            Tab::project()
        } else {
            Tab::preferences()
        };
        let front = if project {
            self.file_pages.project_tab
        } else {
            self.file_pages.prefs_tab
        };
        let searching = !self.file_pages.settings_search.trim().is_empty();
        let across = ui.available_rect_before_wrap().x_range();
        let mut chosen = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = size::SUB_TAB_GAP;
            for entry in unbuilt::filter(tabs) {
                match entry {
                    Entry::Built(tab) => {
                        if sub_tab(ui, tab.name(), !searching && tab == front).clicked() {
                            chosen = Some(tab);
                        }
                    }
                    Entry::Unbuilt(id) => drop(unbuilt::sub_tab(ui, id)),
                }
            }
        });
        tokens::hline(ui.painter(), across, ui.cursor().top(), tokens::line());
        if let Some(tab) = chosen {
            self.file_pages.settings_search.clear();
            self.show_settings(tab);
        }
    }

    /// A sub-tab's sections.
    fn sections(&mut self, ui: &mut Ui, tab: Tab, asked: &mut Vec<WorkbenchAction>) {
        for (n, (caps, entries)) in unbuilt::filter_groups(tab.sections())
            .into_iter()
            .enumerate()
        {
            if n == 0 {
                rows::section_caps(ui, caps);
            } else {
                section(ui, caps);
            }
            for entry in entries {
                match entry {
                    Entry::Built(setting) => self.setting(ui, setting, asked),
                    Entry::Unbuilt(id) => drop(unbuilt::setting_row(ui, id)),
                }
            }
        }
    }

    /// The column beside a sub-tab's settings.
    fn side(&mut self, ui: &mut Ui, tab: Tab, asked: &mut [WorkbenchAction]) {
        if tab.is_project() {
            self.project_side(ui, tab);
        } else {
            self.preferences_side(ui, tab, asked);
        }
    }

    /// One setting as a row: its words on the left (with the changed dot
    /// and Reset), its control at the right.
    pub(super) fn setting(
        &mut self,
        ui: &mut Ui,
        setting: Setting,
        asked: &mut Vec<WorkbenchAction>,
    ) {
        let name = match setting {
            Setting::AutoUpdate => format!("Look for updates when {} starts", self.info.name),
            _ => setting.name().to_string(),
        };
        let changed = setting.tab().is_project()
            && self
                .changed_settings()
                .iter()
                .any(|(s, _, _)| *s == setting);
        let mut reset = false;
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            rows::setting_row_with(
                ui,
                |ui| {
                    ui.horizontal(|ui| {
                        if changed {
                            let at = ui.cursor().left_center()
                                + vec2(-size::GAP - size::STATUS_DOT, 0.0);
                            let middle = pos2(
                                at.x,
                                ui.cursor().top() + size::SETTING_ROW_MIN_H / 2.0 - 4.0,
                            );
                            ui.painter().circle_filled(
                                middle,
                                size::STATUS_DOT / 2.0,
                                crate::interface_look::accent_fill(),
                            );
                        }
                        plain(ui, &name, text::setting_name(), tokens::text_primary());
                        if changed {
                            reset = widgets::button::link(ui, "Reset")
                                .on_hover_text("Back to its standard value")
                                .clicked();
                        }
                    });
                    super::sentence(ui, setting.hint());
                },
                |ui| {
                    if setting.tab().is_project() {
                        self.project_control(ui, setting, asked);
                    } else {
                        self.preference_control(ui, setting, asked);
                    }
                },
            );
        });
        if reset {
            self.reset_setting(setting, asked);
        }
    }
}

/// "2 changed from defaults": the accent-tinted pill in the heading.
fn changed_pill(ui: &mut Ui, count: usize, accent: crate::theme::Accent) {
    let words = format!("{count} changed from defaults");
    let font = text::ui();
    let width =
        widgets::width_of(ui, &words, font.clone()) + 3.0 * size::FIELD_PAD + size::STATUS_DOT;
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, size::PAGE_BUTTON_H), egui::Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &words));
    let round = egui::CornerRadius::same((size::PAGE_BUTTON_H / 2.0) as u8);
    ui.painter()
        .rect_filled(rect, round, tokens::changes_pill(accent));
    let dot = pos2(
        rect.left() + size::FIELD_PAD + size::STATUS_DOT / 2.0,
        rect.center().y,
    );
    ui.painter().circle_filled(
        dot,
        size::STATUS_DOT / 2.0,
        crate::interface_look::accent_fill(),
    );
    let room = Rect::from_min_max(pos2(dot.x + size::FIELD_PAD, rect.top()), rect.max);
    widgets::words(
        ui,
        room,
        egui::Align2::LEFT_CENTER,
        &words,
        font,
        tokens::text_primary(),
    );
}
