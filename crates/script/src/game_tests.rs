//! Game code on several cores (decision 0033): the same results on one
//! shard or many, the start-of-tick world, and the order changes land in.

use super::*;
use crate::code::Code;
use reflect::Value;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// A small world: named entities at places, standing alone.
#[derive(Debug, Clone, Default, PartialEq)]
struct TestWorld {
    entities: Vec<(EntityId, String, Mat4)>,
    said: Vec<String>,
    time: f64,
    shots: Vec<crate::Shot>,
    seed: u64,
}

impl TestWorld {
    fn with(names: &[&str]) -> Self {
        Self {
            entities: names
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    (
                        id(i as u64 + 1),
                        (*name).to_string(),
                        Mat4::from_translation(Vec3::new(i as f32, 0.0, 0.0)),
                    )
                })
                .collect(),
            ..Self::default()
        }
    }

    fn x(&self, n: u64) -> f32 {
        self.entities
            .iter()
            .find(|(e, _, _)| *e == id(n))
            .unwrap()
            .2
            .w_axis
            .x
    }
}

impl GameWorld for TestWorld {
    fn view(&mut self) -> WorldView {
        let entities: Vec<_> = self
            .entities
            .iter()
            .map(|(e, n, _)| Some((*e, n.clone())))
            .collect();
        WorldView {
            time: self.time,
            index: Arc::new(ViewIndex::new(&entities)),
            places: Arc::new(self.entities.iter().map(|(_, _, m)| *m).collect()),
            frame_time: 1.0 / 60.0,
            seed: self.seed,
            physics: Arc::default(),
        }
    }

    fn apply(&mut self, changes: &[Change]) {
        for change in changes {
            if let Change::Seed(seed) = change {
                self.seed = *seed;
            }
            if let Change::Move(e, place) = change
                && let Some(entity) = self.entities.iter_mut().find(|(id, _, _)| id == e)
            {
                entity.2 = Mat4::from_translation(*place);
            }
            if let Change::Fire(shot) = change {
                self.shots.push(*shot);
            }
        }
    }

    fn say(&mut self, words: &str) {
        self.said.push(words.to_string());
    }
}

fn compiled(main: &str) -> Arc<Compiled> {
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push(("main.lst".into(), main.to_string()));
    let code = Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    match crate::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    }
}

/// Walkers step along x; each tells the next one it moved; a Watcher says
/// where the first walker is.
const BUSY: &str = "get \"solis/world\":[say, find, time];\n\
    get \"solis/entity\":[positionOf, moveTo, nameOf];\n\
    event Stepped(entity by, float x);\n\
    component Walker {\n\
        has {\n\
            float speed = 1;\n\
            entity next = nothing;\n\
            int heard = 0;\n\
        }\n\
        method update(float dt) {\n\
            vec3 at = positionOf(me) + [speed, 0, 0];\n\
            moveTo(me, at);\n\
            send Stepped(me, at.x) to next;\n\
        }\n\
        on Stepped(entity by, float x) {\n\
            heard = heard + 1;\n\
            say(nameOf(me) + \" heard \" + nameOf(by));\n\
        }\n\
    }\n\
    component Watcher {\n\
        has {\n\
            int looked = 0;\n\
        }\n\
        method update(float dt) {\n\
            looked = looked + 1;\n\
            say(\"walker 1 at \" + positionOf(find(\"W1\")).x.text());\n\
        }\n\
    }\n\
    pass Walker, Watcher;\nend\n";

/// A game of `count` walkers and a watcher on `shards` shards, run for
/// `ticks`; its world, what was said, and the failures.
fn run_busy(shards: usize, count: u64, ticks: usize) -> (TestWorld, Vec<Failure>, Vec<i64>) {
    let mut names: Vec<String> = (1..=count).map(|n| format!("W{n}")).collect();
    names.push("Eye".into());
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut world = TestWorld::with(&names);
    let mut game = Game::new(shards).unwrap();
    assert_eq!(game.shards(), shards);
    let code = compiled(BUSY);
    game.load(&code, &mut world).unwrap();
    let mut walkers = Vec::new();
    for n in 1..=count {
        let mut data = ComponentData::default();
        data.set("speed", Value::Float(n as f64 * 0.5));
        let next = if n < count { Some(id(n + 1)) } else { None };
        data.set("next", Value::Entity(next));
        walkers.push((id(n), data));
    }
    let mut components: Vec<(EntityId, &str, &ComponentData)> =
        walkers.iter().map(|(e, d)| (*e, "Walker", d)).collect();
    let empty = ComponentData::default();
    components.push((id(count + 1), "Watcher", &empty));
    assert!(game.sync_many(components, &mut world).is_empty());
    let mut failures = Vec::new();
    for t in 0..ticks {
        world.time = t as f64 / 60.0;
        failures.extend(game.tick(1.0 / 60.0, &mut world).failures);
    }
    let heard = (1..=count)
        .map(
            |n| match game.values(id(n), "Walker").unwrap().get("heard") {
                Some(Value::Int(h)) => *h,
                _ => -1,
            },
        )
        .collect();
    (world, failures, heard)
}

#[test]
fn the_same_game_on_one_core_or_four_comes_out_the_same() {
    let one = run_busy(1, 40, 20);
    let four = run_busy(4, 40, 20);
    assert!(one.1.is_empty(), "{:?}", one.1);
    assert_eq!(one.0, four.0, "the same world and Console");
    assert_eq!(one.2, four.2, "the same events heard");
    // And it did something: walkers walked, every one after the first
    // heard each tick, the watcher looked.
    assert!(one.0.x(1) > 5.0);
    assert_eq!(one.2[0], 0);
    assert!(one.2[1..].iter().all(|&h| h == 20));
    assert!(one.0.said.iter().any(|l| l.starts_with("walker 1 at ")));
}

#[test]
fn game_code_sees_the_start_of_the_tick_but_its_own_changes_at_once() {
    // A Mover moves itself, then reads its own place (the new one); a
    // Reader, after it in the order, reads the Mover's place (the one at
    // the start of the tick).
    const SEEING: &str = "get \"solis/world\":[say, find];\n\
        get \"solis/entity\":[positionOf, moveTo];\n\
        component Mover {\n\
            has {\n\
                int n = 0;\n\
            }\n\
            method update(float dt) {\n\
                moveTo(me, positionOf(me) + [10, 0, 0]);\n\
                say(\"mover sees itself at \" + positionOf(me).x.text());\n\
            }\n\
        }\n\
        component Reader {\n\
            has {\n\
                int n = 0;\n\
            }\n\
            method update(float dt) {\n\
                say(\"reader sees the mover at \" + positionOf(find(\"M\")).x.text());\n\
            }\n\
        }\n\
        pass Mover, Reader;\nend\n";
    for shards in [1, 3] {
        let mut world = TestWorld::with(&["M", "R"]);
        let mut game = Game::new(shards).unwrap();
        game.load(&compiled(SEEING), &mut world).unwrap();
        let empty = ComponentData::default();
        game.sync_many(
            [(id(1), "Mover", &empty), (id(2), "Reader", &empty)],
            &mut world,
        );
        game.tick(1.0 / 60.0, &mut world);
        assert_eq!(
            world.said,
            ["mover sees itself at 10.0", "reader sees the mover at 0.0"],
            "on {shards} shards"
        );
        assert_eq!(world.x(1), 10.0, "the move landed");
        world.said.clear();
        game.tick(1.0 / 60.0, &mut world);
        assert_eq!(world.said[1], "reader sees the mover at 10.0");
    }
}

#[test]
fn two_components_moving_one_entity_the_last_in_the_order_wins() {
    // Both on entity 1: Alpha then Beta by name; Beta's move lands last.
    const TWO: &str = "get \"solis/entity\":[moveTo];\n\
        component Alpha {\n\
            has {\n\
                int n = 0;\n\
            }\n\
            method update(float dt) {\n\
                moveTo(me, [1, 0, 0]);\n\
            }\n\
        }\n\
        component Beta {\n\
            has {\n\
                int n = 0;\n\
            }\n\
            method update(float dt) {\n\
                moveTo(me, [2, 0, 0]);\n\
            }\n\
        }\n\
        pass Alpha, Beta;\nend\n";
    let mut world = TestWorld::with(&["Box"]);
    let mut game = Game::new(2).unwrap();
    game.load(&compiled(TWO), &mut world).unwrap();
    let empty = ComponentData::default();
    game.sync_many(
        [(id(1), "Beta", &empty), (id(1), "Alpha", &empty)],
        &mut world,
    );
    game.tick(1.0 / 60.0, &mut world);
    assert_eq!(world.x(1), 2.0);
}

#[test]
fn swapping_values_and_stopping_reach_every_shard() {
    let mut world = TestWorld::with(&["W1", "W2", "W3", "Eye"]);
    let mut game = Game::new(3).unwrap();
    game.load(&compiled(BUSY), &mut world).unwrap();
    let empty = ComponentData::default();
    let all = [
        (id(1), "Walker", &empty),
        (id(2), "Walker", &empty),
        (id(3), "Walker", &empty),
        (id(4), "Watcher", &empty),
    ];
    game.sync_many(all, &mut world);
    assert_eq!(game.len(), 4);
    assert!(game.has_component("Walker"));
    game.tick(1.0 / 60.0, &mut world);
    // New code: every shard's components keep their values.
    let faster =
        compiled(&BUSY.replace("float speed = 1;", "float speed = 1;\n bool fast = true;"));
    let swapped = game.swap(&faster, &mut world).unwrap();
    assert_eq!(swapped.changed, ["Walker"]);
    assert_eq!(
        game.values(id(4), "Watcher").unwrap().get("looked"),
        Some(&Value::Int(1))
    );
    game.retain_on([(id(2), Vec::new())]);
    assert_eq!(game.len(), 3);
    game.retain_only([(id(4), "Watcher".to_string())].into());
    assert_eq!(game.len(), 1);
    game.leave_out(["Watcher".to_string()].into());
    assert!(!game.has_component("Watcher"));
}

#[test]
fn shots_fired_land_in_order_on_any_number_of_shards() {
    // Each gun fires towards +Y from where it is, a little faster the
    // later it is in the list.
    const GUNS: &str = "get \"solis/world\":[fire];\n\
        get \"solis/entity\":[positionOf];\n\
        component Gun {\n\
            has {\n\
                float speed = 10;\n\
            }\n\
            method update(float dt) {\n\
                fire(positionOf(me), [0, 1, 0], speed, 2, [4, 0.2, 0.1]);\n\
            }\n\
        }\n\
        pass Gun;\nend\n";
    let run = |shards| {
        let names: Vec<String> = (1..=12).map(|n| format!("G{n}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut world = TestWorld::with(&names);
        let mut game = Game::new(shards).unwrap();
        game.load(&compiled(GUNS), &mut world).unwrap();
        let data: Vec<ComponentData> = (1..=12)
            .map(|n| {
                let mut d = ComponentData::default();
                d.set("speed", Value::Float(f64::from(n)));
                d
            })
            .collect();
        game.sync_many(
            (1..=12u64).map(|n| (id(n), "Gun", &data[n as usize - 1])),
            &mut world,
        );
        for _ in 0..3 {
            assert!(game.tick(1.0 / 60.0, &mut world).failures.is_empty());
        }
        world.shots
    };
    let one = run(1);
    assert_eq!(one.len(), 36, "every gun, every tick");
    // In entity order within a tick.
    assert_eq!(one[0].from, Vec3::ZERO);
    assert_eq!(one[1].from, Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(one[0].speed, 1.0);
    assert_eq!(one[0].colour, [4.0, 0.2, 0.1]);
    assert_eq!(one, run(4));
}

#[test]
fn spawns_make_the_same_entities_on_any_number_of_shards() {
    const HIVES: &str = "get \"solis/world\":[spawn, despawn];\n\
        component Hive {\n\
            has {\n\
                entity last = nothing;\n\
            }\n\
            method update(float dt) {\n\
                last = spawn(\"drone\", [1, 2, 3]);\n\
                despawn(last);\n\
            }\n\
        }\n\
        pass Hive;\nend\n";
    let run = |shards| {
        let names: Vec<String> = (1..=9).map(|n| format!("H{n}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut world = SpawnWorld(TestWorld::with(&names), Vec::new());
        let mut game = Game::new(shards).unwrap();
        game.load(&compiled(HIVES), &mut world).unwrap();
        let empty = ComponentData::default();
        game.sync_many((1..=9u64).map(|n| (id(n), "Hive", &empty)), &mut world);
        for t in 0..2 {
            world.0.time = f64::from(t) / 60.0;
            assert!(game.tick(1.0 / 60.0, &mut world).failures.is_empty());
        }
        world.1
    };
    let one = run(1);
    assert_eq!(
        one.len(),
        9 * 2 * 2,
        "a spawn and a despawn per hive per tick"
    );
    match (&one[0], &one[1]) {
        (Change::Spawn(made, file, place), Change::Despawn(gone)) => {
            assert_eq!(made, gone, "the entity spawn gave back");
            assert_eq!(file, "drone");
            assert_eq!(*place, Vec3::new(1.0, 2.0, 3.0));
        }
        other => panic!("{other:?}"),
    }
    let ids: std::collections::HashSet<_> = one
        .iter()
        .filter_map(|c| match c {
            Change::Spawn(id, _, _) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(ids.len(), 18, "every spawn its own entity");
    assert_eq!(one, run(3));
}

/// A test world keeping the spawns and despawns game code asks for.
struct SpawnWorld(TestWorld, Vec<Change>);

impl GameWorld for SpawnWorld {
    fn view(&mut self) -> WorldView {
        self.0.view()
    }
    fn apply(&mut self, changes: &[Change]) {
        self.1.extend(
            changes
                .iter()
                .filter(|c| matches!(c, Change::Spawn(..) | Change::Despawn(_)))
                .cloned(),
        );
    }
    fn say(&mut self, words: &str) {
        self.0.say(words);
    }
}

#[test]
fn a_runaway_loop_can_t_keep_unbounded_changes() {
    let mut world = TestWorld::with(&["A"]);
    let mut host = ViewHost::new(world.view(), 0);
    for _ in 0..MAX_KEPT + 5 {
        host.move_to(id(1), Vec3::ONE);
    }
    host.fire(crate::Shot {
        from: Vec3::ZERO,
        direction: Vec3::Y,
        speed: 1.0,
        life: 1.0,
        colour: [1.0; 3],
    });
    assert_eq!(host.spawn("ship", Vec3::ZERO), None);
    host.say("hello");
    assert_eq!(host.kept.changes.len(), MAX_KEPT);
    assert_eq!(host.kept.dropped, 8);
    // Long names fill their own budget first.
    let mut host = ViewHost::new(world.view(), 0);
    let name = "x".repeat(1024);
    let made = (0..10_000)
        .filter(|_| host.spawn(&name, Vec3::ZERO).is_some())
        .count();
    assert_eq!(made, MAX_SPAWN_BYTES / 1024);
}

#[test]
fn spawns_in_different_rounds_of_events_make_different_entities() {
    // Each round's only event (number 0 in its round) reaches the same
    // component, which spawns and sends the next.
    const ECHO: &str = "get \"solis/world\":[spawn];\n\
        event Ping(int left);\n\
        component Echo {\n\
            has {\n\
                int started = 0;\n\
            }\n\
            method update(float dt) {\n\
                is started == 0? then {\n\
                    started = 1;\n\
                    send Ping(3) to me;\n\
                }\n\
            }\n\
            on Ping(int left) {\n\
                entity made = spawn(\"drone\", [0, 0, 0]);\n\
                is left > 0? then {\n\
                    send Ping(left - 1) to me;\n\
                }\n\
            }\n\
        }\n\
        pass Echo;\nend\n";
    let mut world = SpawnWorld(TestWorld::with(&["E"]), Vec::new());
    let mut game = Game::new(1).unwrap();
    game.load(&compiled(ECHO), &mut world).unwrap();
    let empty = ComponentData::default();
    game.sync_many([(id(1), "Echo", &empty)], &mut world);
    assert!(game.tick(1.0 / 60.0, &mut world).failures.is_empty());
    let ids: std::collections::HashSet<_> = world
        .1
        .iter()
        .filter_map(|c| match c {
            Change::Spawn(id, _, _) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(ids.len(), 4, "four rounds, four entities");
}

#[test]
fn failing_starts_are_told_in_the_order_asked_on_any_number_of_shards() {
    // C10 of the Phase 5 review: shards finish in their own order, but
    // the Console hears about failures in the order the entities came.
    let broken = RustKind {
        schema: reflect::ComponentSchema {
            name: "Bad".into(),
            help: "Never starts.".into(),
            fields: Vec::new(),
        },
        make: |_| panic!("make broke"),
    };
    let order: Vec<u64> = vec![9, 2, 7, 4, 11, 1, 6, 3, 10, 5, 8, 12];
    for shards in [1, 3, 4] {
        let mut world = TestWorld::with(&["e"; 12]);
        let mut game = Game::new(shards).unwrap();
        game.add_rust(broken.clone());
        let empty = ComponentData::default();
        let failures = game.sync_many(order.iter().map(|&n| (id(n), "Bad", &empty)), &mut world);
        let told: Vec<EntityId> = failures.iter().filter_map(|f| f.entity).collect();
        let wanted: Vec<EntityId> = order.iter().map(|&n| id(n)).collect();
        assert_eq!(told, wanted, "on {shards} shards");
    }
}

#[test]
fn spawned_entities_share_out_across_shards() {
    // Spawned IDs are all odd; split by the raw ID, two shards would
    // leave one idle (E8 of the Phase 5 review).
    let game = Game::new(2).unwrap();
    if game.shards() < 2 {
        return;
    }
    let mut counts = [0usize; 2];
    for n in 0..1000u64 {
        counts[game.shard_of(id(2 * n + 1))] += 1;
    }
    assert!(counts.iter().all(|&c| c > 400), "{counts:?}");
}

/// Lust and Lua dice: each rolls and says what came up, every tick; the
/// Lust one sets the seed back to 7 on its tenth tick.
const DICE: &str = "get \"solis/world\":[random, randomInt, randomRange, setSeed, say];\n\
    component Die {\n\
        has {\n\
            int rolls = 0;\n\
        }\n\
        method update(float dt) {\n\
            rolls = rolls + 1;\n\
            say(\"lust \" + randomInt(1, 6).text() + \" \" + random().text() + \" \" + randomRange(-2, 2).text());\n\
            is rolls == 10? then {\n\
                setSeed(7);\n\
            }\n\
        }\n\
    }\n\
    pass Die;\nend\n";

const LUA_DIE: &str = "local LuaDie = component(\"LuaDie\", {})\n\
    function LuaDie:update(dt)\n\
        solis.say(\"lua \" .. math.random(6) .. \" \" .. math.random(10, 12) .. \" \" .. math.random())\n\
    end\n";

/// What `count` Lust dice and a Lua one say over `ticks` on `shards`
/// shards, starting from `seed`.
fn roll(shards: usize, seed: u64, ticks: usize) -> Vec<String> {
    roll_and_sum(shards, seed, ticks).0
}

/// As [`roll`], with the game code's checksum after each tick.
fn roll_and_sum(shards: usize, seed: u64, ticks: usize) -> (Vec<String>, Vec<u64>) {
    let names: Vec<String> = (1..=9).map(|n| format!("D{n}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut world = TestWorld::with(&names);
    world.seed = seed;
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push(("main.lst".into(), DICE.to_string()));
    let code = Code {
        files,
        lua: vec![("die.lua".into(), LUA_DIE.to_string())],
        notes: Vec::new(),
    };
    let code = Arc::new(crate::compile(&code).unwrap().unwrap());
    let mut game = Game::new(shards).unwrap();
    game.load(&code, &mut world).unwrap();
    let empty = ComponentData::default();
    let mut components: Vec<(EntityId, &str, &ComponentData)> =
        (1..=8).map(|n| (id(n), "Die", &empty)).collect();
    components.push((id(9), "LuaDie", &empty));
    assert!(game.sync_many(components, &mut world).is_empty());
    let mut sums = Vec::new();
    for t in 0..ticks {
        world.time = t as f64 / 60.0;
        let ticked = game.tick(1.0 / 60.0, &mut world);
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
        sums.push(game.checksum());
    }
    (world.said, sums)
}

#[test]
fn random_numbers_follow_the_seed_on_any_number_of_shards() {
    let one = roll(1, 42, 12);
    assert_eq!(one.len(), 9 * 12);
    assert_eq!(roll(3, 42, 12), one, "the same on 3 shards");
    assert_eq!(roll(4, 42, 12), one, "and on 4");
    assert_ne!(roll(1, 43, 12), one, "another seed, other numbers");
    // In range, and not all the same.
    let dice: Vec<i64> = one
        .iter()
        .filter(|l| l.starts_with("lust "))
        .map(|l| l.split(' ').nth(1).unwrap().parse().unwrap())
        .collect();
    assert!(dice.iter().all(|d| (1..=6).contains(d)), "{dice:?}");
    assert!(dice.iter().any(|&d| d != dice[0]));
    for line in one.iter().filter(|l| l.starts_with("lua ")) {
        let parts: Vec<f64> = line
            .split(' ')
            .skip(1)
            .map(|p| p.parse().unwrap())
            .collect();
        assert!(
            (1.0..=6.0).contains(&parts[0]) && parts[0].fract() == 0.0,
            "{line}"
        );
        assert!((10.0..=12.0).contains(&parts[1]), "{line}");
        assert!((0.0..1.0).contains(&parts[2]), "{line}");
    }
}

#[test]
fn set_seed_lands_at_the_end_of_the_tick_and_repeats_the_numbers() {
    // The dice set the seed to 7 on their tenth tick: from the eleventh,
    // the numbers are those a game started at 7 rolls at the same time.
    let reset = roll(2, 42, 12);
    let from_seven = roll(2, 7, 12);
    assert_ne!(reset[..9 * 10], from_seven[..9 * 10]);
    assert_eq!(reset[9 * 10..], from_seven[9 * 10..]);
}

#[test]
fn game_code_s_checksum_is_the_same_on_any_number_of_shards() {
    let (_, one) = roll_and_sum(1, 5, 12);
    assert_eq!(roll_and_sum(3, 5, 12).1, one);
    assert_eq!(roll_and_sum(4, 5, 12).1, one);
    // Each tick changes the dice's counts, so each tick's sum differs.
    for pair in one.windows(2) {
        assert_ne!(pair[0], pair[1]);
    }
}
