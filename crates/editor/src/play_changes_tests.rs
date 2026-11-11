use super::*;
use crate::test_support::{harness, info};
use egui_kittest::kittest::Queryable;

fn fresh() -> Workbench {
    Workbench::new(info(), None)
}

#[test]
fn the_editor_takes_a_colour_while_the_game_runs() {
    let mut w = fresh();
    assert_eq!(w.play_tint(), None);
    assert_eq!(w.tinted(theme::header()), theme::header());
    w.play();
    assert_eq!(w.play_tint(), Some(PLAYING));
    assert_ne!(w.tinted(theme::header()), theme::header());
    w.pause();
    assert_eq!(w.play_tint(), Some(theme::WARNING));
    w.stop();
    assert_eq!(w.play_tint(), None);
}

#[test]
fn stopping_with_nothing_changed_asks_nothing() {
    let mut w = fresh();
    w.play();
    w.stop();
    assert!(w.play_changes().is_none());
}

#[test]
fn changes_made_while_playing_wait_for_keep() {
    let mut w = fresh();
    let before = w.document().area().iter().count();
    w.play();
    let lamp = w.add_point_light();
    w.stop();
    // Back as it was, and asked.
    assert!(!w.document().area().contains(lamp));
    assert_eq!(w.play_changes().unwrap(), ["Add Point light 1"]);
    // Play waits for the answer.
    w.play();
    assert_eq!(w.play_state(), PlayState::Editing);
    w.keep_play_changes();
    assert!(w.play_changes().is_none());
    assert!(w.document().area().contains(lamp));
    assert_eq!(w.document().area().iter().count(), before + 1);
    // Still undoable.
    w.undo();
    assert!(!w.document().area().contains(lamp));
}

#[test]
fn undo_all_leaves_the_area_as_it_was_before_play() {
    let mut w = fresh();
    w.play();
    let lamp = w.add_point_light();
    w.add_spotlight();
    w.stop();
    assert_eq!(w.play_changes().unwrap().len(), 2);
    w.undo_play_changes();
    assert!(w.play_changes().is_none());
    assert!(!w.document().area().contains(lamp));
}

#[test]
fn the_question_is_answered_with_its_buttons() {
    let mut w = fresh();
    w.play();
    let lamp = w.add_point_light();
    w.stop();
    let mut h = harness(w);
    h.run();
    let _ = h.get_by_label("Keep the changes made while playing?");
    let _ = h.get_by_label("•  Add Point light 1");
    h.get_by_label("Keep").click();
    h.run();
    assert!(h.state().0.play_changes().is_none());
    assert!(h.state().0.document().area().contains(lamp));
}

#[test]
fn many_changes_are_counted_after_the_first_few() {
    let mut w = fresh();
    w.play();
    for _ in 0..11 {
        w.add_point_light();
    }
    w.stop();
    let mut h = harness(w);
    h.run();
    let _ = h.get_by_label("and 3 more");
    h.get_by_label("Undo all").click();
    h.run();
    assert!(h.state().0.play_changes().is_none());
}

#[test]
fn colours_blend_between_two() {
    let a = Color32::from_rgb(0, 100, 200);
    let b = Color32::from_rgb(200, 100, 0);
    assert_eq!(blend(a, b, 0.0), a);
    assert_eq!(blend(a, b, 1.0), b);
    assert_eq!(blend(a, b, 0.5), Color32::from_rgb(100, 100, 100));
    assert_eq!(count(1, "change"), "1 change");
    assert_eq!(count(0, "change"), "0 changes");
}

// What the game did is never kept: only the user's own edits (found by
// review: Keep kept the game's spawns, despawns and loaded saves too).

use crate::properties::PropertyRequest;
use area::builtin::{Light, Transform};
use area::{Edit, EntityData};
use glam::Vec3;
use reflect::{ComponentData, EntityId, Value};
use std::sync::Arc;

/// A hive that spawns a drone every tick, a component that takes its own
/// entity out on its first tick, and one that counts and climbs.
const GAME: &str = "fetch \"solis/world\":World;\nfetch \"solis/project\":Assembly;\n\n\
    component Hive {\n\
        method update(float dt) {\n\
            entity made = World:spawn(Assembly.Drones.Drone, [0, 0, 1]);\n\
        }\n\
    }\n\
    component Doomed {\n\
        method update(float dt) {\n\
            despawn();\n\
        }\n\
    }\n\
    component Counter {\n\
        has {\n\
            int count = 0;\n\
        }\n\
        method update(float dt) {\n\
            count = count + 1;\n\
            moveTo(position() + [0, 0, 1]);\n\
        }\n\
    }\n\
    pass Hive, Doomed, Counter;\nend\n";

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

const HIVE: u64 = 10;
const DOOMED: u64 = 11;
const DOOMED_CHILD: u64 = 12;
const CART: u64 = 13;
const LAMP: u64 = 14;
const CRATE: u64 = 15;
const POST: u64 = 16;
const BARREL: u64 = 17;

/// An entity called `name` carrying game code's `component`.
fn carrying(name: &str, component: &str) -> EntityData {
    let mut data = EntityData::new(name).with(&Transform::default());
    data.components
        .insert(component.into(), ComponentData::default());
    data
}

/// A workbench whose game spawns drones, takes "Doomed" (and its child)
/// out and counts on "Cart", in an area that also has a lamp, a crate, a
/// barrel and a placed copy of the lamp post assembly. Not yet playing;
/// nothing to undo.
fn with_a_busy_game() -> Workbench {
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::with_assemblies(&["drones/drone"]),
    );
    files.push(("main.lst".into(), GAME.to_string()));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let program = match script::compile(&code) {
        Ok(Some(program)) => Arc::new(program),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    let mut w = fresh();
    w.project = Some(crate::library::ProjectInfo {
        name: "Keeping".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.set_program(Some(program));
    // The drone the hive spawns.
    let mut drones = area::Area::new();
    drones.insert(id(1), EntityData::new("Drone").with(&Transform::default()));
    let drone = reflect::AssetId(reflect::Id::from_raw(901).unwrap());
    w.set_assembly(
        drone,
        Arc::new(area::text::AreaFile {
            kind: area::text::FileKind::Assembly { root: id(1) },
            area: drones,
            overrides: Default::default(),
        }),
    );
    let (post, post_file) = crate::test_support::lamp_post_file();
    w.set_assembly(post, post_file);
    w.set_library(vec![crate::library::LibraryItem {
        id: Some(drone),
        kind: crate::library::LibraryKind::Assembly,
        name: "drone".into(),
        path: "drones/drone.solasm".into(),
        thumbnail: None,
    }]);
    let mut child = EntityData::new("Doomed's child");
    child.parent = Some(id(DOOMED));
    let mut area = area::builtin::starter_area();
    area.insert(id(HIVE), carrying("Hive", "Hive"));
    area.insert(id(DOOMED), carrying("Doomed", "Doomed"));
    area.insert(id(DOOMED_CHILD), child);
    area.insert(id(CART), carrying("Cart", "Counter"));
    area.insert(
        id(LAMP),
        EntityData::new("Lamp")
            .with(&Transform::default())
            .with(&Light::point()),
    );
    area.insert(
        id(CRATE),
        EntityData::new("Crate").with(&Transform::default()),
    );
    area.insert(id(BARREL), EntityData::new("Barrel"));
    let lookup = |asset| w.assemblies().get(&asset).cloned();
    let placed = area::assembly::Expander::new(&lookup)
        .place(post, id(POST), &EntityData::new("Lamp post"))
        .unwrap();
    for (entity, data) in placed {
        area.insert(entity, data);
    }
    w.replace_document(Document::new(area));
    w
}

/// Runs `ticks` ticks of the game.
fn run(w: &mut Workbench, ticks: usize) {
    for _ in 0..ticks {
        w.play_for(world::TICK as f32);
    }
}

fn move_to(w: &mut Workbench, entity: EntityId, at: Vec3, gesture: u64) {
    w.apply_property(
        PropertyRequest::SetField {
            entity,
            component: "Transform".into(),
            field: "position".into(),
            value: Value::vec3(at),
        },
        gesture,
    );
}

fn drones(w: &Workbench) -> usize {
    w.document()
        .area()
        .iter()
        .filter(|(_, e)| e.name == "drone")
        .count()
}

#[test]
fn keep_keeps_your_own_edits_and_nothing_the_game_did() {
    let mut w = with_a_busy_game();
    let before = w.document().area().clone();
    w.play();
    run(&mut w, 5);
    // The game spawned, despawned and counted…
    assert_eq!(drones(&w), 5);
    assert!(!w.document().area().contains(id(DOOMED)));
    // …and the user moved one lamp.
    move_to(&mut w, id(LAMP), Vec3::new(3.0, 0.0, 0.0), 1);
    run(&mut w, 5);
    w.stop();
    assert_eq!(w.play_changes().unwrap(), ["Change Transform.position"]);
    w.keep_play_changes();
    // The area as it was at Play, with the lamp moved: no drones, Doomed
    // and its child back, the Cart where it stood.
    let mut expected = before;
    expected
        .apply(Edit::SetField {
            entity: id(LAMP),
            component: "Transform".into(),
            field: "position".into(),
            value: Some(Value::vec3(Vec3::new(3.0, 0.0, 0.0))),
        })
        .unwrap();
    assert_eq!(drones(&w), 0, "the game's spawns went with the game");
    assert!(w.document().area().contains(id(DOOMED)));
    assert!(w.document().area().contains(id(DOOMED_CHILD)));
    assert_eq!(*w.document().area(), expected);
}

#[test]
fn a_game_that_only_spawned_and_despawned_asks_nothing() {
    let mut w = with_a_busy_game();
    let before = w.document().area().clone();
    w.play();
    run(&mut w, 5);
    assert_ne!(*w.document().area(), before);
    w.stop();
    assert!(w.play_changes().is_none());
    assert_eq!(*w.document().area(), before);
}

fn rename(w: &mut Workbench, entity: EntityId, name: &str, gesture: u64) {
    w.apply_property(
        PropertyRequest::Rename {
            entity,
            name: name.into(),
        },
        gesture,
    );
}

fn a_drone(w: &Workbench) -> EntityId {
    let area = w.document().area();
    area.iter().find(|(_, e)| e.name == "drone").unwrap().0
}

fn said(w: &Workbench, words: &str) -> bool {
    w.log_lines().iter().any(|l| l.text.contains(words))
}

#[test]
fn a_change_to_something_the_game_made_goes_with_the_game() {
    let mut w = with_a_busy_game();
    let before = w.document().area().clone();
    w.play();
    run(&mut w, 3);
    let drone = a_drone(&w);
    rename(&mut w, drone, "Mine", 1);
    move_to(&mut w, drone, Vec3::X, 2);
    w.stop();
    // Nothing of the user's is left to keep, and the Console says why.
    assert!(w.play_changes().is_none());
    assert!(said(
        &w,
        "2 changes made while playing were to things the game made, which went with the game, \
         so they can't be kept: Rename, Change Transform.position."
    ));
    assert_eq!(*w.document().area(), before);
    // With a change of their own as well, only that one is offered.
    w.play();
    run(&mut w, 3);
    let drone = a_drone(&w);
    rename(&mut w, drone, "Mine", 3);
    rename(&mut w, id(CRATE), "Box", 4);
    w.stop();
    assert_eq!(w.play_changes().unwrap(), ["Rename"]);
    assert!(said(&w, "1 change made while playing was to things"));
    w.keep_play_changes();
    assert_eq!(w.document().area().get(id(CRATE)).unwrap().name, "Box");
    assert_eq!(drones(&w), 0);
    w.undo();
    assert_eq!(*w.document().area(), before);
}

#[test]
fn a_step_about_your_entities_and_the_games_keeps_your_share() {
    let mut w = with_a_busy_game();
    let before = w.document().area().clone();
    w.play();
    run(&mut w, 3);
    // The crate and a drone, deleted together; then the lamp put under
    // another drone (moved, so it stays where it was in the world).
    let drone = a_drone(&w);
    w.document.selection.select_all([drone], id(CRATE));
    w.delete_selection();
    let other = a_drone(&w);
    move_to(&mut w, other, Vec3::new(4.0, 0.0, 0.0), 1);
    w.reparent(&[id(LAMP)], Some(other));
    w.stop();
    assert_eq!(w.play_changes().unwrap(), ["Delete 2 entities"]);
    w.keep_play_changes();
    // The crate is deleted; the lamp is where it was, under nothing.
    let mut expected = before.clone();
    expected.apply(Edit::Despawn { root: id(CRATE) }).unwrap();
    assert_eq!(*w.document().area(), expected);
    w.undo();
    assert_eq!(*w.document().area(), before);
}

#[test]
fn a_change_to_an_entity_the_game_then_took_out_is_kept() {
    let mut w = with_a_busy_game();
    let before = w.document().area().clone();
    w.play();
    rename(&mut w, id(DOOMED), "Spared", 1);
    run(&mut w, 3);
    assert!(
        !w.document().area().contains(id(DOOMED)),
        "the game's doing"
    );
    w.stop();
    w.keep_play_changes();
    let mut expected = before;
    expected
        .apply(Edit::Rename {
            entity: id(DOOMED),
            name: "Spared".into(),
        })
        .unwrap();
    assert_eq!(*w.document().area(), expected);
}

#[test]
fn an_entity_you_deleted_stays_deleted_whatever_the_game_did_next() {
    let mut w = with_a_busy_game();
    let before = w.document().area().clone();
    w.play();
    // Deleted before its own code would have taken it out.
    w.select(id(DOOMED));
    w.delete_selection();
    run(&mut w, 3);
    w.stop();
    assert_eq!(w.play_changes().unwrap(), ["Delete Doomed"]);
    w.keep_play_changes();
    assert!(!w.document().area().contains(id(DOOMED)));
    assert!(!w.document().area().contains(id(DOOMED_CHILD)));
    // Undo brings it back whole, child and all.
    w.undo();
    assert_eq!(*w.document().area(), before);
}

#[test]
fn a_value_you_set_is_kept_as_you_set_it_though_the_game_changes_it_every_tick() {
    let mut w = with_a_busy_game();
    w.play();
    run(&mut w, 3);
    w.apply_property(
        PropertyRequest::SetField {
            entity: id(CART),
            component: "Counter".into(),
            field: "count".into(),
            value: Value::Int(100),
        },
        1,
    );
    move_to(&mut w, id(CART), Vec3::new(5.0, 0.0, 0.0), 2);
    run(&mut w, 4);
    // The game went on from them: it counts, and climbs.
    let now = w.values_now(id(CART), "Counter").unwrap();
    assert_eq!(now.get("count"), Some(&Value::Int(104)));
    let height = w.world().unwrap().global(id(CART)).unwrap().w_axis.z;
    assert!(height > 3.5, "{height}");
    w.stop();
    w.keep_play_changes();
    let cart = w.document().area().get(id(CART)).unwrap();
    assert_eq!(
        cart.components["Counter"].get("count"),
        Some(&Value::Int(100))
    );
    assert_eq!(
        cart.get::<Transform>().unwrap().position,
        Vec3::new(5.0, 0.0, 0.0)
    );
}

/// Five edits of different kinds: a move, a new component, a delete, a
/// new parent and a new name.
fn five_edits(w: &mut Workbench) {
    move_to(w, id(LAMP), Vec3::new(3.0, 1.0, 0.0), 1);
    w.apply_property(
        PropertyRequest::AddComponent {
            entity: id(CRATE),
            component: "Light".into(),
        },
        2,
    );
    w.select(id(BARREL));
    w.delete_selection();
    w.reparent(&[id(LAMP)], Some(id(CRATE)));
    rename(w, id(CART), "Wagon", 3);
}

#[test]
fn play_edit_stop_then_keep_or_undo_all_is_exactly_your_edits_or_none() {
    let mut w = with_a_busy_game();
    let before = w.document().area().clone();
    // What the five edits come to with no game running.
    five_edits(&mut w);
    let edited = w.document().area().clone();
    assert_eq!(w.document().history().undo_len(), 5);
    for _ in 0..5 {
        w.undo();
    }
    assert_eq!(*w.document().area(), before);

    // Undo all: the area from before Play, exactly.
    w.play();
    run(&mut w, 3);
    five_edits(&mut w);
    run(&mut w, 3);
    w.stop();
    assert_eq!(w.play_changes().unwrap().len(), 5);
    w.undo_play_changes();
    assert_eq!(*w.document().area(), before);
    assert_eq!(w.document().history().undo_len(), 0);

    // Keep: the area from before Play with exactly those five made.
    w.play();
    run(&mut w, 3);
    five_edits(&mut w);
    run(&mut w, 3);
    w.stop();
    assert_eq!(*w.document().area(), before, "until the answer");
    w.keep_play_changes();
    assert_eq!(*w.document().area(), edited);
    assert!(w.document().is_dirty());
    // Ctrl + Z takes them back one at a time, newest first, and Ctrl + Y
    // makes them again.
    assert_eq!(w.document().history().undo_len(), 5);
    assert_eq!(w.document().history().undo_label(), Some("Rename"));
    for _ in 0..5 {
        w.undo();
    }
    assert_eq!(*w.document().area(), before);
    for _ in 0..5 {
        w.redo();
    }
    assert_eq!(*w.document().area(), edited);
}

#[test]
fn undoing_a_change_from_before_play_can_be_kept_too() {
    let mut w = with_a_busy_game();
    let start = w.document().area().clone();
    rename(&mut w, id(CRATE), "Box", 1);
    w.play();
    run(&mut w, 3);
    w.undo();
    run(&mut w, 3);
    w.stop();
    assert_eq!(
        w.play_changes().unwrap(),
        ["Undid a change made before Play"]
    );
    assert_eq!(w.document().area().get(id(CRATE)).unwrap().name, "Box");
    w.keep_play_changes();
    assert_eq!(*w.document().area(), start);
    // And it can be redone, as if it had been undone without playing.
    w.redo();
    assert_eq!(w.document().area().get(id(CRATE)).unwrap().name, "Box");
    assert_eq!(drones(&w), 0);
}

#[test]
fn keep_after_loading_a_save_puts_your_edits_on_the_area_from_before_play() {
    let mut w = with_a_busy_game();
    let before = w.document().area().clone();
    w.play();
    run(&mut w, 3);
    let saved = w.save_text().expect("a save");
    move_to(&mut w, id(LAMP), Vec3::new(3.0, 0.0, 0.0), 1);
    run(&mut w, 2);
    w.load_game(&saved, "quick.solsave");
    // The editor shows the saved game's area: its drones, Doomed gone,
    // and the lamp post as plain entities, not a copy of its assembly.
    assert_eq!(drones(&w), 3);
    assert!(!w.document().area().contains(id(DOOMED)));
    assert!(w.document().area().get(id(POST)).unwrap().origin.is_none());
    // Edits after the load: one to the area's own crate, one to a drone
    // the save holds.
    rename(&mut w, id(CRATE), "Box", 2);
    let drone = a_drone(&w);
    rename(&mut w, drone, "Mine", 3);
    run(&mut w, 2);
    w.stop();
    assert_eq!(
        w.play_changes().unwrap(),
        ["Change Transform.position", "Rename"]
    );
    w.keep_play_changes();
    // The area from before Play (not the save's) with the two edits.
    let mut expected = before;
    for edit in [
        Edit::SetField {
            entity: id(LAMP),
            component: "Transform".into(),
            field: "position".into(),
            value: Some(Value::vec3(Vec3::new(3.0, 0.0, 0.0))),
        },
        Edit::Rename {
            entity: id(CRATE),
            name: "Box".into(),
        },
    ] {
        expected.apply(edit).unwrap();
    }
    assert_eq!(*w.document().area(), expected);
    assert!(
        w.document().area().get(id(POST)).unwrap().origin.is_some(),
        "still a copy of its assembly"
    );
    assert_eq!(w.document().history().undo_len(), 2);
}

/// The lamp post's file with its lamp's strength set to `strength`.
fn lamp_post_with(strength: f64) -> Arc<area::text::AreaFile> {
    let (_, file) = crate::test_support::lamp_post_file();
    let mut file = (*file).clone();
    assert!(area::assembly::set_in_file(
        &mut file,
        &[id(2)],
        "Light",
        "strength",
        Value::Float(strength)
    ));
    Arc::new(file)
}

#[test]
fn an_assembly_changed_on_disk_while_playing_is_no_change_of_yours() {
    let mut w = with_a_busy_game();
    let (post, _) = crate::test_support::lamp_post_file();
    let lamp = area::assembly::derive(id(POST), &[id(2)]);
    let strength = |w: &Workbench| {
        let lamp = w.document().area().get(lamp).unwrap();
        lamp.get::<Light>().unwrap().strength
    };
    w.play();
    run(&mut w, 3);
    w.set_assembly(post, lamp_post_with(5.0));
    assert_eq!(strength(&w), 5.0, "the game sees it at once");
    assert_eq!(drones(&w), 3, "and keeps what it spawned");
    run(&mut w, 2);
    w.stop();
    // Nothing is asked, and the copy is rebuilt in the area put back.
    assert!(w.play_changes().is_none());
    assert_eq!(strength(&w), 5.0);
    assert_eq!(drones(&w), 0);
    // With a change of the user's as well, Keep's area has theirs, the
    // new lamp post (changed again while the question waited), and still
    // none of the game's.
    let steps = w.document().history().undo_len();
    w.play();
    run(&mut w, 3);
    w.set_assembly(post, lamp_post_with(7.0));
    rename(&mut w, id(CRATE), "Box", 1);
    w.stop();
    assert_eq!(w.play_changes().unwrap(), ["Rename"]);
    w.set_assembly(post, lamp_post_with(9.0));
    w.keep_play_changes();
    assert_eq!(strength(&w), 9.0);
    assert_eq!(w.document().area().get(id(CRATE)).unwrap().name, "Box");
    assert_eq!(drones(&w), 0);
    assert!(w.document().area().get(id(POST)).unwrap().origin.is_some());
    // The rename is a step of its own, under the update of the copies.
    assert_eq!(w.document().history().undo_len(), steps + 2);
    w.undo();
    w.undo();
    assert_eq!(w.document().area().get(id(CRATE)).unwrap().name, "Crate");
}
