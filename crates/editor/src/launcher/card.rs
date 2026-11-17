//! One project's card on the launcher (board 01): the picture of its kind
//! with the kind in its corner, its name with the ⋯ menu, its folder,
//! when it was last opened, and Open. A project whose folder is gone
//! says so, with Locate… and Remove. Double-clicking a card opens it.
//!
//! The ⋯ menu (also a right-click) holds Open, Show in Explorer and
//! Remove from list as drawn, then what the launcher before it could do
//! to a project: Pin, Rename… and Make a copy….

use super::{KnownProject, ago, pictures};
use crate::file_pages::{BuildMark, plain};
use crate::tokens::{self, launcher as l, size, text};
use crate::widgets::{self, menu};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Align2, CornerRadius, Layout, Rect, Sense, Ui, UiBuilder, pos2, vec2};
use std::path::{Component, Path};

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
        let selected = self.launcher.selected.as_ref() == Some(&project.root);
        let [pad_y, pad_x] = l::CARD_PAD;
        // A row of words is at least as tall as a control in it.
        let row = |font: &egui::FontId| {
            widgets::line_height(ui, font)
                .max(ui.spacing().interact_size.y)
                .max(l::CARD_MENU)
        };
        let words_h = 2.0 * pad_y
            + row(&text::card_name())
            + widgets::line_height(ui, &text::mono_small())
            + row(&text::row())
            + 2.0 * l::CARD_LINE_GAP;
        let wanted = vec2(width, l::CARD_PICTURE_H + words_h);
        let (rect, card) = ui.allocate_exact_size(wanted, Sense::click());
        card.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, &project.name)
        });
        let gone = project.problem.is_some();
        let lit = selected || card.hovered();
        if ui.is_rect_visible(rect) {
            draw_frame(ui, rect, selected, card.hovered(), &project);
        }
        let inside =
            Rect::from_min_max(pos2(rect.left(), rect.top() + l::CARD_PICTURE_H), rect.max)
                .shrink2(vec2(pad_x, pad_y));
        let mut words = ui.new_child(
            UiBuilder::new()
                .max_rect(inside)
                .layout(Layout::top_down(Align::Min)),
        );
        words.spacing_mut().item_spacing.y = l::CARD_LINE_GAP;
        let faint = |ink| if gone { tokens::text_meta() } else { ink };
        words.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let more = more_button(ui, &project.name);
                egui::Popup::menu(&more).show(|ui| self.card_menu(ui, &project, asked, changes));
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    painted(
                        ui,
                        &project.name,
                        text::card_name(),
                        faint(tokens::text_primary()),
                    );
                });
            });
        });
        painted(
            &mut words,
            &short_folder(&project.root),
            text::mono_small(),
            tokens::text_meta(),
        );
        words.horizontal(|ui| {
            if gone {
                self.gone_line(ui, &project, asked, changes);
                return;
            }
            let when = project.last_opened.map_or_else(
                || "Not opened yet".to_string(),
                |t| opened_words(&ago(t, now)),
            );
            plain(ui, &when, text::row(), tokens::text_meta());
            if lit {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let open = widgets::button::link(ui, "Open");
                    open.widget_info(|| {
                        let name = format!("Open {}", project.name);
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
                    });
                    if open.clicked() {
                        asked.push(WorkbenchAction::OpenRecent(project.root.clone()));
                    }
                });
            }
        });
        let root = project.root.clone();
        if card.clicked() || card.secondary_clicked() {
            self.launcher.selected = Some(root.clone());
        }
        if card.double_clicked() && !gone {
            asked.push(WorkbenchAction::OpenRecent(root));
        }
        card.context_menu(|ui| self.card_menu(ui, &project, asked, changes));
        card.on_hover_text(self.card_hover(&project));
    }

    /// What the card says under the pointer: how to open it, or what's
    /// wrong, and the build that last opened it.
    fn card_hover(&self, project: &KnownProject) -> String {
        let first = project
            .problem
            .clone()
            .unwrap_or_else(|| "Double-click to open".into());
        match BuildMark::of(project.saved_by, self.this_build) {
            Some(mark) => format!("{first}. {}.", mark.look().2),
            None => format!("{first}."),
        }
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
            let remove = widgets::button::link(ui, "Remove");
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
            let locate = widgets::button::link(ui, "Locate…");
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

    /// A card's menu (⋯ or a right-click): Open, Show in Explorer and
    /// Remove from list, then Pin, Rename… and Make a copy….
    fn card_menu(
        &mut self,
        ui: &mut Ui,
        project: &KnownProject,
        asked: &mut Vec<WorkbenchAction>,
        changes: &mut Vec<WorkbenchAction>,
    ) {
        let root = project.root.clone();
        let gone = project.problem.is_some();
        if !gone && menu::item(ui, "Open", "").clicked() {
            asked.push(WorkbenchAction::OpenRecent(root.clone()));
        }
        if !gone && menu::item(ui, "Show in Explorer", "").clicked() {
            asked.push(WorkbenchAction::ShowProject(root.clone()));
        }
        let remove = menu::item(ui, "Remove from list", "")
            .on_hover_text("Forgets it here; its files stay where they are");
        if remove.clicked() {
            changes.push(WorkbenchAction::ForgetProject(root.clone()));
        }
        if gone {
            return;
        }
        menu::line(ui);
        let pin_word = if project.pinned {
            "Unpin"
        } else {
            "Pin to the top"
        };
        if menu::item(ui, pin_word, "").clicked() {
            changes.push(WorkbenchAction::PinProject(root.clone(), !project.pinned));
        }
        let rename = menu::item(ui, "Rename…", "")
            .on_hover_text("Changes the project's name; its folder keeps its name");
        if rename.clicked() {
            self.launcher.renaming = Some(super::Renaming {
                root: root.clone(),
                name: project.name.clone(),
                focused: false,
            });
        }
        let copy = menu::item(ui, "Make a copy…", "").on_hover_text(
            "Copies the whole folder to an empty one you choose (leaving out the import cache, \
             which is made again)",
        );
        if copy.clicked() {
            asked.push(WorkbenchAction::CopyProject(root));
        }
    }
}

/// "Opened 2 hours ago", from "2 hours ago".
fn opened_words(ago: &str) -> String {
    match ago {
        "Just now" => "Opened just now".to_string(),
        "Yesterday" => "Opened yesterday".to_string(),
        other => format!("Opened {other}"),
    }
}

/// A folder as a card shows it: whole when short, otherwise its start,
/// "…" and its last two folders (`C:\…\Solis projects\Farm Days`).
pub(crate) fn short_folder(folder: &Path) -> String {
    let parts: Vec<Component<'_>> = folder.components().collect();
    if parts.len() <= 4 {
        return folder.display().to_string();
    }
    let separator = std::path::MAIN_SEPARATOR;
    let start: std::path::PathBuf = parts
        .iter()
        .take_while(|c| !matches!(c, Component::Normal(_)))
        .collect();
    let start = start.display().to_string();
    let start = start.trim_end_matches(separator);
    let tail: Vec<String> = parts[parts.len() - 2..]
        .iter()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let tail = tail.join(&separator.to_string());
    if folder.has_root() {
        format!("{start}{separator}…{separator}{tail}")
    } else {
        format!("…{separator}{tail}")
    }
}

/// The ⋯ at the right of a card's name: its menu opens from it.
fn more_button(ui: &mut Ui, name: &str) -> egui::Response {
    let (spot, response) = ui.allocate_exact_size(vec2(l::CARD_MENU, l::CARD_MENU), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(
            spot,
            CornerRadius::same(size::radius::SMALL),
            tokens::hover(),
        );
    }
    let ink = if response.hovered() {
        tokens::text_primary()
    } else {
        tokens::text_meta()
    };
    let inner = spot.shrink(3.0);
    crate::icons::icon(ui.painter(), inner, crate::icons::Icon::More, ink);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("More for {name}"))
    });
    response.on_hover_text("Open, show in Explorer, remove from the list and more")
}

/// Words drawn without a name of their own for screen readers: the card
/// itself is named after the project.
fn painted(ui: &mut Ui, words: &str, font: egui::FontId, ink: egui::Color32) {
    let width = widgets::width_of(ui, words, font.clone()).min(ui.available_width());
    let height = widgets::line_height(ui, &font);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    widgets::cut_off(ui, rect, words, font, ink);
}

/// The card's box: the panel's grey, its picture, its kind in the
/// picture's corner, and a border in the accent while it's picked.
fn draw_frame(ui: &Ui, rect: Rect, selected: bool, hovered: bool, project: &KnownProject) {
    let radius = size::radius::CARD;
    let fill = if hovered {
        tokens::card()
    } else {
        tokens::panel()
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(radius), fill);
    let picture = Rect::from_min_size(rect.min, vec2(rect.width(), l::CARD_PICTURE_H));
    let top = CornerRadius {
        nw: radius,
        ne: radius,
        sw: 0,
        se: 0,
    };
    let painter = ui
        .painter()
        .with_clip_rect(picture.intersect(ui.clip_rect()));
    pictures::kind_picture(&painter, picture, project.kind, top);
    if project.problem.is_some() {
        painter.rect_filled(
            picture,
            top,
            tokens::mix(tokens::viewport(), tokens::app(), 0.4),
        );
    }
    kind_badge(ui, picture, project.kind.badge());
    tokens::hline(
        ui.painter(),
        picture.x_range(),
        picture.bottom(),
        tokens::card_border(),
    );
    let edge = if selected {
        tokens::open_card_border(crate::interface_look::in_use().accent)
    } else {
        tokens::card_border()
    };
    widgets::border(ui, rect, radius, edge);
}

/// The kind's word in a small dark box in the picture's corner.
pub(crate) fn kind_badge(ui: &Ui, picture: Rect, word: &str) {
    let font = text::mono_small();
    let width = widgets::width_of(ui, word, font.clone()) + 2.0 * l::BADGE_PAD;
    let badge = Rect::from_min_size(
        picture.min + vec2(l::BADGE_INSET, l::BADGE_INSET),
        vec2(width, l::BADGE_H),
    );
    ui.painter().rect_filled(
        badge,
        CornerRadius::same(size::radius::SMALL),
        tokens::app(),
    );
    widgets::border(ui, badge, size::radius::SMALL, tokens::pill_border());
    widgets::words(
        ui,
        badge,
        Align2::CENTER_CENTER,
        word,
        font,
        tokens::text_primary(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_folder_is_shortened_to_its_last_two() {
        let sep = std::path::MAIN_SEPARATOR;
        let long: std::path::PathBuf = ["users", "sam", "Documents", "Solis projects", "Farm Days"]
            .iter()
            .collect();
        assert_eq!(
            short_folder(&long),
            format!("…{sep}Solis projects{sep}Farm Days")
        );
        let short: std::path::PathBuf = ["games", "Kart"].iter().collect();
        assert_eq!(short_folder(&short), short.display().to_string());
    }

    #[test]
    fn when_it_was_opened_reads_as_a_sentence() {
        assert_eq!(opened_words("2 hours ago"), "Opened 2 hours ago");
        assert_eq!(opened_words("Yesterday"), "Opened yesterday");
        assert_eq!(opened_words("Just now"), "Opened just now");
    }
}
