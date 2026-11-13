//! What the search can find: the editor's commands (from the one table,
//! `shell/commands.rs`), the panels, the area's entities and the
//! project's files.
//!
//! The entities and files are read into a list once and kept until the
//! area or the project's files change, and scored once for what's typed,
//! so an open search doesn't walk the whole area every frame.

use super::{SearchItem, SearchKind, SearchTarget};
use crate::panel::Panel;
use crate::shell::ShellCommand;
use crate::workbench::Workbench;
use std::sync::Arc;

/// The area's entities and the project's files as results, and what
/// they were read from.
#[derive(Debug, Clone, Default)]
pub(crate) struct Found {
    /// The area as it was ([`crate::Document::revision`]).
    revision: u64,
    /// The project's files as they were (`Workbench::sources`).
    sources: u64,
    /// How many files there were (the list is handed over whole).
    files: usize,
    items: Vec<SearchItem>,
    /// The best of `items` for what was typed, and what that was.
    best: Option<(String, Arc<Vec<SearchItem>>)>,
}

impl Found {
    /// The best of the entities and files for `query`, best first (at
    /// most `MAX_SHOWN`): scored once for each thing typed, not
    /// every frame (the review, BUG-28).
    fn best_for(&mut self, query: &str) -> Arc<Vec<SearchItem>> {
        match &self.best {
            Some((typed, best)) if typed == query => Arc::clone(best),
            _ => {
                let best: Arc<Vec<SearchItem>> = Arc::new(
                    super::score::results(&self.items, query)
                        .into_iter()
                        .cloned()
                        .collect(),
                );
                self.best = Some((query.to_owned(), Arc::clone(&best)));
                best
            }
        }
    }
}

impl Workbench {
    /// Everything the search can find now: the commands that can be done
    /// (one that can't, such as Pause while editing, is left out), the
    /// panels, the area's entities and the project's files. The open
    /// search shows the best of these through [`Self::search_results`],
    /// without copying them all; tests look at the whole list.
    #[cfg(test)]
    pub(crate) fn search_items(&self) -> Vec<SearchItem> {
        let mut items = self.command_items();
        items.extend(self.panel_items());
        match &self.search.found {
            Some(found) if self.found_is_current(found) => items.extend(found.items.clone()),
            _ => items.extend(self.find_entities_and_files().items),
        }
        items
    }

    /// The results shown for what's typed: the commands that can be done
    /// and the panels, scored each frame (whether a command can be done
    /// changes), with the best of the entities and files, scored once
    /// for what's typed. The same as `score::results` over
    /// [`Self::search_items`]: the best of the commands, panels and the
    /// best entities and files are the best of everything.
    pub(crate) fn search_results(&mut self) -> Vec<SearchItem> {
        self.keep_found();
        let query = self.search.query.clone();
        let mut items = self.command_items();
        items.extend(self.panel_items());
        if let Some(found) = &mut self.search.found {
            items.extend(found.best_for(&query).iter().cloned());
        }
        super::score::results(&items, &query)
            .into_iter()
            .cloned()
            .collect()
    }

    /// The commands that can be done now, in the table's order.
    fn command_items(&self) -> Vec<SearchItem> {
        ShellCommand::ALL
            .into_iter()
            .filter(|command| command.why_not(self).is_none())
            .map(|command| SearchItem {
                name: command.name(self),
                kind: SearchKind::Action,
                detail: command.place().to_owned(),
                keys: command.keys(),
                target: SearchTarget::Command(command),
            })
            .collect()
    }

    /// Every panel there is now, wherever it lives (a document's only
    /// while it's open).
    fn panel_items(&self) -> Vec<SearchItem> {
        Panel::ALL
            .into_iter()
            .chain(Panel::OPEN_WITH_A_FILE)
            .filter(|panel| crate::shell::workspaces::tab_for(*panel, self.open_now()).is_some())
            .map(|panel| SearchItem {
                name: panel.title().to_string(),
                kind: SearchKind::Panel,
                detail: String::new(),
                keys: "",
                target: SearchTarget::Panel(panel),
            })
            .collect()
    }

    fn found_is_current(&self, found: &Found) -> bool {
        found.revision == self.open.document.revision()
            && found.sources == self.sources
            && found.files == self.open.library.len()
    }

    /// Reads the area's entities and the project's files as results.
    fn find_entities_and_files(&self) -> Found {
        let entities = self
            .open
            .document
            .area()
            .iter()
            .map(|(id, entity)| SearchItem {
                name: entity.name.clone(),
                kind: SearchKind::Entity,
                detail: String::new(),
                keys: "",
                target: SearchTarget::Entity(id),
            });
        let files = self
            .open
            .library
            .iter()
            .filter(|_| self.open.project.is_some())
            .map(|item| SearchItem {
                name: item.name.clone(),
                kind: SearchKind::File,
                detail: item.folder().to_string(),
                keys: "",
                target: SearchTarget::File(item.kind, item.path.clone()),
            });
        Found {
            revision: self.open.document.revision(),
            sources: self.sources,
            files: self.open.library.len(),
            items: entities.chain(files).collect(),
            best: None,
        }
    }

    /// Keeps the entities and files found until the area or the files
    /// change: called each frame the search is open.
    pub(crate) fn keep_found(&mut self) {
        let current = self
            .search
            .found
            .as_ref()
            .is_some_and(|found| self.found_is_current(found));
        if !current {
            self.search.found = Some(self.find_entities_and_files());
        }
    }

    /// The best entities and files kept for what's typed, if any.
    #[cfg(test)]
    pub(crate) fn best_kept(&self) -> Option<Arc<Vec<SearchItem>>> {
        let (_, best) = self.search.found.as_ref()?.best.as_ref()?;
        Some(Arc::clone(best))
    }

    /// How many times the area was walked for the list kept: none while
    /// nothing changed.
    #[cfg(test)]
    pub(crate) fn found_from(&self) -> Option<(u64, u64)> {
        self.search
            .found
            .as_ref()
            .map(|found| (found.revision, found.sources))
    }
}
