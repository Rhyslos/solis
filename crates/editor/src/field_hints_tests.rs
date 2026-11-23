//! Game code's field hints and the project's names in Properties
//! (decision 0127): a Guard written in Lust, shown with its slider, tip,
//! group and pickers; a slider's drag as one undo step; a value outside
//! the range left alone; ▶ playing a sound; a renamed file followed; and
//! each kind of name kept through a level, a module and a save game.

use crate::library::{LibraryFiles, LibraryItem, LibraryKind};
use crate::properties::PropertyRequest;
use crate::test_support::*;
use crate::workbench::{Workbench, WorkbenchAction};
use egui_kittest::kittest::Queryable as _;
use level::EntityData;
use level::builtin::Transform;
use reflect::{EntityId, Value};
use std::sync::Arc;

const GUARD: &str = "fetch \"solis/project\":[Sound, Clip, Picture, Module, Screen, Level, Tag];\n\
\n\
component Guard {\n\
    has {\n\
        float speed = 4.0; -- [range 0 10, step 0.5, unit \"m/s\", tip \"Walking speed\"]\n\
        Sound alertSound = Sound.Sfx.Alert;\n\
        Clip walk = Clip.Sprites.Guard.Walk;\n\
        Picture portrait = Picture.Portraits.GuardFace;\n\
        Module drops = Module.Coin;\n\
        Screen shows = Screen.Hud;\n\
        Level home = Level.Town;\n\
        Tag hunts = Tag.Enemy;\n\
        float sightRange = 8.0; -- [range 0 20, unit \"m\", group \"Seeing\"]\n\
        colour tint = [0.35, 0.63, 0.88];\n\
        string says = \"Halt!\"; -- [multiline]\n\
        string notes = \"\"; -- [hidden]\n\
    }\n\
}\n\
pass Guard;\n\
end\n";

/// The project's names the Guard picks from.
fn names() -> script::project_names::Names {
    let mut names = script::project_names::Names::with_modules(&["coin", "loot/gem"])
        .and_sounds(&["sfx/alert", "sfx/alarm"])
        .and_clips(&["sprites/guard/walk", "sprites/guard/run"], &[])
        .and_screens(&["Hud", "PauseMenu"])
        .and_levels(&["town", "forest/cave"])
        .and_tags(&["Enemy", "Friend"]);
    names.pictures = vec!["portraits/guardFace".into(), "portraits/captain".into()];
    names
}

fn program() -> Arc<script::Compiled> {
    let (mut files, _) = script::describe::descriptions_for("Test Engine 9.9.9", "solis", &names());
    files.push(("main.lst".into(), GUARD.into()));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    Arc::new(
        script::test_support::compile_unhurried(&code)
            .unwrap()
            .unwrap(),
    )
}

/// A workbench with the Guard program, a guard selected, and the Library
/// holding the sounds' files.
pub(crate) fn with_a_guard() -> (Workbench, EntityId) {
    let mut w = Workbench::new(info());
    w.open.project = Some(crate::library::ProjectInfo {
        name: "Guards".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.set_program(Some(program()));
    let id = EntityId(reflect::Id::from_raw(7).unwrap());
    w.open
        .document
        .edit(
            "Add",
            vec![level::Edit::Spawn {
                entities: vec![(id, EntityData::new("Guard").with(&Transform::default()))],
            }],
            None,
            None,
        )
        .unwrap();
    w.apply_property(
        PropertyRequest::AddComponent {
            entity: id,
            component: "Guard".into(),
        },
        1,
    );
    w.select(id);
    w.open.library = LibraryFiles::from(vec![LibraryItem {
        kind: LibraryKind::SoundEvent,
        name: "alert".into(),
        path: "sfx/alert.solsound".into(),
        ..LibraryItem::default()
    }]);
    (w, id)
}

fn guard_value(w: &Workbench, id: EntityId, field: &str) -> Value {
    w.document()
        .level()
        .get(id)
        .and_then(|e| e.components.get("Guard"))
        .and_then(|c| c.get(field))
        .cloned()
        .unwrap_or(Value::Bool(false))
}

#[test]
fn properties_shows_the_hints_and_the_pickers() {
    let (w, _) = with_a_guard();
    let mut h = harness(w);
    h.run();
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::Slider, "Speed");
    h.get_by_label("Walking speed · 0 to 10");
    h.get_by_label("SEEING");
    h.get_by_label("0 to 20");
    for (label, shown) in [
        ("Alert sound", "Sfx.Alert"),
        ("Walk", "Sprites.Guard.Walk"),
        ("Portrait", "Portraits.GuardFace"),
        ("Drops", "Coin"),
        ("Shows", "Hud"),
        ("Home", "Town"),
        ("Hunts", "Enemy"),
    ] {
        let picker = h.get_by_role_and_label(egui::accesskit::Role::ComboBox, label);
        assert_eq!(picker.value().as_deref(), Some(shown), "{label}");
    }
    assert!(
        h.query_by_label("Notes").is_none(),
        "a hidden field isn't shown"
    );
}

#[test]
fn a_slider_s_drag_is_one_undo_step_kept_to_the_step_and_range() {
    let (w, id) = with_a_guard();
    let mut h = harness(w);
    h.run();
    h.run();
    let before = h.state().0.document().history().undo_len();
    let slider = h
        .get_by_role_and_label(egui::accesskit::Role::Slider, "Speed")
        .rect();
    drag_for_frames(&mut h, slider.center(), egui::vec2(9.0, 0.0), 12);
    let state = &h.state().0;
    assert_eq!(
        state.document().history().undo_len(),
        before + 1,
        "one step"
    );
    let Value::Float(speed) = guard_value(state, id, "speed") else {
        panic!("a number");
    };
    assert!((0.0..=10.0).contains(&speed) && speed > 4.0, "{speed}");
    assert_eq!((speed * 2.0).fract(), 0.0, "a step of 0.5: {speed}");
}

#[test]
fn a_value_outside_the_range_is_shown_and_left_as_it_is() {
    let (mut w, id) = with_a_guard();
    w.apply_property(
        PropertyRequest::SetField {
            entity: id,
            component: "Guard".into(),
            field: "speed".into(),
            value: Value::Float(14.0),
        },
        2,
    );
    let mut h = harness(w);
    for _ in 0..4 {
        h.run();
    }
    assert_eq!(guard_value(&h.state().0, id, "speed"), Value::Float(14.0));
}

#[test]
fn play_asks_the_app_to_play_the_sound_s_file() {
    let (w, _) = with_a_guard();
    let mut h = harness(w);
    h.run();
    h.run();
    h.get_by_label("Play Alert sound").click();
    h.run();
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::PlaySound("sfx/alert.solsound".into())),
        "{:?}",
        h.state().1.actions
    );
}

#[test]
fn a_renamed_file_is_followed_as_one_undo_step() {
    let (mut w, id) = with_a_guard();
    let before = w.document().history().undo_len();
    w.follow_renamed_names("sfx/alert.solsound", "sfx/warn.solsound");
    assert_eq!(
        guard_value(&w, id, "alertSound"),
        Value::Text("Sfx.Warn".into())
    );
    assert_eq!(w.document().history().undo_len(), before + 1);
    // A folder moved takes the names inside it along.
    w.follow_renamed_names("portraits", "faces");
    assert_eq!(
        guard_value(&w, id, "portrait"),
        Value::Text("Faces.GuardFace".into())
    );
    // A file nothing names changes nothing.
    w.follow_renamed_names("sfx/other.solsound", "sfx/else.solsound");
    assert_eq!(w.document().history().undo_len(), before + 2);
    w.undo();
    w.undo();
    assert_eq!(
        guard_value(&w, id, "alertSound"),
        Value::Text("Sfx.Alert".into())
    );
}

/// The Guard's name fields, each given its second name.
const PICKED: [(&str, &str); 7] = [
    ("alertSound", "Sfx.Alarm"),
    ("walk", "Sprites.Guard.Run"),
    ("portrait", "Portraits.Captain"),
    ("drops", "Loot.Gem"),
    ("shows", "PauseMenu"),
    ("home", "Forest.Cave"),
    ("hunts", "Friend"),
];

fn picked() -> (Workbench, EntityId) {
    let (mut w, id) = with_a_guard();
    for (field, option) in PICKED {
        w.apply_property(
            PropertyRequest::SetField {
                entity: id,
                component: "Guard".into(),
                field: field.into(),
                value: Value::Text(option.into()),
            },
            3,
        );
    }
    (w, id)
}

#[test]
fn each_kind_of_name_is_kept_through_a_level_and_a_module() {
    let (w, id) = picked();
    let registry = &w.document().registry;
    let text = level::text::write(w.document().level(), registry, &|_| None);
    let read = level::text::read(&text, registry).unwrap();
    let mut module = level::text::LevelFile::plain(w.document().level().clone());
    module.kind = level::text::FileKind::Module { root: id };
    let module_text = level::module_file::write(&module, registry).unwrap();
    let module_read = level::module_file::read(&module_text, registry).unwrap();
    for (field, option) in PICKED {
        let in_level = read
            .get(id)
            .and_then(|e| e.components.get("Guard"))
            .and_then(|c| c.get(field));
        assert_eq!(
            in_level,
            Some(&Value::Text(option.into())),
            "level: {field}"
        );
        assert!(text.contains(option), "written as its name: {option}");
        let in_module = module_read
            .level
            .get(id)
            .and_then(|e| e.components.get("Guard"))
            .and_then(|c| c.get(field));
        assert_eq!(
            in_module,
            Some(&Value::Text(option.into())),
            "module: {field}"
        );
    }
}

#[test]
fn each_kind_of_name_is_kept_through_a_save_game() {
    let (mut w, id) = picked();
    unhurried(&mut w);
    w.play();
    w.run_game(world::TICK as f32 * 2.0);
    let text = w.save_of_game().unwrap().write().unwrap();
    w.stop();
    w.play();
    w.load_game(&text, "quick");
    let values = w
        .open
        .playing
        .as_mut()
        .unwrap()
        .game
        .as_mut()
        .unwrap()
        .values(id, "Guard")
        .expect("the guard runs");
    for (field, option) in PICKED {
        assert_eq!(
            values.get(field),
            Some(&Value::Text(option.into())),
            "{field}"
        );
    }
}
