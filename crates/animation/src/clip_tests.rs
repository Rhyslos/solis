//! Clips: which frame shows when, in every direction, and which events
//! are passed, exactly once.

use super::*;
use proptest::prelude::*;

/// Four frames of 100 ms (sheet frames 10 to 13), an event on the third.
fn walk(direction: Direction, repeats: u32) -> Clip {
    let frames = (0..4)
        .map(|i| ClipFrame {
            frame: 10 + i,
            ms: 100,
            events: if i == 2 {
                vec!["footstep".into()]
            } else {
                Vec::new()
            },
        })
        .collect();
    Clip::new("walk", frames, direction, repeats, 1.0, true)
}

fn frames_at(clip: &Clip, times_ms: &[u32]) -> Vec<u32> {
    times_ms
        .iter()
        .map(|&ms| clip.at(f64::from(ms) / 1000.0).unwrap().frame)
        .collect()
}

#[test]
fn each_direction_shows_its_frames_in_order() {
    let every_100 = [0, 100, 200, 300, 400, 500, 600];
    assert_eq!(
        frames_at(&walk(Direction::Forward, 0), &every_100),
        [10, 11, 12, 13, 10, 11, 12]
    );
    assert_eq!(
        frames_at(&walk(Direction::Reverse, 0), &every_100),
        [13, 12, 11, 10, 13, 12, 11]
    );
    // There and back, without showing the ends twice: 1 2 3 4 3 2, 1 …
    assert_eq!(
        frames_at(&walk(Direction::PingPong, 0), &every_100),
        [10, 11, 12, 13, 12, 11, 10]
    );
    assert_eq!(
        frames_at(&walk(Direction::PingPongReverse, 0), &every_100),
        [13, 12, 11, 10, 11, 12, 13]
    );
    assert_eq!(walk(Direction::PingPong, 0).pass_seconds(), 0.6);
    assert_eq!(
        walk(Direction::PingPong, 0)
            .pass_order()
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 2, 1]
    );
}

#[test]
fn a_frame_shows_for_its_own_time() {
    let frames = vec![
        ClipFrame {
            frame: 0,
            ms: 50,
            events: Vec::new(),
        },
        ClipFrame {
            frame: 1,
            ms: 250,
            events: Vec::new(),
        },
    ];
    let clip = Clip::new("swing", frames, Direction::Forward, 0, 1.0, false);
    assert_eq!(frames_at(&clip, &[0, 49, 50, 299, 300]), [0, 0, 1, 1, 0]);
}

#[test]
fn a_clip_played_a_number_of_times_stops_on_its_last_frame() {
    let once = walk(Direction::Forward, 1);
    assert_eq!(once.length_seconds(), Some(0.4));
    let end = once.at(0.399).unwrap();
    assert!(!end.finished);
    assert_eq!(end.frame, 13);
    let after = once.at(5.0).unwrap();
    assert!(after.finished);
    assert_eq!(after.frame, 13);
    // Twice there and back ends where a pass ends: on the second frame.
    let twice = walk(Direction::PingPong, 2);
    assert_eq!(twice.length_seconds(), Some(1.2));
    let end = twice.at(1.2).unwrap();
    assert!(end.finished);
    assert_eq!(end.frame, 11);
    assert_eq!(walk(Direction::Forward, 0).length_seconds(), None);
}

#[test]
fn events_happen_once_as_their_frame_starts() {
    let clip = walk(Direction::Forward, 0);
    let mut out = Vec::new();
    // Ticks of a sixtieth of a second for two passes: two footsteps.
    let tick = 1.0 / 60.0;
    let mut t = 0.0;
    for _ in 0..48 {
        clip.events_between(t, t + tick, &mut out);
        t += tick;
    }
    assert_eq!(out, ["footstep", "footstep"]);
    // The first frame's events come as the clip starts, at 0.
    let first = Clip::new(
        "hit",
        vec![ClipFrame {
            frame: 0,
            ms: 100,
            events: vec!["swing".into()],
        }],
        Direction::Forward,
        1,
        1.0,
        false,
    );
    let mut out = Vec::new();
    first.events_between(0.0, tick, &mut out);
    assert_eq!(out, ["swing"]);
    // Played once: no more after it ends.
    first.events_between(0.1, 10.0, &mut out);
    assert_eq!(out, ["swing"]);
}

#[test]
fn ping_pong_passes_its_event_frame_twice_a_pass() {
    // 1 2 3 4 3 2: the third frame shows twice.
    let clip = walk(Direction::PingPong, 0);
    let mut out = Vec::new();
    clip.events_between(0.0, 0.6, &mut out);
    assert_eq!(out.len(), 2);
}

#[test]
fn odd_numbers_are_safe() {
    let clip = walk(Direction::Forward, 0);
    // Before the start, not a number, endless: the first frame, no events.
    assert_eq!(clip.at(-3.0).unwrap().frame, 10);
    assert_eq!(clip.at(f64::NAN).unwrap().frame, 10);
    assert_eq!(clip.at(f64::INFINITY).unwrap().frame, 10);
    let mut out = Vec::new();
    clip.events_between(f64::NAN, 1.0, &mut out);
    clip.events_between(1.0, 0.5, &mut out);
    assert!(out.is_empty());
    // A huge jump looks through at most MAX_PASSES_A_CALL passes.
    clip.events_between(0.0, 1e12, &mut out);
    assert_eq!(out.len() as u64, MAX_PASSES_A_CALL);
    // No frames: nothing.
    let empty = Clip::new("none", Vec::new(), Direction::Forward, 0, 1.0, false);
    assert_eq!(empty.at(0.0), None);
    // A zero-length frame lasts 1 ms; a speed below 0 or not a number is
    // made sensible.
    let zero = Clip::new(
        "z",
        vec![ClipFrame {
            frame: 0,
            ms: 0,
            events: Vec::new(),
        }],
        Direction::Forward,
        0,
        f32::NAN,
        false,
    );
    assert_eq!(zero.frames[0].ms, 1);
    assert_eq!(zero.speed, 1.0);
    assert_eq!(
        Clip::new("n", Vec::new(), Direction::Forward, 0, -2.0, false).speed,
        0.0
    );
}

proptest! {
    /// However time is cut into steps, the same events come out.
    #[test]
    fn events_don_t_depend_on_how_time_is_cut(cuts in proptest::collection::vec(1u32..400, 1..60)) {
        let clip = walk(Direction::PingPong, 3);
        let mut whole = Vec::new();
        let end: u32 = cuts.iter().sum();
        clip.events_between(0.0, f64::from(end) / 1000.0, &mut whole);
        let mut pieces = Vec::new();
        let mut t = 0u32;
        for c in cuts {
            clip.events_between(f64::from(t) / 1000.0, f64::from(t + c) / 1000.0, &mut pieces);
            t += c;
        }
        prop_assert_eq!(whole, pieces);
    }
}
