//! One search for everything (the editor overhaul's O4, decision 0085): the
//! box in the tab bar, or **Space** with nothing being typed, opens a list
//! that finds everything the editor can do (with its shortcut beside it, so
//! the list teaches the keys), every file in the project, every entity in
//! the area and every panel. Type to narrow it, Up and Down to choose,
//! Enter to do it, Esc to close.

//!
//! - `search/list.rs`: what can be found (the commands come from the one
//!   table in `shell/commands.rs`);
//! - `search/score.rs`: how well a name matches what is typed;
//! - `search/draw.rs`: the field and the list while the search is open.
//!
//! The box that opens it is in the tab bar (`shell/tab_bar.rs`).

mod draw;
mod list;
mod score;

#[cfg(test)]
pub(crate) use score::{MAX_SHOWN, results, score};

use crate::key_owner::KeyOwner;
use crate::library::{LibraryKind, LibraryRequest};
use crate::panel::Panel;
use crate::shell::ShellCommand;
use crate::workbench::{Workbench, WorkbenchAction};
use egui::Ui;
use reflect::EntityId;

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
    /// Something the editor does (Undo, Save area, Play…).
    Command(ShellCommand),
    /// Bring a panel to the front (opening it if it's closed).
    Panel(Panel),
    /// Select an entity and point the camera at it.
    Entity(EntityId),
    /// Open a file (or, for one that's placed rather than opened, show it
    /// in the Library): its kind and its path inside the assets.
    File(LibraryKind, String),
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
    pub(crate) focus: bool,
    /// The area's entities and the project's files, as last read.
    pub(crate) found: Option<list::Found>,
}

impl Workbench {
    /// Opens the search.
    pub(crate) fn open_search(&mut self) {
        self.search = SearchState {
            open: true,
            query: String::new(),
            chosen: 0,
            focus: true,
            found: self.search.found.take(),
        };
    }

    /// Space, with nothing being typed, the pointer not over the Frames
    /// tab (where Space plays the clip) and the game not holding the keys,
    /// opens the search.
    pub(crate) fn search_key(&mut self, ui: &Ui) {
        let keys = self.keys;
        if self.search.open || keys == KeyOwner::Frames || !keys.takes_shortcuts() {
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
            SearchTarget::Command(command) => command.run(self, ctx, actions),
            SearchTarget::Panel(Panel::Library) if !self.library_pinned => {
                self.drawer_open = true;
            }
            SearchTarget::Panel(panel) => {
                crate::layout::bring_to_front(&mut self.dock, panel);
            }
            SearchTarget::Entity(id) => {
                if self.open.document.area().contains(id) {
                    self.select(id);
                    self.frame_selection();
                }
            }
            SearchTarget::File(kind, path) => {
                if kind.placeable() && !kind.opens_on_double_click() {
                    // Placed, not opened: picked in the Library, in its
                    // folder, with Properties following as for a click
                    // there. Nothing is asked of the computer's own
                    // programs.
                    let before = self.open.library_state.selected.clone();
                    self.pick_in_library(&path);
                    if !self.open.library_state.filter.passes(kind) {
                        self.open.library_state.filter = crate::library::Filter::All;
                    }
                    self.follow_sound_selection(&before);
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
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
