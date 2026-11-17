//! Tools 1a's gate, as far as the editor's tests can try it (the plan,
//! `docs/plans/tools-1.md`, B5): a small level blocked out with brushes
//! drawn out, edited (a wall stretched on its card, a platform's top
//! extruded with E, the mouse and a click), played (a character standing
//! on the floor, walking into the wall and stopped by it, landing on the
//! extruded platform), and saved and reopened, the same brushes coming
//! back and playing the same. Play and Stop in the editor leave the level
//! as it was. The project lead tries the rest by hand (how it looks and
//! feels).

use crate::brushing::draw::Planned;
use crate::brushing::edit::{Mode, Part, PickHow};
use crate::panel::Panel;
use crate::shell::ShellCommand;
use crate::test_support::*;
use egui::Key;
use glam::{Quat, Vec3};
use level::builtin::{Brush, CharacterBody, Transform};
use level::{EntityData, Level};
use reflect::EntityId;

type H = egui_kittest::Harness<'static, State>;

/// A box brush `size` metres, its base's middle at `at`, drawn out as
/// the brush list and the mouse would hand it over.
fn draw_box(h: &mut H, at: Vec3, size: Vec3) -> EntityId {
    let shape = mesh::Shape::Box;
    let id = h.state_mut().0.add_drawn_brush(Planned {
        shape,
        position: at,
        rotation: Quat::IDENTITY,
        settings: crate::brushing::settings_for(shape, size),
    });
    h.run();
    id
}

fn steps(h: &H) -> usize {
    h.state().0.document().history().undo_len()
}

fn brush(level: &Level, id: EntityId) -> Brush {
    level.get(id).unwrap().get::<Brush>().unwrap()
}

/// The blockout: a floor 20 m across with its top at 0, a wall standing on
/// it 6 m east, and a 1 m platform 6 m west.
struct Blockout {
    floor: EntityId,
    wall: EntityId,
    platform: EntityId,
}

fn blocked_out(h: &mut H) -> Blockout {
    let before = steps(h);
    let floor = draw_box(h, Vec3::new(0.0, 0.0, -1.0), Vec3::new(20.0, 20.0, 1.0));
    let wall = draw_box(h, Vec3::new(6.0, 0.0, 0.0), Vec3::new(1.0, 8.0, 2.0));
    let platform = draw_box(h, Vec3::new(-6.0, 0.0, 0.0), Vec3::new(4.0, 4.0, 1.0));
    assert_eq!(steps(h), before + 3, "a step for each brush");
    let names: Vec<String> = [floor, wall, platform]
        .iter()
        .map(|id| {
            h.state()
                .0
                .document()
                .level()
                .get(*id)
                .unwrap()
                .name
                .clone()
        })
        .collect();
    assert_eq!(names, ["Box_01", "Box_02", "Box_03"]);
    Blockout {
        floor,
        wall,
        platform,
    }
}

/// The wall stretched to 3 m tall on the Brush card (one step), and the
/// platform's top extruded 1 m with E, the mouse and a click.
fn edited(h: &mut H, blockout: &Blockout) {
    let before = steps(h);
    h.state_mut()
        .0
        .resize_brush(blockout.wall, Vec3::new(1.0, 8.0, 3.0), 1);
    h.run();
    let wall = brush(h.state().0.document().level(), blockout.wall);
    assert!((wall.mesh.bounds().unwrap().size().z - 3.0).abs() < 1e-4);
    // The platform picked, its insides edited, its top picked.
    h.state_mut()
        .0
        .open
        .document
        .selection
        .select_only(blockout.platform);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    let top = {
        let edit = h
            .state()
            .0
            .viewport
            .brushing
            .edit
            .as_ref()
            .expect("editing");
        let mesh = edit.parts.as_ref().unwrap().mesh();
        mesh.faces()
            .map(|(f, _)| f)
            .find(|f| mesh.face_normal(*f).z > 0.99)
            .unwrap()
    };
    let w = &mut h.state_mut().0;
    w.brush_mode(Mode::Faces);
    w.pick_parts(&[Part::Face(top)], PickHow::Replace);
    w.shape_brush(ShellCommand::BrushExtrude);
    let pull = w
        .viewport
        .brushing
        .edit
        .as_mut()
        .and_then(|e| e.pull.as_mut())
        .expect("E starts the extrude");
    pull.set(1.0);
    w.pulled(crate::brushing::pull::PullOutcome::Going(true));
    w.pulled(crate::brushing::pull::PullOutcome::Keep);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    assert!(h.state().0.viewport.brushing.edit.is_none(), "Tab ends it");
    let platform = brush(h.state().0.document().level(), blockout.platform);
    assert_eq!(platform.mesh.face_count(), 10, "the top pulled out");
    assert!((platform.mesh.bounds().unwrap().max.z - 2.0).abs() < 1e-4);
    assert_eq!(steps(h), before + 2, "a step for each change");
}

/// A character standing `at`, added to `level`.
fn with_walker(level: &Level, at: Vec3) -> (Level, EntityId) {
    let mut level = level.clone();
    let walker = EntityId::random();
    level.insert(
        walker,
        EntityData::new("Walker")
            .with(&Transform {
                position: at,
                ..Transform::default()
            })
            .with(&CharacterBody::default()),
    );
    (level, walker)
}

/// Walks `walker` at `speed` metres a second (falling when not on
/// anything) for `ticks` sixtieths of a second; where its feet end up.
fn walk(world: &mut world::World, walker: EntityId, speed: Vec3, ticks: usize) -> Vec3 {
    let mut fall = 0.0f32;
    for _ in 0..ticks {
        let grounded = world
            .physics()
            .state()
            .character(walker)
            .is_some_and(|c| c.grounded);
        fall = if grounded { 0.0 } else { fall - 9.81 / 60.0 };
        world.move_character(walker, (speed + Vec3::Z * fall) / 60.0);
        world.step_physics();
    }
    world.global(walker).unwrap().w_axis.truncate()
}

/// What playing `level` gives: where a character dropped on the floor
/// stands, how far east it gets walking into the wall, and where one
/// dropped over the platform lands.
fn played(level: &Level) -> (Vec3, Vec3, Vec3) {
    let (on_floor, walker) = with_walker(level, Vec3::new(0.0, 0.0, 1.0));
    let mut world = world::World::from_level(&on_floor);
    let standing = walk(&mut world, walker, Vec3::ZERO, 90);
    let stopped = walk(&mut world, walker, Vec3::new(4.0, 0.0, 0.0), 180);
    let (over, walker) = with_walker(level, Vec3::new(-6.0, 0.0, 4.0));
    let mut world = world::World::from_level(&over);
    let landed = walk(&mut world, walker, Vec3::ZERO, 120);
    (standing, stopped, landed)
}

fn assert_plays_as_built((standing, stopped, landed): (Vec3, Vec3, Vec3)) {
    assert!(standing.z.abs() < 0.05, "stands on the floor: {standing}");
    // The wall's west face is at x = 5.5; a character is a little wide.
    assert!(
        stopped.x < 5.5 && stopped.x > 4.0,
        "stopped by the wall: {stopped}"
    );
    assert!(stopped.z.abs() < 0.05, "still on the floor: {stopped}");
    assert!(
        (landed.z - 2.0).abs() < 0.05,
        "lands on the extruded platform: {landed}"
    );
}

#[test]
fn a_level_blocked_out_edited_extruded_played_saved_and_reopened() {
    let mut h = rest_over(harness(crate::Workbench::new(info())), Panel::Viewport);
    let blockout = blocked_out(&mut h);
    edited(&mut h, &blockout);
    let level = h.state().0.document().level().clone();
    for id in [blockout.floor, blockout.wall, blockout.platform] {
        let b = brush(&level, id);
        assert!(b.solid && !b.damaged);
        let all: Vec<mesh::VertexId> = b.mesh.vertices().map(|(v, _)| v).collect();
        assert!(mesh::check_whole(&b.mesh, &all).is_ok(), "whole");
    }
    let before = played(&level);
    assert_plays_as_built(before);
    // Play and Stop in the editor: the level as it was.
    h.state_mut().0.play();
    for _ in 0..10 {
        h.run();
    }
    h.state_mut().0.stop();
    h.run();
    assert_eq!(*h.state().0.document().level(), level, "Stop puts it back");
    // Saved, then reopened.
    let registry = level::builtin::registry().unwrap();
    let text = level::text::write(&level, &registry, &|_| None);
    let read = level::text::read(&text, &registry).unwrap();
    assert_eq!(read, level, "the file reads back as saved");
    let mut again = rest_over(harness(crate::Workbench::new(info())), Panel::Viewport);
    again
        .state_mut()
        .0
        .open_level(read, std::path::PathBuf::from("level.solarea"));
    again.run();
    let reopened = again.state().0.document().level().clone();
    assert_eq!(reopened, level);
    for id in [blockout.floor, blockout.wall, blockout.platform] {
        let model = again.state().0.open.brushes.get(id).expect("drawn");
        assert_eq!(*model.mesh, *brush(&level, id).mesh, "the same brush");
        assert!(!model.damaged && !model.unwhole);
    }
    let after = played(&reopened);
    assert_plays_as_built(after);
    assert_eq!(before, after, "it plays the same after reopening");
    // Written again, nothing changes.
    assert_eq!(level::text::write(&reopened, &registry, &|_| None), text);
}
