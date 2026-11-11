//! Who the keys belong to. Several parts of the editor act on the same
//! keys (Delete, Ctrl+D, Ctrl+Z, Esc): the area, and each tab that edits a
//! file of its own. [`Workbench::key_owner`] says once a frame which of
//! them has the keys, from where the pointer rests and what is being
//! typed, and every key handler asks that one answer, so a key is acted
//! on by one part only.
//!
//! | Keys | Acted on by |
//! | --- | --- |
//! | Ctrl+Z, Ctrl+Y, Ctrl+Shift+Z | The owner's own undo steps |
//! | Delete, Ctrl+D, F2, F, Esc, Ctrl+A, X, Y, Z, L, the tools' and the tile painter's keys | The area where [`KeyOwner::is_area`]; over a tab with a file of its own, that tab alone, for the keys it has |
//! | Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S, Space (the search), `'` (the Console) | The editor, where [`KeyOwner::takes_shortcuts`] (over the Frames tab, Space plays the clip instead) |
//! | Ctrl+Space (the Library) | The editor, typing or not, where [`KeyOwner::is_editor`] |

use crate::panel::Panel;
use crate::workbench::Workbench;

/// The part of the editor the keys belong to this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum KeyOwner {
    /// The running game (the Game panel was clicked): every key is its.
    Game,
    /// Keep or Undo all is being asked after Stop: the keys wait for the
    /// answer.
    Question,
    /// The Input actions window is waiting for the input to bind: the
    /// next key is the one bound, and nothing else.
    Binding,
    /// A field being typed in: the keys are its text (Ctrl+Z undoes the
    /// typing).
    Field,
    /// A whole page stands where the area would be (a File page, a
    /// screen that isn't built): the area's keys rest, since nothing they
    /// would change is on screen. The editor's own keys still work.
    Page,
    /// The area: the viewport, the Area panel, and everywhere no other
    /// owner is named (Properties, the Library, the Console, windows).
    #[default]
    Area,
    /// The Map tab, or anywhere that would be the area's while a tile
    /// painting tool is in use: undo and redo are the selected map's.
    /// Every other key is the area's, as the tab shows the selected
    /// entity's map, as Properties shows its components.
    Map,
    /// Properties while it shows a tileset's terrains (the Tileset tab's,
    /// or the one picked in the Library): undo and redo are that
    /// tileset's, so what is changed there is undone there. Every other
    /// key is the area's, as over Properties showing anything else.
    TilesetProperties,
    /// The Clips tab: undo and redo are the clips file's.
    Clips,
    /// The Frames tab: the clips file's undo, Delete and Space.
    Frames,
    /// The Animator tab: the animator's undo and its graph's keys.
    Animator,
    /// The Tileset tab: the tileset's undo.
    Tileset,
    /// The Atlas Crop tab: the crops' undo and Delete.
    Atlas,
    /// The UI editor's Canvas: the screen's undo, its pieces' keys and the
    /// drawing tools' keys.
    Canvas,
    /// The UI editor's Elements: the screen's undo, Delete and Ctrl+D.
    Elements,
}

impl KeyOwner {
    /// Whether the area's own keys are acted on: the tools', the tile
    /// painter's, Delete, Ctrl+D, F2, F, Esc, Ctrl+A and the axis keys.
    pub(crate) fn is_area(self) -> bool {
        matches!(self, Self::Area | Self::Map | Self::TilesetProperties)
    }

    /// Whether the editor acts on keys at all: not while the game has
    /// them, a question waits for its answer, or a key is being bound.
    pub(crate) fn is_editor(self) -> bool {
        !matches!(self, Self::Game | Self::Question | Self::Binding)
    }

    /// Whether the keys that work wherever the pointer rests are acted
    /// on: undo and redo, new, open and save, the search and the Console's
    /// command line. Not while typing, when they'd be typed.
    pub(crate) fn takes_shortcuts(self) -> bool {
        self.is_editor() && self != Self::Field
    }
}

impl Workbench {
    /// Who the keys belong to this frame: worked out here and nowhere
    /// else, at the start of the frame, from the panel the pointer rested
    /// over when the last one was drawn. A tab has them only while its
    /// file is open: an empty Canvas has nothing to act on.
    pub(crate) fn key_owner(&self, ctx: &egui::Context) -> KeyOwner {
        let open = &self.open;
        if self.game_input {
            return KeyOwner::Game;
        }
        if open.play_changes.is_some() {
            return KeyOwner::Question;
        }
        if open
            .input_window
            .as_ref()
            .is_some_and(|window| window.listening.is_some())
        {
            return KeyOwner::Binding;
        }
        if ctx.egui_wants_keyboard_input() {
            return KeyOwner::Field;
        }
        if self.shell.covers_center() {
            return KeyOwner::Page;
        }
        let own = match self.pointer_over {
            Some(Panel::Clips) if open.frames_editor.is_some() => Some(KeyOwner::Clips),
            Some(Panel::Frames) if open.frames_editor.is_some() => Some(KeyOwner::Frames),
            Some(Panel::Animator) if open.animator_editor.is_some() => Some(KeyOwner::Animator),
            Some(Panel::Tileset) if open.tilesets.tab().is_some() => Some(KeyOwner::Tileset),
            Some(Panel::Properties) if open.tilesets.in_properties.is_some() => {
                Some(KeyOwner::TilesetProperties)
            }
            Some(Panel::AtlasCrop) if open.atlas_editor.is_some() => Some(KeyOwner::Atlas),
            Some(Panel::Canvas) if open.screen_doc.is_some() => Some(KeyOwner::Canvas),
            Some(Panel::Elements) if open.screen_doc.is_some() => Some(KeyOwner::Elements),
            _ => None,
        };
        own.unwrap_or_else(|| {
            let painting = self.viewport.tile_overlay.paint.painting();
            let over_the_map = self.pointer_over == Some(Panel::Map);
            if open.map_edits.path.is_some() && (over_the_map || painting) {
                KeyOwner::Map
            } else {
                KeyOwner::Area
            }
        })
    }
}

#[cfg(test)]
#[path = "key_owner_tests.rs"]
mod tests;
