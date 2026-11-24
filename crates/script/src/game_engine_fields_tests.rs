//! Fields the engine reads (Fixes 3, decision 0136): a component's
//! `Facing facing` turns its sprite and its `Look look` dresses it, from
//! its first tick and then whenever its code changes them; the same on
//! one core or four.

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
    let names = crate::project_names::Names::default().and_looks(&["KnightMain", "KnightMana"]);
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

/// The fields that set what the calls did (U5, decision 0137): a door
/// that opens on its third tick (its clip, its collider, its screen) and
/// hides on its fifth, flipped from the start.
#[test]
fn a_component_s_own_settings_are_fields_solis_reads() {
    let main = "fetch \"solis/project\":[Clip, Screen];\n\
        component Door {\n\
            has {\n\
                Clip animation = Clip.DoorShut;\n\
                bool flipped = true;\n\
                bool visible = true;\n\
                bool solid = true;\n\
                Screen screen = Screen.None;\n\
                int ticks = 0;\n\
            }\n\
            method update(float dt) {\n\
                ticks = ticks + 1;\n\
                is ticks == 3? then {\n\
                    animation = Clip.DoorOpen;\n\
                    solid = false;\n\
                    screen = Screen.Hint;\n\
                }\n\
                is ticks == 5? then {\n\
                    visible = false;\n\
                    screen = Screen.None;\n\
                }\n\
            }\n\
        }\n\
        end\n";
    let names = crate::project_names::Names::default()
        .and_clips(&["door/shut", "door/open"], &[])
        .and_screens(&["Hint"]);
    let (mut files, _) = crate::describe::descriptions_for("Test 1.0", "solis", &names);
    files.push(("main.lst".into(), main.to_string()));
    let code = Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let code = match crate::code::compile_within(&code, crate::luau::Limits::unhurried()) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{problems:#?}"),
    };
    let played = |shards: usize| {
        let mut world = TestWorld::with(&["A"]);
        let mut game = unhurried(shards);
        game.load(&code, &mut world).unwrap();
        let empty = ComponentData::default();
        assert!(
            game.sync_many(vec![(id(1), "Door", &empty)], &mut world)
                .is_empty()
        );
        let mut landed = Vec::new();
        for tick in 1..=6_u32 {
            world.set_clock(u64::from(tick), f64::from(tick) / 60.0);
            let ticked = game.tick(1.0 / 60.0, &mut world);
            assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
            landed.extend(world.sprite_changes.drain(..).map(|c| (tick, c)));
        }
        landed
    };
    let landed = played(1);
    assert_eq!(
        landed,
        [
            (1, Change::PlayClip(id(1), "shut".into(), false)),
            (1, Change::Visible(id(1), true)),
            (1, Change::Solid(id(1), true)),
            (1, Change::Flip(id(1), true, false)),
            (3, Change::PlayClip(id(1), "open".into(), false)),
            (3, Change::Solid(id(1), false)),
            (3, Change::ShowScreen(id(1), "Hint".into(), true)),
            (5, Change::Visible(id(1), false)),
            (5, Change::ShowScreen(id(1), "Hint".into(), false)),
        ],
        "Screen.None at the start shows nothing; flippedDown isn't held, so it's false"
    );
    assert_eq!(played(4), landed);
}

/// A pause menu and an inn, setting the engine objects' fields (U5b): on
/// its second tick the menu halves the game's speed and reads it back at
/// once, and the inn sets the hour; on the third the menu makes the text
/// bigger from what it reads; on the fourth it reads the speed that
/// landed and fills the screen.
const SETTINGS: &str = "fetch \"solis/world\":World;\n\
    fetch \"solis/screens\":Screens;\n\
    fetch \"solis/window\":Window;\n\n\
    component Menu {\n\
        has {\n\
            int ticks = 0;\n\
        }\n\
        method update(float dt) {\n\
            ticks = ticks + 1;\n\
            is ticks == 2? then {\n\
                World.gameSpeed = 0.5;\n\
                is World.gameSpeed == 0.5? then {\n\
                    Screens.captions = true;\n\
                }\n\
            }\n\
            is ticks == 3? then {\n\
                Screens.textSize = Screens.textSize + 0.5;\n\
            }\n\
            is ticks == 4? then {\n\
                is World.gameSpeed == 0.5? then {\n\
                    Window.fullscreen = true;\n\
                }\n\
            }\n\
        }\n\
    }\n\
    component Inn {\n\
        has {\n\
            int ticks = 0;\n\
        }\n\
        method update(float dt) {\n\
            ticks = ticks + 1;\n\
            is ticks == 2? then {\n\
                World.timeOfDay = 6;\n\
            }\n\
        }\n\
    }\n\
    pass Menu, Inn;\n\
    end\n";

/// The menu on A and the inn on B, ticked five times on `shards` cores:
/// the game-wide changes, each with its tick.
fn settings_played(shards: usize) -> Vec<(u32, Change)> {
    let mut world = TestWorld::with(&["A", "B"]);
    let mut game = unhurried(shards);
    game.load(&compile(SETTINGS), &mut world).unwrap();
    let empty = ComponentData::default();
    let components = vec![(id(1), "Menu", &empty), (id(2), "Inn", &empty)];
    assert!(game.sync_many(components, &mut world).is_empty());
    let mut landed = Vec::new();
    for tick in 1..=5 {
        world.set_clock(u64::from(tick), f64::from(tick) / 60.0);
        let ticked = game.tick(1.0 / 60.0, &mut world);
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
        landed.extend(world.world_changes.drain(..).map(|c| (tick, c)));
    }
    landed
}

#[test]
fn the_engine_objects_settings_are_fields_set_and_read_back() {
    use crate::settings::Setting;
    let landed = settings_played(1);
    assert_eq!(
        landed,
        vec![
            (2, Change::GameSpeed(0.5)),
            (2, Change::Setting(Setting::Captions(true))),
            (2, Change::TimeOfDay(6.0)),
            (3, Change::Setting(Setting::TextSize(1.5))),
            (4, Change::Setting(Setting::Fullscreen(true))),
        ]
    );
    assert_eq!(settings_played(4), landed, "the same on four cores");
}
