//! Choosing a layout (decision 0118, board 06): a click on its card
//! applies it at once, on every tab, and a small note offers Undo, which
//! puts back the layout before it and every tab's hidden panels and
//! folds as they were. Reset sizes forgets every edge dragged in the
//! layout in use.

use super::layouts::PanelLayout;
use super::workspaces::{Arrangement, Hidden};
use super::{RibbonTab, Shell};
use crate::viewport::ViewLayout;
use crate::workbench::Workbench;
use std::collections::BTreeMap;

/// What a layout's choice changed, for its Undo.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayoutUndo {
    /// The layout chosen.
    pub(crate) now: PanelLayout,
    /// The layout before it.
    before: PanelLayout,
    /// Every tab's arrangement before it.
    arrangements: BTreeMap<RibbonTab, Arrangement>,
    /// How many views the centre had before it.
    views: ViewLayout,
}

impl Shell {
    /// Every tab there is a frame for, the contextual ones too.
    fn every_tab() -> impl Iterator<Item = RibbonTab> {
        RibbonTab::ALL
            .into_iter()
            .chain(RibbonTab::CONTEXT)
            .filter(|tab| *tab != RibbonTab::File)
    }
}

impl Workbench {
    /// The layout in use.
    pub fn layout(&self) -> PanelLayout {
        self.interface.layout
    }

    /// Applies `layout` at once, on every tab: every hidden panel shown
    /// again, or with Just the view every panel hidden and every bottom
    /// panel folded; the centre's views two for Two views (and one again
    /// when leaving it). Keeps what it changed for Undo.
    pub fn choose_layout(&mut self, ctx: &egui::Context, layout: PanelLayout) {
        let before = self.interface.layout;
        let undo = LayoutUndo {
            now: layout,
            before,
            arrangements: self.shell.arrangements.clone(),
            views: self.viewport.layout,
        };
        self.choose_interface_look(
            ctx,
            crate::interface_look::InterfaceLook {
                layout,
                ..self.interface
            },
        );
        for tab in Shell::every_tab() {
            let standard = Arrangement::standard(tab);
            let arrangement = self.shell.arrangement_mut(tab);
            if layout.hides_everything() {
                arrangement.hidden = Hidden::ALL;
                arrangement.bottom_folded = true;
                arrangement.placing = false;
            } else {
                arrangement.hidden = Hidden::default();
                if before.hides_everything() {
                    arrangement.bottom_folded = standard.bottom_folded;
                }
            }
        }
        if layout.two_views() {
            self.viewport.layout = ViewLayout::SideBySide;
            self.viewport.maximized = None;
        } else if before.two_views() {
            self.viewport.layout = ViewLayout::One;
            self.viewport.maximized = None;
        }
        self.shell.layout_undo = Some(undo);
    }

    /// Puts back what the last layout chosen changed, and forgets it.
    pub fn undo_layout(&mut self, ctx: &egui::Context) {
        let Some(undo) = self.shell.layout_undo.take() else {
            return;
        };
        self.choose_interface_look(
            ctx,
            crate::interface_look::InterfaceLook {
                layout: undo.before,
                ..self.interface
            },
        );
        self.shell.arrangements = undo.arrangements;
        self.viewport.layout = undo.views;
        self.viewport.maximized = None;
    }

    /// The layout the note under the cards names, while it shows.
    pub(crate) fn layout_note(&self) -> Option<PanelLayout> {
        self.shell.layout_undo.as_ref().map(|undo| undo.now)
    }

    /// Puts the note away (its ×), keeping the layout.
    pub(crate) fn close_layout_note(&mut self) {
        self.shell.layout_undo = None;
    }

    /// Puts every dragged edge of every tab back where the layout in use
    /// starts.
    pub fn reset_sizes(&mut self) {
        let layout = self.interface.layout;
        self.shell.sizes.forget_layout(layout);
    }
}

#[cfg(test)]
#[path = "layout_choice_tests.rs"]
mod tests;
