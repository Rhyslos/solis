//! A module's own page (the approved addition to board 07): Open, a
//! double-click on its picture or New module puts the module in a tab of
//! its own beside the level's, where it's built on its own as a level is,
//! with the Level panel, the view and Properties, and a save and undo of
//! its own. Saving it updates every copy in the open level at once, each
//! keeping its own changes ([`Workbench::set_module`]).
//!
//! The page is a second [`Document`]. While its tab is in front the
//! workbench's document *is* the module's, for the whole of
//! [`Workbench::show`] (`page_in`, `page_out`): every panel, key and
//! command works on it unchanged. Between frames the level is the
//! document again, so whatever the app asks of the level finds the level.

use crate::Workbench;
use crate::console::LogLevel;
use crate::document::Document;
use crate::panel::Panel;
use crate::shell::RibbonTab;
use crate::workbench::WorkbenchAction;
use level::module::Expander;
use level::module_follow::ValueChanges;
use level::text::{FileKind, LevelFile};
use reflect::{AssetId, EntityId};
use std::sync::Arc;

/// The module open on its own page.
pub(crate) struct ModulePage {
    /// The module.
    pub(crate) asset: AssetId,
    /// Its root entity.
    pub(crate) root: EntityId,
    /// The document not in use: the module's while the level is the
    /// workbench's, and the level while the module's is.
    pub(crate) aside: Document,
    /// Whether the module's document is the workbench's now.
    pub(crate) swapped_in: bool,
    /// Whether its tab was in front last frame (switching gives the
    /// document coming in a new revision, so what's drawn is made again).
    pub(crate) was_front: bool,
    /// Its tab's × was clicked with changes not saved: the question.
    pub(crate) asking_to_close: bool,
}

impl Workbench {
    /// Opens module `id`'s page and brings it to the front: its contents,
    /// with the modules placed inside it expanded, as a document of its
    /// own. Another module's page closes first (one is open at a time),
    /// unless it has unsaved changes, which the Console says.
    pub fn open_module_page(&mut self, id: AssetId) {
        if self.open.playing.is_some() {
            self.log(
                LogLevel::Warning,
                "Stop playing first, then open the module.",
            );
            return;
        }
        if let Some(page) = &self.open.module_page {
            if page.asset == id {
                self.show_panel(Panel::ModulePage);
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
        let Some(file) = self.open.modules.get(&id).cloned() else {
            self.open.outbox.push(WorkbenchAction::OpenModule(id));
            return;
        };
        let FileKind::Module { root } = file.kind else {
            return;
        };
        let expanded = {
            let lookup = |asset| self.open.modules.get(&asset).cloned();
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
        let path = self.module_path(id);
        let mut document = match path {
            Some(path) => Document::opened(level, path),
            None => Document::new(level),
        };
        document.registry = self.open.document.registry.clone();
        document.selection.select_only(root);
        self.open.module_page = Some(ModulePage {
            asset: id,
            root,
            aside: document,
            swapped_in: false,
            was_front: false,
            asking_to_close: false,
        });
        self.shell.tab = RibbonTab::Home;
        self.show_panel(Panel::ModulePage);
    }

    /// The module's file on this computer, from the Library.
    fn module_path(&self, id: AssetId) -> Option<std::path::PathBuf> {
        let project = self.open.project.as_ref()?;
        let item = self.open.library.iter().find(|i| i.id == Some(id))?;
        Some(project.assets.join(&item.path))
    }

    /// The module whose page is open, if one is, and its name.
    pub fn module_page(&self) -> Option<(AssetId, String)> {
        let page = self.open.module_page.as_ref()?;
        Some((page.asset, self.asset_name(page.asset)))
    }

    /// Whether the module's page is in front on Home: its document is the
    /// one the panels show.
    pub(crate) fn page_in_front(&self) -> bool {
        self.open.module_page.as_ref().is_some_and(|p| p.swapped_in)
    }

    /// Whether Home's front document is the module's page.
    fn page_wanted(&self) -> bool {
        self.open.module_page.is_some()
            && self
                .shell
                .arrangement(RibbonTab::Home)
                .front
                .contains(&Some(Panel::ModulePage))
    }

    /// At the start of a frame: the module's document becomes the
    /// workbench's while its page is in front.
    pub(crate) fn page_in(&mut self) {
        let wanted = self.page_wanted();
        let Some(page) = &mut self.open.module_page else {
            return;
        };
        if !wanted || page.swapped_in {
            return;
        }
        std::mem::swap(&mut self.open.document, &mut page.aside);
        page.swapped_in = true;
        self.open.document.registry = page.aside.registry.clone();
        if !page.was_front {
            // Everything drawn of the level is drawn again of the module.
            self.open.document.touch();
            page.was_front = true;
        }
    }

    /// At the end of a frame: the level is the workbench's document again.
    pub(crate) fn page_out(&mut self) {
        let wanted = self.page_wanted();
        let Some(page) = &mut self.open.module_page else {
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
    /// module's page stays open behind it.
    pub(crate) fn level_to_front(&mut self) {
        if !self.page_in_front() {
            return;
        }
        let home = self.shell.arrangement_mut(RibbonTab::Home);
        for front in &mut home.front {
            if *front == Some(Panel::ModulePage) {
                *front = Some(Panel::Viewport);
            }
        }
        if let Some(page) = &mut self.open.module_page {
            std::mem::swap(&mut self.open.document, &mut page.aside);
            page.swapped_in = false;
            page.was_front = false;
        }
        self.open.document.touch();
    }

    /// The module's document, whichever is the workbench's now.
    fn page_document(&self) -> Option<&Document> {
        let page = self.open.module_page.as_ref()?;
        Some(if page.swapped_in {
            &self.open.document
        } else {
            &page.aside
        })
    }

    /// Saves the module's page (Ctrl + S while it's in front): its
    /// contents as a module's file, written by the app, which then hands
    /// them back ([`Self::module_saved`]).
    pub(crate) fn save_module_page(&mut self) {
        let Some(page) = &self.open.module_page else {
            return;
        };
        let (asset, root) = (page.asset, page.root);
        let Some(document) = self.page_document() else {
            return;
        };
        if !document.level().contains(root) {
            self.log(
                LogLevel::Warning,
                "The module's top entity is gone: undo, or put one back, to save it.",
            );
            return;
        }
        let collapsed = {
            let lookup = |id| self.open.modules.get(&id).cloned();
            Expander::new(&lookup).collapse(document.level(), FileKind::Module { root })
        };
        match collapsed {
            Ok(file) => self
                .open
                .outbox
                .push(WorkbenchAction::SaveModule { asset, file }),
            Err(why) => self.log(LogLevel::Error, format!("Couldn't save the module: {why}.")),
        }
    }

    /// The module `id` was saved as `file`: its page counts as saved, and
    /// every copy in the open level follows it, keeping its own changes.
    pub fn module_saved(&mut self, id: AssetId, file: Arc<LevelFile>, path: std::path::PathBuf) {
        if let Some(page) = &mut self.open.module_page
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
        self.set_module(id, file);
        if swapped {
            self.swap_page_document();
        }
        let name = self.asset_name(id);
        self.log(LogLevel::Info, format!("Saved the module {name}."));
    }

    /// Makes every copy of module `id` in the open level follow `file`,
    /// its new contents, by the values that changed alone
    /// (`level::module_follow`), and makes `file` the module's: true when
    /// that was all it took. False, with nothing done, when more than
    /// values changed, or the module is placed inside another module (its
    /// copies there are that module's parts): then [`Self::set_module`]
    /// rebuilds the copies the whole way. Each copy keeps its own changes,
    /// as one undo step; while the game plays, as the game's change.
    pub(crate) fn follow_values(&mut self, id: AssetId, file: &Arc<LevelFile>) -> bool {
        let nested = self.open.modules.iter().any(|(other, f)| {
            *other != id
                && f.level
                    .iter()
                    .any(|(_, e)| e.origin.as_ref().is_some_and(|o| o.module == id))
        });
        if nested {
            return false;
        }
        let old = {
            let lookup = |asset| self.open.modules.get(&asset).cloned();
            Expander::new(&lookup).module(id)
        };
        let (new, mended) = {
            let lookup = |asset| {
                if asset == id {
                    Some(file.clone())
                } else {
                    self.open.modules.get(&asset).cloned()
                }
            };
            let mut expander = Expander::new(&lookup);
            (expander.module(id), expander.mended().to_vec())
        };
        let (Ok(old), Ok(new)) = (old, new) else {
            return false;
        };
        let Some(changes) = ValueChanges::between((&old.0, old.1), (&new.0, new.1)) else {
            return false;
        };
        self.open.modules.insert(id, file.clone());
        self.sources += 1;
        self.log_mended(&mended);
        let edits = changes.follow(self.open.document.level(), id);
        if edits.is_empty() {
            return true;
        }
        let name = self.asset_name(id);
        let updated = if self.open.playing.is_some() {
            // Not one of the user's edits, so not a step for Keep to make
            // again; the game takes it in as a change.
            self.open.document.edit_by_game(edits);
            self.open.document.touch();
            Ok(())
        } else {
            self.open
                .document
                .rebuild_by(edits, &format!("Update copies of {name}"))
        };
        match updated {
            Ok(()) => self.log(LogLevel::Info, format!("Updated the copies of {name}.")),
            Err(why) => self.log(
                LogLevel::Error,
                format!("Couldn't update copies of {name}: {why}."),
            ),
        }
        true
    }

    /// Swaps the level and the module's document, for a moment.
    fn swap_page_document(&mut self) {
        if let Some(page) = &mut self.open.module_page {
            std::mem::swap(&mut self.open.document, &mut page.aside);
            page.swapped_in = !page.swapped_in;
        }
    }

    /// Closes the module's page (its tab's ×): at once when it's saved,
    /// otherwise after asking.
    pub(crate) fn close_module_page(&mut self) {
        let dirty = self.page_document().is_some_and(Document::is_dirty);
        if dirty {
            if let Some(page) = &mut self.open.module_page {
                page.asking_to_close = true;
            }
            return;
        }
        self.drop_module_page();
    }

    fn drop_module_page(&mut self) {
        self.level_to_front();
        self.open.module_page = None;
    }

    /// The question about a module page's unsaved changes, when its × was
    /// clicked: Save, Don't save, or Cancel.
    pub(crate) fn module_page_question(&mut self, ctx: &egui::Context) {
        let Some(page) = &self.open.module_page else {
            return;
        };
        if !page.asking_to_close {
            return;
        }
        let name = self.asset_name(page.asset);
        use crate::widgets::dialog::{Answer, dialog};
        let words = format!("The module {name} has changes that aren't saved.");
        let answers = [
            Answer::left("Don't save"),
            Answer::right("Cancel"),
            Answer::main("Save"),
        ];
        let picked = dialog(
            ctx,
            "unsaved module",
            "Unsaved changes",
            |ui| drop(ui.label(words)),
            &answers,
        );
        match picked {
            Some(0) => self.drop_module_page(),
            Some(2) => {
                self.save_module_page();
                if let Some(page) = &mut self.open.module_page {
                    page.asking_to_close = false;
                }
            }
            Some(_) => {
                if let Some(page) = &mut self.open.module_page {
                    page.asking_to_close = false;
                }
            }
            None => {}
        }
    }

    /// The names on the document tabs while the page is open: the
    /// module's and the level's.
    pub(crate) fn page_names(&self) -> Option<crate::panels::PageNames> {
        let page = self.open.module_page.as_ref()?;
        let level = if page.swapped_in {
            page.aside.name()
        } else {
            self.open.document.name()
        };
        Some(crate::panels::PageNames {
            module: self.asset_name(page.asset),
            level,
        })
    }

    /// Whether the module's page has changes that aren't saved.
    pub fn module_page_dirty(&self) -> bool {
        self.page_document().is_some_and(Document::is_dirty)
    }
}

#[cfg(test)]
#[path = "module_page_tests.rs"]
mod tests;
