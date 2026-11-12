//! What HOME TOOLS · Tile map's ribbon reads and does (the approved
//! additions, round 3, `01-tile-map-painting-a.png`): the painter's
//! tools, the terrain or copy in hand, the layer painted, Exact, and what
//! the viewport lays over the selected map. Each control is a command
//! (`shell::ShellCommand`), so the search finds them too; the painting
//! itself stays in `workbench_paint.rs`.

use crate::shell::ShellCommand;
use crate::workbench::Workbench;

impl Workbench {
    /// Why the tile map's tools can't be used now, or `None` when they
    /// can: a map is selected, it has a layer, and the game isn't running.
    pub(crate) fn why_not_paint(&self) -> Option<String> {
        if self.open.map_edits.path.is_none() {
            return Some("Select an entity with a tile map first".to_owned());
        }
        if self.open.playing.is_some() {
            return Some("Stop the game first: while it runs, the map is the game's".to_owned());
        }
        (!self.viewport.tile_overlay.paint.allowed)
            .then(|| "The map has no layer to paint on: add one in the Map tab".to_owned())
    }

    /// Does one of the tile map's commands.
    pub(crate) fn run_paint_command(&mut self, command: ShellCommand) {
        use ShellCommand as C;
        let paint = &mut self.viewport.tile_overlay.paint;
        match command {
            C::PaintWith(tool) => paint.tool = Some(tool),
            C::PaintOnLayer(layer) => paint.layer = layer.min(paint.layers.len().saturating_sub(1)),
            C::PaintExact => paint.exact ^= true,
            C::MapMeaning(meaning) => paint.meaning = meaning,
            C::ShowValue(index) => {
                if let Some(name) = self.map_value_names().get(index).cloned() {
                    let shown = &mut self.open.map_edits.shown_values;
                    if shown.contains(&name) {
                        shown.retain(|n| *n != name);
                    } else {
                        shown.push(name);
                    }
                }
            }
            C::MapChunks => self.open.map_edits.chunks ^= true,
            C::MapWalls => self.open.map_edits.walls ^= true,
            C::TurnCopy(turn) => self.turn_stamp(turn),
            C::PutCopyAway => paint.stamp = None,
            _ => {}
        }
    }

    /// The selected map's values, by name, as the viewport has the map.
    pub(crate) fn map_value_names(&self) -> Vec<String> {
        self.viewport
            .tile_overlay
            .map
            .as_ref()
            .map(|m| m.map.values().iter().map(|v| v.name.clone()).collect())
            .unwrap_or_default()
    }

    /// The terrain painted with now, by name: the one chosen if the
    /// selected map's tileset has it, otherwise its first.
    pub(crate) fn painted_terrain(&self) -> Option<String> {
        let overlay = &self.viewport.tile_overlay;
        let set = overlay
            .map
            .as_ref()
            .and_then(|m| m.map.tileset)
            .and_then(|id| self.viewport.sheets.tileset(id))?;
        let code = overlay.paint.code_on(Some(set))?;
        set.by_code(char::from(code)).map(|t| t.name.clone())
    }
}
