//! Undo and redo for an editor of one small file (a tileset, a clips file,
//! an atlas, an animator, a screen): each step keeps the whole file as it
//! was, which for files this small is simpler and safer than undoing each
//! kind of change. One history for every such editor, so a fix reaches them
//! all (rule 21); each editor keeps its own limit and what it does after an
//! undo (letting go of picks, running the file again).
//!
//! The Map tab's steps go through it too, by hand ([`FileHistory::push`],
//! [`FileHistory::take`], [`FileHistory::put`]): its maps live with the
//! other files read from disk, a step names which map it was, and an undo
//! that can't be saved goes back where it was.

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
        self.push(before);
        true
    }

    /// Keeps `before`, the file as it was before a change made elsewhere,
    /// as an undo step: the oldest goes past the limit, and the redo steps
    /// are let go.
    pub(crate) fn push(&mut self, before: T) {
        self.undo.push_back(before);
        if self.undo.len() > self.limit {
            self.undo.pop_front();
        }
        self.redo.clear();
    }

    /// Lets the oldest undo steps go, keeping the newest, while `too_much`
    /// says the steps kept hold too much.
    pub(crate) fn drop_oldest_while(&mut self, too_much: impl Fn(&VecDeque<T>) -> bool) {
        while self.undo.len() > 1 && too_much(&self.undo) {
            self.undo.pop_front();
        }
    }

    /// Takes the last redo step out (or the last undo step), to be put in
    /// force by hand.
    pub(crate) fn take(&mut self, redo: bool) -> Option<T> {
        if redo {
            self.redo.pop()
        } else {
            self.undo.pop_back()
        }
    }

    /// Puts `step` last among the redo steps (or the undo steps), the
    /// others kept: the file as it was before an undo (or redo) made by
    /// hand, or a step taken out that couldn't be put in force.
    pub(crate) fn put(&mut self, redo: bool, step: T) {
        if redo {
            self.redo.push(step);
        } else {
            self.undo.push_back(step);
        }
    }

    /// Puts `file` back as it was before the last change. Whether there
    /// was one.
    pub(crate) fn undo(&mut self, file: &mut T) -> bool {
        let Some(before) = self.take(false) else {
            return false;
        };
        self.put(true, std::mem::replace(file, before));
        true
    }

    /// Makes the last change undone again. Whether there was one.
    pub(crate) fn redo(&mut self, file: &mut T) -> bool {
        let Some(after) = self.take(true) else {
            return false;
        };
        self.put(false, std::mem::replace(file, after));
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

    /// How many undo and redo steps there are.
    #[cfg(test)]
    pub(crate) fn steps(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }
}

#[cfg(test)]
#[path = "file_history_tests.rs"]
mod tests;
