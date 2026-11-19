//! Text typing out on the game's screens (stage 1, text effects): starts
//! kept while words stay, `TypingFinished` once, `finishTyping`, the
//! typing sound, and the same ticks at any frame rate.

use super::*;
use crate::test_support::*;
use crate::workbench::Workbench;
use level::EntityData;
use level::builtin::Screen;
use ui::view::Node;

fn view(texts: &[&str]) -> Node {
    Node {
        kind: "view".into(),
        children: texts
            .iter()
            .map(|t| Node {
                kind: "text".into(),
                text: (*t).into(),
                ..Node::default()
            })
            .collect(),
        ..Node::default()
    }
}

fn typist(speed: f32, sound: Option<&str>) -> Typist {
    Typist {
        speed,
        sound: sound.map(str::to_string),
        every: SoundEvery::Letter,
    }
}

fn entity() -> EntityId {
    EntityId(reflect::Id::from_raw(7).unwrap())
}

#[test]
fn a_text_keeps_its_start_while_its_words_stay_and_starts_again_when_they_change() {
    let mut t = ScreenTyping::default();
    let e = entity();
    let first = t.update(
        (e, "Talk"),
        &view(&["Mira", "Hello there"]),
        typist(10.0, None),
        1.0,
    );
    assert_eq!(first.at, [1.0, 1.0]);
    let same = t.update(
        (e, "Talk"),
        &view(&["Mira", "Hello there"]),
        typist(10.0, None),
        2.0,
    );
    assert!(
        Arc::ptr_eq(&first, &same),
        "nothing changed: the same starts"
    );
    let next = t.update(
        (e, "Talk"),
        &view(&["Mira", "Next line"]),
        typist(10.0, None),
        3.0,
    );
    assert_eq!(next.at, [1.0, 3.0], "only the changed text starts again");
    t.keep_only(&[(e, "Talk".into())]);
    assert!(t.active);
    t.keep_only(&[]);
    assert!(
        !t.active && t.started(e, "Talk").is_none(),
        "hidden: forgotten"
    );
}

#[test]
fn typing_finished_is_said_once_when_every_text_has_typed_out() {
    let mut t = ScreenTyping::default();
    let e = entity();
    t.update(
        (e, "Talk"),
        &view(&["Hi", "Hello"]),
        typist(10.0, None),
        0.0,
    );
    // "Hello" types for 0.4 s.
    assert!(t.tick(0.3).0.is_empty());
    let (sent, _) = t.tick(0.45);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].name, FINISHED);
    assert_eq!(sent[0].to, e);
    assert_eq!(sent[0].values, [reflect::Value::Text("Talk".into())]);
    assert!(t.tick(0.5).0.is_empty(), "once");
    // A new line types and is said again.
    t.update((e, "Talk"), &view(&["Hi", "Bye"]), typist(10.0, None), 1.0);
    assert!(t.tick(1.1).0.is_empty());
    assert_eq!(t.tick(1.25).0.len(), 1);
}

#[test]
fn a_screen_that_shows_text_at_once_says_nothing() {
    let mut t = ScreenTyping::default();
    t.update(
        (entity(), "Hud"),
        &view(&["Gold 5"]),
        typist(0.0, Some("Sfx.Blip")),
        0.0,
    );
    let (sent, sounds) = t.tick(1.0);
    assert!(sent.is_empty() && sounds.is_empty());
    t.keep_only(&[(entity(), "Hud".into())]);
    assert!(!t.active);
}

#[test]
fn finish_typing_shows_it_all_and_typing_finished_follows() {
    let mut t = ScreenTyping::default();
    let e = entity();
    t.update(
        (e, "Talk"),
        &view(&["A long line of dialogue"]),
        typist(10.0, None),
        0.0,
    );
    t.finish(e, "talk", 0.2);
    let started = t.started(e, "Talk").unwrap();
    assert_eq!(started.at, [f64::NEG_INFINITY], "shown whole");
    assert_eq!(t.tick(0.3).0.len(), 1, "finished at once");
    // Finishing what has finished does nothing.
    t.finish(e, "Talk", 0.4);
    assert!(t.tick(0.5).0.is_empty());
}

#[test]
fn the_typing_sound_plays_at_most_once_a_tick_and_never_on_spaces() {
    let mut t = ScreenTyping::default();
    let e = entity();
    t.update(
        (e, "Talk"),
        &view(&["ab  cd"]),
        typist(60.0, Some("Sfx.Blip")),
        0.0,
    );
    let tick = world::TICK;
    // Half way through each tick, so no letter is due exactly on one.
    let sounds: Vec<usize> = (0..8)
        .map(|i| t.tick((i as f64 + 0.5) * tick).1.len())
        .collect();
    // a, b, (space), (space), c, d: one a tick, silent on the spaces.
    assert_eq!(sounds, [1, 1, 0, 0, 1, 1, 0, 0]);
    // Each word only.
    let mut t = ScreenTyping::default();
    let words = Typist {
        every: SoundEvery::Word,
        ..typist(60.0, Some("Sfx.Blip"))
    };
    t.update((e, "Talk"), &view(&["ab cd"]), words, 0.0);
    let sounds: Vec<usize> = (0..6)
        .map(|i| t.tick((i as f64 + 0.5) * tick).1.len())
        .collect();
    assert_eq!(sounds, [1, 0, 0, 1, 0, 0]);
}

#[test]
fn a_load_that_takes_the_clock_back_starts_typing_again() {
    let mut t = ScreenTyping::default();
    let e = entity();
    t.update((e, "Talk"), &view(&["Hello"]), typist(10.0, None), 50.0);
    let again = t.update((e, "Talk"), &view(&["Hello"]), typist(10.0, None), 2.0);
    assert_eq!(again.at, [2.0]);
}

#[test]
fn mistakes_in_text_worked_out_as_the_game_runs_are_said() {
    let said = mistakes_in("Talk", &view(&["fine", "[glow]oops", "[wave]ok[/wave]"]));
    assert_eq!(said.len(), 1);
    assert!(
        said[0].contains("Talk") && said[0].contains("isn't a text effect"),
        "{}",
        said[0]
    );
}

/// Game code that shows the Talk screen, counts `TypingFinished` and the
/// tick it came on, and skips typing on tick `skip_at` (0 for never).
fn talker(skip_at: i64) -> String {
    format!(
        "fetch \"solis/project\":Screen;\n\
         fetch \"solis/screens\":TypingFinished;\n\
         component Talker {{\n\
             has {{\n\
                 int ticks = 0;\n\
                 int finished = 0;\n\
                 int finishedAt = 0;\n\
             }}\n\
             method update(float dt) {{\n\
                 ticks = ticks + 1;\n\
                 is ticks == {skip_at}? then {{ finishTyping(Screen.Talk); }}\n\
             }}\n\
             on TypingFinished(Screen which) {{\n\
                 is which == Screen.Talk? then {{\n\
                     finished = finished + 1;\n\
                     finishedAt = ticks;\n\
                 }}\n\
             }}\n\
         }}\n\
         pass Talker;\nend\n"
    )
}

const TALK: &str = "assembly Talk {\n    view {\n        column [width 600] {\n            \
    text \"Mira\";\n            text \"Oh! A new face in the village. [pause 0.2]Have you seen my [colour=red]red roses[/colour]?\";\n        }\n    }\n}\npass Talk;\nend\n";

fn talk_program(skip_at: i64) -> Arc<script::Compiled> {
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::standard().and_screens(&["Talk"]),
    );
    files.push(("main.lst".into(), talker(skip_at)));
    files.push(("ui/talk.lst".into(), TALK.into()));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    }
}

/// A workbench playing the talk screen at 40 letters a second, typing
/// sound on, on an entity with a Talker.
fn talking(skip_at: i64) -> (Workbench, EntityId) {
    let mut w = Workbench::new(info());
    unhurried(&mut w);
    w.set_program(Some(talk_program(skip_at)));
    let id = EntityId::random();
    let entity = EntityData::new("Mira").with(&Screen {
        typing_speed: 40.0,
        typing_sound: "Sfx.Blip".into(),
        ..Screen::showing("Talk")
    });
    w.open
        .document
        .edit(
            "Add",
            vec![level::Edit::Spawn {
                entities: vec![(id, entity)],
            }],
            None,
            None,
        )
        .unwrap();
    w.apply_property(
        crate::properties::PropertyRequest::AddComponent {
            entity: id,
            component: "Talker".into(),
        },
        1,
    );
    (w, id)
}

fn talker_field(w: &mut Workbench, id: EntityId, field: &str) -> i64 {
    let game = w.open.playing.as_mut().unwrap().game.as_mut().unwrap();
    match game.values(id, "Talker").unwrap().get(field) {
        Some(reflect::Value::Int(n)) => *n,
        other => panic!("{other:?}"),
    }
}

/// Plays `ticks` ticks at `fps` frames a second, giving the UI clock and
/// how many letters of the line show on each frame.
fn play_at(fps: u32, ticks: u64, skip_at: i64) -> (Workbench, EntityId, Vec<(f64, usize)>) {
    let (mut w, id) = talking(skip_at);
    w.play();
    let mut frames = Vec::new();
    let mut fonts = crate::game_screens::editor_fonts();
    while w.world().unwrap().clock().ticks < ticks {
        w.run_game(1.0 / fps as f32);
        let world = w.world().unwrap();
        let now = (world.clock().ticks as f64 + f64::from(world.progress())) * world::TICK;
        let picture = w.open.game_screens.pictures[0].clone();
        fonts.set_clock(now);
        fonts.begin_frame();
        let shapes = ui::layout_with(&picture, [1280, 720], None, Some(&mut fonts));
        let letters = shapes
            .iter()
            .filter(|s| s.kind == ui::ShapeKind::Letter)
            .count();
        frames.push((now, letters));
    }
    (w, id, frames)
}

#[test]
fn the_line_types_out_the_same_and_finishes_on_the_same_tick_at_30_and_144_frames_a_second() {
    let (mut slow, id, slow_frames) = play_at(30, 240, 0);
    let (mut fast, fast_id, fast_frames) = play_at(144, 240, 0);
    // Every moment both draw shows the same letters.
    let mut shared = 0;
    for (now, shown) in &slow_frames {
        if let Some((_, other)) = fast_frames.iter().find(|(n, _)| (n - now).abs() < 1e-6) {
            assert_eq!(shown, other, "at {now}");
            shared += 1;
        }
    }
    assert!(shared > 20, "{shared}");
    // Half way it was half typed; at the end, whole.
    // Mira and the line's 46 letters.
    assert!(
        slow_frames.iter().any(|(_, n)| *n > 4 && *n < 50),
        "{slow_frames:?}"
    );
    assert_eq!(slow_frames.last().unwrap().1, 50);
    // Game code heard TypingFinished once, on the same tick.
    assert_eq!(talker_field(&mut slow, id, "finished"), 1);
    assert_eq!(talker_field(&mut fast, fast_id, "finished"), 1);
    let at = talker_field(&mut slow, id, "finishedAt");
    assert_eq!(talker_field(&mut fast, fast_id, "finishedAt"), at);
    // 58 characters at 40 a second and a pause of 0.2: about 1.6 s.
    assert!((95..=105).contains(&at), "{at}");
    // The typing sound played, a tick at a time.
    assert!(
        slow.open.game_sounds.len() > 30,
        "{}",
        slow.open.game_sounds.len()
    );
}

#[test]
fn finish_typing_from_game_code_shows_it_all_and_typing_finished_follows() {
    let (mut w, id, frames) = play_at(60, 30, 10);
    // Skipped on tick 10: whole from the frame after.
    assert_eq!(frames.last().unwrap().1, 50, "whole");
    assert_eq!(talker_field(&mut w, id, "finished"), 1);
    let at = talker_field(&mut w, id, "finishedAt");
    assert!((11..=13).contains(&at), "{at}");
}
