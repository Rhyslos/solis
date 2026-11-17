//! The launcher's Projects page (board 01): the heading with Open folder…
//! and New project; the search, the filter by kind and Sort by; every
//! project Solis knows as a card, pinned ones first. The four arrows walk
//! the cards and Enter opens the one picked, as on the page before it.

use super::{Filter, Sort, now, shown};
use crate::file_pages::{page_head, plain, sentence};
use crate::icons::Icon;
use crate::tokens::{self, launcher as l, size, text};
use crate::widgets::{self, button::Button, field, menu, segmented::segmented};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Layout, Ui};

/// What a key asked of the cards this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Walk {
    Left,
    Right,
    Up,
    Down,
}

impl Walk {
    fn pressed(ui: &Ui) -> Option<Self> {
        // Not while a field has the keys: there they move the caret.
        if ui.ctx().memory(|m| m.focused().is_some()) {
            return None;
        }
        ui.input(|i| {
            [
                (egui::Key::ArrowLeft, Self::Left),
                (egui::Key::ArrowRight, Self::Right),
                (egui::Key::ArrowUp, Self::Up),
                (egui::Key::ArrowDown, Self::Down),
            ]
            .into_iter()
            .find(|(key, _)| i.key_pressed(*key))
            .map(|(_, walk)| walk)
        })
    }
}

/// Where a walk from `at` (a place in a list `count` long, laid out
/// `across` to a row) lands. With nothing picked, any arrow picks the
/// first.
pub(crate) fn walk_to(at: Option<usize>, count: usize, across: usize, walk: Walk) -> Option<usize> {
    let last = count.checked_sub(1)?;
    let Some(at) = at else {
        return Some(0);
    };
    let across = across.max(1);
    Some(match walk {
        Walk::Left => at.saturating_sub(1),
        Walk::Right => (at + 1).min(last),
        Walk::Up => at.checked_sub(across).unwrap_or(at),
        Walk::Down => (at + across).min(last),
    })
}

/// How many cards fit across `width`, each at least the least width, with
/// the gap between them.
pub(crate) fn cards_across(width: f32) -> usize {
    let one = l::CARD_MIN_W + l::CARD_GAP;
    (((width + l::CARD_GAP) / one).floor() as usize).max(1)
}

impl Workbench {
    pub(super) fn projects_page(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        page_head(
            ui,
            "Projects",
            "Pick one to open it in the editor. This window closes as the editor opens.",
            |ui| {
                let new = Button::primary("New project")
                    .icon(Icon::Plus)
                    .header()
                    .show(ui)
                    .on_hover_text("Name it, choose 2D or 3D and what to start from");
                if new.clicked() {
                    self.start_new_project();
                }
                let open = Button::plain("Open folder…")
                    .icon(Icon::FolderOpen)
                    .header()
                    .show(ui)
                    .on_hover_text(
                        "Opens a project's folder. Any other folder is looked through for \
                         projects to add to the list",
                    );
                if open.clicked() {
                    asked.push(WorkbenchAction::OpenProject);
                }
            },
        );
        ui.add_space(l::TOOLS_GAP);
        self.projects_tools(ui);
        self.notice_strip(ui);
        ui.add_space(size::CARD_GAP);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| self.cards(ui, asked));
    }

    /// The search, the filter by kind, and Sort by at the right.
    fn projects_tools(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = size::GAP + size::GAP;
            let launcher = &mut self.launcher;
            field::search_field(
                ui,
                "Search projects",
                &mut launcher.search,
                "Search projects",
                l::SEARCH_W,
            );
            segmented(ui, "kind filter", &mut launcher.filter, &Filter::choices());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let sort = field::dropdown_button(ui, "Sort by", launcher.sort.name(), l::SORT_W);
                egui::Popup::menu(&sort).show(|ui| {
                    for choice in [Sort::LastOpened, Sort::Name] {
                        if menu::choice(ui, choice.name(), launcher.sort == choice).clicked() {
                            launcher.sort = choice;
                        }
                    }
                });
                plain(ui, "Sort by", text::row(), tokens::text_meta());
            });
        });
    }

    /// What the app last said (opened, copied, found, couldn't), until
    /// closed.
    fn notice_strip(&mut self, ui: &mut Ui) {
        let Some(notice) = self.launcher.notice.clone() else {
            return;
        };
        ui.add_space(size::CARD_GAP);
        widgets::card::frame(ui)
            .fill(tokens::changes_pill(self.accent))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    // A long note (a folder's whole path) wraps inside the
                    // card, beside its close button.
                    let room = (ui.available_width() - size::ICON_BUTTON - size::GAP).max(0.0);
                    ui.scope(|ui| {
                        ui.set_max_width(room);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&notice)
                                    .font(text::ui())
                                    .color(tokens::text_primary()),
                            )
                            .wrap()
                            .selectable(false),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::button::icon(ui, Icon::Close, "Close the note").clicked() {
                            self.launcher.notice = None;
                        }
                    });
                });
            });
    }

    /// Every project shown as a card, and what the arrows and Enter do.
    fn cards(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        if self.known_projects.is_empty() {
            plain(
                ui,
                "No projects yet",
                text::heading(),
                tokens::text_primary(),
            );
            sentence(
                ui,
                "Make one with New project, open a project's folder with Open folder…, or make \
                 an example on Learn.",
            );
            return;
        }
        let launcher = &self.launcher;
        let order = shown(
            &self.known_projects,
            &launcher.search,
            launcher.filter,
            launcher.sort,
        );
        if order.is_empty() {
            let line = if launcher.search.trim().is_empty() {
                "No project of that kind.".to_string()
            } else {
                format!(
                    "No project matches \u{201c}{}\u{201d}.",
                    launcher.search.trim()
                )
            };
            plain(ui, &line, text::ui(), tokens::text_meta());
            return;
        }
        let width = ui.available_width();
        let across = cards_across(width);
        let at = launcher.selected.as_ref().and_then(|s| {
            order
                .iter()
                .position(|&i| &self.known_projects[i].root == s)
        });
        if let Some(walk) = Walk::pressed(ui)
            && let Some(to) = walk_to(at, order.len(), across, walk)
        {
            self.launcher.selected = Some(self.known_projects[order[to]].root.clone());
        }
        let typing = self.launcher.renaming.is_some()
            || self.launcher.new_project.is_some()
            || ui.ctx().egui_wants_keyboard_input();
        if !typing
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
            && let Some(i) = at.map(|at| order[at])
            && self.known_projects[i].problem.is_none()
        {
            asked.push(WorkbenchAction::OpenRecent(
                self.known_projects[i].root.clone(),
            ));
        }
        let gap = l::CARD_GAP;
        let card_w = (width - gap * (across as f32 - 1.0)) / across as f32;
        let now = now();
        let mut changes = Vec::new();
        for row in order.chunks(across) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &i in row {
                    self.project_card(ui, i, card_w, now, asked, &mut changes);
                }
            });
            ui.add_space(gap - ui.spacing().item_spacing.y);
        }
        for change in changes {
            match &change {
                WorkbenchAction::PinProject(root, pinned) => {
                    if let Some(p) = self.known_projects.iter_mut().find(|p| &p.root == root) {
                        p.pinned = *pinned;
                    }
                }
                WorkbenchAction::ForgetProject(root) => {
                    self.known_projects.retain(|p| &p.root != root);
                }
                _ => {}
            }
            asked.push(change);
        }
    }
}

impl Workbench {
    /// The Rename… question, while it's asked: Enter or Rename gives the
    /// project its new name, Esc or Cancel leaves it.
    pub(super) fn rename_question(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let Some(renaming) = &mut self.launcher.renaming else {
            return;
        };
        let mut done = None;
        egui::Modal::new(egui::Id::new("rename project")).show(ui.ctx(), |ui| {
            ui.set_width(size::UNBUILT_CARD_W - 20.0);
            ui.spacing_mut().item_spacing.y = size::CARD_INNER_GAP;
            plain(
                ui,
                "Rename the project",
                text::heading(),
                tokens::text_primary(),
            );
            sentence(
                ui,
                "Its name in the title bar and the list. Its folder keeps its name.",
            );
            let field =
                field::text_field(ui, "New name", &mut renaming.name, "", ui.available_width());
            if !renaming.focused {
                field.request_focus();
                renaming.focused = true;
            }
            let why = crate::file_pages::name_problem(&renaming.name);
            if let Some(why) = &why {
                plain(ui, why, text::row(), tokens::WARNING);
            }
            let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let ok =
                        ui.add_enabled_ui(why.is_none(), |ui| Button::primary("Rename").show(ui));
                    if (ok.inner.clicked() || enter) && why.is_none() {
                        done = Some(true);
                    }
                    if Button::plain("Cancel").show(ui).clicked() {
                        done = Some(false);
                    }
                });
            });
        });
        if done.is_none() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            done = Some(false);
        }
        match done {
            Some(true) => {
                if let Some(renaming) = self.launcher.renaming.take() {
                    asked.push(WorkbenchAction::RenameProject(
                        renaming.root,
                        renaming.name.trim().to_string(),
                    ));
                }
            }
            Some(false) => self.launcher.renaming = None,
            None => {}
        }
    }
}
