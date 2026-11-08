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

/// What the editor knows of one tileset (`.tileset.lst`, Phase 9, D4).
#[derive(Debug, Clone, PartialEq)]
pub enum TilesetState {
    /// Read, with what was put right while reading it.
    Read(Arc<tiles::Tileset>, Vec<String>),
    /// Couldn't be read, and why.
    Failed(String),
}

/// What the editor knows of one tile map file (`.tilemap.lst`, D4).
#[derive(Debug, Clone, PartialEq)]
pub enum MapFileState {
    /// Read, with what was put right while reading it.
    Read(Arc<tiles::TileMap>, Vec<String>),
    /// Couldn't be read, and why.
    Failed(String),
}

/// What the editor knows of one tileset's picture (Phase 9, D5).
#[derive(Debug, Clone, PartialEq)]
pub enum ArtState {
    /// Read whole, every frame.
    Read(Arc<assets::tile_art::TileArt>),
    /// Couldn't be read, and why.
    Failed(String),
}

/// What the editor knows of one atlas file (`.atlas.lst`, Atlas Crop).
#[derive(Debug, Clone, PartialEq)]
pub enum AtlasState {
    /// Read, with what was put right or left out.
    Read(Arc<animation::atlas::Atlas>, Vec<String>),
    /// Couldn't be read, and why.
    Failed(String),
}

/// Whether `path` is a picture a tileset can take its art from.
pub fn is_picture(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".aseprite", ".ase"]
        .iter()
        .any(|e| lower.ends_with(e))
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
    /// Each tileset (`.tileset.lst`), by path inside the assets.
    pub tilesets: HashMap<String, TilesetState>,
    /// Each tile map file (`.tilemap.lst`), by path inside the assets.
    pub tile_maps: HashMap<String, MapFileState>,
    /// Each tileset's picture (a PNG, JPEG or Aseprite file), by path
    /// inside the assets: read whole, for cutting into tiles.
    pub tile_art: HashMap<String, ArtState>,
    /// Each atlas file (`.atlas.lst`, Atlas Crop), by path inside the
    /// assets; its picture is read whole too (into `tile_art`).
    pub atlases: HashMap<String, AtlasState>,
    /// Each sheet's, clips file's, animator's, tileset's, map's and
    /// picture's path, by asset ID.
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
    /// The game's screens' names (decision 0087), for a Screen's choices:
    /// from the program, set when the code changes.
    pub screens: Vec<String>,
}

impl SpriteSheets {
    /// Each tile map game code can reach, by its names (decision 0079),
    /// sorted by path: its layers, its tileset's terrains and its values.
    pub fn map_names(&self) -> Vec<script::map_code::MapNames> {
        use script::map_code::{MapNames, ValueNames};
        let mut all: Vec<MapNames> = self
            .tile_maps
            .iter()
            .filter_map(|(path, state)| match state {
                MapFileState::Read(map, _) => Some((path, map)),
                MapFileState::Failed(_) => None,
            })
            .map(|(path, map)| MapNames {
                path: reflect::data_files::path_stem(path),
                layers: map.layers().iter().map(|l| l.name.clone()).collect(),
                terrains: map
                    .tileset
                    .and_then(|id| self.tileset(id))
                    .map(|set| {
                        set.terrains
                            .iter()
                            .filter_map(|t| {
                                u8::try_from(u32::from(t.code))
                                    .ok()
                                    .map(|c| (c, t.name.clone()))
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                values: map
                    .values()
                    .iter()
                    .map(|v| {
                        let kind = match &v.kind {
                            tiles::ValueKind::Number => ValueNames::Number,
                            tiles::ValueKind::Switch => ValueNames::Switch,
                            tiles::ValueKind::Choice(options) => {
                                ValueNames::Choice(options.clone())
                            }
                        };
                        (v.name.clone(), kind)
                    })
                    .collect(),
            })
            .collect();
        all.sort_by(|a, b| a.path.cmp(&b.path));
        all
    }

    /// Each animator game code can reach, by its names (decision 0078),
    /// sorted by path.
    pub fn animator_names(&self) -> Vec<script::animator_code::AnimatorNames> {
        use script::animator_code::{AnimatorNames, ParamKind};
        let mut all: Vec<AnimatorNames> = self
            .animators
            .iter()
            .filter_map(|(path, state)| match state {
                AnimatorState::Read(file, _) => Some((path, file)),
                AnimatorState::Failed(_) => None,
            })
            .map(|(path, file)| AnimatorNames {
                path: path
                    .rsplit_once('.')
                    .map_or(path.as_str(), |(stem, _)| stem)
                    .to_owned(),
                parameters: file
                    .parameters
                    .iter()
                    .map(|p| {
                        let kind = match p.kind {
                            animation::ParameterKind::Number(_) => ParamKind::Number,
                            animation::ParameterKind::Switch(_) => ParamKind::Switch,
                            animation::ParameterKind::Trigger => ParamKind::Trigger,
                            animation::ParameterKind::Direction(_) => ParamKind::Direction,
                        };
                        (p.name.clone(), kind)
                    })
                    .collect(),
                states: file.states.iter().map(|s| s.name.clone()).collect(),
            })
            .collect();
        all.sort_by(|a, b| a.path.cmp(&b.path));
        all
    }

    /// The pictures cut by Atlas Crop, as `picture/Crop` paths (the
    /// picture's path without its extension), sorted, for the project
    /// file (decision 0089).
    pub fn picture_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .atlases
            .iter()
            .filter_map(|(path, a)| match a {
                AtlasState::Read(atlas, _) => Some((path, atlas)),
                AtlasState::Failed(_) => None,
            })
            .flat_map(|(path, atlas)| {
                let picture = animation::atlas::picture_path(path, atlas);
                let stem = picture
                    .rsplit_once('.')
                    .map_or(picture.as_str(), |(s, _)| s)
                    .to_string();
                atlas
                    .crops
                    .iter()
                    .map(move |c| format!("{stem}/{}", c.name))
            })
            .collect();
        names.sort();
        names
    }

    /// The clips game code can name, as `sheet/clip` paths (the sheet's
    /// path without its extension), and the names of their events (cues),
    /// each sorted, for the project file (decision 0077).
    pub fn clip_names(&self) -> (Vec<String>, Vec<String>) {
        let mut clips = Vec::new();
        let mut cues = Vec::new();
        for (id, set) in self.clip_sets.iter() {
            let Some(path) = self.paths.get(id) else {
                continue;
            };
            let stem = path
                .rsplit_once('.')
                .map_or(path.as_str(), |(stem, _)| stem);
            for clip in &set.clips {
                clips.push(format!("{stem}/{}", clip.name));
                for frame in &clip.frames {
                    cues.extend(frame.events.iter().cloned());
                }
            }
        }
        clips.sort();
        clips.dedup();
        cues.sort();
        cues.dedup();
        (clips, cues)
    }

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

    /// Which terrains of each tileset read are solid, by asset ID, for the
    /// running game's walls (decision 0071). Worked out as asked: tilesets
    /// are few and small.
    pub fn solid_sets(&self) -> world::SolidSets {
        self.paths
            .iter()
            .filter_map(|(&id, path)| match self.tilesets.get(path)? {
                TilesetState::Read(set, _) => Some((id, tiles::walls::solid_codes(set))),
                TilesetState::Failed(_) => None,
            })
            .collect()
    }

    /// The tileset with asset ID `id`, if it's been read.
    pub fn tileset(&self, id: AssetId) -> Option<&Arc<tiles::Tileset>> {
        match self.tilesets.get(self.paths.get(&id)?)? {
            TilesetState::Read(set, _) => Some(set),
            TilesetState::Failed(_) => None,
        }
    }

    /// The picture with asset ID `id`, if it's been read as a tileset's.
    pub fn tile_art(&self, id: AssetId) -> Option<&Arc<assets::tile_art::TileArt>> {
        match self.tile_art.get(self.paths.get(&id)?)? {
            ArtState::Read(art) => Some(art),
            ArtState::Failed(_) => None,
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

    /// What each tile map file's tileset still needs drawn, by the map
    /// file's asset ID: the terrains with art but slots without, in plain
    /// words (Phase 9, D5).
    pub fn tile_gaps(&self) -> HashMap<AssetId, String> {
        let mut out = HashMap::new();
        for (&id, map) in self.map_set.iter() {
            let Some(tileset) = map.tileset else {
                continue;
            };
            let Some(set) = self.tileset(tileset) else {
                continue;
            };
            let gaps: Vec<String> = set
                .terrains
                .iter()
                .filter(|t| set.has_art(t.id))
                .filter_map(|t| {
                    let missing = tiles::autotile::missing_slots(set, t);
                    (!missing.is_empty()).then(|| {
                        let slots: Vec<String> = missing.iter().map(u32::to_string).collect();
                        format!("{} needs slot {}", t.name, slots.join(", "))
                    })
                })
                .collect();
            if !gaps.is_empty() {
                let name = self
                    .paths
                    .get(&tileset)
                    .map_or_else(|| "its tileset".to_owned(), |p| name_of(p));
                out.insert(
                    id,
                    format!(
                        "{name}'s art is missing tiles ({}), drawn pink: open the tileset to add them",
                        gaps.join("; ")
                    ),
                );
            }
        }
        out
    }

    /// Whether any tileset with a picture read has an animated piece.
    pub fn tiles_animate(&self) -> bool {
        self.tilesets.values().any(|t| match t {
            TilesetState::Read(set, _) => {
                set.picture.is_some_and(|p| self.tile_art(p).is_some())
                    && set
                        .pieces
                        .iter()
                        .any(|p| p.animation != tiles::Animation::None)
            }
            TilesetState::Failed(_) => false,
        })
    }

    /// Every picture a tileset can take its art from (PNG, JPEG or
    /// Aseprite), by path with its ID, sorted by path.
    pub fn pictures(&self) -> Vec<(String, AssetId)> {
        let mut list: Vec<(String, AssetId)> = self
            .paths
            .iter()
            .filter(|(_, path)| is_picture(path))
            .map(|(id, path)| (path.clone(), *id))
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
    reflect::data_files::path_stem(file)
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
    /// Its entity's Parallax, which makes it a layer (decision 0094).
    pub parallax: Option<area::builtin::Parallax>,
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
    /// The sprites (shared: a tile map layer's are kept from frame to
    /// frame while nothing in them changes).
    pub sprites: Arc<Vec<ViewSprite>>,
    /// Each one's pick number, in the same order.
    pub picks: Arc<Vec<u32>>,
}

/// sRGB (0 to 1) to linear, as the renderer works.
pub(crate) fn linear(c: [f32; 3]) -> [f32; 3] {
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
/// each, leaving out parallax layers (`sprite_parallax::parallax_layers`) (cut-out ones, then see-through ones, sorted far to near, then
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
    // Gathered in plain lists, then shared (SpriteBatch holds them
    // shared so tile layers can be kept between frames).
    let mut cut: HashMap<AssetId, (Vec<ViewSprite>, Vec<u32>)> = HashMap::new();
    let mut clear: Vec<(AssetId, ViewSprite, u32)> = Vec::new();
    for s in sprites.iter().filter(|s| s.parallax.is_none()) {
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
            let (sprites, picks) = cut.entry(id).or_default();
            sprites.push(drawn);
            picks.push(s.pick);
        }
    }
    let mut out: Vec<SpriteBatch> = cut
        .into_iter()
        .map(|(sheet, (sprites, picks))| SpriteBatch {
            sheet,
            see_through: false,
            sprites: Arc::new(sprites),
            picks: Arc::new(picks),
        })
        .collect();
    out.sort_by_key(|b| b.sheet);
    // See-through: far to near, then from the top of the screen down,
    // a run each time the sheet changes.
    clear.sort_by(|a, b| {
        a.1.corner[2]
            .total_cmp(&b.1.corner[2])
            .then(b.1.foot[1].total_cmp(&a.1.foot[1]))
    });
    let mut runs: Vec<(AssetId, Vec<ViewSprite>, Vec<u32>)> = Vec::new();
    for (id, drawn, pick) in clear {
        match runs.last_mut() {
            Some(last) if last.0 == id => {
                last.1.push(drawn);
                last.2.push(pick);
            }
            _ => runs.push((id, vec![drawn], vec![pick])),
        }
    }
    out.extend(runs.into_iter().map(|(sheet, sprites, picks)| SpriteBatch {
        sheet,
        see_through: true,
        sprites: Arc::new(sprites),
        picks: Arc::new(picks),
    }));
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
