//! One search for everything (the editor overhaul's O4, decision 0085): the
//! box in the top bar, or **Space** with nothing being typed, opens a list
//! that finds every action in the menus (with its shortcut beside it, so
//! the list teaches the keys), every file in the project, every entity in
//! the area and every panel. Type to narrow it, Up and Down to choose,
//! Enter to do it, Esc to close.

use crate::library::{LibraryKind, LibraryRequest};
use crate::menu_items::MenuCommand;
use crate::panel::Panel;
use crate::play::PlayState;
use crate::theme;
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{RichText, Ui};
use reflect::EntityId;

/// The most results shown at once.
pub(crate) const MAX_SHOWN: usize = 50;
/// The list's width, in points.
const WIDTH: f32 = 560.0;

/// What a result is, which decides its group and its little label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SearchKind {
    /// Something to do: a menu item.
    Action,
    /// A panel to bring to the front.
    Panel,
    /// An entity in the area.
    Entity,
    /// A file in the project.
    File,
}

impl SearchKind {
    /// Its little label on the right of a result.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Action => "action",
            Self::Panel => "panel",
            Self::Entity => "entity",
            Self::File => "file",
        }
    }
}

/// What picking a result does.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SearchTarget {
    /// A menu command (Undo, Add point light…).
    Menu(MenuCommand),
    /// Something the app does (New project, Save area…).
    App(WorkbenchAction),
    /// Something else the editor does.
    Do(Doing),
    /// Bring a panel to the front (opening it if it's closed).
    Panel(Panel),
    /// Select an entity and point the camera at it.
    Entity(EntityId),
    /// Open a file (or, for one that's placed rather than opened, show it
    /// in the Library): its kind and its path inside the assets.
    File(LibraryKind, String),
}

/// The editor's own actions the search can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Doing {
    Launcher,
    Settings,
    InputActions,
    Play,
    Pause,
    Step,
    Stop,
    PlayAndRecord,
    ResetLayout,
    PinLibrary,
    UnpinLibrary,
    Workspace(crate::workspace::Workspace),
    About,
}

/// One result.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SearchItem {
    /// What it's called.
    pub(crate) name: String,
    /// What it is.
    pub(crate) kind: SearchKind,
    /// More, greyed after the name: an action's menu, a file's folder.
    pub(crate) detail: String,
    /// An action's shortcut, if it has one.
    pub(crate) keys: &'static str,
    /// What picking it does.
    pub(crate) target: SearchTarget,
}

impl SearchItem {
    fn action(
        name: impl Into<String>,
        menu: &str,
        keys: &'static str,
        target: SearchTarget,
    ) -> Self {
        Self {
            name: name.into(),
            kind: SearchKind::Action,
            detail: menu.to_string(),
            keys,
            target,
        }
    }
}

/// The search's state: whether it's open, what's typed, and which result
/// is chosen.
#[derive(Debug, Clone, Default)]
pub(crate) struct SearchState {
    /// Whether the list is open.
    pub(crate) open: bool,
    /// What's typed.
    pub(crate) query: String,
    /// The chosen result, counted in the list shown.
    pub(crate) chosen: usize,
    /// Set on the frame it opens, to put the cursor in its box.
    focus: bool,
}

/// How well `query` matches `text`, or none: each word of the query must
/// be in the text. A word at the start of one of the text's words counts
/// most, anywhere in it less, and its letters in order (`svar` for "Save
/// area") least.
pub(crate) fn score(query: &str, text: &str) -> Option<u32> {
    let text = text.to_lowercase();
    let starts: Vec<usize> = std::iter::once(0)
        .chain(
            text.char_indices()
                .filter(|(_, c)| !c.is_alphanumeric())
                .map(|(i, c)| i + c.len_utf8()),
        )
        .collect();
    let mut total = 0;
    for word in query.to_lowercase().split_whitespace() {
        total += if starts.iter().any(|&s| text[s..].starts_with(word)) {
            3
        } else if text.contains(word) {
            2
        } else if in_order(word, &text) {
            1
        } else {
            return None;
        };
    }
    Some(total)
}

/// Whether `word`'s letters appear in `text` in order.
fn in_order(word: &str, text: &str) -> bool {
    let mut letters = text.chars();
    word.chars().all(|w| letters.any(|t| t == w))
}

/// The results for `query` among `items`: the best first, then actions
/// before panels, entities and files, then the shortest name; at most
/// [`MAX_SHOWN`]. With nothing typed, every action, in menu order.
pub(crate) fn results<'a>(items: &'a [SearchItem], query: &str) -> Vec<&'a SearchItem> {
    if query.trim().is_empty() {
        return items
            .iter()
            .filter(|i| i.kind == SearchKind::Action)
            .take(MAX_SHOWN)
            .collect();
    }
    let mut found: Vec<(u32, &SearchItem)> = items
        .iter()
        .filter_map(|i| {
            let by_name = score(query, &i.name);
            let by_detail = score(query, &format!("{} {}", i.name, i.detail)).map(|s| s / 2);
            by_name.max(by_detail).map(|s| (s, i))
        })
        .collect();
    found.sort_by(|(a, x), (b, y)| {
        b.cmp(a)
            .then(x.kind.cmp(&y.kind))
            .then(x.name.len().cmp(&y.name.len()))
    });
    found.into_iter().take(MAX_SHOWN).map(|(_, i)| i).collect()
}

impl Workbench {
    /// Everything the search can find now: the actions that can be done
    /// (an action that can't, such as Pause while editing, is left out),
    /// the panels, the area's entities and the project's files.
    pub(crate) fn search_items(&self) -> Vec<SearchItem> {
        use SearchTarget as T;
        let mut items = Vec::new();
        let state = crate::menu_items::MenuState::of(self);
        let playing = self.play_state();
        let project = self.project.is_some();
        let mut push = |ok: bool, item: SearchItem| {
            if ok {
                items.push(item);
            }
        };
        // File.
        push(
            true,
            SearchItem::action(
                "New project…",
                "File",
                "",
                T::App(WorkbenchAction::NewProject),
            ),
        );
        push(
            true,
            SearchItem::action(
                "Open project…",
                "File",
                "",
                T::App(WorkbenchAction::OpenProject),
            ),
        );
        push(
            true,
            SearchItem::action(
                "Projects (the launcher)",
                "File",
                "",
                T::Do(Doing::Launcher),
            ),
        );
        push(
            true,
            SearchItem::action(
                "New area",
                "File",
                "Ctrl+N",
                T::App(WorkbenchAction::NewArea),
            ),
        );
        push(
            true,
            SearchItem::action(
                "Open area…",
                "File",
                "Ctrl+O",
                T::App(WorkbenchAction::OpenArea),
            ),
        );
        push(
            true,
            SearchItem::action(
                "Save area",
                "File",
                "Ctrl+S",
                T::App(WorkbenchAction::SaveArea),
            ),
        );
        push(
            true,
            SearchItem::action(
                "Save area as…",
                "File",
                "Ctrl+Shift+S",
                T::App(WorkbenchAction::SaveAreaAs),
            ),
        );
        push(
            true,
            SearchItem::action(
                "Import model…",
                "File",
                "",
                T::App(WorkbenchAction::ImportModel),
            ),
        );
        // Edit.
        if let Some(undo) = &state.undo {
            push(
                true,
                SearchItem::action(undo.clone(), "Edit", "Ctrl+Z", T::Menu(MenuCommand::Undo)),
            );
        }
        if let Some(redo) = &state.redo {
            push(
                true,
                SearchItem::action(redo.clone(), "Edit", "Ctrl+Y", T::Menu(MenuCommand::Redo)),
            );
        }
        let s = state.selection;
        push(
            s,
            SearchItem::action(
                "Duplicate",
                "Edit",
                "Ctrl+D",
                T::Menu(MenuCommand::Duplicate),
            ),
        );
        push(
            s,
            SearchItem::action("Delete", "Edit", "Delete", T::Menu(MenuCommand::Delete)),
        );
        push(
            s,
            SearchItem::action("Rename", "Edit", "F2", T::Menu(MenuCommand::Rename)),
        );
        push(
            true,
            SearchItem::action(
                "Select all",
                "Edit",
                "Ctrl+A",
                T::Menu(MenuCommand::SelectAll),
            ),
        );
        push(
            s,
            SearchItem::action(
                "Select nothing",
                "Edit",
                "Esc",
                T::Menu(MenuCommand::SelectNone),
            ),
        );
        push(
            true,
            SearchItem::action("Input actions…", "Edit", "", T::Do(Doing::InputActions)),
        );
        push(
            true,
            SearchItem::action("Settings…", "Edit", "", T::Do(Doing::Settings)),
        );
        // Entity.
        push(
            true,
            SearchItem::action("Add entity", "Entity", "", T::Menu(MenuCommand::AddEntity)),
        );
        push(
            true,
            SearchItem::action(
                "Add point light",
                "Entity",
                "",
                T::Menu(MenuCommand::AddPointLight),
            ),
        );
        push(
            true,
            SearchItem::action(
                "Add spotlight",
                "Entity",
                "",
                T::Menu(MenuCommand::AddSpotlight),
            ),
        );
        push(
            s,
            SearchItem::action(
                "Create assembly from selection",
                "Entity",
                "",
                T::Menu(MenuCommand::CreateAssembly),
            ),
        );
        push(
            true,
            SearchItem::action("Add camera", "Entity", "", T::Menu(MenuCommand::AddCamera)),
        );
        // Play.
        let editing = playing == PlayState::Editing;
        push(
            playing != PlayState::Playing,
            SearchItem::action("Play", "Play", "", T::Do(Doing::Play)),
        );
        push(
            !editing,
            SearchItem::action("Pause", "Play", "", T::Do(Doing::Pause)),
        );
        push(
            playing == PlayState::Paused,
            SearchItem::action("Step one tick", "Play", "", T::Do(Doing::Step)),
        );
        push(
            !editing,
            SearchItem::action("Stop", "Play", "", T::Do(Doing::Stop)),
        );
        push(
            true,
            SearchItem::action(
                "Load a save…",
                "Play",
                "",
                T::App(WorkbenchAction::OpenSave),
            ),
        );
        push(
            editing,
            SearchItem::action(
                "Play and record inputs",
                "Play",
                "",
                T::Do(Doing::PlayAndRecord),
            ),
        );
        push(
            true,
            SearchItem::action(
                "Replay a recording…",
                "Play",
                "",
                T::App(WorkbenchAction::OpenRecording),
            ),
        );
        // Window.
        push(
            true,
            SearchItem::action("Reset layout", "Window", "", T::Do(Doing::ResetLayout)),
        );
        for workspace in crate::workspace::Workspace::ALL {
            push(
                workspace != self.workspace,
                SearchItem::action(
                    format!("{} workspace", workspace.name()),
                    "Workspaces",
                    "",
                    T::Do(Doing::Workspace(workspace)),
                ),
            );
        }
        push(
            !self.library_pinned,
            SearchItem::action("Pin the Library", "Window", "", T::Do(Doing::PinLibrary)),
        );
        push(
            self.library_pinned,
            SearchItem::action(
                "Unpin the Library",
                "Window",
                "",
                T::Do(Doing::UnpinLibrary),
            ),
        );
        // Help.
        push(
            true,
            SearchItem::action(
                "Add the sample area",
                "Help",
                "",
                T::App(WorkbenchAction::AddSampleArea),
            ),
        );
        push(
            true,
            SearchItem::action(
                "Make the sample game…",
                "Help",
                "",
                T::App(WorkbenchAction::MakeSampleGame),
            ),
        );
        push(
            true,
            SearchItem::action(
                "Make the UI sample…",
                "Help",
                "",
                T::App(WorkbenchAction::MakeUiSample),
            ),
        );
        push(
            true,
            SearchItem::action(
                "Make the benchmark…",
                "Help",
                "",
                T::App(WorkbenchAction::MakeBenchmark),
            ),
        );
        push(
            true,
            SearchItem::action(
                "Make the playground…",
                "Help",
                "",
                T::App(WorkbenchAction::MakePlayground),
            ),
        );
        push(
            true,
            SearchItem::action(
                format!("About {}", self.info.name),
                "Help",
                "",
                T::Do(Doing::About),
            ),
        );
        // Panels: those the layout has (a document's tab only while open).
        for panel in [
            Panel::Viewport,
            Panel::Game,
            Panel::Area,
            Panel::Properties,
            Panel::Library,
            Panel::Code,
            Panel::Console,
            Panel::Problems,
            Panel::Profiler,
            Panel::Mixer,
        ] {
            let there = self.dock.find_tab(&panel).is_some()
                || (panel == Panel::Library && !self.library_pinned);
            if there {
                items.push(SearchItem {
                    name: panel.title().to_string(),
                    kind: SearchKind::Panel,
                    detail: String::new(),
                    keys: if panel == Panel::Library {
                        "Ctrl+Space"
                    } else {
                        ""
                    },
                    target: T::Panel(panel),
                });
            }
        }
        // The area's entities.
        for (id, entity) in self.document.area().iter() {
            items.push(SearchItem {
                name: entity.name.clone(),
                kind: SearchKind::Entity,
                detail: String::new(),
                keys: "",
                target: T::Entity(id),
            });
        }
        // The project's files.
        if project {
            for item in &self.library {
                items.push(SearchItem {
                    name: item.name.clone(),
                    kind: SearchKind::File,
                    detail: item.folder().to_string(),
                    keys: "",
                    target: T::File(item.kind, item.path.clone()),
                });
            }
        }
        items
    }

    /// Opens the search.
    pub(crate) fn open_search(&mut self) {
        self.search = SearchState {
            open: true,
            query: String::new(),
            chosen: 0,
            focus: true,
        };
    }

    /// Space, with nothing being typed, the pointer not over the Frames
    /// tab (where Space plays the clip) and the game not holding the keys,
    /// opens the search.
    pub(crate) fn search_key(&mut self, ui: &Ui) {
        if self.search.open || self.frames_hovered || ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let space = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space));
        if space {
            self.open_search();
        }
    }

    /// Does what a result is for.
    pub(crate) fn do_search(
        &mut self,
        target: SearchTarget,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        match target {
            SearchTarget::Menu(command) => self.run(command, ctx),
            SearchTarget::App(action) => self.request(action, actions),
            SearchTarget::Do(doing) => match doing {
                Doing::Launcher => self.open_launcher(),
                Doing::Settings => self.open_settings(),
                Doing::InputActions => self.open_input_window(),
                Doing::Play => self.play(),
                Doing::Pause => self.pause(),
                Doing::Step => self.step(),
                Doing::Stop => self.stop(),
                Doing::PlayAndRecord => self.play_and_record(),
                Doing::ResetLayout => self.reset_layout(),
                Doing::PinLibrary => self.set_library_pinned(true),
                Doing::UnpinLibrary => self.set_library_pinned(false),
                Doing::Workspace(workspace) => self.switch_workspace(workspace),
                Doing::About => self.about_open = true,
            },
            SearchTarget::Panel(Panel::Library) if !self.library_pinned => {
                self.drawer_open = true;
            }
            SearchTarget::Panel(panel) => {
                crate::layout::bring_to_front(&mut self.dock, panel);
            }
            SearchTarget::Entity(id) => {
                if self.document.area().contains(id) {
                    self.select(id);
                    self.frame_selection();
                }
            }
            SearchTarget::File(kind, path) => {
                if kind.placeable() && !kind.opens_on_double_click() {
                    // Placed, not opened: shown in the Library, chosen.
                    self.library_state.selected = Some(path.clone());
                    self.library_request(LibraryRequest::Show(path), actions);
                    if self.library_pinned {
                        crate::layout::bring_to_front(&mut self.dock, Panel::Library);
                    } else {
                        self.drawer_open = true;
                    }
                } else {
                    self.library_request(LibraryRequest::Open(kind, path), actions);
                }
            }
        }
    }

    /// The search box in the top bar: clicking it opens the search.
    pub(crate) fn search_box(&mut self, ui: &mut Ui) {
        let width = ui.available_width().clamp(120.0, 320.0);
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::click());
        let painter = ui.painter();
        let hovered = response.hovered();
        painter.rect(
            rect,
            5.0,
            theme::FIELD,
            egui::Stroke::new(
                1.0,
                if hovered {
                    theme::TEXT_WEAK
                } else {
                    theme::BUTTON_BORDER
                },
            ),
            egui::StrokeKind::Inside,
        );
        let icon = egui::Rect::from_center_size(
            rect.left_center() + egui::vec2(14.0, 0.0),
            egui::vec2(12.0, 12.0),
        );
        crate::icons::icon(painter, icon, crate::icons::Icon::Search, theme::TEXT_WEAK);
        painter.text(
            rect.left_center() + egui::vec2(26.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "Search or do anything…",
            egui::FontId::proportional(12.5),
            theme::TEXT_WEAK,
        );
        painter.text(
            rect.right_center() - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            "Space",
            egui::FontId::proportional(11.0),
            theme::TEXT_WEAK,
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Search or do anything")
        });
        let response = response.on_hover_text(
            "Finds any action, file, entity or panel and does it: press Space with nothing being \
             typed",
        );
        if response.clicked() {
            self.open_search();
        }
    }

    /// The search list, at the top middle of the window, while it's open.
    pub(crate) fn search_list(&mut self, ui: &Ui, actions: &mut Vec<WorkbenchAction>) {
        if !self.search.open {
            return;
        }
        let items = self.search_items();
        let shown = results(&items, &self.search.query);
        // Keys: Up and Down choose, Enter does it, Esc closes.
        let (up, down, enter, escape) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });
        if down && !shown.is_empty() {
            self.search.chosen = (self.search.chosen + 1).min(shown.len() - 1);
        }
        if up {
            self.search.chosen = self.search.chosen.saturating_sub(1);
        }
        self.search.chosen = self.search.chosen.min(shown.len().saturating_sub(1));
        let screen = ui.ctx().content_rect();
        let at = egui::pos2(screen.center().x - WIDTH / 2.0, screen.top() + 44.0);
        let mut picked: Option<SearchTarget> = None;
        let shown_area = egui::Area::new(egui::Id::new("search"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_width(WIDTH);
                        let field = ui.add(
                            egui::TextEdit::singleline(&mut self.search.query)
                                .hint_text("Search actions, files, entities and panels")
                                .desired_width(WIDTH),
                        );
                        field.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search")
                        });
                        if self.search.focus {
                            field.request_focus();
                            self.search.focus = false;
                        }
                        if field.changed() {
                            self.search.chosen = 0;
                        }
                        ui.add_space(4.0);
                        if shown.is_empty() {
                            ui.label(
                                RichText::new(format!(
                                    "Nothing is called “{}”.",
                                    self.search.query.trim()
                                ))
                                .color(theme::TEXT_WEAK),
                            );
                        }
                        egui::ScrollArea::vertical()
                            .max_height(360.0)
                            .show(ui, |ui| {
                                for (n, item) in shown.iter().enumerate() {
                                    let chosen = n == self.search.chosen;
                                    let row = result_row(ui, item, chosen);
                                    if chosen && (down || up) {
                                        row.scroll_to_me(None);
                                    }
                                    if row.clicked() {
                                        picked = Some(item.target.clone());
                                    }
                                }
                            });
                    });
            });
        if enter && let Some(item) = shown.get(self.search.chosen) {
            picked = Some(item.target.clone());
        }
        let clicked_outside = ui.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !shown_area.response.rect.contains(p))
        });
        if let Some(target) = picked {
            self.search.open = false;
            self.do_search(target, ui.ctx(), actions);
        } else if escape || clicked_outside {
            self.search.open = false;
        }
    }
}

/// One result's row: its name, greyed detail, shortcut and kind; lit when
/// chosen.
fn result_row(ui: &mut Ui, item: &SearchItem, chosen: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(WIDTH, 24.0), egui::Sense::click());
    let painter = ui.painter();
    if chosen || response.hovered() {
        painter.rect_filled(
            rect,
            4.0,
            ui.visuals()
                .selection
                .bg_fill
                .gamma_multiply(if chosen { 0.6 } else { 0.3 }),
        );
    }
    let font = egui::FontId::proportional(13.0);
    let name = painter.text(
        rect.left_center() + egui::vec2(8.0, 0.0),
        egui::Align2::LEFT_CENTER,
        &item.name,
        font,
        theme::TEXT_STRONG,
    );
    if !item.detail.is_empty() {
        painter.text(
            name.right_center() + egui::vec2(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            &item.detail,
            egui::FontId::proportional(12.0),
            theme::TEXT_WEAK,
        );
    }
    let kind = painter.text(
        rect.right_center() - egui::vec2(8.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        item.kind.label(),
        egui::FontId::proportional(11.0),
        theme::TEXT_WEAK,
    );
    if !item.keys.is_empty() {
        painter.text(
            kind.left_center() - egui::vec2(12.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            item.keys,
            egui::FontId::monospace(11.5),
            theme::TEXT,
        );
    }
    let label = item.name.clone();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
    response
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
