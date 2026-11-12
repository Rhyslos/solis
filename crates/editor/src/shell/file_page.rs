//! The File tab (specification, Screen 7): a page in place of the
//! ribbon and the editor, with a column of items down its left side.
//!
//! Recent projects, Learn, Updates and About are pages of their own
//! (`file_pages`); Solis starts on Recent projects. Until a project is
//! open, Back to editor reads "Carry on without a project".

use super::{RibbonTab, ShellCommand};
use crate::file_pages::Page;
use crate::icons::{self, Icon};
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::widgets::{self, words};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align2, CornerRadius, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};

/// What an item of the File page's column does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileItem {
    /// Leaves File for the tab that was in front.
    Back,
    /// Shows one of the pages in the page area.
    Show(Page),
    /// Does a command.
    Do(ShellCommand),
}

/// An item as the column draws it.
type Row = (Icon, &'static str, FileItem);

/// The column's items, in the design's three blocks, with the
/// unfinished ones in their places.
pub(crate) fn blocks() -> Vec<((), Vec<Entry<Row>>)> {
    use ShellCommand as C;
    let item = |icon, label, what| Entry::Built((icon, label, what));
    unbuilt::filter_groups(vec![
        (
            (),
            vec![
                item(Icon::ChevronLeft, "Back to editor", FileItem::Back),
                item(
                    Icon::Clock,
                    "Recent projects",
                    FileItem::Show(Page::Projects),
                ),
                item(Icon::Plus, "New project", FileItem::Do(C::NewProject)),
                item(Icon::FolderOpen, "Open…", FileItem::Do(C::OpenProject)),
                item(Icon::Save, "Save all", FileItem::Do(C::SaveArea)),
            ],
        ),
        (
            (),
            vec![
                item(
                    Icon::Gear,
                    "Project settings",
                    FileItem::Show(Page::ProjectSettings),
                ),
                item(
                    Icon::Sliders,
                    "Preferences",
                    FileItem::Show(Page::Preferences),
                ),
                Entry::Unbuilt(UnbuiltId::FilePlugins),
            ],
        ),
        (
            (),
            vec![
                item(Icon::Book, "Learn", FileItem::Show(Page::Learn)),
                item(Icon::Refresh, "Updates", FileItem::Show(Page::Updates)),
                item(Icon::Info, "About", FileItem::Show(Page::About)),
                item(Icon::Exit, "Exit", FileItem::Do(C::Exit)),
            ],
        ),
    ])
}

impl Workbench {
    /// Brings the File tab to the front on `page`. The tab that was in
    /// front is remembered, for Back to editor.
    pub(crate) fn show_file_page(&mut self, page: Page) {
        if self.shell.tab != RibbonTab::File {
            self.shell.before_file = self.shell.tab;
            // The projects are read afresh each time File opens.
            self.open.outbox.push(WorkbenchAction::ListProjects);
            self.file_pages.focus_search = true;
        }
        self.shell.tab = RibbonTab::File;
        self.shell.dropped = false;
        self.file_pages.page = page;
    }

    /// Leaves the File tab for the tab that was in front before it.
    pub(crate) fn back_to_editor(&mut self) {
        if self.shell.tab == RibbonTab::File {
            self.shell.tab = self.shell.before_file;
        }
    }

    /// Draws the File page under the tab bar: the column, and the page
    /// in front beside it.
    pub(crate) fn file_page(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        // Esc leaves File, as Back to editor does; while a project is
        // being renamed it only calls that off.
        let ask_first = self.open.unsaved_prompt.is_some() || self.open.play_changes.is_some();
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) && !ask_first {
            if self.file_pages.renaming.is_some() {
                self.file_pages.renaming = None;
            } else if !self.search.open {
                self.back_to_editor();
                return;
            }
        }
        let [pad_y, pad_x] = size::FILE_MENU_PAD;
        let mut clicked = None;
        egui::Panel::left("file menu")
            .exact_size(size::FILE_MENU_W)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(tokens::ribbon())
                    .inner_margin(egui::Margin::symmetric(pad_x as i8, pad_y as i8)),
            )
            .show(ui, |ui| {
                clicked = self.file_column(ui);
                let whole = ui.max_rect().expand2(vec2(pad_x, pad_y));
                tokens::vline(
                    ui.painter(),
                    whole.right() - size::LINE,
                    whole.y_range(),
                    tokens::line(),
                );
            });
        if self.file_page_body(ui, actions) {
            self.back_to_editor();
        }
        match clicked {
            Some(FileItem::Back) => self.back_to_editor(),
            Some(FileItem::Show(page)) => self.show_file_page(page),
            Some(FileItem::Do(command)) => command.run(self, ui.ctx(), actions),
            None => {}
        }
    }

    /// The column's items. Returns the one clicked.
    fn file_column(&self, ui: &mut Ui) -> Option<FileItem> {
        ui.spacing_mut().item_spacing = vec2(0.0, size::FILE_MENU_GAP);
        let mut clicked = None;
        for (n, (_, rows)) in blocks().into_iter().enumerate() {
            if n > 0 {
                block_line(ui);
            }
            for row in rows {
                let (icon, label, what) = match row {
                    Entry::Unbuilt(part) => {
                        unbuilt::menu_item(ui, part, Icon::Plugin);
                        continue;
                    }
                    Entry::Built(row) => row,
                };
                let front = what == FileItem::Show(self.file_pages.page);
                let (why_not, hover) = match what {
                    FileItem::Do(command) => (command.why_not(self), command.hover_text(self)),
                    FileItem::Back if self.open.project.is_none() => {
                        (None, "The editor, with no project open (Esc)".to_string())
                    }
                    FileItem::Back => (
                        None,
                        format!("Back to {} (Esc)", self.shell.before_file.name()),
                    ),
                    FileItem::Show(_) => (None, String::new()),
                };
                let label = match what {
                    FileItem::Back if self.open.project.is_none() => "Carry on without a project",
                    _ => label,
                };
                let response = file_item(ui, icon, label, front, why_not.is_none());
                let response = if hover.is_empty() {
                    response
                } else {
                    response.on_hover_text(hover)
                };
                if response.clicked() && why_not.is_none() {
                    clicked = Some(what);
                }
                if what == FileItem::Show(Page::Updates) && self.update.newer_build().is_some() {
                    new_badge(ui, response.rect);
                }
                if what == FileItem::Back {
                    ui.add_space(size::FILE_MENU_BACK_GAP);
                }
            }
        }
        clicked
    }
}

/// The small "New" at the right of Updates while a newer build is out.
fn new_badge(ui: &Ui, item: Rect) {
    let font = text::ribbon_small();
    let width = widgets::width_of(ui, "New", font.clone()) + 2.0 * size::GAP;
    let badge = Rect::from_center_size(
        pos2(
            item.right() - size::FILE_MENU_ITEM_PAD - width / 2.0,
            item.center().y,
        ),
        vec2(width, size::STATUS_DOT * 3.0),
    );
    ui.painter().rect_filled(
        badge,
        CornerRadius::same(size::radius::SMALL),
        crate::interface_look::accent_fill(),
    );
    words(
        ui,
        badge,
        Align2::CENTER_CENTER,
        "New",
        font,
        tokens::ON_ACCENT,
    );
}

/// The line between two blocks of the column.
fn block_line(ui: &mut Ui) {
    let [pad_y, pad_x] = size::FILE_MENU_LINE_PAD;
    let wanted = vec2(ui.available_width(), 2.0 * pad_y + size::LINE);
    let (rect, _) = ui.allocate_exact_size(wanted, Sense::hover());
    tokens::hline(
        ui.painter(),
        rect.x_range().shrink(pad_x),
        rect.center().y,
        tokens::line(),
    );
}

/// One item of the column: its icon and its words, on a lighter fill
/// under the pointer and while its page is the one in front.
fn file_item(ui: &mut Ui, icon: Icon, label: &str, front: bool, enabled: bool) -> Response {
    let wanted = vec2(ui.available_width(), size::FILE_MENU_ITEM_H);
    let (rect, response) = ui.allocate_exact_size(wanted, Sense::click());
    response.widget_info(|| WidgetInfo::selected(egui::WidgetType::Button, enabled, front, label));
    if ui.is_rect_visible(rect) {
        let lit = enabled && (front || response.hovered());
        if lit {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(size::radius::MENU),
                tokens::group_gap(),
            );
        }
        let ink = if !enabled {
            tokens::text_faint()
        } else if front {
            tokens::text_emphasis()
        } else if response.hovered() {
            tokens::text_primary()
        } else if icon == Icon::ChevronLeft {
            // Back to editor is a shade brighter than the items under it.
            tokens::text_body()
        } else {
            tokens::text_label()
        };
        let spot = Rect::from_min_size(
            pos2(
                rect.left() + size::FILE_MENU_ITEM_PAD,
                rect.center().y - size::TOOL_ICON / 2.0,
            ),
            vec2(size::TOOL_ICON, size::TOOL_ICON),
        );
        icons::icon(ui.painter(), spot, icon, ink);
        let room = Rect::from_min_max(
            pos2(spot.right() + size::FILE_MENU_ITEM_GAP, rect.top()),
            rect.right_bottom(),
        );
        words(ui, room, Align2::LEFT_CENTER, label, text::menu_item(), ink);
        widgets::focus_ring(ui, &response, rect, size::radius::MENU);
    }
    response
}
