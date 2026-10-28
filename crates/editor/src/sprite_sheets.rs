//! Sprite sheets in the editor (Phase 9, D2): the project's Aseprite
//! files as the app read them, the sprites an area draws, and how each
//! becomes what the renderer draws ([`ViewSprite`], laid out as
//! `render::SpriteInstance`).

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

/// The project's sprite sheets, by path inside the assets and by asset ID.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpriteSheets {
    /// Each Aseprite file, by path inside the assets (`/` between folders).
    pub by_path: HashMap<String, SheetState>,
    /// Each one's path, by asset ID.
    pub paths: HashMap<AssetId, String>,
}

impl SpriteSheets {
    /// The sheet with asset ID `id`, if it's been read.
    pub fn get(&self, id: AssetId) -> Option<&Arc<SpriteSheet>> {
        match self.by_path.get(self.paths.get(&id)?)? {
            SheetState::Read(sheet) => Some(sheet),
            _ => None,
        }
    }

    /// The sheet at `path`, if it's been read.
    pub fn at(&self, path: &str) -> Option<&Arc<SpriteSheet>> {
        match self.by_path.get(path)? {
            SheetState::Read(sheet) => Some(sheet),
            _ => None,
        }
    }

    /// Every sheet that's been read, by name (its file's, without the
    /// extension) with its ID, sorted by name: what a Sprite's Sheet field
    /// offers.
    pub fn listed(&self) -> Vec<(String, AssetId)> {
        let mut list: Vec<(String, AssetId)> = self
            .paths
            .iter()
            .filter(|(_, path)| matches!(self.by_path.get(*path), Some(SheetState::Read(_))))
            .map(|(id, path)| (name_of(path), *id))
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
    let index = sprite.frame_index(sheet.frames.len());
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
/// top of the screen to bottom), ready to draw.
pub fn batches(
    sprites: &[AreaSprite],
    sheets: &SpriteSheets,
    pixel: f32,
    snap: Option<f32>,
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
        let Some(drawn) = view_sprite(&s.sprite, sheet, s.transform, pixel, snap) else {
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
mod tests;
