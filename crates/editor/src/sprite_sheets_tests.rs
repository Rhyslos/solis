//! Sprites as the renderer draws them: where they stand, flipped, snapped
//! to pixels, and gathered into runs.

use super::*;
use assets::aseprite::{AseSlice, SliceKey};
use assets::sprite_sheet::SheetFrame;
use glam::Vec3;

/// A 16 × 24 sheet of two frames, each trimmed to an 8 × 20 picture at
/// (4, 4), with a "feet" slice whose pivot is at (8, 23).
pub(crate) fn test_sheet() -> SpriteSheet {
    let frame = |x: u32| SheetFrame {
        rect: [x, 1, 8, 20],
        offset: [4, 4],
        duration_ms: 100,
        events: Vec::new(),
    };
    SpriteSheet {
        size: [16, 24],
        atlas_size: [32, 32],
        atlas: vec![255; 32 * 32 * 4],
        frames: vec![frame(1), frame(12)],
        tags: Vec::new(),
        slices: vec![AseSlice {
            name: "feet".into(),
            keys: vec![SliceKey {
                frame: 0,
                rect: [6, 22, 4, 2],
                center: None,
                pivot: Some([2, 1]),
            }],
            text: None,
        }],
        layers: Vec::new(),
        palette: Vec::new(),
        notes: Vec::new(),
    }
}

/// The sheets, with `test_sheet` as asset `id` at `sprites/walker.aseprite`.
pub(crate) fn sheets_with(id: AssetId) -> SpriteSheets {
    let mut sheets = SpriteSheets::default();
    sheets.by_path.insert(
        "sprites/walker.aseprite".into(),
        SheetState::Read(Arc::new(test_sheet())),
    );
    sheets.paths.insert(id, "sprites/walker.aseprite".into());
    sheets
}

fn sprite(id: AssetId) -> Sprite {
    Sprite {
        sheet: Some(reflect::AssetRef {
            asset: id,
            part: None,
        }),
        pivot: "feet".into(),
        ..Sprite::default()
    }
}

#[test]
fn a_sprite_stands_on_its_pivot() {
    let sheet = test_sheet();
    let at = Mat4::from_translation(Vec3::new(10.0, 5.0, 2.0));
    let id = AssetId::random();
    let v = view_sprite(&sprite(id), &sheet, at, 1.0 / 16.0, None).unwrap();
    // The picture starts 4 pixels left of the pivot's 8 and 19 above its
    // 23: a pixel is a sixteenth of a metre, and down the picture is -Y.
    let c = corner_of(&v);
    assert!((c.x - (10.0 - 4.0 / 16.0)).abs() < 1e-5, "{c}");
    assert!((c.y - (5.0 + 19.0 / 16.0)).abs() < 1e-5, "{c}");
    assert_eq!(c.z, 2.0);
    assert_eq!(v.foot, [10.0, 5.0]);
    assert_eq!(v.axis_x, [1.0 / 16.0, 0.0]);
    assert_eq!(v.axis_y, [0.0, -1.0 / 16.0]);
    assert_eq!(v.rect, [1.0, 1.0, 8.0, 20.0]);
    assert_eq!(v.tint, [1.0, 1.0, 1.0, 0.0]);
    // Without a pivot slice: the bottom middle.
    let plain = Sprite {
        pivot: String::new(),
        ..sprite(id)
    };
    let v = view_sprite(&plain, &sheet, at, 1.0 / 16.0, None).unwrap();
    assert!((corner_of(&v).y - (5.0 + 20.0 / 16.0)).abs() < 1e-5);
}

#[test]
fn flipping_mirrors_round_the_pivot_and_the_frame_picks_the_picture() {
    let sheet = test_sheet();
    let id = AssetId::random();
    let flipped = Sprite {
        flip_x: true,
        frame: 2,
        ..sprite(id)
    };
    let v = view_sprite(&flipped, &sheet, Mat4::IDENTITY, 1.0, None).unwrap();
    // The canvas mirrored round x = 8: the picture now spans 4 to 12 (the
    // same, as it's centred), and the flag tells the shader to mirror it.
    assert_eq!(v.corner[0], 4.0 - 8.0);
    assert_eq!(v.flags & 1, 1);
    assert_eq!(v.rect[0], 12.0, "the second frame");
    // A frame past the end shows the first.
    let past = Sprite {
        frame: 99,
        ..sprite(id)
    };
    assert_eq!(
        view_sprite(&past, &sheet, Mat4::IDENTITY, 1.0, None)
            .unwrap()
            .rect[0],
        1.0
    );
}

#[test]
fn snapping_puts_where_it_stands_on_a_whole_pixel_unless_it_s_turned() {
    let sheet = test_sheet();
    let id = AssetId::random();
    let px = 1.0 / 16.0;
    let at = Mat4::from_translation(Vec3::new(3.03, 1.0 + 0.4 * px, 0.0));
    let v = view_sprite(&sprite(id), &sheet, at, px, Some(px)).unwrap();
    assert!((v.foot[0] * 16.0 - (v.foot[0] * 16.0).round()).abs() < 1e-4);
    assert_eq!(v.foot[1], 1.0);
    let turned = at * Mat4::from_rotation_z(0.3);
    let v = view_sprite(&sprite(id), &sheet, turned, px, Some(px)).unwrap();
    assert_eq!(v.foot[0], 3.03, "turned: not snapped");
    assert!(v.axis_x[1].abs() > 0.0);
}

#[test]
fn sprites_gather_into_a_run_per_sheet_and_see_through_ones_last_far_to_near() {
    let a = AssetId::random();
    let mut sheets = sheets_with(a);
    let b = AssetId::random();
    sheets.by_path.insert(
        "b.aseprite".into(),
        SheetState::Read(Arc::new(test_sheet())),
    );
    sheets.paths.insert(b, "b.aseprite".into());
    let at = |x: f32, y: f32, z: f32| Mat4::from_translation(Vec3::new(x, y, z));
    let entity = EntityId::random;
    let clear = |id| Sprite {
        see_through: true,
        ..sprite(id)
    };
    let list = vec![
        AreaSprite {
            entity: entity(),
            sprite: sprite(a),
            transform: at(0.0, 0.0, 0.0),
            pick: 1,
        },
        AreaSprite {
            entity: entity(),
            sprite: sprite(b),
            transform: at(1.0, 0.0, 0.0),
            pick: 2,
        },
        AreaSprite {
            entity: entity(),
            sprite: sprite(a),
            transform: at(2.0, 0.0, 0.0),
            pick: 3,
        },
        AreaSprite {
            entity: entity(),
            sprite: clear(a),
            transform: at(0.0, 0.0, 5.0),
            pick: 4,
        },
        AreaSprite {
            entity: entity(),
            sprite: clear(a),
            transform: at(0.0, 3.0, 1.0),
            pick: 5,
        },
        // No sheet, and a sheet not read: left out.
        AreaSprite {
            entity: entity(),
            sprite: Sprite::default(),
            transform: at(0.0, 0.0, 0.0),
            pick: 6,
        },
        AreaSprite {
            entity: entity(),
            sprite: sprite(AssetId::random()),
            transform: at(0.0, 0.0, 0.0),
            pick: 7,
        },
    ];
    let runs = batches(&list, &sheets, 1.0, None);
    let cut: Vec<&SpriteBatch> = runs.iter().filter(|r| !r.see_through).collect();
    assert_eq!(cut.len(), 2);
    let of_a = cut.iter().find(|r| r.sheet == a).unwrap();
    assert_eq!(of_a.picks, [1, 3]);
    let clear: Vec<u32> = runs
        .iter()
        .filter(|r| r.see_through)
        .flat_map(|r| r.picks.clone())
        .collect();
    assert_eq!(clear, [5, 4], "far (lower) first");
    assert!(runs.last().unwrap().see_through, "see-through ones last");
}

#[test]
fn the_sheets_list_what_s_read_by_name() {
    let a = AssetId::random();
    let mut sheets = sheets_with(a);
    sheets
        .by_path
        .insert("broken.aseprite".into(), SheetState::Failed("no".into()));
    sheets
        .paths
        .insert(AssetId::random(), "broken.aseprite".into());
    assert_eq!(sheets.listed(), vec![("walker".to_owned(), a)]);
    assert!(sheets.get(a).is_some());
    assert!(sheets.at("broken.aseprite").is_none());
    assert_eq!(name_of("a/b/c.aseprite"), "c");
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_batching_100000_sprites() {
    // The plan's budget for a 2D game: 100,000 sprites over four sheets,
    // a tenth of them see-through, turned into runs each frame they move.
    const TARGET_MS: f64 = 4.0;
    let mut sheets = SpriteSheets::default();
    let ids: Vec<AssetId> = (0..4).map(|_| AssetId::random()).collect();
    for (n, id) in ids.iter().enumerate() {
        let path = format!("sheet{n}.aseprite");
        sheets
            .by_path
            .insert(path.clone(), SheetState::Read(Arc::new(test_sheet())));
        sheets.paths.insert(*id, path);
    }
    let list: Vec<AreaSprite> = (0..100_000u32)
        .map(|i| AreaSprite {
            entity: EntityId::random(),
            sprite: Sprite {
                see_through: i % 10 == 0,
                ..sprite(ids[i as usize % 4])
            },
            transform: Mat4::from_translation(Vec3::new(
                (i % 300) as f32 * 0.3,
                (i / 300) as f32 * 0.3,
                0.0,
            )),
            pick: i + 1,
        })
        .collect();
    let px = 1.0 / 16.0;
    let frames = 20;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        let runs = batches(&list, &sheets, px, Some(px));
        assert_eq!(runs.iter().map(|r| r.sprites.len()).sum::<usize>(), 100_000);
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: batching 100,000 sprites (four sheets, a tenth see-through) {ms:.2} ms (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "batching sprites takes {ms:.2} ms");
}
