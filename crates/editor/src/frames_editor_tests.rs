//! Editing a clips file: clips made, copied, renamed and taken away;
//! speeds, directions, frame times and events; frames added, moved and
//! taken out; picking; undo and redo; and playing in the Frames tab.

use super::*;
use assets::aseprite::AseTag;
use assets::sprite_sheet::SheetFrame;

/// Six frames of 100 ms; frame 3 has Aseprite's event "step". Tags: walk
/// (frames 1 to 4, ping-pong) and idle (frame 6).
fn sheet() -> SpriteSheet {
    let frames = (0..6)
        .map(|i| SheetFrame {
            rect: [0, 0, 1, 1],
            offset: [0, 0],
            duration_ms: 100 + i * 10,
            events: if i == 2 {
                vec!["step".into()]
            } else {
                Vec::new()
            },
        })
        .collect();
    let tag = |name: &str, from, to, direction| AseTag {
        name: name.into(),
        from,
        to,
        direction,
        repeats: 0,
        text: None,
    };
    SpriteSheet {
        size: [1, 1],
        atlas_size: [1, 1],
        atlas: vec![0; 4],
        frames,
        tags: vec![
            tag("walk", 0, 3, Direction::PingPong),
            tag("idle", 5, 5, Direction::Forward),
        ],
        slices: Vec::new(),
        layers: Vec::new(),
        palette: Vec::new(),
        notes: Vec::new(),
    }
}

fn editor() -> (FramesEditor, SpriteSheet) {
    let sheet = sheet();
    let file = FramesFile {
        sheet: Some(AssetId::random()),
        ..FramesFile::default()
    };
    let clips = ClipSet::of_sheet(&sheet, Some(&file));
    (
        FramesEditor::new("sprites/walker.solframes".into(), file, &clips),
        sheet,
    )
}

#[test]
fn it_opens_on_the_first_clip_and_tag_speeds_and_events_are_kept_apart() {
    let (mut e, sheet) = editor();
    assert_eq!(e.clip, "walk");
    // A speed for a tag is a tag line; back to 1 it goes again.
    assert!(e.set_speed(2.0));
    assert_eq!(e.file.tags[0].speed, 2.0);
    assert!(e.set_speed(1.0));
    assert!(e.file.tags.is_empty(), "nothing left to say about walk");
    // An event on a tag's frame: Solis's, beside Aseprite's.
    e.pick(2, false, false);
    let clips = e.clips(&sheet);
    assert!(e.set_event(&clips, " footstep "));
    assert_eq!(e.file.tags[0].events, [(2, "footstep".to_owned())]);
    let walk = e.clips(&sheet).get("walk").unwrap().clone();
    assert_eq!(walk.frames[2].events, ["step", "footstep"]);
    // Taking Solis's event away again.
    let clips = e.clips(&sheet);
    assert!(e.set_event(&clips, ""));
    assert!(e.file.tags.is_empty());
    // Nothing changed: nothing to save.
    assert!(!e.set_speed(1.0));
}

#[test]
fn clips_are_made_copied_renamed_played_first_and_taken_away() {
    let (mut e, sheet) = editor();
    let clips = e.clips(&sheet);
    assert!(e.copy_clip(&clips));
    assert_eq!(e.clip, "walk copy");
    let copy = &e.file.clips[0];
    assert_eq!(copy.frames.len(), 4);
    assert_eq!(copy.direction, Direction::PingPong);
    assert_eq!(copy.frames[2].events, ["step"], "one event a frame");
    let clips = e.clips(&sheet);
    assert!(e.new_clip(&clips, Vec::new()));
    assert_eq!(e.clip, "clip");
    let clips = e.clips(&sheet);
    assert!(e.new_clip(&clips, Vec::new()));
    assert_eq!(e.clip, "clip 2", "a free name");
    let clips = e.clips(&sheet);
    assert!(!e.rename(&clips, "clip 2", "walk"), "a name taken");
    assert!(!e.rename(&clips, "clip 2", "   "));
    assert!(e.rename(&clips, "clip 2", " hoe "));
    assert_eq!(e.clip, "hoe");
    assert!(e.toggle_first());
    assert_eq!(e.file.first, "hoe");
    let clips = e.clips(&sheet);
    assert!(e.rename(&clips, "hoe", "swing"));
    assert_eq!(e.file.first, "swing", "the first follows a rename");
    assert!(e.delete_clip());
    assert!(e.file.first.is_empty());
    assert!(!e.file.clips.iter().any(|c| c.name == "swing"));
}

#[test]
fn frames_of_a_clip_made_here_are_added_timed_moved_and_taken_out() {
    let (mut e, sheet) = editor();
    let clips = e.clips(&sheet);
    e.new_clip(&clips, Vec::new());
    assert!(e.add_frames(&sheet, &[0, 1, 2, 99]));
    let frames =
        |e: &FramesEditor| -> Vec<u32> { e.file.clips[0].frames.iter().map(|f| f.frame).collect() };
    assert_eq!(
        frames(&e),
        [0, 1, 2],
        "a frame the sheet hasn't is left out"
    );
    assert_eq!(e.file.clips[0].frames[1].ms, 110, "Aseprite's time");
    assert_eq!(e.selected, [0, 1, 2]);
    // Added after the last picked frame.
    e.pick(0, false, false);
    e.add_frames(&sheet, &[5]);
    assert_eq!(frames(&e), [0, 5, 1, 2]);
    // Move frames 1 and 2 (5 and 1) to the end, in their order.
    e.pick(1, false, false);
    e.pick(2, false, true);
    assert_eq!(e.selected, [1, 2]);
    assert!(e.move_frames(4));
    assert_eq!(frames(&e), [0, 2, 5, 1]);
    assert_eq!(e.selected, [2, 3]);
    // And to the start.
    assert!(e.move_frames(0));
    assert_eq!(frames(&e), [5, 1, 0, 2]);
    // Times and events for every frame picked.
    e.pick(0, false, false);
    e.pick(3, true, false);
    assert!(e.set_ms(250));
    let clips = e.clips(&sheet);
    assert!(e.set_event(&clips, "swing"));
    let c = &e.file.clips[0];
    assert_eq!(
        (c.frames[0].ms, c.frames[3].ms, c.frames[1].ms),
        (250, 250, 110)
    );
    assert_eq!(c.frames[3].events, ["swing"]);
    assert!(e.set_ms(0), "made 1 ms");
    assert_eq!(e.file.clips[0].frames[0].ms, 1);
    assert!(e.delete_frames());
    assert_eq!(frames(&e), [1, 0]);
    // Direction and repeats.
    assert!(e.set_direction(Direction::Reverse));
    assert!(e.set_repeats(3));
    assert_eq!(e.file.clips[0].repeats, 3);
}

#[test]
fn a_tag_s_frames_aren_t_changed_here() {
    let (mut e, sheet) = editor();
    e.pick(0, false, false);
    assert!(!e.add_frames(&sheet, &[1]));
    assert!(!e.set_ms(500));
    assert!(!e.delete_frames());
    assert!(!e.move_frames(3));
    assert!(!e.set_direction(Direction::Reverse));
    assert!(e.file.clips.is_empty());
}

#[test]
fn undo_and_redo_step_through_the_changes() {
    let (mut e, sheet) = editor();
    let clips = e.clips(&sheet);
    e.new_clip(&clips, Vec::new());
    e.add_frames(&sheet, &[0, 1]);
    assert_eq!(e.can_undo_redo(), (true, false));
    assert!(e.undo());
    assert!(e.file.clips[0].frames.is_empty());
    assert!(e.undo());
    assert!(e.file.clips.is_empty());
    assert!(!e.undo());
    assert!(e.redo());
    assert!(e.redo());
    assert_eq!(e.file.clips[0].frames.len(), 2);
    assert!(!e.redo());
    // A new change forgets what could be redone.
    e.undo();
    e.set_repeats(2);
    assert_eq!(e.can_undo_redo().1, false);
    // The file as text reads back as it is.
    let back = frames_file::read(&e.text()).unwrap().file;
    assert_eq!(back, e.file);
}

#[test]
fn playing_in_the_frames_tab_loops_or_stops_at_the_end() {
    let (mut e, sheet) = editor();
    let walk = e.clips(&sheet).get("walk").unwrap().clone();
    e.play_on(&walk, 1.0);
    assert_eq!(e.time, 0.0, "not playing");
    e.playing = true;
    e.play_on(&walk, 0.1);
    assert!((e.time - 0.1).abs() < 1e-9);
    // Ping-pong over and over: within one pass.
    e.play_on(&walk, 10.0);
    assert!(e.time < walk.pass_seconds());
    assert!(e.playing);
    let once = Clip::new(
        "once",
        walk.frames.clone(),
        Direction::Forward,
        1,
        1.0,
        false,
    );
    e.time = 0.0;
    e.play_on(&once, 5.0);
    assert!(!e.playing);
    assert_eq!(Some(e.time), once.length_seconds());
    // Choosing another clip starts it from the beginning.
    e.choose("idle");
    assert_eq!(e.time, 0.0);
}
