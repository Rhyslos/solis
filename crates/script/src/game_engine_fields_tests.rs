//! Fields the engine reads (Fixes 3, decision 0136): a component's
//! `Facing facing` turns its sprite and its `Look look` dresses it, from
//! its first tick and then whenever its code changes them; the same on
//! one core or four. The uniformity pass's U5 adds the clip, the flip,
//! visible, solid, soundPlaying and the screen shown.

use super::tests::{TestWorld, id, unhurried};
use super::*;
use crate::code::Code;

/// A knight who walks east on its third tick and puts on its mana armour
/// on its fifth; a guard that never changes; an orc with a facing of
/// another type, which the engine leaves alone.
const GAME: &str = "fetch \"solis/animation\":Facing;\n\
    get \"solis/animation\":nearestFacing;\n\
    fetch \"solis/project\":Look;\n\n\
    component Knight {\n\
        has {\n\
            Facing facing = Facing.South;\n\
            Look look = Look.KnightMain;\n\
            int ticks = 0;\n\
        }\n\
        method update(float dt) {\n\
            ticks = ticks + 1;\n\
            is ticks == 3? then {\n\
                facing = nearestFacing([1, 0.2]);\n\
            }\n\
            is ticks == 5? then {\n\
                look = Look.KnightMana;\n\
            }\n\
        }\n\
    }\n\n\
    component Guard {\n\
        has {\n\
            Facing facing = Facing.West;\n\
        }\n\
    }\n\n\
    component Orc {\n\
        has {\n\
            string facing = \"East\";\n\
        }\n\
    }\n\
    end\n";

fn compile(main: &str) -> Arc<Compiled> {
    let names = crate::project_names::Names::default()
        .and_looks(&["KnightMain", "KnightMana"])
        .and_clips(&["sprites/lamp/glow", "sprites/lamp/flicker"], &[])
        .and_screens(&["Hud", "PauseMenu"]);
    let (mut files, _) = crate::describe::descriptions_for("Test 1.0", "solis", &names);
    files.push(("main.lst".into(), main.to_string()));
    let code = Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    match crate::code::compile_within(&code, crate::luau::Limits::unhurried()) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{problems:#?}"),
    }
}

/// The knight on A, the guard on B and the orc on C, ticked `ticks`
/// times: what landed on the sprites, each with its tick.
fn played(shards: usize, ticks: u32) -> Vec<(u32, Change)> {
    let mut world = TestWorld::with(&["A", "B", "C"]);
    let mut game = unhurried(shards);
    game.load(&compile(GAME), &mut world).unwrap();
    let empty = ComponentData::default();
    let components = vec![
        (id(1), "Knight", &empty),
        (id(2), "Guard", &empty),
        (id(3), "Orc", &empty),
    ];
    assert!(game.sync_many(components, &mut world).is_empty());
    let mut landed = Vec::new();
    for tick in 1..=ticks {
        world.set_clock(u64::from(tick), f64::from(tick) / 60.0);
        let ticked = game.tick(1.0 / 60.0, &mut world);
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
        landed.extend(world.sprite_changes.drain(..).map(|c| (tick, c)));
    }
    landed
}

#[test]
fn the_engine_reads_which_components_hold_a_facing_or_a_look() {
    let code = compile(GAME);
    let read: Vec<(&str, crate::engine_fields::Read)> = code
        .engine_reads
        .iter()
        .map(|w| (w.component.as_str(), w.read))
        .collect();
    use crate::engine_fields::Read;
    assert_eq!(
        read,
        [
            ("Knight", Read::Facing),
            ("Knight", Read::Look),
            ("Guard", Read::Facing)
        ]
    );
}

#[test]
fn sprites_follow_their_components_from_the_first_tick_and_on_each_change() {
    let landed = played(1, 6);
    assert_eq!(
        landed,
        [
            (1, Change::Facing(id(1), "South".into())),
            (1, Change::Look(id(1), "KnightMain".into())),
            (1, Change::Facing(id(2), "West".into())),
            (3, Change::Facing(id(1), "East".into())),
            (5, Change::Look(id(1), "KnightMana".into())),
        ],
        "nothing lands while nothing changes, and the orc's string is its own"
    );
}

#[test]
fn the_same_on_four_cores() {
    assert_eq!(played(4, 6), played(1, 6));
}

#[test]
fn a_component_taken_away_and_given_back_lands_again() {
    let mut world = TestWorld::with(&["A"]);
    let mut game = unhurried(1);
    game.load(&compile(GAME), &mut world).unwrap();
    let empty = ComponentData::default();
    assert!(
        game.sync_many(vec![(id(1), "Guard", &empty)], &mut world)
            .is_empty()
    );
    game.tick(1.0 / 60.0, &mut world);
    game.retain_only(std::collections::BTreeSet::new());
    game.tick(1.0 / 60.0, &mut world);
    assert!(
        game.sync_many(vec![(id(1), "Guard", &empty)], &mut world)
            .is_empty()
    );
    game.tick(1.0 / 60.0, &mut world);
    assert_eq!(
        world.sprite_changes,
        [
            Change::Facing(id(1), "West".into()),
            Change::Facing(id(1), "West".into())
        ]
    );
}

#[test]
fn nearest_facing_rounds_a_way_to_one_of_eight() {
    let main = "fetch \"solis/animation\":Facing;\n\
    get \"solis/animation\":nearestFacing;\n\
        fetch \"solis/world\":World;\n\
        component Compass {\n\
            method update(float dt) {\n\
                World:say(nearestFacing([0, 1]).text() + \" \" + nearestFacing([-1, -1]).text() \
                    + \" \" + nearestFacing([0, 0]).text() + \" \" + nearestFacing([3, 0.4]).text());\n\
            }\n\
        }\n\
        end\n";
    let mut world = TestWorld::with(&["A"]);
    let mut game = unhurried(1);
    game.load(&compile(main), &mut world).unwrap();
    let empty = ComponentData::default();
    assert!(
        game.sync_many(vec![(id(1), "Compass", &empty)], &mut world)
            .is_empty()
    );
    let ticked = game.tick(1.0 / 60.0, &mut world);
    assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
    assert_eq!(world.said, ["North SouthWest South East"]);
}

/// A lamp whose fields change on its second to fifth ticks: every field
/// U5 added, set with `=`.
const LAMP: &str = "fetch \"solis/project\":[Clip, Screen];\n\
    fetch \"solis/animation\":Flip;\n\n\
    component Lamp {\n\
        has {\n\
            Clip animation = Clip.LampGlow;\n\
            Flip flip = Flip.None;\n\
            bool visible = true;\n\
            bool solid = true;\n\
            bool soundPlaying = false;\n\
            Screen screen = Screen.None;\n\
            int ticks = 0;\n\
        }\n\
        method update(float dt) {\n\
            ticks += 1;\n\
            is ticks == 2? then {\n\
                animation = Clip.LampFlicker;\n\
                flip = Flip.Both;\n\
                screen = Screen.Hud;\n\
            }\n\
            is ticks == 3? then {\n\
                visible = false;\n\
                solid = false;\n\
                soundPlaying = true;\n\
                screen = Screen.PauseMenu;\n\
            }\n\
            is ticks == 4? then {\n\
                animation = Clip.None;\n\
                screen = Screen.None;\n\
                restartAnimation();\n\
            }\n\
        }\n\
    }\n\
    pass Lamp;\n\
    end\n";

/// The lamp on A, ticked five times: what landed, each with its tick.
fn lamp(shards: usize) -> (Vec<(u32, Change)>, Vec<Change>) {
    let mut world = TestWorld::with(&["A"]);
    let mut game = unhurried(shards);
    game.load(&compile(LAMP), &mut world).unwrap();
    let empty = ComponentData::default();
    assert!(
        game.sync_many(vec![(id(1), "Lamp", &empty)], &mut world)
            .is_empty()
    );
    let mut landed = Vec::new();
    for tick in 1..=5 {
        world.set_clock(u64::from(tick), f64::from(tick) / 60.0);
        let ticked = game.tick(1.0 / 60.0, &mut world);
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
        landed.extend(world.sprite_changes.drain(..).map(|c| (tick, c)));
    }
    (landed, world.sounds.clone())
}

#[test]
fn every_field_a_component_sets_on_its_own_entity_lands_when_it_changes() {
    let (landed, sounds) = lamp(1);
    let e = id(1);
    assert_eq!(
        landed,
        [
            (1, Change::PlayClip(e, "glow".into(), false)),
            (1, Change::Flip(e, false, false)),
            (1, Change::Visible(e, true)),
            (1, Change::Solid(e, true)),
            (2, Change::PlayClip(e, "flicker".into(), false)),
            (2, Change::Flip(e, true, true)),
            (2, Change::ShowScreen(e, "Hud".into(), true)),
            (3, Change::Visible(e, false)),
            (3, Change::Solid(e, false)),
            (3, Change::ShowScreen(e, "Hud".into(), false)),
            (3, Change::ShowScreen(e, "PauseMenu".into(), true)),
            (4, Change::PlayClip(e, String::new(), true)),
            (4, Change::StopClip(e)),
            (4, Change::ShowScreen(e, "PauseMenu".into(), false)),
        ],
        "a clip is found by its option, the screen shown before is hidden, Screen.None at the \
         start shows nothing, and the restart lands with the tick's calls, before the fields"
    );
    assert_eq!(
        sounds,
        [
            Change::SoundPlaying(e, false),
            Change::SoundPlaying(e, true)
        ]
    );
}

#[test]
fn every_field_lands_the_same_on_four_cores() {
    assert_eq!(lamp(4), lamp(1));
}

#[test]
fn a_component_gone_hides_the_screen_it_showed() {
    let main = "fetch \"solis/project\":Screen;\n\
        component Hud {\n    has {\n        Screen screen = Screen.Hud;\n    }\n}\n\
        pass Hud;\nend\n";
    let mut world = TestWorld::with(&["A"]);
    let mut game = unhurried(1);
    game.load(&compile(main), &mut world).unwrap();
    let empty = ComponentData::default();
    assert!(
        game.sync_many(vec![(id(1), "Hud", &empty)], &mut world)
            .is_empty()
    );
    game.tick(1.0 / 60.0, &mut world);
    game.retain_only(std::collections::BTreeSet::new());
    game.tick(1.0 / 60.0, &mut world);
    assert_eq!(
        world.sprite_changes,
        [
            Change::ShowScreen(id(1), "Hud".into(), true),
            Change::ShowScreen(id(1), "Hud".into(), false)
        ]
    );
}
