//! A tileset open in the Tileset tab (Phase 9, D5): its terrains, its
//! picture and tile size, which tile fills each terrain's slots (pieces,
//! variations and their weights, animation), and a rule terrain's rules.
//! Every change is one undo step and keeps the file whole; what's picked
//! (a terrain, a tile of the picture, a slot, a rule) and the patch to
//! try edges on are the tab's own, never saved.

use reflect::Id;
use tiles::autotile;
use tiles::{Animation, Cond, Piece, Rule, Terrain, TileKind, Tileset};

/// The most undo steps kept.
const MAX_UNDO: usize = 100;

/// The patch to try a terrain's edges on: cells across and down.
pub(crate) const TRY_SIZE: [usize; 2] = [8, 6];

/// The step of setting up a terrain's art the strip at the top shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// Choose the picture.
    Picture,
    /// Check the tile size.
    Grid,
    /// Choose the terrain's kind.
    Kind,
    /// Say which tile fills each slot.
    MarkUp,
}

/// A tileset open for editing.
#[derive(Debug, Clone)]
pub(crate) struct TilesetEditor {
    /// Its path inside the assets.
    pub(crate) path: String,
    /// The tileset as it is now.
    pub(crate) file: Tileset,
    undo: Vec<Tileset>,
    redo: Vec<Tileset>,
    /// What was last saved, to tell Solis's own saves from changes on disk.
    saved: Option<Tileset>,
    /// The terrain chosen on the left.
    pub(crate) terrain: Option<Id>,
    /// The picture's tile picked: column and row.
    pub(crate) tile: Option<[u32; 2]>,
    /// The chosen terrain's slot picked.
    pub(crate) slot: Option<u32>,
    /// The rule picked (a terrain of its own rules).
    pub(crate) rule: Option<Id>,
    /// The patch to try edges on: which cells are the terrain.
    pub(crate) try_cells: Vec<bool>,
    /// How big the picture is drawn: screen pixels per pixel of it.
    pub(crate) zoom: f32,
    /// Whether the file has been read from disk (a new one isn't yet).
    pub(crate) on_disk: bool,
}

impl TilesetEditor {
    /// Opens `file`, read from `path`.
    pub(crate) fn new(path: String, file: Tileset) -> Self {
        let terrain = file
            .terrains
            .iter()
            .find(|t| t.kind != TileKind::Plain)
            .or(file.terrains.first())
            .map(|t| t.id);
        let mut try_cells = vec![false; TRY_SIZE[0] * TRY_SIZE[1]];
        // A blob to start with: edges, corners and an inside corner.
        for (x, y) in [
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 1),
            (1, 2),
            (2, 2),
            (3, 2),
            (4, 2),
            (5, 2),
            (6, 2),
            (1, 3),
            (2, 3),
            (3, 3),
            (5, 3),
            (6, 3),
            (6, 4),
        ] {
            try_cells[y * TRY_SIZE[0] + x] = true;
        }
        Self {
            path,
            saved: Some(file.clone()),
            file,
            undo: Vec::new(),
            redo: Vec::new(),
            terrain,
            tile: None,
            slot: None,
            rule: None,
            try_cells,
            zoom: 3.0,
            on_disk: true,
        }
    }

    /// The terrain chosen, if it's still there.
    pub(crate) fn chosen(&self) -> Option<&Terrain> {
        self.terrain.and_then(|id| self.file.terrain(id))
    }

    /// The step the strip shows as next for the chosen terrain.
    pub(crate) fn step(&self) -> Step {
        let Some(t) = self.chosen() else {
            return Step::Picture;
        };
        if self.file.picture.is_none() {
            Step::Picture
        } else if !self.file.pieces.iter().any(|p| p.terrain == t.id) && t.kind == TileKind::Plain {
            Step::Kind
        } else {
            Step::MarkUp
        }
    }

    /// Makes a change as one undo step. Whether it changed anything.
    pub(crate) fn change(&mut self, change: impl FnOnce(&mut Tileset)) -> bool {
        let before = self.file.clone();
        change(&mut self.file);
        if self.file == before {
            return false;
        }
        self.undo.push(before);
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
        true
    }

    /// Undoes the last change. Whether there was one.
    pub(crate) fn undo(&mut self) -> bool {
        let Some(before) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.file, before));
        self.keep_picks();
        true
    }

    /// Redoes the last change undone. Whether there was one.
    pub(crate) fn redo(&mut self) -> bool {
        let Some(after) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.file, after));
        self.keep_picks();
        true
    }

    /// Lets go of picks no longer there.
    fn keep_picks(&mut self) {
        if self.chosen().is_none() {
            self.terrain = self.file.terrains.first().map(|t| t.id);
        }
        if self
            .rule
            .is_some_and(|r| !self.file.rules.iter().any(|x| x.id == r))
        {
            self.rule = None;
        }
    }

    /// The file as text, noting it's Solis's own save.
    pub(crate) fn saving(&mut self) -> String {
        self.saved = Some(self.file.clone());
        tiles::tileset::write(&self.file)
    }

    /// Whether `file` (read from disk) is what Solis last saved.
    pub(crate) fn is_own(&self, file: &Tileset) -> bool {
        self.saved.as_ref() == Some(file)
    }

    /// Takes in the file as changed on disk, as one undo step.
    pub(crate) fn taken_in(&mut self, file: Tileset) {
        let before = std::mem::replace(&mut self.file, file);
        self.saved = Some(self.file.clone());
        if before != self.file {
            self.undo.push(before);
            self.redo.clear();
        }
        self.keep_picks();
    }

    /// Adds a terrain (plain, with a free code and name), and chooses it.
    pub(crate) fn add_terrain(&mut self) -> bool {
        let Some(code) = self.file.free_code() else {
            return false;
        };
        if self.file.terrains.len() >= tiles::tileset::MAX_TERRAINS {
            return false;
        }
        let name = (1..)
            .map(|n| {
                if n == 1 {
                    "terrain".to_owned()
                } else {
                    format!("terrain{n}")
                }
            })
            .find(|n| self.file.by_name(n).is_none())
            .unwrap_or_default();
        let id = Id::random();
        self.terrain = Some(id);
        self.change(|f| {
            f.terrains.push(Terrain {
                id,
                name,
                code,
                colour: [0.5; 3],
                kind: TileKind::Plain,
            });
        })
    }

    /// Takes a terrain away, with its pieces and rules (maps keep its
    /// code, drawn pink).
    pub(crate) fn remove_terrain(&mut self, id: Id) -> bool {
        let changed = self.change(|f| {
            f.terrains.retain(|t| t.id != id);
            f.pieces.retain(|p| p.terrain != id);
            f.rules.retain(|r| r.terrain != id);
        });
        self.keep_picks();
        changed
    }

    /// Changes a terrain's name, code or colour (`None` keeps it).
    pub(crate) fn set_terrain(
        &mut self,
        id: Id,
        name: Option<String>,
        code: Option<char>,
        colour: Option<[f32; 3]>,
    ) -> bool {
        if let Some(n) = &name
            && self.file.by_name(n).is_some_and(|t| t.id != id)
        {
            return false;
        }
        if let Some(c) = code
            && (!tiles::tileset::usable_code(c) || self.file.by_code(c).is_some_and(|t| t.id != id))
        {
            return false;
        }
        self.change(|f| {
            if let Some(t) = f.terrains.iter_mut().find(|t| t.id == id) {
                if let Some(n) = name {
                    t.name = n;
                }
                if let Some(c) = code {
                    t.code = c;
                }
                if let Some(c) = colour {
                    t.colour = c;
                }
            }
        })
    }

    /// Changes a terrain's kind. Its pieces and rules no longer fit, so
    /// they go (undo brings them back).
    pub(crate) fn set_kind(&mut self, id: Id, kind: TileKind) -> bool {
        self.slot = None;
        self.change(|f| {
            if let Some(t) = f.terrains.iter_mut().find(|t| t.id == id)
                && t.kind != kind
            {
                t.kind = kind;
                f.pieces.retain(|p| p.terrain != id);
                f.rules.retain(|r| r.terrain != id);
            }
        })
    }

    /// Chooses the picture (by asset ID) and its tile size.
    pub(crate) fn set_picture(
        &mut self,
        picture: Option<reflect::AssetId>,
        tile: [u32; 2],
    ) -> bool {
        let tile = tile.map(|t| t.clamp(1, tiles::tileset::MAX_TILE));
        self.tile = None;
        self.change(|f| {
            f.picture = picture;
            f.tile = tile;
        })
    }

    /// Adds the picture's tile `at` to the chosen terrain's `slot` (a
    /// second tile in a slot is a variation). Whether it was added.
    pub(crate) fn assign(&mut self, slot: u32, at: [u32; 2]) -> bool {
        let Some(t) = self.chosen().map(|t| (t.id, t.kind)) else {
            return false;
        };
        if slot >= autotile::slot_count(t.1)
            || at[0] > tiles::tileset::MAX_AT
            || at[1] > tiles::tileset::MAX_AT
            || self.file.pieces.len() >= tiles::tileset::MAX_PIECES
            || self.file.pieces_of(t.0, slot).any(|(_, p)| p.at == at)
        {
            return false;
        }
        self.slot = Some(slot);
        self.change(|f| {
            f.pieces.push(Piece {
                id: Id::random(),
                terrain: t.0,
                slot,
                at,
                weight: 1.0,
                animation: Animation::None,
            });
        })
    }

    /// Takes a piece away.
    pub(crate) fn remove_piece(&mut self, id: Id) -> bool {
        self.change(|f| f.pieces.retain(|p| p.id != id))
    }

    /// Changes a piece's weight and animation.
    pub(crate) fn set_piece(&mut self, id: Id, weight: f32, animation: Animation) -> bool {
        let weight = if weight.is_finite() {
            weight.clamp(0.0, tiles::tileset::MAX_WEIGHT)
        } else {
            1.0
        };
        let animation = match animation {
            Animation::Columns { count, ms } => Animation::Columns {
                count: count.clamp(2, tiles::tileset::MAX_FRAMES),
                ms: ms.clamp(tiles::tileset::FRAME_MS.0, tiles::tileset::FRAME_MS.1),
            },
            other => other,
        };
        self.change(|f| {
            if let Some(p) = f.pieces.iter_mut().find(|p| p.id == id) {
                p.weight = weight;
                p.animation = animation;
            }
        })
    }

    /// Marks up the chosen terrain from a picture drawn over its kind's
    /// template, its top-left tile at `from`: each slot's tile from its
    /// place, where it shows anything. Replaces the terrain's pieces.
    pub(crate) fn mark_up(&mut self, from: [u32; 2], shown: impl Fn([u32; 2]) -> bool) -> bool {
        let Some((id, kind)) = self.chosen().map(|t| (t.id, t.kind)) else {
            return false;
        };
        let marks = autotile::mark_up(kind, from, shown);
        if marks.is_empty() {
            return false;
        }
        self.change(|f| {
            f.pieces.retain(|p| p.terrain != id);
            for (slot, at) in marks {
                f.pieces.push(Piece {
                    id: Id::random(),
                    terrain: id,
                    slot,
                    at,
                    weight: 1.0,
                    animation: Animation::None,
                });
            }
        })
    }

    /// Adds a rule to the chosen terrain (of its own rules): a 3 × 3
    /// pattern of anything, showing slot 0. Picks it.
    pub(crate) fn add_rule(&mut self) -> bool {
        let Some(t) = self
            .chosen()
            .filter(|t| t.kind == TileKind::Rules)
            .map(|t| t.id)
        else {
            return false;
        };
        if self.file.rules.len() >= tiles::tileset::MAX_RULES {
            return false;
        }
        let id = Id::random();
        self.rule = Some(id);
        self.change(|f| {
            let mut pattern = vec![Cond::Any; 9];
            pattern[4] = Cond::Same;
            f.rules.push(Rule {
                id,
                terrain: t,
                size: 3,
                pattern,
                slot: 0,
            });
        })
    }

    /// Takes a rule away.
    pub(crate) fn remove_rule(&mut self, id: Id) -> bool {
        let changed = self.change(|f| f.rules.retain(|r| r.id != id));
        self.keep_picks();
        changed
    }

    /// Moves a rule earlier (tried sooner) or later.
    pub(crate) fn move_rule(&mut self, id: Id, earlier: bool) -> bool {
        self.change(|f| {
            let Some(i) = f.rules.iter().position(|r| r.id == id) else {
                return;
            };
            let terrain = f.rules[i].terrain;
            let other = if earlier {
                f.rules[..i].iter().rposition(|r| r.terrain == terrain)
            } else {
                f.rules[i + 1..]
                    .iter()
                    .position(|r| r.terrain == terrain)
                    .map(|j| i + 1 + j)
            };
            if let Some(j) = other {
                f.rules.swap(i, j);
            }
        })
    }

    /// Changes one cell of a rule's pattern (never its middle).
    pub(crate) fn set_cond(&mut self, id: Id, index: usize, cond: Cond) -> bool {
        self.change(|f| {
            if let Some(r) = f.rules.iter_mut().find(|r| r.id == id)
                && index < r.pattern.len()
                && index != r.pattern.len() / 2
            {
                r.pattern[index] = cond;
            }
        })
    }

    /// Changes a rule's slot.
    pub(crate) fn set_rule_slot(&mut self, id: Id, slot: u32) -> bool {
        let slot = slot.min(autotile::RULE_SLOTS - 1);
        self.change(|f| {
            if let Some(r) = f.rules.iter_mut().find(|r| r.id == id) {
                r.slot = slot;
            }
        })
    }

    /// Makes a rule's pattern `size` across (1 to 9, odd), keeping what
    /// still fits round the middle.
    pub(crate) fn set_rule_size(&mut self, id: Id, size: usize) -> bool {
        let size =
            (size.clamp(1, tiles::tileset::MAX_PATTERN) | 1).min(tiles::tileset::MAX_PATTERN);
        self.change(|f| {
            if let Some(r) = f.rules.iter_mut().find(|r| r.id == id)
                && r.size != size
            {
                let mut pattern = vec![Cond::Any; size * size];
                let (old_mid, mid) = ((r.size / 2) as i64, (size / 2) as i64);
                for y in 0..size as i64 {
                    for x in 0..size as i64 {
                        let (ox, oy) = (x - mid + old_mid, y - mid + old_mid);
                        if ox >= 0 && oy >= 0 && ox < r.size as i64 && oy < r.size as i64 {
                            pattern[(y * size as i64 + x) as usize] =
                                r.pattern[(oy * r.size as i64 + ox) as usize];
                        }
                    }
                }
                pattern[size * size / 2] = Cond::Same;
                r.size = size;
                r.pattern = pattern;
            }
        })
    }

    /// A small map of the try-it patch: its ground the first other plain
    /// terrain (or nothing), and the chosen terrain over it (its second
    /// layer) where it's on.
    pub(crate) fn try_map(&self) -> Option<tiles::TileMap> {
        let t = self.chosen()?;
        let under = self
            .file
            .terrains
            .iter()
            .find(|o| o.id != t.id && o.kind == TileKind::Plain)
            .map(|o| o.code as u8);
        let [w, h] = TRY_SIZE;
        let mut map = tiles::TileMap::new([w as u32, h as u32], None, under);
        for y in 0..h {
            for x in 0..w {
                if self.try_cells[y * w + x] {
                    let _ = map.set_code(1, x as u32, y as u32, t.code as u8);
                }
            }
        }
        Some(map)
    }
}

#[cfg(test)]
#[path = "tileset_editor_tests.rs"]
mod tests;
