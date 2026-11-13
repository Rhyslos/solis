//! File › Recent projects (Screen 7; round 2's `09-recent-projects-a`):
//! the heading with the search, the order, Open… and New project; every
//! project Solis knows as a card, pinned ones first; and the strip of
//! templates to start from.

use super::{FilePages, Renaming, Sort, examples, name_problem, now, page_head, plain, shown};
use crate::icons::{self, Icon};
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::widgets::{self, button::Button, field, menu};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Align2, CornerRadius, Layout, Rect, Sense, Ui, vec2};

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
        // Not while the search has the keys: there they move the caret.
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
/// `across` to a row) lands: all four arrows walk the cards (answer
/// 25). With nothing picked, any arrow picks the first.
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

/// How many cards fit across `width`, each at least the design's least
/// width, with the design's gap between them.
pub(crate) fn cards_across(width: f32) -> usize {
    let one = size::PROJECT_CARD_MIN_W + size::PROJECT_CARD_GAP;
    (((width + size::PROJECT_CARD_GAP) / one).floor() as usize).max(1)
}

impl Workbench {
    pub(super) fn projects_page(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let subtitle = if self.open.project.is_some() {
            "Pick up where you left off, or start from a template."
        } else {
            "Pick a project to work on, or start from a template."
        };
        page_head(ui, "Recent projects", subtitle, |ui| {
            self.projects_tools(ui, asked);
        });
        self.notice_strip(ui);
        ui.add_space(size::PAGE_BLOCK_GAP);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.cards(ui, asked);
                ui.add_space(size::PAGE_BLOCK_GAP);
                templates(ui, asked);
            });
    }

    /// The header's tools, laid out from the right: New project, Open…,
    /// Sort by and the search.
    fn projects_tools(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let new = Button::primary("New project")
            .icon(Icon::Plus)
            .header()
            .show(ui)
            .on_hover_text("Makes a folder into a project for your game's files");
        if new.clicked() {
            asked.push(WorkbenchAction::NewProject);
        }
        let open = Button::plain("Open… ▾").header().show(ui);
        egui::Popup::menu(&open).show(|ui| {
            if menu::item(ui, "Open a project folder…", "").clicked() {
                asked.push(WorkbenchAction::OpenProject);
            }
            let scan = menu::item(ui, "Scan a folder for projects…", "").on_hover_text(
                "Looks through a folder, and the folders in it, for projects to add to the list",
            );
            if scan.clicked() {
                asked.push(WorkbenchAction::ScanForProjects);
            }
        });
        let pages = &mut self.file_pages;
        let shown_sort = format!("Sort by {}", pages.sort.name());
        let width =
            widgets::width_of(ui, &shown_sort, text::ribbon_small()) + 4.0 * size::FIELD_PAD;
        let sort = field::dropdown_button(ui, "Sort by", &shown_sort, width);
        egui::Popup::menu(&sort).show(|ui| {
            for choice in [Sort::LastOpened, Sort::Name] {
                if menu::choice(ui, choice.name(), pages.sort == choice).clicked() {
                    pages.sort = choice;
                }
            }
        });
        let search = field::search_field(
            ui,
            "Search projects",
            &mut pages.search,
            "Search projects",
            size::PAGE_SEARCH_W,
        );
        if std::mem::take(&mut pages.focus_search) && self.open.project.is_none() {
            search.request_focus();
        }
    }

    /// What the app last said (renamed, copied, found), until closed.
    fn notice_strip(&mut self, ui: &mut Ui) {
        let Some(notice) = self.file_pages.notice.clone() else {
            return;
        };
        ui.add_space(size::CARD_GAP);
        widgets::card::frame(ui)
            .fill(tokens::changes_pill(self.accent))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    plain(ui, &notice, text::ui(), tokens::text_primary());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::button::icon(ui, Icon::Close, "Close the note").clicked() {
                            self.file_pages.notice = None;
                        }
                    });
                });
            });
    }

    /// Every project as a card, pinned ones first, and what the arrows
    /// and Enter do to them.
    fn cards(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        if self.known_projects.is_empty() {
            plain(
                ui,
                "No projects yet",
                text::heading(),
                tokens::text_primary(),
            );
            super::sentence(
                ui,
                "Make one with New project, open a folder with Open…, or start from a template \
                 below.",
            );
            return;
        }
        let pages = &self.file_pages;
        let (pinned, rest) = shown(&self.known_projects, &pages.search, pages.sort);
        let order: Vec<usize> = pinned.iter().chain(&rest).copied().collect();
        if order.is_empty() {
            let line = format!("No project matches \u{201c}{}\u{201d}.", pages.search);
            plain(ui, &line, text::ui(), tokens::text_meta());
            return;
        }
        let width = ui.available_width();
        let across = cards_across(width);
        let at = pages.selected.as_ref().and_then(|s| {
            order
                .iter()
                .position(|&i| &self.known_projects[i].root == s)
        });
        if let Some(walk) = Walk::pressed(ui)
            && let Some(to) = walk_to(at, order.len(), across, walk)
        {
            self.file_pages.selected = Some(self.known_projects[order[to]].root.clone());
        }
        // Not while a name is typed: Enter there is the field's (Rename…
        // keeps the name with it, and opened the project as well: the
        // second review pass's N-05).
        let typing = self.file_pages.renaming.is_some() || ui.ctx().egui_wants_keyboard_input();
        if !typing
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
            && let Some(i) = at.map(|at| order[at])
            && self.known_projects[i].problem.is_none()
        {
            asked.push(WorkbenchAction::OpenRecent(
                self.known_projects[i].root.clone(),
            ));
        }
        let gap = size::PROJECT_CARD_GAP;
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

    /// The Rename… question, while it's asked: Enter or Rename gives the
    /// project its new name, Esc or Cancel leaves it.
    pub(super) fn rename_question(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let Some(renaming) = &mut self.file_pages.renaming else {
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
            super::sentence(
                ui,
                "Its name in the title bar and the list. Its folder keeps its name.",
            );
            let field =
                field::text_field(ui, "New name", &mut renaming.name, "", ui.available_width());
            if !renaming.focused {
                field.request_focus();
                renaming.focused = true;
            }
            let why = name_problem(&renaming.name);
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
        match done {
            Some(true) => {
                if let Some(renaming) = self.file_pages.renaming.take() {
                    asked.push(WorkbenchAction::RenameProject(
                        renaming.root,
                        renaming.name.trim().to_string(),
                    ));
                }
            }
            Some(false) => self.file_pages.renaming = None,
            None => {}
        }
    }
}

impl FilePages {
    /// Starts asking for `root`'s new name, beginning from `name`.
    pub(super) fn rename(&mut self, root: std::path::PathBuf, name: String) {
        self.renaming = Some(Renaming {
            root,
            name,
            focused: false,
        });
    }
}

/// The templates: a blank project and Solis's four examples (real), then
/// the design's own as unfinished parts (answer 22; no VR / XR).
fn templates(ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
    ui.horizontal(|ui| {
        plain(
            ui,
            "Start from a template",
            text::heading(),
            tokens::text_body(),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            for entry in unbuilt::filter(vec![Entry::<()>::Unbuilt(UnbuiltId::RecentTemplateAll)]) {
                if let Entry::Unbuilt(id) = entry {
                    unbuilt::link(ui, id);
                }
            }
        });
    });
    ui.add_space(size::CARD_INNER_GAP);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(size::TEMPLATE_GAP, size::TEMPLATE_GAP);
        let blank = template(ui, "Blank project", Icon::CubeOutline, true)
            .on_hover_text("An empty project, in a folder you choose");
        if blank.clicked() {
            asked.push(WorkbenchAction::NewProject);
        }
        for example in examples() {
            let tile = template(ui, example.name, example.icon, false).on_hover_text(format!(
                "{}. Made as a new project, in a folder you choose",
                example.about
            ));
            if tile.clicked() {
                asked.push(example.make);
            }
        }
        let later = unbuilt::filter(vec![
            Entry::<()>::Unbuilt(UnbuiltId::RecentTemplateFirstPerson),
            Entry::Unbuilt(UnbuiltId::RecentTemplateTopDown),
            Entry::Unbuilt(UnbuiltId::RecentTemplatePlatformer),
        ]);
        for (entry, icon) in later
            .into_iter()
            .zip([Icon::Camera, Icon::FourSquares, Icon::Lines])
        {
            if let Entry::Unbuilt(id) = entry {
                unbuilt::template(ui, id, icon);
            }
        }
    });
}

/// A template's tile: its icon in a box (dashed for the blank project),
/// its name under it.
fn template(ui: &mut Ui, name: &str, icon: Icon, dashed: bool) -> egui::Response {
    // Named as a template, so it isn't taken for a project of that name.
    let said = if dashed {
        name.to_string()
    } else {
        format!("{name} template")
    };
    let height = size::TEMPLATE_BOX_H
        + size::TEMPLATE_NAME_GAP
        + widgets::line_height(ui, &text::template_name());
    let (rect, response) = ui.allocate_exact_size(vec2(size::TEMPLATE_W, height), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &said));
    if ui.is_rect_visible(rect) {
        let tile = Rect::from_min_size(rect.min, vec2(size::TEMPLATE_W, size::TEMPLATE_BOX_H));
        let fill = if response.hovered() {
            tokens::hover()
        } else {
            tokens::panel()
        };
        let radius = size::radius::CARD;
        ui.painter()
            .rect_filled(tile, CornerRadius::same(radius), fill);
        if dashed {
            widgets::dashed::rounded(ui, tile, radius, tokens::field_border());
        } else {
            widgets::border(ui, tile, radius, tokens::field_border());
        }
        let spot = Rect::from_center_size(
            tile.center(),
            vec2(size::RIBBON_BIG_ICON, size::RIBBON_BIG_ICON),
        );
        icons::icon(ui.painter(), spot, icon, tokens::text_label());
        let words = Rect::from_min_max(
            egui::pos2(rect.left(), tile.bottom() + size::TEMPLATE_NAME_GAP),
            rect.right_bottom(),
        );
        widgets::words(
            ui,
            words,
            Align2::LEFT_TOP,
            name,
            text::template_name(),
            tokens::text_primary(),
        );
        widgets::focus_ring(ui, &response, tile, radius);
    }
    response
}
