//! Parallax layers ([decision 0094](../../../docs/decisions/0094-parallax-layers.md)):
//! sprites with a Parallax, moved for the camera the game looks through,
//! repeated across the view, and gathered into layers drawn behind
//! everything or in front of it.

use crate::sprite_sheets::{AreaSprite, SpriteBatch, SpriteSheets, ViewSprite, view_frame};
use assets::sprite_sheet::SpriteSheet;
use glam::{Mat4, Vec2};
use reflect::{AssetId, EntityId};
use std::sync::Arc;

/// A tile map's layer under everything: `render::SpriteInstance::UNDER`.
pub(crate) const UNDER: u32 = 16;
/// A tile map's layer over everything: `render::SpriteInstance::OVER`.
pub(crate) const OVER: u32 = 32;
/// A parallax layer (decision 0094): `render::SpriteInstance::PARALLAX`,
/// with [`UNDER`] or [`OVER`]. Picked where nothing else is (behind) or
/// over everything (in front).
pub(crate) const PARALLAX: u32 = 64;

/// The most copies of one repeated layer drawn (the view zoomed far out).
pub const MAX_REPEATS: usize = 4096;

/// Where parallax layers are drawn from (decision 0094).
pub struct ParallaxView<'a> {
    /// How far the camera the game looks through has moved from where it
    /// started (x, y); `None` in the editor, where layers are drawn where
    /// they're placed.
    pub moved: Option<Vec2>,
    /// The part of the ground at a height a view sees (for repeating),
    /// `None` when it can't tell (a view up to the horizon).
    pub seen: &'a dyn Fn(f32) -> Option<[Vec2; 2]>,
}

/// One parallax layer's sprites, ready to draw: its speeds and side, and
/// its runs (one sheet each).
#[derive(Debug, Clone, PartialEq)]
pub struct ParallaxLayer {
    /// Its speeds across and up.
    pub scroll: [f32; 2],
    /// Whether it's in front of everything.
    pub in_front: bool,
    /// Its sprites, a run per sheet.
    pub batches: Vec<SpriteBatch>,
}

/// One sprite's copies, with what orders them: side, speeds, height,
/// entity.
struct Gathered {
    in_front: bool,
    scroll: [f32; 2],
    height: f32,
    entity: EntityId,
    sheet: AssetId,
    sprites: Vec<ViewSprite>,
    pick: u32,
}

/// The sprites of `sprites` with a Parallax (decision 0094), as layers:
/// those behind everything (slowest first) and those in front (slowest
/// first). Each goes along with `view`'s camera by what it doesn't
/// scroll, put on whole pixels with
/// `snap`, and repeated across `view`'s ground if it asks. Sprites with
/// the same speeds and side make one layer.
pub fn parallax_layers(
    sprites: &[AreaSprite],
    sheets: &SpriteSheets,
    pixel: f32,
    snap: Option<f32>,
    ahead: f64,
    view: &ParallaxView<'_>,
) -> (Vec<ParallaxLayer>, Vec<ParallaxLayer>) {
    let mut drawn: Vec<Gathered> = Vec::new();
    for s in sprites {
        let Some(parallax) = s.parallax.map(|p| p.sanitized()) else {
            continue;
        };
        let Some(id) = s.sprite.sheet.map(|r| r.asset) else {
            continue;
        };
        let Some(sheet) = sheets.get(id) else {
            continue;
        };
        let place = s.transform.w_axis.truncate();
        let transform = match view.moved {
            Some(moved) => {
                let [x, y] = parallax.offset(moved.into());
                Mat4::from_translation(glam::Vec3::new(x, y, 0.0)) * s.transform
            }
            None => s.transform,
        };
        let frame = sheets.shown_frame(&s.sprite, sheet.frames.len(), ahead);
        let Some(mut one) = view_frame(&s.sprite, sheet, frame, transform, pixel, snap) else {
            continue;
        };
        one.flags |= PARALLAX | if parallax.in_front { OVER } else { UNDER };
        let copies = repeats(&one, sheet, parallax, (view.seen)(one.corner[2]));
        drawn.push(Gathered {
            in_front: parallax.in_front,
            scroll: [parallax.scroll_x, parallax.scroll_y],
            height: place.z,
            entity: s.entity,
            sheet: id,
            sprites: copies,
            pick: s.pick,
        });
    }
    drawn.sort_by(|a, b| {
        a.in_front
            .cmp(&b.in_front)
            .then(a.scroll[0].total_cmp(&b.scroll[0]))
            .then(a.scroll[1].total_cmp(&b.scroll[1]))
            .then(a.height.total_cmp(&b.height))
            .then(a.entity.cmp(&b.entity))
    });
    let mut behind: Vec<ParallaxLayer> = Vec::new();
    let mut front: Vec<ParallaxLayer> = Vec::new();
    for Gathered {
        in_front,
        scroll,
        sheet,
        sprites,
        pick,
        ..
    } in drawn
    {
        let side = if in_front { &mut front } else { &mut behind };
        if side.last().is_none_or(|l| l.scroll != scroll) {
            side.push(ParallaxLayer {
                scroll,
                in_front,
                batches: Vec::new(),
            });
        }
        let Some(layer) = side.last_mut() else {
            continue;
        };
        let picks = vec![pick; sprites.len()];
        match layer.batches.last_mut() {
            Some(last) if last.sheet == sheet => {
                Arc::make_mut(&mut last.sprites).extend(sprites);
                Arc::make_mut(&mut last.picks).extend(picks);
            }
            _ => layer.batches.push(SpriteBatch {
                sheet,
                see_through: false,
                sprites: Arc::new(sprites),
                picks: Arc::new(picks),
            }),
        }
    }
    (behind, front)
}

/// `one` and its copies side by side (and above and below) as `parallax`
/// repeats it, at `sheet`'s canvas size, covering `seen` (the ground in
/// view) with a copy to spare each side; at most [`MAX_REPEATS`]. With
/// no `seen`, a few copies each way.
fn repeats(
    one: &ViewSprite,
    sheet: &SpriteSheet,
    parallax: area::builtin::Parallax,
    seen: Option<[Vec2; 2]>,
) -> Vec<ViewSprite> {
    let step_x = Vec2::from(one.axis_x) * sheet.size[0] as f32;
    let step_y = -Vec2::from(one.axis_y) * sheet.size[1] as f32;
    let corner = Vec2::new(one.corner[0], one.corner[1]);
    // How many steps reach from the copy at `at` to cover `lo`..`hi`
    // along an axis `step` long.
    let span = |on: bool, at: f32, step: f32, lo: f32, hi: f32| -> (i64, i64) {
        if !on || step.abs().partial_cmp(&1e-6) != Some(std::cmp::Ordering::Greater) {
            return (0, 0);
        }
        let (a, b) = ((lo - at) / step, (hi - at) / step);
        let (a, b) = (a.min(b), a.max(b));
        if !(a.is_finite() && b.is_finite()) {
            return (-8, 8);
        }
        let limit = MAX_REPEATS as f32;
        (
            (a.floor() - 1.0).clamp(-limit, limit) as i64,
            (b.ceil() + 1.0).clamp(-limit, limit) as i64,
        )
    };
    let ([lo, hi], known) = match seen {
        Some(r) if r[0].is_finite() && r[1].is_finite() => (r, true),
        _ => ([corner, corner], false),
    };
    let (mut x0, mut x1) = span(parallax.repeat_x, corner.x, step_x.x, lo.x, hi.x);
    let (mut y0, mut y1) = span(parallax.repeat_y, corner.y, step_y.y, lo.y, hi.y);
    if !known {
        if parallax.repeat_x {
            (x0, x1) = (-8, 8);
        }
        if parallax.repeat_y {
            (y0, y1) = (-8, 8);
        }
    }
    let mut out = Vec::new();
    'all: for j in y0..=y1 {
        for i in x0..=x1 {
            if out.len() >= MAX_REPEATS {
                break 'all;
            }
            let shift = step_x * i as f32 + step_y * j as f32;
            let mut copy = *one;
            copy.corner[0] += shift.x;
            copy.corner[1] += shift.y;
            copy.foot[0] += shift.x;
            copy.foot[1] += shift.y;
            out.push(copy);
        }
    }
    out
}

/// Everything a frame draws, in order: parallax layers behind (slowest
/// first), then `tiles` (a tile map's layers, which the renderer draws
/// under and over the rest), then `world`, then layers in front.
pub fn in_order(
    behind: Vec<ParallaxLayer>,
    tiles: Vec<SpriteBatch>,
    world: Vec<SpriteBatch>,
    front: Vec<ParallaxLayer>,
) -> Vec<SpriteBatch> {
    behind
        .into_iter()
        .flat_map(|l| l.batches)
        .chain(tiles)
        .chain(world)
        .chain(front.into_iter().flat_map(|l| l.batches))
        .collect()
}

#[cfg(test)]
#[path = "sprite_parallax_tests.rs"]
mod tests;
