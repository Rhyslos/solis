//! A tile map (Phase 9, D4): a grid of cells in layers, each cell holding
//! a terrain's code (its meaning: grass, tilled, water), and named values
//! per cell (crop, growth, watered). The same model serves the editor (a
//! map file open) and the running game (a map that changes as it plays).
//!
//! Cells count from the map's top-left corner: x to the right, y down the
//! screen, as a map's rows read in its file. Changes mark their chunk
//! (32 × 32 cells), so drawing and the checksum work again only on what
//! changed. Layers' cells and values' columns are shared until changed
//! (`Arc`), so a copy of a map (an undo step, a save) is cheap.

use reflect::{AssetId, Id};
use std::sync::Arc;

/// A chunk's side, in cells.
pub const CHUNK: u32 = 32;
/// The longest side a map has, in cells.
pub const MAX_SIDE: u32 = 4096;
/// The most layers a map has.
pub const MAX_LAYERS: usize = 16;
/// The most values a map's cells have.
pub const MAX_VALUES: usize = 32;
/// The most options a choice value has.
pub const MAX_OPTIONS: usize = 64;
/// The biggest (and, negated, smallest) number value.
pub const MAX_NUMBER: i32 = 1_000_000_000;

/// A layer of cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    /// Its ID, which never changes.
    pub id: Id,
    /// Its name.
    pub name: String,
    /// Whether it's drawn over sprites (things the player walks behind).
    pub over: bool,
}

/// What kind of value a cell holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueKind {
    /// A whole number (growth 0 to 5).
    Number,
    /// On or off (watered).
    Switch,
    /// One of these options, or none (crop: wheat, corn).
    Choice(Vec<String>),
}

impl ValueKind {
    /// Its name, as the file writes it.
    pub fn word(&self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Switch => "switch",
            Self::Choice(_) => "choice",
        }
    }
}

/// A value every cell of the map has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueDef {
    /// Its ID, which never changes.
    pub id: Id,
    /// Its name: letters, digits and `_`.
    pub name: String,
    /// What it holds.
    pub kind: ValueKind,
}

/// One cell's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellValue {
    /// A whole number.
    Number(i32),
    /// On or off.
    Switch(bool),
    /// An option, by its place in the value's list, or none.
    Choice(Option<u8>),
}

/// Every cell's value of one kind, in rows from the top left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Column {
    Number(Vec<i32>),
    Switch(Vec<bool>),
    /// 0 for none; otherwise the option's place plus one.
    Choice(Vec<u8>),
}

impl Column {
    fn new(kind: &ValueKind, cells: usize) -> Self {
        match kind {
            ValueKind::Number => Self::Number(vec![0; cells]),
            ValueKind::Switch => Self::Switch(vec![false; cells]),
            ValueKind::Choice(_) => Self::Choice(vec![0; cells]),
        }
    }

    fn get(&self, i: usize) -> CellValue {
        match self {
            Self::Number(v) => CellValue::Number(v[i]),
            Self::Switch(v) => CellValue::Switch(v[i]),
            Self::Choice(v) => CellValue::Choice(v[i].checked_sub(1)),
        }
    }

    fn is_default(&self, i: usize) -> bool {
        match self {
            Self::Number(v) => v[i] == 0,
            Self::Switch(v) => !v[i],
            Self::Choice(v) => v[i] == 0,
        }
    }
}

/// A tile map.
#[derive(Debug)]
pub struct TileMap {
    /// The tileset whose terrains its codes name.
    pub tileset: Option<AssetId>,
    size: [u32; 2],
    layers: Vec<Layer>,
    /// Each layer's cells: a terrain's code (ASCII), 0 for empty.
    cells: Vec<Arc<Vec<u8>>>,
    values: Vec<ValueDef>,
    columns: Vec<Arc<Column>>,
    /// Which chunks changed since last taken (drawing).
    dirty: Vec<bool>,
    dirty_list: Vec<u32>,
    /// Each chunk's checksum, until it changes (kept from one checksum
    /// to the next, through a shared view of the map).
    sums: std::sync::Mutex<Vec<Option<u64>>>,
    /// Goes up with every change (drawing notices it).
    revision: u64,
    /// The revision each layer last changed at.
    layer_revisions: Vec<u64>,
    /// The revision each chunk last changed at (drawing redraws a chunk,
    /// and those beside it, only when it changes).
    chunk_revisions: Vec<u64>,
}

impl Clone for TileMap {
    fn clone(&self) -> Self {
        Self {
            tileset: self.tileset,
            size: self.size,
            layers: self.layers.clone(),
            cells: self.cells.clone(),
            values: self.values.clone(),
            columns: self.columns.clone(),
            dirty: self.dirty.clone(),
            dirty_list: self.dirty_list.clone(),
            sums: std::sync::Mutex::new(self.sums().clone()),
            revision: self.revision,
            layer_revisions: self.layer_revisions.clone(),
            chunk_revisions: self.chunk_revisions.clone(),
        }
    }
}

/// Two maps are the same when their tileset, size, layers, values and
/// cells are (what changed since drawn isn't part of a map).
impl PartialEq for TileMap {
    fn eq(&self, other: &Self) -> bool {
        self.tileset == other.tileset
            && self.size == other.size
            && self.layers == other.layers
            && self.cells == other.cells
            && self.values == other.values
            && self.columns == other.columns
    }
}

/// What can go wrong changing a map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    /// The cell is outside the map.
    Outside,
    /// No such layer or value.
    NoSuch,
    /// The value is another kind (its kind's name).
    WrongKind(&'static str),
    /// A choice's option isn't one of its options, or a number is out of
    /// range.
    NotUsable,
    /// Past a limit.
    Full,
}

impl TileMap {
    /// A map of `size` cells (each side 1 to [`MAX_SIDE`]), with the
    /// layers Ground (filled with `fill`, or empty), Decoration and Above
    /// (over sprites).
    pub fn new(size: [u32; 2], tileset: Option<AssetId>, fill: Option<u8>) -> Self {
        let size = size.map(|s| s.clamp(1, MAX_SIDE));
        let mut map = Self::bare(size, tileset);
        for (name, over) in [("Ground", false), ("Decoration", false), ("Above", true)] {
            let _ = map.add_layer(name, over);
        }
        if let Some(code) = fill {
            Arc::make_mut(&mut map.cells[0]).fill(code);
        }
        map.all_dirty();
        map
    }

    /// A map of `size` with no layers or values (a file being read).
    pub fn bare(size: [u32; 2], tileset: Option<AssetId>) -> Self {
        let size = size.map(|s| s.clamp(1, MAX_SIDE));
        let chunks = (size[0].div_ceil(CHUNK) * size[1].div_ceil(CHUNK)) as usize;
        Self {
            tileset,
            size,
            layers: Vec::new(),
            cells: Vec::new(),
            values: Vec::new(),
            columns: Vec::new(),
            dirty: vec![false; chunks],
            dirty_list: Vec::new(),
            sums: std::sync::Mutex::new(vec![None; chunks]),
            revision: 0,
            layer_revisions: Vec::new(),
            chunk_revisions: vec![0; chunks],
        }
    }

    /// Its size in cells: width and height.
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    fn cell_count(&self) -> usize {
        self.size[0] as usize * self.size[1] as usize
    }

    /// Its layers, bottom first.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Its values.
    pub fn values(&self) -> &[ValueDef] {
        &self.values
    }

    /// The layer called `name`.
    pub fn layer_named(&self, name: &str) -> Option<usize> {
        self.layers.iter().position(|l| l.name == name)
    }

    /// The value called `name`.
    pub fn value_named(&self, name: &str) -> Option<usize> {
        self.values.iter().position(|v| v.name == name)
    }

    /// A number that goes up with every change.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The revision layer `layer` last changed at (0 past the end):
    /// drawing makes its picture again when this changes.
    pub fn layer_revision(&self, layer: usize) -> u64 {
        self.layer_revisions.get(layer).copied().unwrap_or(0)
    }

    /// How many chunks across and down.
    pub fn chunks(&self) -> [u32; 2] {
        [self.size[0].div_ceil(CHUNK), self.size[1].div_ceil(CHUNK)]
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        (x < self.size[0] && y < self.size[1]).then(|| (y * self.size[0] + x) as usize)
    }

    fn chunk_of(&self, x: u32, y: u32) -> usize {
        ((y / CHUNK) * self.chunks()[0] + x / CHUNK) as usize
    }

    fn touch(&mut self, x: u32, y: u32) {
        let c = self.chunk_of(x, y);
        self.sums_mut()[c] = None;
        if !self.dirty[c] {
            self.dirty[c] = true;
            self.dirty_list.push(c as u32);
        }
        self.revision += 1;
        self.chunk_revisions[c] = self.revision;
    }

    fn all_dirty(&mut self) {
        self.dirty_list.clear();
        for (i, d) in self.dirty.iter_mut().enumerate() {
            *d = true;
            self.dirty_list.push(i as u32);
        }
        self.sums_mut().iter_mut().for_each(|s| *s = None);
        self.revision += 1;
        let r = self.revision;
        self.layer_revisions.iter_mut().for_each(|l| *l = r);
        self.chunk_revisions.iter_mut().for_each(|c| *c = r);
    }

    /// The revision chunk `chunk` (column and row) last changed at: 0 past
    /// the map's edge.
    pub fn chunk_revision(&self, chunk: [u32; 2]) -> u64 {
        let [across, down] = self.chunks();
        if chunk[0] >= across || chunk[1] >= down {
            return 0;
        }
        self.chunk_revisions[(chunk[1] * across + chunk[0]) as usize]
    }

    /// The chunks changed since last asked, as (x, y) in chunks.
    pub fn take_dirty(&mut self) -> Vec<[u32; 2]> {
        let across = self.chunks()[0];
        let list = std::mem::take(&mut self.dirty_list);
        list.into_iter()
            .map(|c| {
                self.dirty[c as usize] = false;
                [c % across, c / across]
            })
            .collect()
    }

    /// Every cell of layer `layer`: a code, 0 for empty, in rows from the
    /// top left.
    pub fn layer_cells(&self, layer: usize) -> Option<&[u8]> {
        self.cells.get(layer).map(|c| c.as_slice())
    }

    /// The code in a cell of a layer (0 for empty).
    pub fn code(&self, layer: usize, x: u32, y: u32) -> Option<u8> {
        let i = self.index(x, y)?;
        Some(self.cells.get(layer)?[i])
    }

    /// Sets a cell of a layer to `code` (0 empties it). Whether it changed.
    pub fn set_code(&mut self, layer: usize, x: u32, y: u32, code: u8) -> Result<bool, MapError> {
        let i = self.index(x, y).ok_or(MapError::Outside)?;
        let cells = self.cells.get_mut(layer).ok_or(MapError::NoSuch)?;
        if cells[i] == code {
            return Ok(false);
        }
        Arc::make_mut(cells)[i] = code;
        self.touch(x, y);
        self.layer_revisions[layer] = self.revision;
        Ok(true)
    }

    /// A cell's value.
    pub fn value(&self, value: usize, x: u32, y: u32) -> Option<CellValue> {
        let i = self.index(x, y)?;
        Some(self.columns.get(value)?.get(i))
    }

    /// Sets a cell's value. Whether it changed.
    pub fn set_value(
        &mut self,
        value: usize,
        x: u32,
        y: u32,
        to: CellValue,
    ) -> Result<bool, MapError> {
        let i = self.index(x, y).ok_or(MapError::Outside)?;
        let def = self.values.get(value).ok_or(MapError::NoSuch)?;
        let options = match &def.kind {
            ValueKind::Choice(o) => o.len(),
            _ => 0,
        };
        let column = Arc::make_mut(self.columns.get_mut(value).ok_or(MapError::NoSuch)?);
        let changed = match (column, to) {
            (Column::Number(v), CellValue::Number(n)) => {
                if n.unsigned_abs() > MAX_NUMBER as u32 {
                    return Err(MapError::NotUsable);
                }
                std::mem::replace(&mut v[i], n) != n
            }
            (Column::Switch(v), CellValue::Switch(b)) => std::mem::replace(&mut v[i], b) != b,
            (Column::Choice(v), CellValue::Choice(o)) => {
                let stored = match o {
                    None => 0,
                    Some(o) if usize::from(o) < options => o + 1,
                    Some(_) => return Err(MapError::NotUsable),
                };
                std::mem::replace(&mut v[i], stored) != stored
            }
            (_, _) => return Err(MapError::WrongKind(def.kind.word())),
        };
        if changed {
            self.touch(x, y);
        }
        Ok(changed)
    }

    /// Every cell whose value `value` passes `keep`, in rows from the top
    /// left.
    pub fn cells_where(&self, value: usize, keep: impl Fn(CellValue) -> bool) -> Vec<[u32; 2]> {
        let Some(column) = self.columns.get(value) else {
            return Vec::new();
        };
        let w = self.size[0];
        (0..self.cell_count())
            .filter(|&i| keep(column.get(i)))
            .map(|i| [i as u32 % w, i as u32 / w])
            .collect()
    }

    /// Whether a cell's value is its kind's start (0, off, none): what a
    /// file leaves out.
    pub(crate) fn is_default(&self, value: usize, x: u32, y: u32) -> bool {
        self.index(x, y)
            .zip(self.columns.get(value))
            .is_none_or(|(i, c)| c.is_default(i))
    }

    // Layers and values.

    /// Adds a layer on top. Its index.
    pub fn add_layer(&mut self, name: &str, over: bool) -> Result<usize, MapError> {
        self.add_layer_with_id(Id::random(), name, over)
    }

    /// Adds a layer with a known ID (a file being read).
    pub fn add_layer_with_id(&mut self, id: Id, name: &str, over: bool) -> Result<usize, MapError> {
        if self.layers.len() >= MAX_LAYERS {
            return Err(MapError::Full);
        }
        self.layers.push(Layer {
            id,
            name: name.to_owned(),
            over,
        });
        self.cells.push(Arc::new(vec![0; self.cell_count()]));
        self.layer_revisions.push(0);
        self.all_dirty();
        Ok(self.layers.len() - 1)
    }

    /// Takes a layer away (and its cells).
    pub fn remove_layer(&mut self, layer: usize) -> bool {
        if layer >= self.layers.len() {
            return false;
        }
        self.layers.remove(layer);
        self.cells.remove(layer);
        self.layer_revisions.remove(layer);
        self.all_dirty();
        true
    }

    /// Moves a layer one place up (drawn later) or down.
    pub fn move_layer(&mut self, layer: usize, up: bool) -> bool {
        let other = if up {
            layer + 1
        } else {
            match layer.checked_sub(1) {
                Some(o) => o,
                None => return false,
            }
        };
        if layer >= self.layers.len() || other >= self.layers.len() {
            return false;
        }
        self.layers.swap(layer, other);
        self.cells.swap(layer, other);
        self.all_dirty();
        true
    }

    /// Renames a layer, or sets whether it's drawn over sprites.
    pub fn set_layer(&mut self, layer: usize, name: &str, over: bool) -> bool {
        let Some(l) = self.layers.get_mut(layer) else {
            return false;
        };
        if l.name == name && l.over == over {
            return false;
        }
        l.name = name.to_owned();
        l.over = over;
        self.all_dirty();
        true
    }

    /// Adds a value every cell has, at its kind's start. Its index.
    pub fn add_value(&mut self, name: &str, kind: ValueKind) -> Result<usize, MapError> {
        self.add_value_with_id(Id::random(), name, kind)
    }

    /// Adds a value with a known ID (a file being read).
    pub fn add_value_with_id(
        &mut self,
        id: Id,
        name: &str,
        kind: ValueKind,
    ) -> Result<usize, MapError> {
        if self.values.len() >= MAX_VALUES {
            return Err(MapError::Full);
        }
        if matches!(&kind, ValueKind::Choice(o) if o.len() > MAX_OPTIONS) {
            return Err(MapError::Full);
        }
        self.columns
            .push(Arc::new(Column::new(&kind, self.cell_count())));
        self.values.push(ValueDef {
            id,
            name: name.to_owned(),
            kind,
        });
        self.all_dirty();
        Ok(self.values.len() - 1)
    }

    /// Takes a value away (from every cell).
    pub fn remove_value(&mut self, value: usize) -> bool {
        if value >= self.values.len() {
            return false;
        }
        self.values.remove(value);
        self.columns.remove(value);
        self.all_dirty();
        true
    }

    /// Renames a value.
    pub fn rename_value(&mut self, value: usize, name: &str) -> bool {
        match self.values.get_mut(value) {
            Some(v) if v.name != name => {
                v.name = name.to_owned();
                self.all_dirty();
                true
            }
            _ => false,
        }
    }

    /// Changes a value's kind. The cells keep what still fits: a choice's
    /// options kept by name, everything else back to its start.
    pub fn set_value_kind(&mut self, value: usize, kind: ValueKind) -> bool {
        let Some(def) = self.values.get(value) else {
            return false;
        };
        if def.kind == kind {
            return false;
        }
        if matches!(&kind, ValueKind::Choice(o) if o.len() > MAX_OPTIONS) {
            return false;
        }
        let new = match (&def.kind, &kind, &*self.columns[value]) {
            (ValueKind::Choice(old), ValueKind::Choice(options), Column::Choice(cells)) => {
                let map: Vec<u8> = std::iter::once(0)
                    .chain(old.iter().map(|o| {
                        options
                            .iter()
                            .position(|n| n == o)
                            .map_or(0, |p| p as u8 + 1)
                    }))
                    .collect();
                Column::Choice(cells.iter().map(|&c| map[usize::from(c)]).collect())
            }
            _ => Column::new(&kind, self.cell_count()),
        };
        self.columns[value] = Arc::new(new);
        self.values[value].kind = kind;
        self.all_dirty();
        true
    }

    /// Changes the map's size, keeping the cells that still fit (from the
    /// top left); new cells are empty, at their values' starts.
    pub fn resize(&mut self, size: [u32; 2]) -> bool {
        let size = size.map(|s| s.clamp(1, MAX_SIDE));
        if size == self.size {
            return false;
        }
        let old = self.size;
        let keep = |src: usize| {
            let (x, y) = (src as u32 % old[0], src as u32 / old[0]);
            (x < size[0] && y < size[1]).then(|| (y * size[0] + x) as usize)
        };
        let count = size[0] as usize * size[1] as usize;
        for cells in &mut self.cells {
            let mut fresh = vec![0; count];
            for (i, c) in cells.iter().enumerate() {
                if let Some(j) = keep(i) {
                    fresh[j] = *c;
                }
            }
            *cells = Arc::new(fresh);
        }
        for column in &mut self.columns {
            let fresh = match &**column {
                Column::Number(v) => {
                    let mut f = vec![0; count];
                    v.iter().enumerate().for_each(|(i, c)| {
                        if let Some(j) = keep(i) {
                            f[j] = *c;
                        }
                    });
                    Column::Number(f)
                }
                Column::Switch(v) => {
                    let mut f = vec![false; count];
                    v.iter().enumerate().for_each(|(i, c)| {
                        if let Some(j) = keep(i) {
                            f[j] = *c;
                        }
                    });
                    Column::Switch(f)
                }
                Column::Choice(v) => {
                    let mut f = vec![0; count];
                    v.iter().enumerate().for_each(|(i, c)| {
                        if let Some(j) = keep(i) {
                            f[j] = *c;
                        }
                    });
                    Column::Choice(f)
                }
            };
            *column = Arc::new(fresh);
        }
        let chunks = (size[0].div_ceil(CHUNK) * size[1].div_ceil(CHUNK)) as usize;
        self.size = size;
        self.dirty = vec![false; chunks];
        self.chunk_revisions = vec![0; chunks];
        *self.sums_mut() = vec![None; chunks];
        self.all_dirty();
        true
    }

    /// How many cells of layer `layer` hold each code (by byte).
    pub fn counts(&self, layer: usize) -> [usize; 256] {
        let mut counts = [0; 256];
        if let Some(cells) = self.cells.get(layer) {
            for &c in cells.iter() {
                counts[usize::from(c)] += 1;
            }
        }
        counts
    }

    /// A checksum of the map: its layers, values and every cell, worked
    /// out again only for chunks changed since (what the game's checksum
    /// covers of a map).
    pub fn checksum(&self) -> u64 {
        let mut sum = reflect::Checksum::new();
        sum.u64(u64::from(self.size[0]));
        sum.u64(u64::from(self.size[1]));
        for l in &self.layers {
            sum.bytes(l.name.as_bytes());
            sum.u64(u64::from(l.over));
        }
        for v in &self.values {
            sum.bytes(v.name.as_bytes());
            sum.bytes(v.kind.word().as_bytes());
        }
        let [across, down] = self.chunks();
        for cy in 0..down {
            for cx in 0..across {
                let c = (cy * across + cx) as usize;
                let known = self.sums()[c];
                let s = match known {
                    Some(s) => s,
                    None => {
                        let s = self.chunk_sum(cx, cy);
                        self.sums()[c] = Some(s);
                        s
                    }
                };
                sum.u64(s);
            }
        }
        sum.finish()
    }

    fn sums(&self) -> std::sync::MutexGuard<'_, Vec<Option<u64>>> {
        self.sums
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn sums_mut(&mut self) -> &mut Vec<Option<u64>> {
        self.sums
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn chunk_sum(&self, cx: u32, cy: u32) -> u64 {
        let mut sum = reflect::Checksum::new();
        let w = self.size[0];
        let xs = cx * CHUNK..((cx + 1) * CHUNK).min(w);
        for y in cy * CHUNK..((cy + 1) * CHUNK).min(self.size[1]) {
            let row = (y * w) as usize;
            let (a, b) = (row + xs.start as usize, row + xs.end as usize);
            for cells in &self.cells {
                sum.bytes(&cells[a..b]);
            }
            for column in &self.columns {
                match &**column {
                    Column::Number(v) => v[a..b].iter().for_each(|n| sum.u64(*n as u64)),
                    Column::Switch(v) => v[a..b].iter().for_each(|s| sum.u64(u64::from(*s))),
                    Column::Choice(v) => sum.bytes(&v[a..b]),
                }
            }
        }
        sum.finish()
    }
}

#[cfg(test)]
#[path = "map_tests.rs"]
mod tests;
