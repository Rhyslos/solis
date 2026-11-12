//! What the LIBRARY TOOLS ribbons do (the approved additions, round 3):
//! a tileset's terrains, kinds, template and zoom
//! (`02-tileset-editor.png`), and a picture's crops and zoom
//! (`03-picture-crops.png`). Each is a command (`shell::ShellCommand`);
//! every change to a tileset goes through `change_tileset`, one undo step
//! and one save, as from its own tab.

use crate::shell::ShellCommand;
use crate::workbench::Workbench;

impl Workbench {
    /// Why one of the LIBRARY TOOLS commands can't be done now.
    pub(crate) fn why_not_library_tool(&self, command: ShellCommand) -> Option<String> {
        use ShellCommand as C;
        let say = |why: &str| Some(why.to_owned());
        match command {
            C::TerrainAdd | C::TilesetZoom(_) => match self.open.tilesets.tab() {
                None => say("Open a tileset from the Library first"),
                Some(e) if e.file.terrains.len() >= tiles::tileset::MAX_TERRAINS => {
                    (command == C::TerrainAdd).then(|| {
                        format!(
                            "A tileset has at most {} terrains",
                            tiles::tileset::MAX_TERRAINS
                        )
                    })
                }
                Some(_) => None,
            },
            C::TerrainRemove | C::TerrainKind(_) | C::SaveTemplate => {
                match self.open.tilesets.tab() {
                    None => say("Open a tileset from the Library first"),
                    Some(e) if e.chosen().is_none() => say("Choose a terrain first"),
                    Some(_) => None,
                }
            }
            C::CropEveryTile => match &self.open.atlas_editor {
                None => say("Open a picture's crops from the Library first"),
                Some(_) if self.atlas_picture().0.is_none() => say("The picture is being read"),
                Some(_) => None,
            },
            C::CropDelete => match &self.open.atlas_editor {
                None => say("Open a picture's crops from the Library first"),
                Some(e) if e.picked.is_none() => say("Pick a crop first"),
                Some(_) => None,
            },
            C::CropsZoom(_) => self
                .open
                .atlas_editor
                .is_none()
                .then(|| "Open a picture's crops from the Library first".to_owned()),
            _ => None,
        }
    }

    /// Does one of the LIBRARY TOOLS commands.
    pub(crate) fn run_library_tool(&mut self, command: ShellCommand) {
        use ShellCommand as C;
        let path = self.open.tilesets.tab().map(|e| e.path.clone());
        let chosen = self.open.tilesets.tab().and_then(|e| e.chosen().cloned());
        match (command, path) {
            (C::TerrainAdd, Some(path)) => {
                self.change_tileset(&path, |e| e.add_terrain());
            }
            (C::TerrainRemove, Some(path)) => {
                if let Some(terrain) = chosen {
                    self.change_tileset(&path, |e| e.remove_terrain(terrain.id));
                }
            }
            (C::TerrainKind(kind), Some(path)) => {
                if let Some(terrain) = chosen {
                    self.change_tileset(&path, |e| e.set_kind(terrain.id, kind));
                }
            }
            (C::SaveTemplate, Some(_)) => {
                if let Some(terrain) = chosen {
                    self.save_template(terrain.kind);
                }
            }
            (C::TilesetZoom(zoom), Some(_)) => {
                if let Some(e) = self.open.tilesets.tab_mut() {
                    e.zoom = f32::from(zoom);
                }
            }
            (C::CropEveryTile | C::CropDelete | C::CropsZoom(_), _) => self.run_crops_tool(command),
            _ => {}
        }
    }

    /// Does one of LIBRARY TOOLS · Picture's commands.
    fn run_crops_tool(&mut self, command: ShellCommand) {
        use ShellCommand as C;
        let changed = match command {
            C::CropEveryTile => {
                let Some((art, _)) = self.open.atlas_texture.clone() else {
                    return;
                };
                let Some(rgba) = art.frames.first() else {
                    return;
                };
                let size = art.size;
                let Some(editor) = &mut self.open.atlas_editor else {
                    return;
                };
                let added =
                    editor.crop_every_tile(size, &|r| crate::atlas_view::filled(rgba, size, r));
                self.log(
                    crate::console::LogLevel::Info,
                    format!("Crops: added {added}."),
                );
                added > 0
            }
            C::CropDelete => self
                .open
                .atlas_editor
                .as_mut()
                .is_some_and(crate::atlas_editor::AtlasEditor::delete_picked),
            C::CropsZoom(zoom) => {
                if let Some(editor) = &mut self.open.atlas_editor {
                    editor.zoom = f32::from(zoom);
                }
                false
            }
            _ => false,
        };
        if changed {
            self.save_open_atlas();
        }
    }
}
