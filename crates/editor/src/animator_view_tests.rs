//! What the animator tab writes in its boxes and lists.

use super::*;
use animation::{Clip, ClipFrame, Direction, State};

fn clips() -> ClipSet {
    let frames = vec![ClipFrame {
        frame: 0,
        ms: 100,
        events: Vec::new(),
    }];
    ClipSet {
        clips: vec![
            Clip::new(
                "walk_down",
                frames.clone(),
                Direction::Forward,
                0,
                1.0,
                false,
            ),
            Clip::new("hoe_down", frames, Direction::Forward, 1, 1.0, false),
        ],
        first: Some(0),
        notes: Vec::new(),
    }
}

fn state(plays: Plays, speed_by: &str) -> State {
    State {
        id: Id::from_raw(1).unwrap(),
        name: "S".into(),
        place: [0.0, 0.0],
        plays,
        speed: 1.0,
        speed_by: speed_by.into(),
    }
}

#[test]
fn a_box_says_what_it_plays() {
    let set = clips();
    let by = |clips: Vec<(Facing, &str)>| Plays::ByDirection {
        by: "facing".into(),
        clips: clips.into_iter().map(|(f, c)| (f, c.to_owned())).collect(),
    };
    assert_eq!(
        state_line(&state(Plays::Clip(String::new()), ""), Some(&set)),
        ("no clip".to_owned(), None)
    );
    assert_eq!(
        state_line(&state(Plays::Clip("hoe_down".into()), ""), Some(&set)),
        ("hoe_down · once".to_owned(), None)
    );
    assert_eq!(
        state_line(
            &state(
                by(vec![(Facing::Up, "walk_up"), (Facing::Down, "walk_down")]),
                "speed"
            ),
            Some(&set)
        ),
        (
            "walk_down … · ×speed".to_owned(),
            Some("2 directions".to_owned())
        )
    );
    assert_eq!(
        state_line(&state(by(vec![(Facing::Left, "hoe_down")]), ""), None),
        ("hoe_down …".to_owned(), Some("1 direction".to_owned()))
    );
    assert_eq!(state_line(&state(by(Vec::new()), ""), None).0, "no clips");
}

#[test]
fn every_direction_has_an_arrow() {
    let texts: Vec<String> = Facing::ALL.into_iter().map(facing_text).collect();
    assert_eq!(texts[0], "↑ up");
    assert_eq!(texts[5], "↘ down-right");
    let mut arrows: Vec<char> = texts.iter().filter_map(|t| t.chars().next()).collect();
    arrows.sort_unstable();
    arrows.dedup();
    assert_eq!(arrows.len(), 8);
}
