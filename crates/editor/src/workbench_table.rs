//! The workbench's side of data tables (stage 1; decision 0128): the
//! table open in the Table tab (beside the level's view on Home), each
//! change saved at once as a file of the editor's own
//! (`Workbench::save_file`), its undo, a file changed on disk taken in,
//! a new table from the Library, CSV in and out, and the tables as the
//! running game and its recordings and saves know them.

use crate::console::LogLevel;
use crate::own_files::{Followed, OwnFile, Told, follow};
use crate::panel::Panel;
use crate::sprite_sheets::TableState;
use crate::table_editor::TableEditor;
use crate::workbench::{Workbench, WorkbenchAction};
use std::sync::Arc;

impl Workbench {
    /// The project's tables as the running game reads them.
    pub(crate) fn tables_now(&self) -> script::Tables {
        let list: Vec<(String, Arc<level::table::Table>)> = self
            .viewport
            .sheets
            .table_names()
            .into_iter()
            .map(|n| (n.path, n.table))
            .collect();
        script::Tables::new(&list, &script::describe::owners_of_natives())
    }

    /// Says once that the project's tables aren't as they were when
    /// something (`what`: "farm.solrec was recorded") was, so it may not
    /// play the same; nothing when that isn't known (`then` is none).
    pub(crate) fn warn_if_tables_changed(&mut self, then: Option<u64>, what: &str) {
        let Some(then) = then else {
            return;
        };
        if then != self.tables_now().hash() {
            self.log(
                LogLevel::Warning,
                format!(
                    "The project's data tables have changed since {what}, so the game may not \
                     play the same: what game code reads from them is different now."
                ),
            );
        }
    }

    /// Opens the table at `path` (inside the project's files) in the Table
    /// tab, beside the level's view on Home, in front.
    pub(crate) fn open_table(&mut self, path: &str) {
        let table = match self.viewport.sheets.tables.get(path) {
            Some(TableState::Read(table, notes)) => {
                let (table, notes) = ((**table).clone(), notes.clone());
                for note in notes {
                    self.log(LogLevel::Warning, format!("{path}: {note}"));
                }
                table
            }
            Some(TableState::Failed(why)) => {
                self.log(
                    LogLevel::Error,
                    format!("Couldn't open {path}: {why}. Fix it in a text editor, or delete it."),
                );
                return;
            }
            None => {
                self.log(
                    LogLevel::Warning,
                    format!("{path} is still being read; open it again in a moment."),
                );
                return;
            }
        };
        if self.open.table_editor.as_ref().map(|e| e.path.as_str()) != Some(path) {
            self.open.table_editor = Some(TableEditor::new(path.to_owned(), table));
        }
        self.show_panel(Panel::Table);
    }

    /// Closes the Table tab (its undo steps go with it).
    pub(crate) fn close_table(&mut self) {
        self.open.table_editor = None;
    }

    /// Saves the open table as its editor has it, shown at once. Its
    /// text (a big table's is megabytes) is written on a worker and handed
    /// to the app once it's ready ([`Self::poll_table_saves`]), so typing
    /// in a table of 10,000 rows stays within the frame.
    pub(crate) fn save_table(&mut self) {
        let Some(editor) = &self.open.table_editor else {
            return;
        };
        let path = editor.path.clone();
        let table = Arc::new(editor.file.clone());
        let file = OwnFile::Table(Arc::clone(&table));
        let (send, text) = std::sync::mpsc::channel();
        let written = std::thread::Builder::new()
            .name("table text".into())
            .spawn(move || {
                let _ = send.send(level::table::file::write(&table));
            });
        if written.is_err() {
            // No thread to be had: written here instead.
            let text = level::table::file::write(&editor.file);
            self.save_file(&path, file, text);
            return;
        }
        self.open.unanswered.sent(&path, file.clone());
        // Nothing the level's problems or a prefab's part look at changed,
        // so `sources` stays as it is.
        self.show_file(&path, file.clone());
        self.open.table_saves.0.push_back((path, file, text));
    }

    /// Hands the app each table save whose text is ready, in the order
    /// they were made; with `wait`, waits for every one (before the
    /// project is left or the editor closes).
    pub(crate) fn poll_table_saves(&mut self, wait: bool) {
        while let Some((_, _, text)) = self.open.table_saves.0.front() {
            let ready = if wait {
                text.recv()
                    .map_err(|_| std::sync::mpsc::TryRecvError::Disconnected)
            } else {
                text.try_recv()
            };
            let text = match ready {
                Ok(text) => Some(text),
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => None,
            };
            let Some((path, file, _)) = self.open.table_saves.0.pop_front() else {
                return;
            };
            match text {
                Some(text) => self.open.outbox.push(WorkbenchAction::SaveFile {
                    path: path.into(),
                    file,
                    text,
                }),
                None => {
                    let text = match &file {
                        OwnFile::Table(table) => level::table::file::write(table),
                        _ => continue,
                    };
                    self.open.outbox.push(WorkbenchAction::SaveFile {
                        path: path.into(),
                        file,
                        text,
                    });
                }
            }
        }
    }

    /// Says what the open table's last change had to say, and saves it.
    pub(crate) fn table_changed(&mut self) {
        let said = self
            .open
            .table_editor
            .as_mut()
            .map(|e| std::mem::take(&mut e.said))
            .unwrap_or_default();
        for line in said {
            self.log(LogLevel::Warning, line);
        }
        self.save_table();
    }

    /// Undoes (or redoes) the open table's last change.
    pub(crate) fn table_undo(&mut self, redo: bool, undo: bool) {
        let Some(editor) = &mut self.open.table_editor else {
            return;
        };
        let changed = if redo {
            editor.redo()
        } else if undo {
            editor.undo()
        } else {
            false
        };
        if changed {
            self.save_table();
        }
    }

    /// Makes the open table follow its file (changed by hand, or by Git).
    pub(crate) fn follow_table(
        &mut self,
        sheets: &crate::sprite_sheets::SpriteSheets,
    ) -> Option<Told> {
        let editor = self.open.table_editor.as_mut()?;
        let followed = follow(editor, sheets);
        let told = Told {
            path: editor.path.clone(),
            followed,
            tabs: Some("the Table tab"),
            many: false,
        };
        if followed == Followed::Gone {
            self.close_table();
        }
        Some(told)
    }

    /// Makes a new table in `folder` (`table.table.lst`, or the first free
    /// name), with a Name column and one row, saves it, shows it in the
    /// Library being named, and opens it.
    pub(crate) fn new_table(&mut self, folder: &str) {
        let ending =
            level::table::file::KIND.to_owned() + "." + reflect::data_files::LUST_EXTENSION;
        let path = crate::workbench_tiles::free_path(folder, "table", &ending, |p| self.taken(p));
        let stem = reflect::data_files::stem(std::path::Path::new(&path)).unwrap_or_default();
        let mut table = level::table::Table::new(level::table::row_name_for(&stem));
        table.columns.push(level::table::Column::new(
            "name",
            level::table::ColumnKind::Text,
        ));
        let id = table.free_id(&table.row_name.clone());
        table.rows.push(Arc::new(table.new_row(id)));
        let text = level::table::file::write(&table);
        self.save_file(&path, OwnFile::Table(Arc::new(table)), text);
        self.show_in_library(&path);
        self.open_table(&path);
    }

    /// Exports the open table as CSV beside it (`items.csv`), replacing
    /// one there.
    pub(crate) fn export_table_csv(&mut self) {
        let Some(editor) = &self.open.table_editor else {
            return;
        };
        let path = csv_path(&editor.path);
        let text = level::table::text::to_csv(&editor.file);
        self.open.outbox.push(WorkbenchAction::ExportCsv {
            path: path.clone().into(),
            text,
        });
        self.log(
            LogLevel::Info,
            format!("Exported the table as {path}: open it in a spreadsheet, and Import CSV takes it back."),
        );
    }

    /// Asks the app for the CSV beside the open table (`items.csv`), to
    /// take in ([`Workbench::import_table_csv`]).
    pub(crate) fn ask_table_csv(&mut self) {
        let Some(editor) = &self.open.table_editor else {
            return;
        };
        self.open.outbox.push(WorkbenchAction::ReadCsv {
            path: csv_path(&editor.path).into(),
        });
    }

    /// Takes the CSV `text` (read from `path`) into the open table, as one
    /// undo step; or says why it couldn't.
    pub fn import_table_csv(&mut self, path: &str, text: Result<String, String>) {
        let Some(editor) = &mut self.open.table_editor else {
            return;
        };
        let done = text.and_then(|text| editor.import_csv(&text));
        match done {
            Ok(done) => {
                self.log(
                    LogLevel::Info,
                    format!(
                        "Took in {path}: {} rows added, {} changed, {} columns added.",
                        done.added, done.changed, done.columns
                    ),
                );
                self.table_changed();
            }
            Err(why) => self.log(LogLevel::Error, format!("Couldn't take in {path}: {why}.")),
        }
    }
}

/// The open table's saves whose text is still being written: the file's
/// path, the table, and where its text will come.
#[derive(Debug, Default)]
pub(crate) struct TableSaves(
    pub(crate) std::collections::VecDeque<(String, OwnFile, std::sync::mpsc::Receiver<String>)>,
);

impl TableSaves {
    /// Whether none is waiting.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The CSV beside a table: `data/items.csv` for `data/items.table.lst`.
pub(crate) fn csv_path(table: &str) -> String {
    format!("{}.csv", reflect::data_files::path_stem(table))
}
