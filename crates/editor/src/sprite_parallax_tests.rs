//! Parallax layers (decision 0094): moved for the camera, repeated across
//! the view, sorted into layers behind and in front.

use super::*;
use crate::sprite_sheets::batches;
use crate::sprite_sheets::tests::sheets_with;
use area::builtin::{Parallax, Sprite};
use glam::Vec3;

fn sprite(id: AssetId) -> Sprite {
    Sprite {
        sheet: Some(reflect::AssetRef {
            asset: id,
            part: None,
        }),
        ..Sprite::default()
    }
}

fn layer(id: AssetId, x: f32, y: f32, parallax: Option<Parallax>, pick: u32) -> AreaSprite {
    AreaSprite {
        entity: EntityId::random(),
        sprite: sprite(id),
        transform: Mat4::from_translation(Vec3::new(x, y, 0.0)),
        pick,
        parallax,
    }
}

fn still(scroll: f32) -> Parallax {
    Parallax {
        scroll_x: scroll,
        scroll_y: scroll,
        repeat_x: false,
        repeat_y: false,
        in_front: false,
    }
}

fn nothing_seen(_: f32) -> Option<[Vec2; 2]> {
    None
}

/// The test sheet is 16 × 24 pixels; a pixel is a sixteenth of a metre.
const PX: f32 = 1.0 / 16.0;

#[test]
fn in_the_editor_a_layer_is_where_it_was_put_and_marked_behind() {
    let id = AssetId::random();
    let sheets = sheets_with(id);
    let list = vec![layer(id, 3.0, 2.0, Some(still(0.5)), 1)];
    let view = ParallaxView {
        moved: None,
        seen: &nothing_seen,
    };
    let (behind, front) = parallax_layers(&list, &sheets, PX, Some(PX), 0.0, &view);
    assert!(front.is_empty());
    assert_eq!(behind.len(), 1);
    let drawn = behind[0].batches[0].sprites[0];
    assert_eq!(drawn.foot, [3.0, 2.0]);
    assert_eq!(drawn.flags & (PARALLAX | UNDER), PARALLAX | UNDER);
    assert_eq!(*behind[0].batches[0].picks, [1], "it can be picked");
    // Ordinary batches leave it out.
    assert!(batches(&list, &sheets, PX, Some(PX), 0.0).is_empty());
}

#[test]
fn in_the_game_a_layer_moves_at_its_own_speed_on_whole_pixels() {
    // A whole layer goes along together: two sprites of it stay as far
    // apart as they were put.
    let id = AssetId::random();
    let sheets = sheets_with(id);
    let list = vec![layer(id, 3.0, 2.0, Some(still(0.5)), 0)];
    let at = |moved: Vec2| {
        let view = ParallaxView {
            moved: Some(moved),
            seen: &nothing_seen,
        };
        parallax_layers(&list, &sheets, PX, Some(PX), 0.0, &view).0[0].batches[0].sprites[0].foot
    };
    // The camera where it started: where it was put.
    assert_eq!(at(Vec2::ZERO), [3.0, 2.0]);
    // The camera 8 m right and 2 m up: the layer went along half as far.
    assert_eq!(at(Vec2::new(8.0, 2.0)), [7.0, 3.0]);
    // Between pixels: put on the nearest whole one.
    let foot = at(Vec2::new(0.6 * PX, 0.0));
    assert!((foot[0] / PX - (foot[0] / PX).round()).abs() < 1e-3);
    let two = vec![
        layer(id, 3.0, 2.0, Some(still(0.5)), 0),
        layer(id, 13.0, 2.0, Some(still(0.5)), 0),
    ];
    let view = ParallaxView {
        moved: Some(Vec2::new(20.0, 0.0)),
        seen: &nothing_seen,
    };
    let drawn = &parallax_layers(&two, &sheets, PX, Some(PX), 0.0, &view).0[0].batches[0].sprites;
    assert_eq!((drawn[1].foot[0] - drawn[0].foot[0]).abs(), 10.0);
}

#[test]
fn layers_are_sorted_behind_and_in_front_slowest_first() {
    let id = AssetId::random();
    let sheets = sheets_with(id);
    let front = Parallax {
        in_front: true,
        ..still(2.0)
    };
    let list = vec![
        layer(id, 0.0, 0.0, Some(front), 1),
        layer(id, 0.0, 0.0, Some(still(0.8)), 2),
        layer(id, 0.0, 0.0, None, 3),
        layer(id, 0.0, 0.0, Some(still(0.2)), 4),
        layer(id, 1.0, 0.0, Some(still(0.2)), 5),
        layer(
            id,
            0.0,
            0.0,
            Some(Parallax {
                in_front: true,
                ..still(1.5)
            }),
            6,
        ),
    ];
    let view = ParallaxView {
        moved: None,
        seen: &nothing_seen,
    };
    let (behind, ahead) = parallax_layers(&list, &sheets, PX, None, 0.0, &view);
    let scrolls = |l: &[ParallaxLayer]| l.iter().map(|l| l.scroll[0]).collect::<Vec<_>>();
    assert_eq!(scrolls(&behind), [0.2, 0.8]);
    assert_eq!(scrolls(&ahead), [1.5, 2.0]);
    // The two at 0.2 make one layer, one run of their sheet.
    assert_eq!(behind[0].batches.len(), 1);
    assert_eq!(behind[0].batches[0].sprites.len(), 2);
    for l in &ahead {
        assert_eq!(
            l.batches[0].sprites[0].flags & (PARALLAX | OVER),
            PARALLAX | OVER
        );
    }
    // In order: behind, tiles, the world, in front.
    let world = batches(&list, &sheets, PX, None, 0.0);
    let all = in_order(behind, Vec::new(), world, ahead);
    let picks: Vec<u32> = all.iter().flat_map(|b| b.picks.iter().copied()).collect();
    assert_eq!(picks.len(), 6);
    assert_eq!(picks[2], 2, "the slower layer behind first");
    assert_eq!(picks[3], 3, "then the world");
    assert_eq!(&picks[4..], [6, 1]);
}

#[test]
fn a_repeated_layer_covers_the_view_side_by_side() {
    let id = AssetId::random();
    let sheets = sheets_with(id);
    let p = Parallax {
        repeat_x: true,
        ..still(0.5)
    };
    let list = vec![layer(id, 0.0, 0.0, Some(p), 0)];
    // The view sees from -10 m to 10 m: the sheet is 16 px = 1 m wide.
    let seen = |_: f32| Some([Vec2::new(-10.0, -3.0), Vec2::new(10.0, 3.0)]);
    let view = ParallaxView {
        moved: None,
        seen: &seen,
    };
    let (behind, _) = parallax_layers(&list, &sheets, PX, None, 0.0, &view);
    let feet: Vec<f32> = behind[0].batches[0]
        .sprites
        .iter()
        .map(|s| s.foot[0])
        .collect();
    assert!(feet.len() >= 21, "{}", feet.len());
    assert!(feet.iter().any(|&x| x <= -10.0) && feet.iter().any(|&x| x >= 10.0));
    // A sheet's width apart, all on one row.
    for pair in feet.windows(2) {
        assert!((pair[1] - pair[0] - 1.0).abs() < 1e-4);
    }
    assert!(
        behind[0].batches[0]
            .sprites
            .iter()
            .all(|s| s.foot[1] == 0.0)
    );
    // Up too: rows a sheet's height (24 px = 1.5 m) apart.
    let both = Parallax {
        repeat_y: true,
        ..p
    };
    let list = vec![layer(id, 0.0, 0.0, Some(both), 0)];
    let (behind, _) = parallax_layers(&list, &sheets, PX, None, 0.0, &view);
    let mut rows: Vec<f32> = behind[0].batches[0]
        .sprites
        .iter()
        .map(|s| s.foot[1])
        .collect();
    rows.sort_by(f32::total_cmp);
    rows.dedup();
    assert!(rows.len() >= 5 && rows.iter().any(|&y| y <= -3.0) && rows.iter().any(|&y| y >= 3.0));
    assert!((rows[1] - rows[0] - 1.5).abs() < 1e-4);
}

#[test]
fn a_repeated_layer_has_limits() {
    let id = AssetId::random();
    let sheets = sheets_with(id);
    let p = Parallax {
        repeat_x: true,
        repeat_y: true,
        ..still(0.5)
    };
    let list = vec![layer(id, 0.0, 0.0, Some(p), 0)];
    // Seen to the horizon: a few each way.
    let view = ParallaxView {
        moved: None,
        seen: &nothing_seen,
    };
    let (behind, _) = parallax_layers(&list, &sheets, PX, None, 0.0, &view);
    assert_eq!(behind[0].batches[0].sprites.len(), 17 * 17);
    // Zoomed out to a continent: never more than MAX_REPEATS.
    let huge = |_: f32| Some([Vec2::splat(-1.0e6), Vec2::splat(1.0e6)]);
    let view = ParallaxView {
        moved: None,
        seen: &huge,
    };
    let (behind, _) = parallax_layers(&list, &sheets, PX, None, 0.0, &view);
    assert_eq!(behind[0].batches[0].sprites.len(), MAX_REPEATS);
    // Broken numbers draw nothing strange.
    let broken = |_: f32| Some([Vec2::splat(f32::NAN), Vec2::splat(f32::INFINITY)]);
    let view = ParallaxView {
        moved: Some(Vec2::new(f32::NAN, 0.0)),
        seen: &broken,
    };
    let (behind, _) = parallax_layers(&list, &sheets, PX, None, 0.0, &view);
    assert!(
        behind
            .iter()
            .flat_map(|l| &l.batches)
            .all(|b| b.sprites.len() <= MAX_REPEATS)
    );
}
