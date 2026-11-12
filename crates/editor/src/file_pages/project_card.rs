//! One project's card on Recent projects (Screen 7; round 2's
//! `09-recent-projects-a` and `09-recent-card-menu`): its letter where a
//! picture will be, its name and pin, its folder, when it was last
//! opened and by which build. A project whose folder is gone says so,
//! with Locate… and Remove. A right-click opens its menu.

use super::{BuildMark, KnownProject, ago, plain};
use crate::icons::{self, Icon};
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, UnbuiltId};
use crate::widgets::{self, button::Button, menu};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Align2, CornerRadius, Layout, Rect, Sense, Ui, UiBuilder, vec2};

impl Workbench {
    /// Draws project `i` as a card `width` wide.
    pub(super) fn project_card(
        &mut self,
        ui: &mut Ui,
        i: usize,
        width: f32,
        now: u64,
        asked: &mut Vec<WorkbenchAction>,
        changes: &mut Vec<WorkbenchAction>,
    ) {
        let project = self.known_projects[i].clone();
        let selected = self.file_pages.selected.as_ref() == Some(&project.root);
        let words_h = 2.0 * size::PROJECT_CARD_PAD[0]
            + widgets::line_height(ui, &text::card_name())
            + widgets::line_height(ui, &text::mono_small())
            + size::PAGE_BUTTON_H.max(widgets::line_height(ui, &text::row()))
            + 2.0 * size::PROJECT_CARD_LINE_GAP;
        let wanted = vec2(width, size::PROJECT_THUMB_H + words_h);
        let (rect, card) = ui.allocate_exact_size(wanted, Sense::click());
        card.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, &project.name)
        });
        let gone = project.problem.is_some();
        if ui.is_rect_visible(rect) {
            draw_frame(ui, rect, selected, card.hovered());
            let thumb = Rect::from_min_size(rect.min, vec2(width, size::PROJECT_THUMB_H));
            draw_letter(ui, thumb, &project);
            if !unbuilt::hidden() {
                unbuilt::corner(ui, thumb, UnbuiltId::RecentCardThumbnail);
            }
        }
        let inside = Rect::from_min_max(
            egui::pos2(rect.left(), rect.top() + size::PROJECT_THUMB_H),
            rect.max,
        )
        .shrink2(vec2(size::PROJECT_CARD_PAD[1], size::PROJECT_CARD_PAD[0]));
        let mut words = ui.new_child(
            UiBuilder::new()
                .max_rect(inside)
                .layout(Layout::top_down(Align::Min)),
        );
        words.spacing_mut().item_spacing.y = size::PROJECT_CARD_LINE_GAP;
        let faint = |ink| if gone { tokens::text_meta() } else { ink };
        words.horizontal(|ui| {
            painted(
                ui,
                &project.name,
                text::card_name(),
                faint(tokens::text_primary()),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if !gone && pin(ui, &project).clicked() {
                    changes.push(WorkbenchAction::PinProject(
                        project.root.clone(),
                        !project.pinned,
                    ));
                }
            });
        });
        let folder = project.root.display().to_string();
        painted(&mut words, &folder, text::mono_small(), tokens::text_meta());
        words.horizontal(|ui| {
            if gone {
                self.gone_line(ui, &project, asked, changes);
            } else {
                let when = project
                    .last_opened
                    .map_or_else(|| "—".to_string(), |t| ago(t, now));
                plain(ui, &when, text::row(), tokens::text_meta());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    build_mark(ui, &project, self.this_build);
                });
            }
        });
        let root = project.root.clone();
        if card.clicked() || card.secondary_clicked() {
            self.file_pages.selected = Some(root.clone());
        }
        if card.double_clicked() && !gone {
            asked.push(WorkbenchAction::OpenRecent(root.clone()));
        }
        card.context_menu(|ui| self.card_menu(ui, &project, asked, changes));
        let hover = project
            .problem
            .clone()
            .unwrap_or_else(|| "Double-click to open".into());
        card.on_hover_text(hover);
    }

    /// A gone project's last line: what's wrong, Locate… and Remove.
    fn gone_line(
        &self,
        ui: &mut Ui,
        project: &KnownProject,
        asked: &mut Vec<WorkbenchAction>,
        changes: &mut Vec<WorkbenchAction>,
    ) {
        let problem = project.problem.as_deref().unwrap_or_default();
        plain(ui, problem, text::row(), tokens::WARNING);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = size::GAP;
            let remove = Button::plain("Remove").show(ui);
            remove.widget_info(|| {
                let name = format!("Remove {} from the list", project.name);
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
            });
            if remove
                .on_hover_text("Forgets it here; nothing is deleted")
                .clicked()
            {
                changes.push(WorkbenchAction::ForgetProject(project.root.clone()));
            }
            let locate = Button::plain("Locate…").show(ui);
            locate.widget_info(|| {
                let name = format!("Locate {}", project.name);
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
            });
            if locate
                .on_hover_text("Moved it? Choose the folder where it is now")
                .clicked()
            {
                asked.push(WorkbenchAction::LocateProject(project.root.clone()));
            }
        });
    }

    /// A card's right-click menu: Open, Pin, Show in Explorer, Rename…,
    /// Make a copy…, Remove from list.
    fn card_menu(
        &mut self,
        ui: &mut Ui,
        project: &KnownProject,
        asked: &mut Vec<WorkbenchAction>,
        changes: &mut Vec<WorkbenchAction>,
    ) {
        let root = project.root.clone();
        let gone = project.problem.is_some();
        if !gone && menu::item(ui, "Open", "Enter").clicked() {
            asked.push(WorkbenchAction::OpenRecent(root.clone()));
        }
        let pin_word = if project.pinned { "Unpin" } else { "Pin" };
        if !gone && menu::item(ui, pin_word, "").clicked() {
            changes.push(WorkbenchAction::PinProject(root.clone(), !project.pinned));
        }
        if !gone && menu::item(ui, "Show in Explorer", "").clicked() {
            asked.push(WorkbenchAction::ShowProject(root.clone()));
        }
        menu::line(ui);
        let rename = menu::item(ui, "Rename…", "")
            .on_hover_text("Changes the project's name; its folder keeps its name");
        if !gone && rename.clicked() {
            self.file_pages.rename(root.clone(), project.name.clone());
        }
        let copy = menu::item(ui, "Make a copy…", "").on_hover_text(
            "Copies the whole folder to an empty one you choose (leaving out the import cache, \
             which is made again)",
        );
        if !gone && copy.clicked() {
            asked.push(WorkbenchAction::CopyProject(root.clone()));
        }
        menu::line(ui);
        let remove = menu::item(ui, "Remove from list", "")
            .on_hover_text("Forgets it here; its files stay where they are");
        if remove.clicked() {
            changes.push(WorkbenchAction::ForgetProject(root));
        }
    }
}

/// Words drawn without a name of their own for screen readers: the card
/// itself is named after the project.
fn painted(ui: &mut Ui, words: &str, font: egui::FontId, ink: egui::Color32) {
    let width = widgets::width_of(ui, words, font.clone()).min(ui.available_width());
    let height = widgets::line_height(ui, &font);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    widgets::cut_off(ui, rect, words, font, ink);
}

/// The card's own box: the panel's grey, a 1 px border in the accent's
/// tint while picked, a touch lighter under the pointer.
fn draw_frame(ui: &Ui, rect: Rect, selected: bool, hovered: bool) {
    let radius = size::radius::PROJECT_CARD;
    let fill = if hovered {
        tokens::card()
    } else {
        tokens::panel()
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(radius), fill);
    let thumb = Rect::from_min_size(rect.min, vec2(rect.width(), size::PROJECT_THUMB_H));
    let top = CornerRadius {
        nw: radius,
        ne: radius,
        sw: 0,
        se: 0,
    };
    ui.painter().rect_filled(thumb, top, tokens::viewport());
    let edge = if selected {
        tokens::open_card_border(crate::interface_look::in_use().accent)
    } else {
        tokens::card_border()
    };
    widgets::border(ui, rect, radius, edge);
}

/// The letter that stands for the project's picture: its first letter,
/// or "?" when its folder is gone (round 2: until projects have
/// pictures).
fn draw_letter(ui: &Ui, thumb: Rect, project: &KnownProject) {
    let letter = if project.problem.is_some() {
        "?".to_string()
    } else {
        project
            .name
            .chars()
            .find(|c| c.is_alphanumeric())
            .map_or_else(|| "?".to_string(), |c| c.to_uppercase().collect())
    };
    widgets::words(
        ui,
        thumb,
        Align2::CENTER_CENTER,
        &letter,
        text::project_letter(),
        tokens::stronger_line(),
    );
}

/// The pin at the right of the name: in the accent while pinned.
fn pin(ui: &mut Ui, project: &KnownProject) -> egui::Response {
    let (spot, response) =
        ui.allocate_exact_size(vec2(size::ICON_SMALL, size::ICON_SMALL), Sense::click());
    let ink = if project.pinned {
        crate::interface_look::accent_fill()
    } else if response.hovered() {
        tokens::text_label()
    } else {
        tokens::text_faint()
    };
    icons::icon(ui.painter(), spot, Icon::Pin, ink);
    let what = if project.pinned { "Unpin" } else { "Pin" };
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("{what} {}", project.name),
        )
    });
    response.on_hover_text(if project.pinned {
        "Unpin: back among the others"
    } else {
        "Pin: kept at the top of the list"
    })
}

/// The build that last opened the project, at the card's right.
fn build_mark(ui: &mut Ui, project: &KnownProject, this: Option<u64>) {
    match BuildMark::of(project.saved_by, this) {
        Some(mark) => {
            let (words, ink, why) = mark.look();
            plain(ui, &words, text::mono_small(), ink).on_hover_text(why);
        }
        None => {
            plain(ui, "—", text::mono_small(), tokens::text_faint()).on_hover_text(
                "Not known: last opened before builds were noted, or by a copy built from source",
            );
        }
    }
}
