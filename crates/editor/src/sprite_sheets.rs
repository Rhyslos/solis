//! Sprite sheets in the editor (Phase 9, D2): the project's Aseprite
//! files as the app read them, the sprites an area draws, and how each
//! becomes what the renderer draws ([`ViewSprite`], laid out as
//! `render::SpriteInstance`). With D3, the project's clips files
//! (`.solframes`) too, and the clips each sheet and clips file offers
//! ([`SpriteSheets::clip_sets`]), which pick the frame a playing sprite
//! shows. With D3b, the project's animators (`.solanim`) too, ready to run
//! ([`SpriteSheets::animator_set`]).

use animation::{ClipSet, FramesFile};
use area::builtin::Sprite;
use assets::sprite_sheet::SpriteSheet;
use glam::{Mat4, Vec2};
use reflect::{AssetId, EntityId};
use std::collections::HashMap;
use std::sync::Arc;

/// What the editor knows of one Aseprite file.
#[derive(Debug, Clone, PartialEq)]
pub enum SheetState {
    /// Being read.
    Reading,
    /// Read and packed.
    Read(Arc<SpriteSheet>),
    /// Couldn't be read, and why.
    Failed(String),
}

/// What the editor knows of one clips file (`.solframes`).
#[derive(Debug, Clone, PartialEq)]
pub enum FramesState {
    /// Read, with what was put right while reading it.
    Read(Arc<FramesFile>, Vec<String>),
    /// Couldn't be read, and why.
    Failed(String),
}

/// What the editor knows of one animator file (`.solanim`).
#[derive(Debug, Clone, PartialEq)]
pub enum AnimatorState {
    /// Read, with what was put right while reading it.
    Read(Arc<animation::AnimatorFile>, Vec<String>),
    /// Couldn't be read, and why.
    Failed(String),
}

/// What the editor knows of one tileset (`.soltileset`, Phase 9, D4).
#[derive(Debug, Clone, PartialEq)]
pub enum TilesetState {
    /// Read, with what was put right while reading it.
    Read(Arc<tiles::Tileset>, Vec<String>),
    /// Couldn't be read, and why.
    Failed(String),
}

/// What the editor knows of one tile map file (`.soltilemap`, D4).
#[derive(Debug, Clone, PartialEq)]
pub enum MapFileState {
    /// Read, with what was put right while reading it.
    Read(Arc<tiles::TileMap>, Vec<String>),
    /// Couldn't be read, and why.
    Failed(String),
}

/// The project's sprite sheets, by path inside the assets and by asset ID.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpriteSheets {
    /// Each Aseprite file, by path inside the assets (`/` between folders).
    pub by_path: HashMap<String, SheetState>,
    /// Each clips file (`.solframes`), by path inside the assets.
    pub frames: HashMap<String, FramesState>,
    /// Each animator file (`.solanim`), by path inside the assets.
    pub animators: HashMap<String, AnimatorState>,
    /// Each tileset (`.soltileset`), by path inside the assets.
    pub tilesets: HashMap<String, TilesetState>,
    /// Each tile map file (`.soltilemap`), by path inside the assets.
    pub tile_maps: HashMap<String, MapFileState>,
    /// Each sheet's, clips file's, animator's, tileset's and map's path,
    /// by asset ID.
    pub paths: HashMap<AssetId, String>,
    /// The clips each sheet and clips file offers, by asset ID: worked out
    /// by [`Self::work_out_clips`] whenever a sheet or clips file changes.
    pub clip_sets: Arc<world::ClipSets>,
    /// Each animator file ready to run, by asset ID: worked out by
    /// [`Self::work_out_clips`] too.
    pub animator_set: Arc<world::Animators>,
    /// Each tile map file read, by asset ID, for the running game: worked
    /// out by [`Self::work_out_clips`] too.
    pub map_set: Arc<world::MapFiles>,
    /// Tile maps' layers as pictures, kept from frame to frame (shared by
    /// every copy of these sheets).
    pub tile_pictures: Arc<crate::tile_view::TilePictures>,
}

impl SpriteSheets {
    /// The sheet with asset ID `id` (an Aseprite file, or a clips file
    /// naming one), if it's been read.
    pub fn get(&self, id: AssetId) -> Option<&Arc<SpriteSheet>> {
        let path = self.paths.get(&id)?;
        match self.by_path.get(path) {
            Some(SheetState::Read(sheet)) => Some(sheet),
            Some(_) => None,
            None => match self.frames.get(path)? {
                FramesState::Read(file, _) => {
                    match self.by_path.get(self.paths.get(&file.sheet?)?)? {
                        SheetState::Read(sheet) => Some(sheet),
                        _ => None,
                    }
                }
                FramesState::Failed(_) => None,
            },
        }
    }

    /// The clips file with asset ID `id`, if it's been read.
    pub fn frames_file(&self, id: AssetId) -> Option<&Arc<FramesFile>> {
        match self.frames.get(self.paths.get(&id)?)? {
            FramesState::Read(file, _) => Some(file),
            FramesState::Failed(_) => None,
        }
    }

    /// The clips of the sheet or clips file `id`.
    pub fn clips(&self, id: AssetId) -> Option<&Arc<ClipSet>> {
        self.clip_sets.get(&id)
    }

    /// Works out every sheet's and clips file's clips again (after one
    /// was read or changed).
    pub fn work_out_clips(&mut self) {
        let mut sets = world::ClipSets::new();
        for (&id, path) in &self.paths {
            if let Some(SheetState::Read(sheet)) = self.by_path.get(path) {
                sets.insert(id, Arc::new(ClipSet::of_sheet(sheet, None)));
            } else if let Some(FramesState::Read(file, _)) = self.frames.get(path)
                && let Some(sheet) = file.sheet.and_then(|s| self.get(s))
            {
                sets.insert(id, Arc::new(ClipSet::of_sheet(sheet, Some(file))));
            }
        }
        self.clip_sets = Arc::new(sets);
        self.work_out_animators();
        self.work_out_maps();
    }

    /// Lists every tile map file read by asset ID again (after one was
    /// read or changed).
    pub fn work_out_maps(&mut self) {
        let mut set = world::MapFiles::new();
        for (&id, path) in &self.paths {
            if let Some(MapFileState::Read(map, _)) = self.tile_maps.get(path) {
                set.insert(id, Arc::clone(map));
            }
        }
        self.map_set = Arc::new(set);
    }

    /// The tileset with asset ID `id`, if it's been read.
    pub fn tileset(&self, id: AssetId) -> Option<&Arc<tiles::Tileset>> {
        match self.tilesets.get(self.paths.get(&id)?)? {
            TilesetState::Read(set, _) => Some(set),
            TilesetState::Failed(_) => None,
        }
    }

    /// The tile map file with asset ID `id`, if it's been read.
    pub fn tile_map(&self, id: AssetId) -> Option<&Arc<tiles::TileMap>> {
        self.map_set.get(&id)
    }

    /// Every tileset (or, with `maps`, every tile map file) read, by name
    /// with its ID, sorted by name.
    pub fn listed_tiles(&self, maps: bool) -> Vec<(String, AssetId)> {
        let mut list: Vec<(String, AssetId)> = self
            .paths
            .iter()
            .filter(|(_, path)| {
                if maps {
                    matches!(self.tile_maps.get(*path), Some(MapFileState::Read(..)))
                } else {
                    matches!(self.tilesets.get(*path), Some(TilesetState::Read(..)))
                }
            })
            .map(|(id, path)| (name_of(path), *id))
            .collect();
        list.sort();
        list
    }

    /// Makes every animator file ready to run again (after one was read or
    /// changed).
    pub fn work_out_animators(&mut self) {
        let mut set = world::Animators::new();
        for (&id, path) in &self.paths {
            if let Some(AnimatorState::Read(file, _)) = self.animators.get(path) {
                set.insert(id, Arc::new(animation::Animator::new(file)));
            }
        }
        self.animator_set = Arc::new(set);
    }

    /// The animator file with asset ID `id`, if it's been read.
    pub fn animator_file(&self, id: AssetId) -> Option<&Arc<animation::AnimatorFile>> {
        match self.animators.get(self.paths.get(&id)?)? {
            AnimatorState::Read(file, _) => Some(file),
            AnimatorState::Failed(_) => None,
        }
    }

    /// Every animator file that's been read, by name with its ID, sorted
    /// by name: what an Animator's field offers.
    pub fn listed_animators(&self) -> Vec<(String, AssetId)> {
        let mut list: Vec<(String, AssetId)> = self
            .paths
            .iter()
            .filter(|(_, path)| matches!(self.animators.get(*path), Some(AnimatorState::Read(..))))
            .map(|(id, path)| (name_of(path), *id))
            .collect();
        list.sort();
        list
    }

    /// The frame `sprite` shows (from 0), `ahead` seconds after its own
    /// time (a preview in the editor; 0 in the game, which moves its time
    /// on itself): its clip's frame when it has one, otherwise its Frame.
    pub fn shown_frame(&self, sprite: &Sprite, frames: usize, ahead: f64) -> usize {
        let clip = sprite
            .sheet
            .and_then(|s| self.clip_sets.get(&s.asset))
            .and_then(|set| set.chosen(&sprite.clip));
        if let Some(clip) = clip {
            let ahead = if sprite.playing && ahead > 0.0 {
                ahead * f64::from(sprite.speed) * f64::from(clip.speed)
            } else {
                0.0
            };
            if let Some(m) = clip.at(f64::from(sprite.time) + ahead)
                && (m.frame as usize) < frames
            {
                return m.frame as usize;
            }
        }
        sprite.frame_index(frames)
    }

    /// The sheet at `path`, if it's been read.
    pub fn at(&self, path: &str) -> Option<&Arc<SpriteSheet>> {
        match self.by_path.get(path)? {
            SheetState::Read(sheet) => Some(sheet),
            _ => None,
        }
    }

    /// Every sheet and clips file that's been read, by name (its file's,
    /// without the extension; a clips file's with " (clips)") with its ID,
    /// sorted by name: what a Sprite's Sheet field offers.
    pub fn listed(&self) -> Vec<(String, AssetId)> {
        let mut list: Vec<(String, AssetId)> = self
            .paths
            .iter()
            .filter_map(
                |(id, path)| match (self.by_path.get(path), self.frames.get(path)) {
                    (Some(SheetState::Read(_)), _) => Some((name_of(path), *id)),
                    (_, Some(FramesState::Read(..))) => {
                        Some((format!("{} (clips)", name_of(path)), *id))
                    }
                    _ => None,
                },
            )
            .collect();
        list.sort();
        list
    }
}

/// A file's name without its folder or extension.
pub(crate) fn name_of(path: &str) -> String {
    let file = path.rsplit_once('/').map_or(path, |(_, f)| f);
    file.rsplit_once('.').map_or(file, |(n, _)| n).to_owned()
}

/// A sprite an area draws: the entity's Sprite, where the entity is, and
/// its pick number (0: can't be picked).
#[derive(Debug, Clone, PartialEq)]
pub struct AreaSprite {
    /// The entity.
    pub entity: EntityId,
    /// Its sprite.
    pub sprite: Sprite,
    /// Where it is in the world.
    pub transform: Mat4,
    /// Its number for picking, as models have (`AreaView::pickable`).
    pub pick: u32,
}

/// One sprite to draw, laid out exactly as `render::SpriteInstance`, so
/// a frame's sprites reach the renderer without a copy (a test checks).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewSprite {
    /// Where its frame's top-left corner is in the world.
    pub corner: [f32; 3],
    /// 1 flip across, 2 flip up, 4 outline, 8 see-through.
    pub flags: u32,
    /// One pixel to the right, in the world (x, y).
    pub axis_x: [f32; 2],
    /// One pixel down the picture, in the world (x, y).
    pub axis_y: [f32; 2],
    /// Its frame's place in the atlas: x, y, width, height in texels.
    pub rect: [f32; 4],
    /// Linear tint, and how white it flashes (a).
    pub tint: [f32; 4],
    /// The outline's linear colour.
    pub outline: [f32; 4],
    /// Where it stands, in the world (x, y).
    pub foot: [f32; 2],
}

/// Every sprite of one sheet drawn in a frame: a run the renderer draws
/// in one call, with each one's pick number.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteBatch {
    /// The sheet.
    pub sheet: AssetId,
    /// Whether they're see-through (drawn after the rest, back to front).
    pub see_through: bool,
    /// The sprites.
    pub sprites: Vec<ViewSprite>,
    /// Each one's pick number, in the same order.
    pub picks: Vec<u32>,
}

/// sRGB (0 to 1) to linear, as the renderer works.
fn linear(c: [f32; 3]) -> [f32; 3] {
    c.map(|v| {
        let v = v.clamp(0.0, 1.0);
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    })
}

/// `sprite` of `sheet` at `transform`, as the renderer draws it. One of
/// the sheet's pixels is `pixel` metres in the world (before the entity's
/// own scale). With `snap` (a whole pixel, in metres), where it stands is
/// put on the pixel grid so it never sits between pixels; turned sprites
/// aren't snapped.
pub fn view_sprite(
    sprite: &Sprite,
    sheet: &SpriteSheet,
    transform: Mat4,
    pixel: f32,
    snap: Option<f32>,
) -> Option<ViewSprite> {
    view_frame(
        sprite,
        sheet,
        sprite.frame_index(sheet.frames.len()),
        transform,
        pixel,
        snap,
    )
}

/// As [`view_sprite`], showing the sheet's frame `index` (from 0): the one
/// its clip has reached.
pub fn view_frame(
    sprite: &Sprite,
    sheet: &SpriteSheet,
    index: usize,
    transform: Mat4,
    pixel: f32,
    snap: Option<f32>,
) -> Option<ViewSprite> {
    let frame = sheet.frames.get(index)?;
    let [_, _, w, h] = frame.rect;
    if w == 0 || h == 0 || !transform.is_finite() {
        return None;
    }
    let mut foot = transform.w_axis.truncate();
    let axis_x = transform.x_axis.truncate().truncate() * pixel;
    // Down the picture is down the screen: the world's -Y.
    let axis_y = -transform.y_axis.truncate().truncate() * pixel;
    let turned = axis_x.y.abs() > 1e-6 || axis_y.x.abs() > 1e-6;
    if let Some(step) = snap.filter(|s| *s > 0.0 && !turned) {
        foot.x = (foot.x / step).round() * step;
        foot.y = (foot.y / step).round() * step;
    }
    let [cw, ch] = sheet.size.map(|v| v as f32);
    let mut pivot = Vec2::from(sheet.pivot(&sprite.pivot, index as u32));
    let mut offset = Vec2::new(frame.offset[0] as f32, frame.offset[1] as f32);
    let size = Vec2::new(w as f32, h as f32);
    // Flipping mirrors the canvas round the pivot; the shader mirrors the
    // frame's own pixels.
    if sprite.flip_x {
        offset.x = cw - offset.x - size.x;
        pivot.x = cw - pivot.x;
    }
    if sprite.flip_y {
        offset.y = ch - offset.y - size.y;
        pivot.y = ch - pivot.y;
    }
    let from_pivot = offset - pivot;
    let corner = foot.truncate() + axis_x * from_pivot.x + axis_y * from_pivot.y;
    let mut flags = 0;
    if sprite.flip_x {
        flags |= 1;
    }
    if sprite.flip_y {
        flags |= 2;
    }
    if sprite.outline {
        flags |= 4;
    }
    if sprite.see_through {
        flags |= 8;
    }
    let [tr, tg, tb] = linear(sprite.tint);
    let [or, og, ob] = linear(sprite.outline_color);
    Some(ViewSprite {
        corner: [corner.x, corner.y, foot.z],
        flags,
        axis_x: axis_x.into(),
        axis_y: axis_y.into(),
        rect: frame.rect.map(|v| v as f32),
        tint: [tr, tg, tb, sprite.flash.clamp(0.0, 1.0)],
        outline: [or, og, ob, 0.0],
        foot: [foot.x, foot.y],
    })
}

/// Every sprite of `sprites` that has a sheet read, as runs of one sheet
/// each (cut-out ones, then see-through ones, sorted far to near, then
/// top of the screen to bottom), ready to draw. Each shows its clip's
/// frame, `ahead` seconds after its own time (a preview; 0 to show it as
/// it is).
pub fn batches(
    sprites: &[AreaSprite],
    sheets: &SpriteSheets,
    pixel: f32,
    snap: Option<f32>,
    ahead: f64,
) -> Vec<SpriteBatch> {
    let mut cut: HashMap<AssetId, SpriteBatch> = HashMap::new();
    let mut clear: Vec<(AssetId, ViewSprite, u32)> = Vec::new();
    for s in sprites {
        let Some(id) = s.sprite.sheet.map(|r| r.asset) else {
            continue;
        };
        let Some(sheet) = sheets.get(id) else {
            continue;
        };
        let frame = sheets.shown_frame(&s.sprite, sheet.frames.len(), ahead);
        let Some(drawn) = view_frame(&s.sprite, sheet, frame, s.transform, pixel, snap) else {
            continue;
        };
        if s.sprite.see_through {
            clear.push((id, drawn, s.pick));
        } else {
            let batch = cut.entry(id).or_insert_with(|| SpriteBatch {
                sheet: id,
                see_through: false,
                sprites: Vec::new(),
                picks: Vec::new(),
            });
            batch.sprites.push(drawn);
            batch.picks.push(s.pick);
        }
    }
    let mut out: Vec<SpriteBatch> = cut.into_values().collect();
    out.sort_by_key(|b| b.sheet);
    // See-through: far to near, then from the top of the screen down,
    // a run each time the sheet changes.
    clear.sort_by(|a, b| {
        a.1.corner[2]
            .total_cmp(&b.1.corner[2])
            .then(b.1.foot[1].total_cmp(&a.1.foot[1]))
    });
    for (id, drawn, pick) in clear {
        match out.last_mut() {
            Some(last) if last.see_through && last.sheet == id => {
                last.sprites.push(drawn);
                last.picks.push(pick);
            }
            _ => out.push(SpriteBatch {
                sheet: id,
                see_through: true,
                sprites: vec![drawn],
                picks: vec![pick],
            }),
        }
    }
    out
}

/// Where a sprite's corner lands: for tests.
#[cfg(test)]
pub(crate) fn corner_of(v: &ViewSprite) -> glam::Vec3 {
    glam::Vec3::from(v.corner)
}

#[cfg(test)]
#[path = "sprite_sheets_tests.rs"]
pub(crate) mod tests;
