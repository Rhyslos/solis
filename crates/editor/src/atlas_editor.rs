//! An atlas file open in the Atlas Crop tab (decision 0089): a picture's
//! named crops, changed by drawing boxes on it, with undo. Every change is
//! saved at once by the workbench (`workbench_atlas.rs`).

use animation::atlas::{self, Atlas, Crop};

/// The most undo steps kept.
const MAX_UNDO: usize = 200;

/// An atlas file open for editing.
#[derive(Debug, Clone)]
pub(crate) struct AtlasEditor {
    /// The atlas file, inside the assets (`sprites/tools.atlas.lst`).
    pub(crate) path: String,
    /// The crops as they are now.
    pub(crate) atlas: Atlas,
    /// The crop picked, by its number.
    pub(crate) picked: Option<usize>,
    /// The name being typed for the picked crop.
    pub(crate) naming: String,
    /// How big the picture is drawn: screen points per picture pixel.
    pub(crate) zoom: f32,
    /// How far the picture is moved, in points.
    pub(crate) pan: egui::Vec2,
    /// A box being drawn: the picture pixel it started at.
    pub(crate) drawing: Option<[f32; 2]>,
    undo: Vec<Atlas>,
    redo: Vec<Atlas>,
    /// The text last saved, to tell Solis's own save from a change on disk.
    saved: Option<String>,
    /// Whether the file has been read from disk since it was opened.
    pub(crate) on_disk: bool,
}

impl AtlasEditor {
    /// The atlas at `path`, as `atlas`.
    pub(crate) fn new(path: String, atlas: Atlas) -> Self {
        Self {
            path,
            atlas,
            picked: None,
            naming: String::new(),
            zoom: 4.0,
            pan: egui::Vec2::ZERO,
            drawing: None,
            undo: Vec::new(),
            redo: Vec::new(),
            saved: None,
            on_disk: false,
        }
    }

    /// Makes a change as one undo step; whether anything changed.
    fn change(&mut self, f: impl FnOnce(&mut Atlas)) -> bool {
        let before = self.atlas.clone();
        f(&mut self.atlas);
        if self.atlas == before {
            return false;
        }
        self.undo.push(before);
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
        true
    }

    /// Picks crop `i` (or nothing), ready to rename.
    pub(crate) fn pick(&mut self, i: Option<usize>) {
        self.picked = i.filter(|&i| i < self.atlas.crops.len());
        self.naming = self
            .picked
            .map(|i| self.atlas.crops[i].name.clone())
            .unwrap_or_default();
    }

    /// The box from picture pixel `a` to `b`, snapped outwards to the
    /// grid and kept inside a picture `size` big; `None` if it's empty.
    pub(crate) fn snapped(&self, a: [f32; 2], b: [f32; 2], size: [u32; 2]) -> Option<[u32; 4]> {
        if !a.iter().chain(&b).all(|v| v.is_finite()) {
            return None;
        }
        let grid = self.atlas.grid.map(|g| g.max(1) as f32);
        let mut out = [0u32; 4];
        for axis in 0..2 {
            let (lo, hi) = (a[axis].min(b[axis]), a[axis].max(b[axis]));
            if !(lo.is_finite() && hi.is_finite()) {
                return None;
            }
            let start = (lo / grid[axis]).floor() * grid[axis];
            let end = ((hi / grid[axis]).floor() + 1.0) * grid[axis];
            let start = start.clamp(0.0, size[axis] as f32);
            let end = end.clamp(0.0, size[axis] as f32);
            if end <= start {
                return None;
            }
            out[axis] = start as u32;
            out[axis + 2] = (end - start) as u32;
        }
        Some(out)
    }

    /// Adds a crop of `rect`, named `Crop`, `Crop2`…, and picks it.
    pub(crate) fn add(&mut self, rect: [u32; 4]) -> bool {
        if self.atlas.crops.len() >= atlas::MAX_CROPS {
            return false;
        }
        let name = self.atlas.free_name("Crop");
        let changed = self.change(|a| a.crops.push(Crop { name, rect }));
        if changed {
            self.pick(Some(self.atlas.crops.len() - 1));
        }
        changed
    }

    /// Renames the picked crop to what's typed, if it's a good name no
    /// other crop has; gives why not otherwise.
    pub(crate) fn rename(&mut self) -> Result<bool, String> {
        let Some(i) = self.picked else {
            return Ok(false);
        };
        let name = self.naming.trim().to_string();
        if !atlas::good_name(&name) {
            return Err(
                "A crop's name starts with a letter and holds only letters, digits and _ (at most \
                 64)."
                    .into(),
            );
        }
        if self
            .atlas
            .crops
            .iter()
            .enumerate()
            .any(|(j, c)| j != i && c.name.eq_ignore_ascii_case(&name))
        {
            return Err(format!("Another crop is called {name} already."));
        }
        Ok(self.change(|a| a.crops[i].name = name))
    }

    /// Deletes the picked crop.
    pub(crate) fn delete_picked(&mut self) -> bool {
        let Some(i) = self.picked else {
            return false;
        };
        let changed = self.change(|a| {
            a.crops.remove(i);
        });
        self.pick(None);
        changed
    }

    /// Sets the grid crops snap to (each side 1 to 4096).
    pub(crate) fn set_grid(&mut self, grid: [u32; 2]) -> bool {
        let grid = grid.map(|g| g.clamp(1, 4096));
        self.change(|a| a.grid = grid)
    }

    /// A crop for every grid cell of a picture `size` big that `filled`
    /// says has something in it; gives how many were added.
    pub(crate) fn crop_every_tile(
        &mut self,
        size: [u32; 2],
        filled: &dyn Fn([u32; 4]) -> bool,
    ) -> usize {
        let mut added = 0;
        self.change(|a| added = a.crop_every_tile(size, filled));
        added
    }

    /// The crop at picture pixel `at`, topmost (last) first.
    pub(crate) fn crop_at(&self, at: [f32; 2]) -> Option<usize> {
        self.atlas.crops.iter().rposition(|c| {
            let [x, y, w, h] = c.rect.map(|v| v as f32);
            at[0] >= x && at[0] < x + w && at[1] >= y && at[1] < y + h
        })
    }

    /// Undoes the last change; whether there was one.
    pub(crate) fn undo(&mut self) -> bool {
        let Some(before) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.atlas, before));
        self.pick(None);
        true
    }

    /// Redoes the last change undone; whether there was one.
    pub(crate) fn redo(&mut self) -> bool {
        let Some(after) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.atlas, after));
        self.pick(None);
        true
    }

    /// The file's text to save, remembered so Solis's own save isn't
    /// taken for a change on disk.
    pub(crate) fn saving(&mut self) -> String {
        let text = atlas::write(&self.atlas);
        self.saved = Some(text.clone());
        text
    }

    /// Whether `on_disk` is what Solis last saved (or what's open).
    pub(crate) fn is_own(&self, on_disk: &Atlas) -> bool {
        *on_disk == self.atlas
            || self
                .saved
                .as_deref()
                .and_then(|t| atlas::read(t).ok())
                .is_some_and(|r| r.atlas == *on_disk)
    }

    /// The file changed on disk: taken in as one undo step.
    pub(crate) fn taken_in(&mut self, atlas: Atlas) {
        self.change(|a| *a = atlas);
        self.pick(None);
    }
}

#[cfg(test)]
#[path = "atlas_editor_tests.rs"]
mod tests;
