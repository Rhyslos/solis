//! The workbench's side of the Atlas Crop tab (decision 0089): opening a
//! picture's crops (making its atlas file if it has none), saving each
//! change, undo, a file changed on disk, and the picture as a texture.

use crate::atlas_editor::AtlasEditor;
use crate::atlas_view::AtlasPicture;
use crate::console::LogLevel;
use crate::layout;
use crate::panel::Panel;
use crate::sprite_sheets::{ArtState, AtlasState};
use crate::workbench::{Workbench, WorkbenchAction};
use std::sync::Arc;

/// The open picture's texture: the art it was made from, and the texture.
pub(crate) type AtlasTexture = Option<(Arc<assets::tile_art::TileArt>, egui::TextureHandle)>;

impl Workbench {
    /// Opens the crops of the picture at `picture` (inside the assets) in
    /// the Atlas Crop tab, beside the viewport, making its atlas file
    /// (`tools.atlas.lst` beside `tools.png`) if it has none.
    pub(crate) fn open_atlas(&mut self, picture: &str) {
        let path = animation::atlas::file_for(picture);
        let atlas = match self.viewport.sheets.atlases.get(&path) {
            Some(AtlasState::Read(atlas, _)) => (**atlas).clone(),
            Some(AtlasState::Failed(why)) => {
                self.log(
                    LogLevel::Error,
                    format!("Couldn't open {path}: {why}. Fix it in a text editor, or delete it."),
                );
                return;
            }
            None => {
                let name = picture.rsplit('/').next().unwrap_or(picture);
                animation::atlas::Atlas::new(name)
            }
        };
        if self
            .open
            .atlas_editor
            .as_ref()
            .is_none_or(|e| e.path != path)
        {
            let new = !self.viewport.sheets.atlases.contains_key(&path);
            self.open.atlas_editor = Some(AtlasEditor::new(path, atlas));
            if new {
                // Made at once, so the picture is read for the tab.
                self.save_open_atlas();
            }
        }
        layout::open_first_beside(&mut self.dock, Panel::AtlasCrop, Panel::Viewport);
    }

    /// Closes the atlas open, and its tab.
    pub(crate) fn close_atlas(&mut self) {
        self.open.atlas_editor = None;
        self.open.atlas_texture = None;
        layout::close(&mut self.dock, Panel::AtlasCrop);
    }

    /// Saves the open atlas, and keeps it as read.
    pub(crate) fn save_open_atlas(&mut self) {
        let Some(editor) = &mut self.open.atlas_editor else {
            return;
        };
        let text = editor.saving();
        let (path, atlas) = (editor.path.clone(), editor.atlas.clone());
        self.open.outbox.push(WorkbenchAction::SaveAtlas {
            path: path.clone().into(),
            text,
        });
        Arc::make_mut(&mut self.viewport.sheets)
            .atlases
            .insert(path, AtlasState::Read(Arc::new(atlas), Vec::new()));
    }

    /// Undoes (or redoes) the atlas's last change, from Ctrl+Z (or Ctrl+Y)
    /// over its tab.
    pub(crate) fn atlas_undo(&mut self, redo: bool) {
        let Some(editor) = &mut self.open.atlas_editor else {
            return;
        };
        if if redo { editor.redo() } else { editor.undo() } {
            self.save_open_atlas();
        }
    }

    /// After the app reads the project's atlases again: the open file
    /// changed on disk is taken in as one undo step; one gone closes.
    pub(crate) fn atlas_read_again(&mut self) {
        let Some(editor) = &mut self.open.atlas_editor else {
            return;
        };
        match self.viewport.sheets.atlases.get(&editor.path) {
            Some(AtlasState::Read(atlas, _)) if editor.is_own(atlas) => editor.on_disk = true,
            Some(AtlasState::Read(atlas, _)) => {
                editor.on_disk = true;
                let path = editor.path.clone();
                editor.taken_in((**atlas).clone());
                self.log(
                    LogLevel::Info,
                    format!(
                        "{path} changed on disk, so the Atlas Crop tab shows it as it is now \
                         (Ctrl+Z over the tab undoes that)."
                    ),
                );
            }
            None if editor.on_disk => {
                let path = editor.path.clone();
                self.close_atlas();
                self.log(
                    LogLevel::Warning,
                    format!("{path} is gone, so the Atlas Crop tab closed."),
                );
            }
            _ => {}
        }
    }

    /// The open atlas's picture's path inside the assets.
    fn atlas_picture_path(&self) -> Option<String> {
        let editor = self.open.atlas_editor.as_ref()?;
        Some(animation::atlas::picture_path(&editor.path, &editor.atlas))
    }

    /// Makes (once) the texture of the open atlas's picture (its first
    /// frame), or lets it go.
    pub(crate) fn atlas_texture_now(&mut self, ctx: &egui::Context) {
        let art =
            self.atlas_picture_path()
                .and_then(|p| match self.viewport.sheets.tile_art.get(&p) {
                    Some(ArtState::Read(art)) => Some(Arc::clone(art)),
                    _ => None,
                });
        let Some(art) = art else {
            self.open.atlas_texture = None;
            return;
        };
        if self
            .open
            .atlas_texture
            .as_ref()
            .is_some_and(|(had, _)| Arc::ptr_eq(had, &art))
        {
            return;
        }
        let Some(rgba) = art.frames.first() else {
            self.open.atlas_texture = None;
            return;
        };
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [art.size[0] as usize, art.size[1] as usize],
            rgba,
        );
        let texture = ctx.load_texture("atlas picture", image, egui::TextureOptions::NEAREST);
        self.open.atlas_texture = Some((art, texture));
    }

    /// What the tab draws the picture from, or why it can't.
    pub(crate) fn atlas_picture(&self) -> (Option<AtlasPicture<'_>>, Option<String>) {
        if let Some((art, texture)) = &self.open.atlas_texture
            && let Some(rgba) = art.frames.first()
        {
            return (
                Some(AtlasPicture {
                    texture,
                    size: art.size,
                    rgba,
                }),
                None,
            );
        }
        let problem = match self
            .atlas_picture_path()
            .map(|p| (self.viewport.sheets.tile_art.get(&p), p))
        {
            Some((Some(ArtState::Failed(why)), _)) => {
                Some(format!("Couldn't read the picture: {why}"))
            }
            Some((None, p)) if !self.open.library.iter().any(|i| i.path == p) => Some(format!(
                "The picture {p} isn't in the project: an atlas file sits beside its picture."
            )),
            _ => None,
        };
        (None, problem)
    }
}

#[cfg(test)]
#[path = "workbench_atlas_tests.rs"]
mod tests;
