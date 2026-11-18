//! Tags in game code (stage 1, tags): `hasTag`, `addTag` and `removeTag`
//! on entities and `World:within` and `World:nearest`, in Lust, the same
//! on one core or four; tag changes landing at the end of the tick in the
//! tick's order; and a misspelt tag failing the check before anything
//! runs.

use super::tests::{TestWorld, id, unhurried};
use super::*;
use crate::code::Code;
use reflect::TagId;

/// The project's names: two prefabs, and the tags Enemy, Flying and
/// Ghost.
fn names() -> crate::project_names::Names {
    crate::project_names::Names::with_prefabs(&["drone"]).and_tags(&["Enemy", "Flying", "Ghost"])
}

fn compile(main: &str) -> Result<Arc<Compiled>, Vec<crate::CodeProblem>> {
    let (mut files, _) = crate::describe::descriptions_for("Test 1.0", "solis", &names());
    files.push(("main.lst".into(), main.to_string()));
    let code = Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    crate::code::compile_within(&code, crate::luau::Limits::unhurried())
        .map(|c| Arc::new(c.expect("there's code")))
}

/// A hunter asks the world about enemies, tags itself, and takes a tag
/// off the nearest; a bat says whether it's tagged.
const HUNT: &str = "fetch \"solis/world\":World;\nfetch \"solis/project\":Tag;\n\n\
    component Hunter {\n\
        has {\n\
            int ticks = 0;\n\
        }\n\
        method update(float dt) {\n\
            ticks = ticks + 1;\n\
            entity array near = World:within(Tag.Enemy, position(), 2.5);\n\
            string names = \"\";\n\
            loop (entity e in near) {\n\
                names = names + e:name() + \" \";\n\
            }\n\
            World:say(\"tick \" + ticks.text() + \": within \" + names);\n\
            entity closest = World:nearest(Tag.Enemy, [3, 0, 0]);\n\
            is closest != nothing? then {\n\
                World:say(\"nearest \" + closest:name());\n\
            }\n\
            is World:nearest(Tag.Ghost, position()) == nothing? then {\n\
                World:say(\"no ghosts\");\n\
            }\n\
            is hasTag(Tag.Enemy)? then {\n\
                World:say(\"the hunter is an enemy\");\n\
            }\n\
            addTag(Tag.Enemy);\n\
            is ticks == 1 && closest != nothing? then {\n\
                closest:removeTag(Tag.Enemy);\n\
            }\n\
        }\n\
    }\n\
    component Bat {\n\
        method update(float dt) {\n\
            is hasTag(Tag.Flying)? then {\n\
                World:say(name() + \" flies\");\n\
            }\n\
        }\n\
    }\n\
    pass Hunter, Bat;\nend\n";

/// The tags of a world of a hunter at 0 and bats at 1, 2 and 3 m: the
/// first two bats are enemies, the second flies.
fn tags_of(
    world: &TestWorld,
    carried: &[(u64, &[TagId])],
    tags: &[(TagId, String)],
) -> crate::TagView {
    let _ = world;
    let tagged: Vec<(EntityId, Vec3, Vec<TagId>)> = carried
        .iter()
        .map(|(n, t)| (id(*n), Vec3::new(*n as f32 - 1.0, 0.0, 0.0), t.to_vec()))
        .collect();
    crate::TagView::new(tags, &tagged, false)
}

/// What the hunt says over two ticks, and the tags landed, on `shards`.
fn hunt(shards: usize) -> (Vec<String>, Vec<Change>) {
    let tag = |n| TagId(reflect::Id::from_raw(n).unwrap());
    let (enemy, flying, ghost) = (tag(101), tag(102), tag(103));
    let tags = vec![
        (enemy, "Enemy".to_string()),
        (flying, "Flying".to_string()),
        (ghost, "Ghost".to_string()),
    ];
    let mut world = TestWorld::with(&["Hunter", "Bat1", "Bat2", "Bat3"]);
    world.tags = Arc::new(tags_of(
        &world,
        &[(2, &[enemy]), (3, &[enemy, flying]), (4, &[])],
        &tags,
    ));
    let mut game = unhurried(shards);
    let code = compile(HUNT).unwrap();
    game.load(&code, &mut world).unwrap();
    let empty = ComponentData::default();
    let components = vec![
        (id(1), "Hunter", &empty),
        (id(2), "Bat", &empty),
        (id(3), "Bat", &empty),
        (id(4), "Bat", &empty),
    ];
    assert!(game.sync_many(components, &mut world).is_empty());
    let first = game.tick(1.0 / 60.0, &mut world);
    assert!(first.failures.is_empty(), "{:?}", first.failures);
    // The next tick's view: the changes landed (the hunter's tag, Bat2's
    // taken away), as the editor makes it at the tick's start.
    world.tags = Arc::new(tags_of(
        &world,
        &[(1, &[enemy]), (2, &[enemy]), (3, &[flying])],
        &tags,
    ));
    let second = game.tick(1.0 / 60.0, &mut world);
    assert!(second.failures.is_empty(), "{:?}", second.failures);
    (world.said, world.tag_changes)
}

#[test]
fn tag_questions_reach_lust_the_same_on_any_number_of_shards() {
    let (said, changes) = hunt(1);
    assert_eq!(
        said,
        [
            "tick 1: within Bat1 Bat2 ",
            "nearest Bat2",
            "no ghosts",
            "Bat2 flies",
            "tick 2: within Hunter Bat1 ",
            "nearest Bat1",
            "no ghosts",
            "the hunter is an enemy",
            "Bat2 flies",
        ]
    );
    assert_eq!((said.clone(), changes.clone()), hunt(4));
    // Each tick's changes land at its end, in the tick's order: the
    // hunter's own tag, then the one it took off the nearest.
    let landed: Vec<(EntityId, bool)> = changes
        .iter()
        .map(|c| match c {
            Change::Tag(who, _, on) => (*who, *on),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(landed, [(id(1), true), (id(3), false), (id(1), true)]);
}

#[test]
fn a_misspelt_tag_fails_the_check_before_anything_runs() {
    let misspelt = HUNT.replace("Tag.Ghost", "Tag.Gohst");
    let problems = compile(&misspelt)
        .err()
        .expect("a misspelt tag is a mistake");
    let first = problems[0].to_string();
    assert!(first.contains("Gohst"), "{first}");
    // A tag the project hasn't any more (renamed in Project settings) is
    // the same: the old name stops checking.
    let (mut files, _) = crate::describe::descriptions_for(
        "Test 1.0",
        "solis",
        &crate::project_names::Names::with_prefabs(&[]).and_tags(&["Enemy", "Flying", "Spirit"]),
    );
    files.push(("main.lst".into(), HUNT.to_string()));
    let code = Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    assert!(crate::code::compile_within(&code, crate::luau::Limits::unhurried()).is_err());
}

#[test]
fn a_tag_the_project_hasn_t_or_an_entity_not_in_the_game_changes_nothing() {
    let enemy = TagId::random();
    let tags = vec![(enemy, "Enemy".to_string())];
    let mut world = TestWorld::with(&["Hunter"]);
    world.tags = Arc::new(tags_of(&world, &[(1, &[enemy])], &tags));
    let mut host = ViewHost::new(world.view(), 0);
    host.add_tag(id(1), "Ghost");
    host.remove_tag(id(9), "Enemy");
    assert!(host.kept.changes.is_empty());
    host.add_tag(id(1), "Enemy");
    assert_eq!(host.kept.changes.len(), 1);
    assert!(host.has_tag(id(1), "Enemy"));
    assert!(!host.has_tag(id(1), "Ghost"));
}
