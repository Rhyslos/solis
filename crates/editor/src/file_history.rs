//! Undo and redo for an editor of one small file (a tileset, a clips file,
//! an atlas, an animator): each step keeps the whole file as it was, which
//! for files this small is simpler and safer than undoing each kind of
//! change. One history for every such editor, so a fix reaches them all
//! (rule 21); each editor keeps its own limit and what it does after an
//! undo (letting go of picks, running the file again).

use std::collections::VecDeque;

/// The undo and redo steps of one file being edited.
#[derive(Debug, Clone)]
pub(crate) struct FileHistory<T> {
    /// The file before each change, oldest first.
    undo: VecDeque<T>,
    /// The file after each change undone, newest last.
    redo: Vec<T>,
    /// The most undo steps kept: the oldest goes first.
    limit: usize,
}

impl<T: Clone + PartialEq> FileHistory<T> {
    /// No steps yet, keeping at most `limit` (at least one).
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// Changes `file` with `change` as one step. A change that says it
    /// made nothing (returns false), or leaves the file as it was, isn't a
    /// step, and the file is put back as it was. Whether it changed.
    pub(crate) fn change(&mut self, file: &mut T, change: impl FnOnce(&mut T) -> bool) -> bool {
        let before = file.clone();
        if !change(file) || *file == before {
            *file = before;
            return false;
        }
        self.undo.push_back(before);
        if self.undo.len() > self.limit {
            self.undo.pop_front();
        }
        self.redo.clear();
        true
    }

    /// Puts `file` back as it was before the last change. Whether there
    /// was one.
    pub(crate) fn undo(&mut self, file: &mut T) -> bool {
        let Some(before) = self.undo.pop_back() else {
            return false;
        };
        self.redo.push(std::mem::replace(file, before));
        true
    }

    /// Makes the last change undone again. Whether there was one.
    pub(crate) fn redo(&mut self, file: &mut T) -> bool {
        let Some(after) = self.redo.pop() else {
            return false;
        };
        self.undo.push_back(std::mem::replace(file, after));
        true
    }

    /// Whether there's a change to undo.
    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether there's a change to redo.
    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
#[path = "file_history_tests.rs"]
mod tests;
