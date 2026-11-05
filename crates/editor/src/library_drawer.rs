//! The Library as a drawer (the editor overhaul's O3, decision 0085): pinned,
//! it's a docked panel as before; unpinned, it leaves the layout and
//! **Ctrl + Space** pops it up over the bottom of the editor, as Unreal's
//! content drawer does. It tucks away again on Ctrl + Space, Esc, a click
//! outside it, or as soon as a file is dragged out of it, so the drop lands
//! on what's behind. Ctrl + Space with the Library pinned brings it to the
//! front.

use crate::layout;
use crate::library::{LibraryRequest, Pin};
use crate::panel::Panel;
use crate::theme;
use crate::workbench::Workbench;
use egui::{Rect, Ui};

/// The most of the editor's height the drawer takes.
const DRAWER_SHARE: f32 = 0.45;
/// The drawer's tallest, in points.
const DRAWER_MAX: f32 = 380.0;

impl Workbench {
    /// Whether the Library is pinned (a docked panel) rather than a drawer.
    pub fn library_pinned(&self) -> bool {
        self.library_pinned
    }

    /// Pins the Library (docks it along the bottom, in front) or unpins it
    /// (takes it out of the layout: Ctrl + Space pops it up). The app
    /// remembers the choice.
    pub fn set_library_pinned(&mut self, pinned: bool) {
        self.library_pinned = pinned;
        self.drawer_open = false;
        if pinned {
            let beside = [Panel::Console, Panel::Problems, Panel::Viewport]
                .into_iter()
                .find(|p| self.dock.find_tab(p).is_some())
                .unwrap_or(Panel::Viewport);
            layout::open_first_beside(&mut self.dock, Panel::Library, beside);
        } else {
            layout::close(&mut self.dock, Panel::Library);
        }
    }

    /// Whether the drawer is popped up.
    pub fn drawer_open(&self) -> bool {
        self.drawer_open
    }

    /// Ctrl + Space: pops the drawer up or tucks it away; with the Library
    /// pinned, brings it to the front. Works while typing too, so it can
    /// be reached from anywhere; not while the game has the keys.
    pub(crate) fn library_keys(&mut self, ui: &Ui) {
        let pressed = ui.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::Space,
            ))
        });
        if !pressed {
            return;
        }
        if self.library_pinned {
            layout::bring_to_front(&mut self.dock, Panel::Library);
        } else {
            self.drawer_open = !self.drawer_open;
        }
    }

    /// Draws the drawer over the bottom of `editor` (the docked panels'
    /// space) while it's popped up, and tucks it away when it should go.
    /// What it asks for joins the Library's requests.
    pub(crate) fn library_drawer(
        &mut self,
        ui: &Ui,
        editor: Rect,
        sheets: &crate::sprite_sheets::SpriteSheets,
        requests: &mut Vec<LibraryRequest>,
    ) {
        if self.library_pinned || !self.drawer_open {
            return;
        }
        let height = (editor.height() * DRAWER_SHARE).min(DRAWER_MAX);
        let rect = Rect::from_min_max(
            egui::pos2(editor.left(), editor.bottom() - height),
            editor.max,
        );
        let mut pin_clicked = false;
        let sounds = crate::library::Sounds {
            states: &self.sounds,
            events: &self.sound_events,
            preview: self.preview.as_ref(),
            mixer: &self.mixer.mixer,
            sheets,
            sheet_pictures: &self.sheet_pictures,
        };
        let shown = egui::Area::new(egui::Id::new("library drawer"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ui.ctx(), |ui| {
                ui.set_clip_rect(rect.expand(1.0));
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .stroke(egui::Stroke::new(1.0, theme::BUTTON_BORDER))
                    .show(ui, |ui| {
                        ui.set_min_size(rect.size() - egui::vec2(2.0, 2.0));
                        ui.set_max_size(rect.size() - egui::vec2(2.0, 2.0));
                        let asked = crate::library::library(
                            ui,
                            self.project.as_ref(),
                            (&self.library, &self.folders),
                            &mut self.library_state,
                            self.accent,
                            false,
                            sounds,
                            (Some(Pin::Drawer), &mut pin_clicked),
                        );
                        requests.extend(asked);
                    });
            });
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let clicked_outside = ui.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !shown.response.rect.contains(p))
        });
        // A file dragged out: tuck away so it can be dropped on what's
        // behind.
        let dragging_out =
            egui::DragAndDrop::has_payload_of_type::<crate::library::LibraryDrag>(ui.ctx())
                && ui
                    .input(|i| i.pointer.hover_pos())
                    .is_some_and(|p| !shown.response.rect.contains(p));
        // Opening a file or placing one is what it was popped up for.
        let done = requests
            .iter()
            .any(|r| matches!(r, LibraryRequest::Place(_) | LibraryRequest::Open(..)));
        if pin_clicked {
            self.set_library_pinned(true);
        } else if escape || clicked_outside || dragging_out || done {
            self.drawer_open = false;
        }
    }
}

#[cfg(test)]
#[path = "library_drawer_tests.rs"]
mod tests;
