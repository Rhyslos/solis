//! A sheet's tags and its clips file, put together.

use super::*;
use crate::clip::Direction;
use crate::frames_file::{OwnClip, TagChange};
use assets::aseprite::AseTag;
use assets::sprite_sheet::SheetFrame;

/// Six frames of 100 ms; the third has an Aseprite event, "step". Tags:
/// walk (frames 1 to 4, ping-pong) and idle (frame 6, once).
pub(crate) fn sheet() -> SpriteSheet {
    let frames = (0..6)
        .map(|i| SheetFrame {
            rect: [0, 0, 1, 1],
            offset: [0, 0],
            duration_ms: 100,
            events: if i == 2 {
                vec!["step".into()]
            } else {
                Vec::new()
            },
        })
        .collect();
    let tag = |name: &str, from, to, direction, repeats| AseTag {
        name: name.into(),
        from,
        to,
        direction,
        repeats,
        text: None,
    };
    SpriteSheet {
        size: [1, 1],
        atlas_size: [1, 1],
        atlas: vec![0; 4],
        frames,
        tags: vec![
            tag("walk", 0, 3, Direction::PingPong, 0),
            tag("idle", 5, 5, Direction::Forward, 1),
        ],
        slices: Vec::new(),
        layers: Vec::new(),
        palette: Vec::new(),
        notes: Vec::new(),
    }
}

#[test]
fn a_sheet_alone_gives_its_tags_as_clips() {
    let set = ClipSet::of_sheet(&sheet(), None);
    assert_eq!(set.clips.len(), 2);
    let walk = set.get("walk").unwrap();
    assert!(walk.from_aseprite);
    assert_eq!(walk.direction, Direction::PingPong);
    assert_eq!(walk.frames.len(), 4);
    assert_eq!(walk.frames[2].events, ["step"]);
    assert_eq!(set.get("idle").unwrap().repeats, 1);
    assert_eq!(set.first, None);
    assert_eq!(set.chosen(""), None, "no clip played first without a file");
    assert!(set.notes.is_empty());
}

#[test]
fn a_clips_file_adds_speed_events_its_own_clips_and_the_first() {
    let file = FramesFile {
        sheet: None,
        first: "hoe".into(),
        tags: vec![TagChange {
            name: "walk".into(),
            speed: 2.0,
            events: vec![(0, "dust".into()), (2, "puff".into())],
        }],
        clips: vec![OwnClip {
            name: "hoe".into(),
            speed: 1.0,
            direction: Direction::Forward,
            repeats: 1,
            frames: vec![
                ClipFrame {
                    frame: 4,
                    ms: 50,
                    events: Vec::new(),
                },
                ClipFrame {
                    frame: 1,
                    ms: 200,
                    events: vec!["swing".into()],
                },
            ],
        }],
    };
    let set = ClipSet::of_sheet(&sheet(), Some(&file));
    let walk = set.get("walk").unwrap();
    assert_eq!(walk.speed, 2.0);
    assert_eq!(walk.frames[0].events, ["dust"]);
    assert_eq!(walk.frames[2].events, ["step", "puff"], "Aseprite's, then Solis's");
    let hoe = set.chosen("").unwrap();
    assert_eq!(hoe.name, "hoe");
    assert!(!hoe.from_aseprite);
    assert_eq!(hoe.at(0.06).unwrap().frame, 1);
    assert_eq!(set.index_of("hoe"), Some(2));
    assert!(set.notes.is_empty(), "{:?}", set.notes);
}

#[test]
fn what_doesn_t_fit_the_sheet_is_left_out_with_a_note() {
    let file = FramesFile {
        sheet: None,
        first: "gone".into(),
        tags: vec![TagChange {
            name: "run".into(),
            speed: 1.0,
            events: Vec::new(),
        }],
        clips: vec![
            OwnClip {
                name: "walk".into(),
                speed: 1.0,
                direction: Direction::Forward,
                repeats: 0,
                frames: Vec::new(),
            },
            OwnClip {
                name: "far".into(),
                speed: 1.0,
                direction: Direction::Forward,
                repeats: 0,
                frames: vec![ClipFrame {
                    frame: 99,
                    ms: 100,
                    events: Vec::new(),
                }],
            },
        ],
    };
    let set = ClipSet::of_sheet(&sheet(), Some(&file));
    assert_eq!(set.clips.len(), 3, "walk (the tag), idle, far");
    assert!(set.get("far").unwrap().frames.is_empty());
    assert_eq!(set.first, None);
    assert_eq!(set.notes.len(), 4, "{:?}", set.notes);
    // A tag past the sheet's end keeps the frames that exist.
    let mut short = sheet();
    short.frames.truncate(2);
    let set = ClipSet::of_sheet(&short, None);
    assert_eq!(set.get("walk").unwrap().frames.len(), 2);
    assert!(set.get("idle").unwrap().frames.is_empty());
}
