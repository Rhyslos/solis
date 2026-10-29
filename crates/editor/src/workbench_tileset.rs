//! The workbench's side of the Tileset tab (Phase 9, D5): opening a
//! tileset into it and closing it, saving each change and showing it at
//! once on every map, undo, a file changed on disk, the picture as a
//! texture, and saving a kind's template beside the tileset.

use crate::console::LogLevel;
use crate::layout;
use crate::panel::Panel;
use crate::sprite_sheets::{ArtState, TilesetState};
use crate::tileset_editor::TilesetEditor;
use crate::tileset_view::{Picture, TilesetUi};
use crate::workbench::{Workbench, WorkbenchAction};
use std::collections::HashMap;
use std::sync::Arc;
use tiles::TileKind;

/// A picture's texture for the Tileset tab, by its key (the art read):
/// the texture, and its size in texels.
pub(crate) type TileTextures = HashMap<u64, (egui::TextureHandle, [u32; 2])>;

/// What the tab and its Properties draw from, for the file in `editor`.
pub(crate) fn tileset_ui<'a>(
    editor: &'a mut TilesetEditor,
    sheets: &'a crate::sprite_sheets::SpriteSheets,
    textures: &'a TileTextures,
    project_tile: [u32; 2],
    ms: u64,
    accent: egui::Color32,
) -> TilesetUi<'a> {
    let path_of = editor.file.picture.and_then(|p| sheets.paths.get(&p));
    let art = editor.file.picture.and_then(|p| sheets.tile_art(p));
    let picture_problem = match path_of.and_then(|p| sheets.tile_art.get(p)) {
        Some(ArtState::Failed(why)) => Some(format!("Couldn't read the picture: {why}")),
        _ if editor.file.picture.is_some() && path_of.is_none() => {
            Some("The picture isn't in the project any more.".to_owned())
        }
        _ => None,
    };
    let picture = art.and_then(|art| {
        let (texture, atlas) = textures.get(&(Arc::as_ptr(art) as u64))?;
        Some(Picture {
            texture,
            atlas: *atlas,
            size: art.size,
            durations: &art.durations,
            art,
        })
    });
    let notes = match sheets.tilesets.get(&editor.path) {
        Some(TilesetState::Read(_, notes)) => notes.clone(),
        _ => Vec::new(),
    };
    TilesetUi {
        editor,
        picture,
        pictures: sheets.pictures(),
        picture_problem,
        project_tile,
        notes,
        ms,
        accent,
    }
}

impl Workbench {
    /// Opens the tileset at `path` (inside the assets) in the Tileset tab,
    /// beside the viewport, in front.
    pub(crate) fn open_tileset(&mut self, path: &str) {
        let file = match self.viewport.sheets.tilesets.get(path) {
            Some(TilesetState::Read(file, _)) => (**file).clone(),
            Some(TilesetState::Failed(why)) => {
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
        if self.tileset_editor.as_ref().is_none_or(|e| e.path != path) {
            self.tileset_editor = Some(TilesetEditor::new(path.to_owned(), file));
        }
        layout::open_first_beside(&mut self.dock, Panel::Tileset, Panel::Viewport);
    }

    /// Closes the tileset open, and its tab.
    pub(crate) fn close_tileset(&mut self) {
        self.tileset_editor = None;
        self.tileset_hovered = false;
        layout::close(&mut self.dock, Panel::Tileset);
    }

    /// Saves the open tileset, and shows it at once on every map.
    pub(crate) fn save_open_tileset(&mut self) {
        let Some(editor) = &mut self.tileset_editor else {
            return;
        };
        let text = editor.saving();
        let (path, file) = (editor.path.clone(), editor.file.clone());
        self.outbox.push(WorkbenchAction::SaveTileset {
            path: path.clone().into(),
            text,
        });
        Arc::make_mut(&mut self.viewport.sheets)
            .tilesets
            .insert(path, TilesetState::Read(Arc::new(file), Vec::new()));
        self.sources += 1;
    }

    /// Undoes (or redoes) the tileset's last change, from Ctrl+Z (or
    /// Ctrl+Y) over its tab.
    pub(crate) fn tileset_undo(&mut self, redo: bool, undo: bool) {
        let Some(editor) = &mut self.tileset_editor else {
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
            self.save_open_tileset();
        }
    }

    /// After the app reads the project's tilesets again: the open file
    /// changed on disk (by hand, or by Git) is taken in as one undo step;
    /// one gone closes.
    pub(crate) fn tileset_read_again(&mut self) {
        let Some(editor) = &mut self.tileset_editor else {
            return;
        };
        match self.viewport.sheets.tilesets.get(&editor.path) {
            Some(TilesetState::Read(file, _)) if editor.is_own(file) => editor.on_disk = true,
            Some(TilesetState::Read(file, _)) => {
                editor.on_disk = true;
                let path = editor.path.clone();
                editor.taken_in((**file).clone());
                self.log(
                    LogLevel::Info,
                    format!(
                        "{path} changed on disk, so the Tileset tab shows it as it is now (Ctrl+Z \
                         over the tab undoes that)."
                    ),
                );
            }
            None if editor.on_disk => {
                let path = editor.path.clone();
                self.close_tileset();
                self.log(
                    LogLevel::Warning,
                    format!("{path} is gone, so the Tileset tab closed."),
                );
            }
            _ => {}
        }
    }

    /// Makes (once) the textures of the open tileset's picture and of the
    /// selected map's tileset's (the Map tab shows its tiles, D6), and
    /// lets others go.
    pub(crate) fn tileset_texture(&mut self, ctx: &egui::Context) {
        let sheets = &self.viewport.sheets;
        let open = self.tileset_editor.as_ref().and_then(|e| e.file.picture);
        let mapped = self
            .selected_map_path()
            .and_then(|(_, path)| self.map_file(&path))
            .and_then(|m| m.tileset)
            .and_then(|t| sheets.tileset(t))
            .and_then(|t| t.picture);
        let arts: Vec<Arc<assets::tile_art::TileArt>> = [open, mapped]
            .into_iter()
            .flatten()
            .filter_map(|p| sheets.tile_art(p).cloned())
            .collect();
        let keys: Vec<u64> = arts.iter().map(|a| Arc::as_ptr(a) as u64).collect();
        self.tile_textures.retain(|k, _| keys.contains(k));
        for (art, key) in arts.iter().zip(keys) {
            if self.tile_textures.contains_key(&key) {
                continue;
            }
            let (size, rgba) = crate::tile_view::atlas_rgba(art);
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [size[0] as usize, size[1] as usize],
                &rgba,
            );
            let texture = ctx.load_texture("tileset picture", image, egui::TextureOptions::NEAREST);
            self.tile_textures.insert(key, (texture, size));
        }
    }

    /// Saves `kind`'s template at the open tileset's tile size, as a PNG
    /// beside it with a free name.
    pub(crate) fn save_template(&mut self, kind: TileKind) {
        let Some(editor) = &self.tileset_editor else {
            return;
        };
        let folder = editor.path.rsplit_once('/').map_or("", |(f, _)| f);
        let stem = format!("{} template", kind.word());
        let base = if folder.is_empty() {
            String::new()
        } else {
            format!("{folder}/")
        };
        let taken = |p: &str| self.library.iter().any(|i| i.path == p);
        let path = std::iter::once(format!("{base}{stem}.png"))
            .chain((2..).map(|n| format!("{base}{stem} {n}.png")))
            .find(|p| !taken(p))
            .unwrap_or_default();
        let (size, rgba) =
            tiles::autotile::template_rgba(kind, editor.file.tile, [107, 74, 46], [79, 122, 58]);
        match assets::tile_art::png_of(size, &rgba) {
            Ok(png) => {
                self.outbox.push(WorkbenchAction::SavePicture {
                    path: path.clone().into(),
                    bytes: png,
                });
                self.log(
                    LogLevel::Info,
                    format!(
                        "Saved {path}: draw each tile over its slot, then choose the picture and \
                         Mark up from the template."
                    ),
                );
            }
            Err(why) => self.log(
                LogLevel::Error,
                format!("Couldn't make the template: {why}."),
            ),
        }
    }
}

#[cfg(test)]
#[path = "workbench_tileset_tests.rs"]
mod tests;
