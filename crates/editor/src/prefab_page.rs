//! A prefab's own page (the approved addition to board 07): Open, a
//! double-click on its picture or New prefab puts the prefab in a tab of
//! its own beside the level's, where it's built on its own as a level is,
//! with the Level panel, the view and Properties, and a save and undo of
//! its own. Saving it updates every copy in the open level at once, each
//! keeping its own changes ([`Workbench::set_prefab`]).
//!
//! The page is a second [`Document`]. While its tab is in front the
//! workbench's document *is* the prefab's, for the whole of
//! [`Workbench::show`] (`page_in`, `page_out`): every panel, key and
//! command works on it unchanged. Between frames the level is the
//! document again, so whatever the app asks of the level finds the level.

use crate::Workbench;
use crate::console::LogLevel;
use crate::document::Document;
use crate::panel::Panel;
use crate::shell::RibbonTab;
use crate::workbench::WorkbenchAction;
use level::prefab::Expander;
use level::text::{FileKind, LevelFile};
use reflect::{AssetId, EntityId};
use std::sync::Arc;

/// The prefab open on its own page.
pub(crate) struct PrefabPage {
    /// The prefab.
    pub(crate) asset: AssetId,
    /// Its root entity.
    pub(crate) root: EntityId,
    /// The document not in use: the prefab's while the level is the
    /// workbench's, and the level while the prefab's is.
    pub(crate) aside: Document,
    /// Whether the prefab's document is the workbench's now.
    pub(crate) swapped_in: bool,
    /// Whether its tab was in front last frame (switching gives the
    /// document coming in a new revision, so what's drawn is made again).
    pub(crate) was_front: bool,
    /// Its tab's × was clicked with changes not saved: the question.
    pub(crate) asking_to_close: bool,
}

impl Workbench {
    /// Opens prefab `id`'s page and brings it to the front: its contents,
    /// with the prefabs placed inside it expanded, as a document of its
    /// own. Another prefab's page closes first (one is open at a time),
    /// unless it has unsaved changes, which the Console says.
    pub fn open_prefab_page(&mut self, id: AssetId) {
        if self.open.playing.is_some() {
            self.log(
                LogLevel::Warning,
                "Stop playing first, then open the prefab.",
            );
            return;
        }
        if let Some(page) = &self.open.prefab_page {
            if page.asset == id {
                self.show_panel(Panel::PrefabPage);
                return;
            }
            if page.aside.is_dirty() && !page.swapped_in {
                let name = self.asset_name(page.asset);
                self.log(
                    LogLevel::Warning,
                    format!("Save or close {name}'s page first: it has changes that aren't saved."),
                );
                return;
            }
        }
        let Some(file) = self.open.prefabs.get(&id).cloned() else {
            self.open.outbox.push(WorkbenchAction::OpenPrefab(id));
            return;
        };
        let FileKind::Prefab { root } = file.kind else {
            return;
        };
        let expanded = {
            let lookup = |asset| self.open.prefabs.get(&asset).cloned();
            Expander::new(&lookup).expand(&file)
        };
        let level = match expanded {
            Ok(level) => level,
            Err(why) => {
                let name = self.asset_name(id);
                self.log(LogLevel::Error, format!("Couldn't open {name}: {why}."));
                return;
            }
        };
        let path = self.prefab_path(id);
        let mut document = match path {
            Some(path) => Document::opened(level, path),
            None => Document::new(level),
        };
        document.registry = self.open.document.registry.clone();
        document.selection.select_only(root);
        self.open.prefab_page = Some(PrefabPage {
            asset: id,
            root,
            aside: document,
            swapped_in: false,
            was_front: false,
            asking_to_close: false,
        });
        self.shell.tab = RibbonTab::Home;
        self.show_panel(Panel::PrefabPage);
    }

    /// The prefab's file on this computer, from the Library.
    fn prefab_path(&self, id: AssetId) -> Option<std::path::PathBuf> {
        let project = self.open.project.as_ref()?;
        let item = self.open.library.iter().find(|i| i.id == Some(id))?;
        Some(project.assets.join(&item.path))
    }

    /// The prefab whose page is open, if one is, and its name.
    pub fn prefab_page(&self) -> Option<(AssetId, String)> {
        let page = self.open.prefab_page.as_ref()?;
        Some((page.asset, self.asset_name(page.asset)))
    }

    /// Whether the prefab's page is in front on Home: its document is the
    /// one the panels show.
    pub(crate) fn page_in_front(&self) -> bool {
        self.open.prefab_page.as_ref().is_some_and(|p| p.swapped_in)
    }

    /// Whether Home's front document is the prefab's page.
    fn page_wanted(&self) -> bool {
        self.open.prefab_page.is_some()
            && self
                .shell
                .arrangement(RibbonTab::Home)
                .front
                .contains(&Some(Panel::PrefabPage))
    }

    /// At the start of a frame: the prefab's document becomes the
    /// workbench's while its page is in front.
    pub(crate) fn page_in(&mut self) {
        let wanted = self.page_wanted();
        let Some(page) = &mut self.open.prefab_page else {
            return;
        };
        if !wanted || page.swapped_in {
            return;
        }
        std::mem::swap(&mut self.open.document, &mut page.aside);
        page.swapped_in = true;
        self.open.document.registry = page.aside.registry.clone();
        if !page.was_front {
            // Everything drawn of the level is drawn again of the prefab.
            self.open.document.touch();
            page.was_front = true;
        }
    }

    /// At the end of a frame: the level is the workbench's document again.
    pub(crate) fn page_out(&mut self) {
        let wanted = self.page_wanted();
        let Some(page) = &mut self.open.prefab_page else {
            return;
        };
        if page.swapped_in {
            std::mem::swap(&mut self.open.document, &mut page.aside);
            page.swapped_in = false;
            if !wanted && page.was_front {
                self.open.document.touch();
                page.was_front = false;
            }
        } else if page.was_front && !wanted {
            self.open.document.touch();
            page.was_front = false;
        }
    }

    /// Brings the level's tab to the front (Play, a level opened): the
    /// prefab's page stays open behind it.
    pub(crate) fn level_to_front(&mut self) {
        if !self.page_in_front() {
            return;
        }
        let home = self.shell.arrangement_mut(RibbonTab::Home);
        for front in &mut home.front {
            if *front == Some(Panel::PrefabPage) {
                *front = Some(Panel::Viewport);
            }
        }
        if let Some(page) = &mut self.open.prefab_page {
            std::mem::swap(&mut self.open.document, &mut page.aside);
            page.swapped_in = false;
            page.was_front = false;
        }
        self.open.document.touch();
    }

    /// The prefab's document, whichever is the workbench's now.
    fn page_document(&self) -> Option<&Document> {
        let page = self.open.prefab_page.as_ref()?;
        Some(if page.swapped_in {
            &self.open.document
        } else {
            &page.aside
        })
    }

    /// Saves the prefab's page (Ctrl + S while it's in front): its
    /// contents as a prefab's file, written by the app, which then hands
    /// them back ([`Self::prefab_saved`]).
    pub(crate) fn save_prefab_page(&mut self) {
        let Some(page) = &self.open.prefab_page else {
            return;
        };
        let (asset, root) = (page.asset, page.root);
        let Some(document) = self.page_document() else {
            return;
        };
        if !document.level().contains(root) {
            self.log(
                LogLevel::Warning,
                "The prefab's top entity is gone: undo, or put one back, to save it.",
            );
            return;
        }
        let collapsed = {
            let lookup = |id| self.open.prefabs.get(&id).cloned();
            Expander::new(&lookup).collapse(document.level(), FileKind::Prefab { root })
        };
        match collapsed {
            Ok(file) => self
                .open
                .outbox
                .push(WorkbenchAction::SavePrefab { asset, file }),
            Err(why) => self.log(LogLevel::Error, format!("Couldn't save the prefab: {why}.")),
        }
    }

    /// The prefab `id` was saved as `file`: its page counts as saved, and
    /// every copy in the open level follows it, keeping its own changes.
    pub fn prefab_saved(&mut self, id: AssetId, file: Arc<LevelFile>, path: std::path::PathBuf) {
        if let Some(page) = &mut self.open.prefab_page
            && page.asset == id
        {
            let document = if page.swapped_in {
                &mut self.open.document
            } else {
                &mut page.aside
            };
            document.mark_saved(path);
        }
        let swapped = self.page_in_front();
        if swapped {
            self.swap_page_document();
        }
        self.set_prefab(id, file);
        if swapped {
            self.swap_page_document();
        }
        let name = self.asset_name(id);
        self.log(LogLevel::Info, format!("Saved the prefab {name}."));
    }

    /// Swaps the level and the prefab's document, for a moment.
    fn swap_page_document(&mut self) {
        if let Some(page) = &mut self.open.prefab_page {
            std::mem::swap(&mut self.open.document, &mut page.aside);
            page.swapped_in = !page.swapped_in;
        }
    }

    /// Closes the prefab's page (its tab's ×): at once when it's saved,
    /// otherwise after asking.
    pub(crate) fn close_prefab_page(&mut self) {
        let dirty = self.page_document().is_some_and(Document::is_dirty);
        if dirty {
            if let Some(page) = &mut self.open.prefab_page {
                page.asking_to_close = true;
            }
            return;
        }
        self.drop_prefab_page();
    }

    fn drop_prefab_page(&mut self) {
        self.level_to_front();
        self.open.prefab_page = None;
    }

    /// The question about a prefab page's unsaved changes, when its × was
    /// clicked: Save, Don't save, or Cancel.
    pub(crate) fn prefab_page_question(&mut self, ctx: &egui::Context) {
        let Some(page) = &self.open.prefab_page else {
            return;
        };
        if !page.asking_to_close {
            return;
        }
        let name = self.asset_name(page.asset);
        use crate::widgets::dialog::{Answer, dialog};
        let words = format!("The prefab {name} has changes that aren't saved.");
        let answers = [
            Answer::left("Don't save"),
            Answer::right("Cancel"),
            Answer::main("Save"),
        ];
        let picked = dialog(
            ctx,
            "unsaved prefab",
            "Unsaved changes",
            |ui| drop(ui.label(words)),
            &answers,
        );
        match picked {
            Some(0) => self.drop_prefab_page(),
            Some(2) => {
                self.save_prefab_page();
                if let Some(page) = &mut self.open.prefab_page {
                    page.asking_to_close = false;
                }
            }
            Some(_) => {
                if let Some(page) = &mut self.open.prefab_page {
                    page.asking_to_close = false;
                }
            }
            None => {}
        }
    }

    /// The names on the document tabs while the page is open: the
    /// prefab's and the level's.
    pub(crate) fn page_names(&self) -> Option<crate::panels::PageNames> {
        let page = self.open.prefab_page.as_ref()?;
        let level = if page.swapped_in {
            page.aside.name()
        } else {
            self.open.document.name()
        };
        Some(crate::panels::PageNames {
            prefab: self.asset_name(page.asset),
            level,
        })
    }

    /// Whether the prefab's page has changes that aren't saved.
    pub fn prefab_page_dirty(&self) -> bool {
        self.page_document().is_some_and(Document::is_dirty)
    }
}

#[cfg(test)]
#[path = "prefab_page_tests.rs"]
mod tests;
