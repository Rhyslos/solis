//! The File tab: a page in place of the editor, with a column of items
//! down its left side.

use crate::workbench::{Workbench, WorkbenchAction};

/// The pages of the File tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum FileItem {
    /// The projects the editor knows.
    #[default]
    RecentProjects,
    /// The examples, the first steps and the manual.
    Learn,
    /// This build, the newest one, and updating.
    Updates,
}

impl Workbench {
    /// Brings the File tab to the front on `page`. The tab that was in
    /// front is remembered, for Back to editor.
    pub(crate) fn show_file_page(&mut self, page: FileItem) {
        if self.shell.tab != super::RibbonTab::File {
            self.shell.before_file = self.shell.tab;
            // The list of projects is read afresh each time File opens.
            self.open.outbox.push(WorkbenchAction::ListProjects);
        }
        self.shell.tab = super::RibbonTab::File;
        self.shell.dropped = false;
        self.shell.file_item = page;
    }

    /// Leaves the File tab for the tab that was in front before it.
    pub(crate) fn back_to_editor(&mut self) {
        if self.shell.tab == super::RibbonTab::File {
            self.shell.tab = self.shell.before_file;
        }
    }
}
