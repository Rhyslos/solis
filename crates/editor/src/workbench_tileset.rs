//! The workbench's side of tilesets being changed (Phase 9, D5): one
//! editor for each tileset file, with that file's undo steps, whichever
//! place changes it (the Tileset tab, its Properties, the Map tab);
//! opening a tileset into the tab and closing it, saving each change and
//! showing it at once on every map (`Workbench::save_file`), undo, a file
//! changed on disk (`own_files::follow`), the picture as a texture, and
//! saving a kind's template beside the tileset.

use crate::console::LogLevel;
use crate::key_owner::KeyOwner;
use crate::layout;
use crate::made_from::MadeFrom;
use crate::map_tab::TerrainEdit;
use crate::own_files::{Followed, OwnFile, Told, follow};
use crate::panel::Panel;
use crate::sprite_sheets::{ArtState, SpriteSheets, TilesetState};
use crate::tileset_editor::TilesetEditor;
use crate::tileset_view::{Picture, TilesetUi};
use crate::workbench::{Workbench, WorkbenchAction};
use assets::tile_art::TileArt;
use std::collections::HashMap;
use std::sync::Arc;
use tiles::TileKind;

/// The textures of the pictures the Tileset and Map tabs show: for each,
/// the art it was made from, the texture, and its size in texels.
pub(crate) type TileTextures = Vec<(MadeFrom<TileArt>, egui::TextureHandle, [u32; 2])>;

/// The texture made from `art`, and its size in texels.
pub(crate) fn texture_of<'a>(
    textures: &'a TileTextures,
    art: &Arc<TileArt>,
) -> Option<(&'a egui::TextureHandle, [u32; 2])> {
    textures
        .iter()
        .find(|(from, ..)| from.is(art))
        .map(|(_, texture, size)| (texture, *size))
}

/// The project's tilesets as they are being changed: an editor for each
/// file opened or changed since the project was, holding that file's undo
/// steps. Every change to a tileset goes through its editor, wherever it
/// is made (ground rule 11: edits are undoable), so Ctrl+Z over the
/// Tileset tab or over Properties showing the tileset undoes it. A file's
/// steps last while the file is in the open project: they go with the
/// file (deleted, moved or renamed) and with the project.
#[derive(Debug, Default)]
pub(crate) struct TilesetEdits {
    by_path: HashMap<String, TilesetEditor>,
    /// The file open in the Tileset tab, by its path.
    tab: Option<String>,
    /// The tileset whose terrains Properties showed when it was last
    /// drawn (the Tileset tab's, or the one picked in the Library): undo
    /// and redo over Properties are that tileset's then.
    pub(crate) in_properties: Option<String>,
}

impl TilesetEdits {
    /// The tileset open in the Tileset tab.
    pub(crate) fn tab(&self) -> Option<&TilesetEditor> {
        self.by_path.get(self.tab.as_deref()?)
    }

    /// The tileset open in the Tileset tab, to change.
    pub(crate) fn tab_mut(&mut self) -> Option<&mut TilesetEditor> {
        self.by_path.get_mut(self.tab.as_deref()?)
    }

    /// The path of the tileset whose undo steps Ctrl+Z is for, for who
    /// has the keys: the Tileset tab's, or the one Properties shows.
    pub(crate) fn undone_by(&self, keys: KeyOwner) -> Option<&str> {
        match keys {
            KeyOwner::Tileset => self.tab.as_deref(),
            KeyOwner::TilesetProperties => self.in_properties.as_deref(),
            _ => None,
        }
    }
}

/// What the tab and its Properties draw from, for the file in `editor`.
pub(crate) fn tileset_ui<'a>(
    editor: &'a mut TilesetEditor,
    sheets: &'a SpriteSheets,
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
        let (texture, atlas) = texture_of(textures, art)?;
        Some(Picture {
            texture,
            atlas,
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
    /// The editor of the tileset at `path` (inside the assets), made from
    /// the file as read the first time it's wanted.
    fn tileset_editor(&mut self, path: &str) -> Option<&mut TilesetEditor> {
        let editors = &mut self.open.tilesets.by_path;
        if !editors.contains_key(path) {
            let Some(TilesetState::Read(file, _)) = self.viewport.sheets.tilesets.get(path) else {
                return None;
            };
            let editor = TilesetEditor::new(path.to_owned(), (**file).clone());
            editors.insert(path.to_owned(), editor);
        }
        editors.get_mut(path)
    }

    /// Opens the tileset at `path` (inside the assets) in the Tileset tab,
    /// beside the viewport, in front.
    pub(crate) fn open_tileset(&mut self, path: &str) {
        match self.viewport.sheets.tilesets.get(path) {
            Some(TilesetState::Read(..)) => {}
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
        }
        if self.tileset_editor(path).is_some() {
            self.open.tilesets.tab = Some(path.to_owned());
            layout::open_first_beside(&mut self.dock, Panel::Tileset, Panel::Viewport);
        }
    }

    /// Closes the Tileset tab. The file's undo steps are kept: they are
    /// the file's, not the tab's.
    pub(crate) fn close_tileset(&mut self) {
        self.open.tilesets.tab = None;
        layout::close(&mut self.dock, Panel::Tileset);
    }

    /// Changes the tileset at `path` with `change`, as one undo step of
    /// that file's, saved and shown at once. This is the one way a tileset
    /// is changed. Whether it changed.
    pub(crate) fn change_tileset(
        &mut self,
        path: &str,
        change: impl FnOnce(&mut TilesetEditor) -> bool,
    ) -> bool {
        let changed = self.tileset_editor(path).is_some_and(change);
        if changed {
            self.save_tileset(path);
        }
        changed
    }

    /// Makes a change to a tileset's terrains asked for in its Properties.
    pub(crate) fn edit_terrains(&mut self, path: &str, edit: TerrainEdit) {
        self.change_tileset(path, |editor| match edit {
            TerrainEdit::Add => editor.add_terrain(),
            TerrainEdit::Remove(id) => editor.remove_terrain(id),
            TerrainEdit::Set {
                id,
                name,
                code,
                colour,
            } => editor.set_terrain(id, name, code, colour),
        });
    }

    /// Saves the tileset at `path` as its editor has it, and shows it at
    /// once on every map.
    pub(crate) fn save_tileset(&mut self, path: &str) {
        let Some(editor) = self.open.tilesets.by_path.get(path) else {
            return;
        };
        let (text, file) = (editor.text(), editor.file.clone());
        self.save_file(path, OwnFile::Tileset(Arc::new(file)), text);
    }

    /// Saves the tileset open in the Tileset tab, changed there.
    pub(crate) fn save_open_tileset(&mut self) {
        if let Some(path) = self.open.tilesets.tab.clone() {
            self.save_tileset(&path);
        }
    }

    /// Undoes (or redoes) the last change to the tileset at `path`,
    /// wherever it was made: from Ctrl+Z (or Ctrl+Y) over the Tileset tab,
    /// or over Properties showing the tileset.
    pub(crate) fn tileset_undo(&mut self, path: &str, redo: bool, undo: bool) {
        let Some(editor) = self.open.tilesets.by_path.get_mut(path) else {
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
            self.save_tileset(path);
        }
    }

    /// Makes each tileset's editor follow its file as `sheets` hold it
    /// (changed by hand, or by Git): one that differs is taken in as one
    /// undo step; one gone takes its undo steps with it, and closes the
    /// Tileset tab if it was open there. Gives what each did.
    pub(crate) fn follow_tilesets(&mut self, sheets: &SpriteSheets) -> Vec<Told> {
        let mut paths: Vec<String> = self.open.tilesets.by_path.keys().cloned().collect();
        paths.sort();
        let mut told = Vec::new();
        for path in paths {
            let in_tab = self.open.tilesets.tab.as_deref() == Some(path.as_str());
            let Some(editor) = self.open.tilesets.by_path.get_mut(&path) else {
                continue;
            };
            let followed = follow(editor, sheets);
            if followed == Followed::Gone {
                self.open.tilesets.by_path.remove(&path);
                if in_tab {
                    self.close_tileset();
                }
            }
            told.push(Told {
                path,
                followed,
                tabs: in_tab.then_some("the Tileset tab"),
                many: false,
            });
        }
        told
    }

    /// Makes (once) the textures of the open tileset's picture and of the
    /// selected map's tileset's (the Map tab shows its tiles, D6), and
    /// lets others go.
    pub(crate) fn tileset_texture(&mut self, ctx: &egui::Context) {
        let sheets = &self.viewport.sheets;
        let open = self.open.tilesets.tab().and_then(|e| e.file.picture);
        let mapped = self
            .selected_map_path()
            .and_then(|(_, path)| self.map_file(&path))
            .and_then(|m| m.tileset)
            .and_then(|t| sheets.tileset(t))
            .and_then(|t| t.picture);
        let arts: Vec<Arc<TileArt>> = [open, mapped]
            .into_iter()
            .flatten()
            .filter_map(|p| sheets.tile_art(p).cloned())
            .collect();
        self.open
            .tile_textures
            .retain(|(from, ..)| arts.iter().any(|art| from.is(art)));
        for art in &arts {
            if texture_of(&self.open.tile_textures, art).is_some() {
                continue;
            }
            let (size, rgba) = crate::tile_view::atlas_rgba(art);
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [size[0] as usize, size[1] as usize],
                &rgba,
            );
            let texture = ctx.load_texture("tileset picture", image, egui::TextureOptions::NEAREST);
            self.open
                .tile_textures
                .push((MadeFrom::new(art), texture, size));
        }
    }

    /// Saves `kind`'s template at the open tileset's tile size, as a PNG
    /// beside it with a free name.
    pub(crate) fn save_template(&mut self, kind: TileKind) {
        let Some(editor) = self.open.tilesets.tab() else {
            return;
        };
        let folder = editor.path.rsplit_once('/').map_or("", |(f, _)| f);
        let stem = format!("{} template", kind.word());
        let base = if folder.is_empty() {
            String::new()
        } else {
            format!("{folder}/")
        };
        let taken = |p: &str| self.open.library.iter().any(|i| i.path == p);
        let path = std::iter::once(format!("{base}{stem}.png"))
            .chain((2..).map(|n| format!("{base}{stem} {n}.png")))
            .find(|p| !taken(p))
            .unwrap_or_default();
        let (size, rgba) =
            tiles::autotile::template_rgba(kind, editor.file.tile, [107, 74, 46], [79, 122, 58]);
        match assets::tile_art::png_of(size, &rgba) {
            Ok(png) => {
                self.open.outbox.push(WorkbenchAction::SavePicture {
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
