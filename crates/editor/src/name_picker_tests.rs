//! A field holding one of the project's names: what it shows, finding and
//! picking a name, and ▶ on a sound.

use super::*;
use crate::library::{LibraryItem, LibraryKind};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

fn options() -> Vec<String> {
    ["Alarm", "Alert", "GuardAlert"].map(String::from).to_vec()
}

fn library() -> LibraryFiles {
    let names = script::project_names::Names::standard().and_sounds(&[
        "sfx/alarm",
        "sfx/alert",
        "voices/guardAlert",
    ]);
    let (text, _) = script::project_names::project_file("Test 1.0", &names);
    let paths = script::project_names::paths_in(&[("solis/project.lst".into(), text)]);
    let mut files = LibraryFiles::from(vec![
        LibraryItem {
            kind: LibraryKind::SoundEvent,
            name: "alert".into(),
            path: "sfx/alert.solsound".into(),
            ..LibraryItem::default()
        },
        LibraryItem {
            kind: LibraryKind::SoundEvent,
            name: "guardAlert".into(),
            path: "voices/guardAlert.solsound".into(),
            ..LibraryItem::default()
        },
    ]);
    files.set_name_paths(std::sync::Arc::new(paths));
    files
}

/// A sound field holding `value` (for several entities, if `mixed`); what
/// it asks for is kept.
fn sound(value: &str, mixed: bool) -> Harness<'static, (Value, Vec<NameAsk>)> {
    let options = options();
    let library = library();
    Harness::builder()
        .with_size(egui::vec2(320.0, 400.0))
        .build_ui_state(
            move |ui, (value, asked): &mut (Value, Vec<NameAsk>)| {
                let field = ("Alert sound", &*value, mixed);
                if let Some(ask) = name_picker(ui, field, (&options, NameKind::Sound), &library) {
                    asked.push(ask);
                }
            },
            (Value::Text(value.into()), Vec::new()),
        )
}

fn shown(h: &Harness<'_, (Value, Vec<NameAsk>)>) -> Option<String> {
    h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Alert sound")
        .value()
}

fn open(h: &mut Harness<'_, (Value, Vec<NameAsk>)>) {
    h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Alert sound")
        .click();
    h.run();
}

#[test]
fn the_field_shows_the_name_it_holds() {
    let mut h = sound("Alert", false);
    h.run();
    assert_eq!(shown(&h).as_deref(), Some("Alert"));
    let mut h = sound("Alert", true);
    h.run();
    assert_eq!(shown(&h).as_deref(), Some("—"), "values that differ");
}

#[test]
fn the_list_holds_every_name_and_a_row_picked_is_the_value() {
    let mut h = sound("Alert", false);
    h.run();
    open(&mut h);
    for name in ["Alarm", "Alert", "GuardAlert"] {
        h.get_by_label(name);
    }
    // Names have one dot (decision 0137): no folders to head the list.
    assert!(h.query_by_label("Voices").is_none());
    h.get_by_label("GuardAlert").click();
    h.run();
    assert_eq!(
        h.state().1,
        [NameAsk::Set(Value::Text("GuardAlert".into()))]
    );
}

#[test]
fn typing_finds_names_and_says_how_many() {
    let mut h = sound("Alert", false);
    h.run();
    open(&mut h);
    h.get_by_label("3 names · drag one from the Library too");
    h.get_by_label("Find a name").type_text("guard");
    h.run();
    h.run();
    assert!(h.query_by_label("Alarm").is_none());
    h.get_by_label("GuardAlert");
    h.get_by_label("1 of 3 names");
}

#[test]
fn play_asks_for_the_sound_s_file_and_needs_one() {
    let mut h = sound("Alert", false);
    h.run();
    h.get_by_label("Play Alert sound").click();
    h.run();
    assert_eq!(h.state().1, [NameAsk::Play("sfx/alert.solsound".into())]);
    // Alarm has no file in the Library: its ▶ plays nothing.
    let mut h = sound("Alarm", false);
    h.run();
    h.get_by_label("Play Alert sound").click();
    h.run();
    assert!(h.state().1.is_empty());
    // ▶ in the list plays a row's sound and picks nothing.
    let mut h = sound("Alarm", false);
    h.run();
    open(&mut h);
    h.get_by_label("Play GuardAlert").click();
    h.run();
    assert_eq!(
        h.state().1,
        [NameAsk::Play("voices/guardAlert.solsound".into())]
    );
}

#[test]
fn a_picture_and_its_parts_are_all_names() {
    let options: Vec<String> = ["Guard", "Sheet", "SheetAxe"].map(String::from).to_vec();
    let rows: Vec<String> = (0..options.len())
        .filter(|&at| pickable(&options, at, |o: &str| o == "Sheet"))
        .map(|at| options[at].clone())
        .collect();
    assert_eq!(rows, ["Guard", "Sheet", "SheetAxe"]);
}
