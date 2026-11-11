//! The workbench's side of tile maps (Phase 9, D4): placing a map from
//! the Library, making new tilesets and maps, changing a map file's
//! tileset and size from Properties, and the Map tab beside Properties
//! while a map is selected: its layers and values changed (each one undo
//! step over the tab), saved at once and shown at once. A tileset is
//! changed through its editor (`workbench_tileset.rs`), from here too.

use crate::console::LogLevel;
use crate::layout;
use crate::panel::Panel;
use crate::sprite_sheets::{MapFileState, TilesetState};
use crate::workbench::{Workbench, WorkbenchAction};
use glam::Vec3;
use reflect::{AssetId, EntityId};
use std::sync::Arc;
use tiles::map_file::MapFileError;
use tiles::{TileMap, Tileset};

/// The most undo steps the Map tab keeps.
const MAX_UNDO: usize = 50;
/// The most cells the tile maps' undo steps hold together (a 4096 × 4096
/// map keeps 16 steps).
const MAX_UNDO_BYTES: u64 = 256 << 20;

/// The map file the Map tab is changing: its path, and its undo steps
/// (copies of the map, which share their cells until changed).
#[derive(Debug, Clone)]
pub(crate) struct MapEdits {
    /// The map file's path, while the tab shows one.
    pub(crate) path: Option<String>,
    undo: Vec<(String, TileMap)>,
    redo: Vec<(String, TileMap)>,
    /// The cell under the pointer in the viewport (map, cell), for the
    /// tab's cell inspector.
    pub(crate) hovered: Option<(EntityId, [u32; 2])>,
    /// Which values the viewport lays over the map, by name.
    pub(crate) shown_values: Vec<String>,
    /// Whether the viewport outlines the map's chunks.
    pub(crate) chunks: bool,
    /// Whether the viewport shows what's solid (decision 0071).
    pub(crate) walls: bool,
    /// Layers hidden in the editor, by ID (never saved).
    pub(crate) hidden: std::collections::HashSet<reflect::Id>,
    /// The tile painter's stroke under way (D6).
    pub(crate) stroke: Option<crate::workbench_paint::Stroke>,
    /// The biggest file a map is saved as (what the reader takes,
    /// `tiles::map_file::MAX_BYTES`; less in tests).
    pub(crate) most_bytes: usize,
}

impl Default for MapEdits {
    fn default() -> Self {
        Self {
            path: None,
            undo: Vec::new(),
            redo: Vec::new(),
            hovered: None,
            shown_values: Vec::new(),
            chunks: false,
            walls: false,
            hidden: std::collections::HashSet::new(),
            stroke: None,
            most_bytes: tiles::map_file::MAX_BYTES,
        }
    }
}

impl MapEdits {
    /// Keeps `map` as it was before a change to the file at `path`, as an
    /// undo step (redo steps are let go).
    pub(crate) fn push_undo(&mut self, path: String, map: TileMap) {
        self.undo.push((path, map));
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        // Each step keeps its own copy of what it changed (a layer, at
        // least): big maps keep fewer steps, within MAX_UNDO_BYTES (the
        // stability review).
        let step = |m: &TileMap| u64::from(m.size()[0]) * u64::from(m.size()[1]);
        while self.undo.len() > 1
            && self.undo.iter().map(|(_, m)| step(m)).sum::<u64>() > MAX_UNDO_BYTES
        {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
}

/// What the Console says when a change to the map at `path` isn't made
/// because its file couldn't be opened again (`why`, in the reader's own
/// words): what happened, that nothing was lost, and what to do.
fn outgrown_note(path: &str, why: &MapFileError) -> String {
    format!(
        "{path} wasn't changed: saved like that, the map couldn't be opened again ({why}). \
         It's as it was. Make the map smaller, clear layers, values or exact tiles it doesn't \
         need, or split the world into several maps, then try again."
    )
}

/// A free path for a new file named `stem` in `folder`.
pub(crate) fn free_path(
    folder: &str,
    stem: &str,
    ext: &str,
    taken: impl Fn(&str) -> bool,
) -> String {
    let base = if folder.is_empty() {
        String::new()
    } else {
        format!("{folder}/")
    };
    std::iter::once(format!("{base}{stem}.{ext}"))
        .chain((2..).map(|n| format!("{base}{stem} {n}.{ext}")))
        .find(|p| !taken(p))
        .unwrap_or_default()
}

/// The terrains a new tileset starts with: grass first (what new maps
/// are filled with), then tilled earth and water.
pub(crate) fn starter_tileset() -> Tileset {
    let t = |name: &str, code, colour| tiles::Terrain {
        id: reflect::Id::random(),
        name: name.into(),
        code,
        colour,
        kind: tiles::TileKind::Plain,
        solid: false,
    };
    Tileset {
        terrains: vec![
            t("grass", '.', [0.31, 0.48, 0.23]),
            t("tilled", 't', [0.42, 0.29, 0.18]),
            t("water", '~', [0.18, 0.37, 0.56]),
        ],
        ..Tileset::default()
    }
}

impl Workbench {
    fn taken(&self, path: &str) -> bool {
        self.open.library.iter().any(|i| i.path == path)
            || self.viewport.sheets.tilesets.contains_key(path)
            || self.viewport.sheets.tile_maps.contains_key(path)
    }

    /// Makes a new tileset in `folder` with a few terrains, saved, and shows
    /// it in Properties.
    pub(crate) fn new_tileset(&mut self, folder: &str) {
        let path = free_path(folder, "tileset", &tiles::tileset::ENDING[1..], |p| {
            self.taken(p)
        });
        let set = starter_tileset();
        self.open.outbox.push(WorkbenchAction::SaveTileset {
            path: path.clone().into(),
            text: tiles::tileset::write(&set),
        });
        Arc::make_mut(&mut self.viewport.sheets)
            .tilesets
            .insert(path.clone(), TilesetState::Read(Arc::new(set), Vec::new()));
        self.show_in_library(&path);
        self.open_tileset(&path);
        if let Some(editor) = self.open.tilesets.tab_mut() {
            editor.on_disk = false;
        }
    }

    /// Makes a new tile map in `folder`: 64 × 64 cells, the project's
    /// first tileset (by name), its ground filled with that tileset's
    /// first terrain. Shown in Properties.
    pub(crate) fn new_tile_map(&mut self, folder: &str) {
        let path = free_path(folder, "map", &tiles::map_file::ENDING[1..], |p| {
            self.taken(p)
        });
        let sheets = &self.viewport.sheets;
        let tileset = sheets.listed_tiles(false).first().map(|(_, id)| *id);
        let fill = tileset
            .and_then(|t| sheets.tileset(t))
            .and_then(|t| t.terrains.first())
            .map(|t| t.code as u8);
        let map = TileMap::new(tiles::map_file::STANDARD_SIZE, tileset, fill);
        let text = match tiles::map_file::write(&map) {
            Ok(text) => text,
            Err(why) => {
                self.log(LogLevel::Error, format!("Couldn't make {path}: {why}."));
                return;
            }
        };
        self.open.outbox.push(WorkbenchAction::SaveTileMap {
            path: path.clone().into(),
            text,
        });
        Arc::make_mut(&mut self.viewport.sheets)
            .tile_maps
            .insert(path.clone(), MapFileState::Read(Arc::new(map), Vec::new()));
        if tileset.is_none() {
            self.log(
                LogLevel::Info,
                format!(
                    "{path} has no tileset yet: make one (right-click in the Library > New > Tileset) and \
                     choose it in the map's Properties."
                ),
            );
        }
        self.show_in_library(&path);
    }

    /// Places the tile map file `map` at `target` (or where the camera
    /// looks), its top-left corner on a whole tile, selected.
    pub(crate) fn place_tile_map(
        &mut self,
        map: AssetId,
        target: Option<(Vec3, Option<EntityId>)>,
    ) -> Option<EntityId> {
        self.viewport.sheets.tile_map(map)?;
        let name = self.viewport.sheets.paths.get(&map).map_or_else(
            || "Tile map".to_owned(),
            |p| crate::sprite_sheets::name_of(p),
        );
        let (at, parent) = target.unwrap_or((self.viewport.camera.focus(), None));
        if !at.is_finite() {
            return None;
        }
        let [tx, ty] = self.open.pixels.tile_size();
        let mut data = area::EntityData::new(&name)
            .with(&area::builtin::Transform {
                position: Vec3::new((at.x / tx).round() * tx, (at.y / ty).round() * ty, at.z),
                ..area::builtin::Transform::default()
            })
            .with(&area::builtin::TileMap {
                map: Some(reflect::AssetRef {
                    asset: map,
                    part: None,
                }),
            });
        data.parent = parent.filter(|&p| self.open.document.area().contains(p));
        Some(self.spawn_selected(data))
    }

    /// Saves the map file at `path` as `map`, and runs it at once (a
    /// playing game's copy starts again from it). A map whose file
    /// couldn't be read back isn't saved or shown: nothing changes, and
    /// the mistake says why.
    fn save_tile_map(&mut self, path: &str, map: TileMap) -> Result<(), MapFileError> {
        let text = tiles::map_file::write_within(&map, self.open.map_edits.most_bytes)?;
        self.open.outbox.push(WorkbenchAction::SaveTileMap {
            path: path.into(),
            text,
        });
        let sheets = Arc::make_mut(&mut self.viewport.sheets);
        sheets.tile_maps.insert(
            path.to_owned(),
            MapFileState::Read(Arc::new(map), Vec::new()),
        );
        sheets.work_out_maps();
        self.sources += 1;
        Ok(())
    }

    /// Makes `map` the map file at `path`, saved, with `before` kept as
    /// the undo step back. Every change to a map ends here (the Map tab's,
    /// Properties', a painted stroke). One that would make the file too
    /// big to open again is refused at once (BUG-55): nothing is saved,
    /// no undo step is made, the map stays as `before`, and the Console
    /// says why and what to do. Whether the change was made.
    pub(crate) fn commit_map(&mut self, path: &str, before: &TileMap, map: TileMap) -> bool {
        match self.save_tile_map(path, map) {
            Ok(()) => {
                self.open
                    .map_edits
                    .push_undo(path.to_owned(), before.clone());
                true
            }
            Err(why) => {
                self.log(LogLevel::Warning, outgrown_note(path, &why));
                false
            }
        }
    }

    /// The map file at `path`, as read.
    pub(crate) fn map_file(&self, path: &str) -> Option<Arc<TileMap>> {
        match self.viewport.sheets.tile_maps.get(path)? {
            MapFileState::Read(map, _) => Some(Arc::clone(map)),
            MapFileState::Failed(_) => None,
        }
    }

    /// Changes the map file at `path` with `change`, as one undo step over
    /// the Map tab, and saves it. Whether it changed.
    pub(crate) fn change_map(
        &mut self,
        path: &str,
        change: impl FnOnce(&mut TileMap) -> bool,
    ) -> bool {
        let Some(before) = self.map_file(path) else {
            return false;
        };
        let mut map = (*before).clone();
        if !change(&mut map) || map == *before {
            return false;
        }
        self.commit_map(path, &before, map)
    }

    /// Undoes (or redoes) the last change made in the Map tab.
    pub(crate) fn map_undo(&mut self, redo: bool, undo: bool) {
        let popped = if redo {
            self.open.map_edits.redo.pop()
        } else if undo {
            self.open.map_edits.undo.pop()
        } else {
            None
        };
        let Some((path, map)) = popped else {
            return;
        };
        let now = self.map_file(&path);
        if let Err(why) = self.save_tile_map(&path, map.clone()) {
            // The step stays where it was: nothing was undone.
            let edits = &mut self.open.map_edits;
            let back = if redo {
                &mut edits.redo
            } else {
                &mut edits.undo
            };
            back.push((path.clone(), map));
            self.log(LogLevel::Warning, outgrown_note(&path, &why));
            return;
        }
        if let Some(now) = now {
            let edits = &mut self.open.map_edits;
            let other = if redo {
                &mut edits.undo
            } else {
                &mut edits.redo
            };
            other.push((path, (*now).clone()));
        }
    }

    /// The selected entity's tile map file's path, if it has one.
    pub(crate) fn selected_map_path(&self) -> Option<(EntityId, String)> {
        let id = self.open.document.selection.primary()?;
        let component = self
            .open
            .document
            .area()
            .get(id)?
            .get::<area::builtin::TileMap>()?;
        let path = self.viewport.sheets.paths.get(&component.map?.asset)?;
        Some((id, path.clone()))
    }

    /// Opens the Map tab beside Properties while a map is selected, and
    /// closes it otherwise (each frame, before the panels are drawn).
    pub(crate) fn follow_map_selection(&mut self) {
        let selected = self.selected_map_path().map(|(_, p)| p);
        if selected != self.open.map_edits.path {
            self.open.map_edits.hovered = None;
        }
        match (&selected, self.dock.find_tab(&Panel::Map)) {
            (Some(_), None) => layout::open_beside(&mut self.dock, Panel::Map, Panel::Properties),
            (None, Some(_)) => layout::close(&mut self.dock, Panel::Map),
            _ => {}
        }
        self.open.map_edits.path = selected;
    }
}

impl Workbench {
    /// The selected map as the Map tab shows it (the game's while
    /// playing, otherwise its file's), with its tileset.
    pub(crate) fn map_shown(&self) -> Option<(TileMap, Option<Arc<Tileset>>)> {
        let (id, path) = self.selected_map_path()?;
        let map = match &self.open.playing {
            Some(playing) => playing
                .world
                .tile_map(id)
                .or_else(|| self.map_file(&path).map(|m| (*m).clone()))?,
            None => (*self.map_file(&path)?).clone(),
        };
        let tileset = map
            .tileset
            .and_then(|t| self.viewport.sheets.tileset(t))
            .cloned();
        Some((map, tileset))
    }

    /// Tells the viewport what to lay over the selected map: its values
    /// shown, its chunks, its hidden layers.
    pub(crate) fn set_tile_overlay(&mut self) {
        let selected = self.selected_map_path();
        let overlay = &mut self.viewport.tile_overlay;
        overlay.hidden.clone_from(&self.open.map_edits.hidden);
        overlay.chunks = self.open.map_edits.chunks;
        overlay.walls = self.open.map_edits.walls;
        overlay.solid = None;
        overlay.map = None;
        overlay.values.clear();
        overlay.paint.allowed = false;
        let Some((id, path)) = selected else {
            return;
        };
        let map = match &self.open.playing {
            Some(playing) => playing.world.tile_map(id).map(Arc::new),
            None => None,
        }
        .or_else(|| match self.viewport.sheets.tile_maps.get(&path) {
            Some(MapFileState::Read(m, _)) => Some(Arc::clone(m)),
            _ => None,
        });
        let Some(map) = map else {
            return;
        };
        let transform = self
            .open
            .playing
            .as_ref()
            .and_then(|p| p.world.global(id))
            .unwrap_or_else(|| self.open.document.world_matrix(id));
        let overlay = &mut self.viewport.tile_overlay;
        overlay.values = self
            .open
            .map_edits
            .shown_values
            .iter()
            .filter_map(|n| map.value_named(n))
            .collect();
        // Painting is for the map file, not the game's copy of it.
        let paint = &mut overlay.paint;
        paint.allowed = self.open.playing.is_none() && !map.layers().is_empty();
        paint.layers = map.layers().iter().map(|l| l.name.clone()).collect();
        paint.layer = paint.layer.min(paint.layers.len().saturating_sub(1));
        // Which terrains are solid, from the map's tileset (none solid:
        // nothing to show).
        let solid = map
            .tileset
            .and_then(|t| self.viewport.sheets.tileset(t))
            .map(|set| tiles::walls::solid_codes(set))
            .filter(|s| s.iter().any(|on| *on));
        let overlay = &mut self.viewport.tile_overlay;
        overlay.solid = solid;
        overlay.map = Some(crate::tile_view::ViewTileMap {
            entity: id,
            transform,
            map,
        });
    }

    /// Makes the terrain written `code` in the tileset of the map file at
    /// `map_path` solid or not, as one undo step of the tileset's, and
    /// saves the tileset (decision 0071).
    pub(crate) fn set_terrain_solid(&mut self, map_path: &str, code: u8, solid: bool) {
        let Some(set_id) = self.map_file(map_path).and_then(|m| m.tileset) else {
            return;
        };
        let Some(path) = self.viewport.sheets.paths.get(&set_id).cloned() else {
            return;
        };
        self.change_tileset(&path, |editor| editor.set_solid(code, solid));
    }

    /// Takes in what the menu over the viewport asked for this frame.
    pub(crate) fn tile_overlay_done(&mut self) {
        let overlay = &mut self.viewport.tile_overlay;
        for (name, on) in std::mem::take(&mut overlay.asked) {
            self.open.map_edits.shown_values.retain(|n| *n != name);
            if on {
                self.open.map_edits.shown_values.push(name);
            }
        }
        if std::mem::take(&mut overlay.chunks_asked) {
            self.open.map_edits.chunks = !self.open.map_edits.chunks;
        }
        if std::mem::take(&mut overlay.walls_asked) {
            self.open.map_edits.walls = !self.open.map_edits.walls;
        }
        self.open.map_edits.hovered = overlay
            .hovered
            .zip(overlay.map.as_ref())
            .map(|(cell, m)| (m.entity, cell));
    }

    /// Makes the changes the Map tab asked for, each one undo step.
    pub(crate) fn map_requests_done(&mut self, requests: Vec<crate::map_tab::MapRequest>) {
        use crate::map_tab::MapRequest;
        let Some(path) = self.open.map_edits.path.clone() else {
            return;
        };
        for request in requests {
            let paint = &mut self.viewport.tile_overlay.paint;
            match request {
                MapRequest::PaintWith(code) => {
                    paint.code = Some(code);
                    paint.stamp = None;
                    if !paint.painting() {
                        paint.tool = Some(crate::tile_paint::PaintTool::Brush);
                    }
                }
                MapRequest::SetSolid(code, solid) => self.set_terrain_solid(&path, code, solid),
                MapRequest::PaintPiece(piece) => {
                    paint.piece = Some(piece);
                    paint.exact = true;
                }
                MapRequest::ToggleHidden(id) => {
                    if !self.open.map_edits.hidden.remove(&id) {
                        self.open.map_edits.hidden.insert(id);
                    }
                }
                MapRequest::AddLayer => {
                    self.change_map(&path, |m| {
                        let name = (1..)
                            .map(|n| format!("Layer{n}"))
                            .find(|n| m.layer_named(n).is_none())
                            .unwrap_or_default();
                        m.add_layer(&name, false).is_ok()
                    });
                }
                MapRequest::RemoveLayer(i) => {
                    self.change_map(&path, |m| m.remove_layer(i));
                }
                MapRequest::MoveLayer(i, up) => {
                    self.change_map(&path, |m| m.move_layer(i, up));
                }
                MapRequest::SetLayer(i, name, over) => {
                    self.change_map(&path, |m| {
                        m.layer_named(&name).is_none_or(|j| j == i) && m.set_layer(i, &name, over)
                    });
                }
                MapRequest::AddValue => {
                    self.change_map(&path, |m| {
                        let name = (1..)
                            .map(|n| {
                                if n == 1 {
                                    "value".to_owned()
                                } else {
                                    format!("value{n}")
                                }
                            })
                            .find(|n| m.value_named(n).is_none())
                            .unwrap_or_default();
                        m.add_value(&name, tiles::ValueKind::Switch).is_ok()
                    });
                }
                MapRequest::RemoveValue(i) => {
                    self.change_map(&path, |m| m.remove_value(i));
                }
                MapRequest::RenameValue(i, name) => {
                    self.change_map(&path, |m| {
                        m.value_named(&name).is_none() && m.rename_value(i, &name)
                    });
                }
                MapRequest::SetValueKind(i, kind) => {
                    self.change_map(&path, |m| m.set_value_kind(i, kind));
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "workbench_tiles_tests.rs"]
pub(crate) mod tests;
