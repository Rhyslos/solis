//! Properties for a screen's piece (decision 0089).

use super::*;
use crate::screen_doc::tests::MENU;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn harness(picked: Vec<usize>) -> Harness<'static, (ScreenDoc, bool)> {
    let mut doc = ScreenDoc::new("ui/pauseMenu.lst".into(), "PauseMenu".into(), MENU.into());
    doc.picked = vec![picked];
    Harness::builder()
        .with_size(egui::vec2(420.0, 900.0))
        .build_ui_state(
            |ui, (doc, changed): &mut (ScreenDoc, bool)| {
                *changed |= screen_properties(ui, doc, &Default::default());
            },
            (doc, false),
        )
}

#[test]
fn width_fill_and_auto_and_the_words_change_the_file() {
    let mut h = harness(vec![0, 0, 1]);
    h.run();
    h.get_all_by_label("fill").next().unwrap().click();
    h.run();
    assert!(h.state().1);
    assert!(
        h.state()
            .0
            .text
            .contains("button \"Resume\" [blue, width fill]")
    );
    h.get_all_by_label("auto").next().unwrap().click();
    h.run();
    assert!(h.state().0.text.contains("button \"Resume\" [blue] {"));
    // The words.
    field_type(&mut h, "Resume", "Carry on");
    assert!(
        h.state().0.text.contains("button \"Carry on\" [blue]"),
        "{}",
        h.state().0.text
    );
}

fn field_type(h: &mut Harness<'static, (ScreenDoc, bool)>, old: &str, new: &str) {
    h.get_all_by_value(old).next().unwrap().focus();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(new.into()));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    h.run();
}

#[test]
fn the_align_grid_and_colour_list_write_lust_s_words() {
    let mut h = harness(vec![0]);
    h.run();
    // Bottom right: the stack's center and middle go.
    h.get_by_label("right, bottom").click();
    h.run();
    assert!(
        h.state()
            .0
            .text
            .contains("stack [width fill, height fill, right, bottom]"),
        "{}",
        h.state().0.text
    );
    // A button's colour is a word alone: blue becomes green.
    let mut h = harness(vec![0, 0, 1]);
    h.run();
    h.get_all_by_role(egui::accesskit::Role::ComboBox)
        .next()
        .unwrap()
        .click();
    h.run();
    h.get_by_label("green").click();
    h.run();
    h.run();
    assert!(
        h.state().0.text.contains("button \"Resume\" [green] {"),
        "{}",
        h.state().0.text
    );
}

#[test]
fn all_its_settings_can_be_typed_as_written_and_bad_ones_are_refused() {
    let mut h = harness(vec![0, 0, 0]);
    h.run();
    field_type(&mut h, "size 28", "size 30, bold, color red");
    assert!(
        h.state()
            .0
            .text
            .contains("text \"Paused\" [size 30, bold, color red];")
    );
    field_type(&mut h, "size 30, bold, color red", "size [");
    assert!(
        h.state().0.text.contains("[size 30, bold, color red]"),
        "a [ ] that doesn't read is left as it was"
    );
}

#[test]
fn a_tie_is_chosen_from_the_entity_s_components() {
    use crate::screen_ties::{Kind, Readable};
    let mut doc = ScreenDoc::new("ui/pauseMenu.lst".into(), "PauseMenu".into(), MENU.into());
    doc.picked = vec![vec![0, 0, 0]];
    let game = crate::screen_ties::ScreenGame {
        readable: vec![Readable {
            component: "Health".into(),
            reader: "health".into(),
            module: "main".into(),
            fields: vec![("hp".into(), Kind::Number, 20.0)],
        }],
        events: Vec::new(),
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(420.0, 900.0))
        .build_ui_state(
            move |ui, (doc, changed): &mut (ScreenDoc, bool)| {
                *changed |= screen_properties(ui, doc, &game);
            },
            (doc, false),
        );
    h.run();
    h.get_all_by_label("Tie to…").next().unwrap().click();
    h.run();
    h.run();
    h.get_by_label("Health ⏵").click();
    h.run();
    h.run();
    h.get_by_label("hp").click();
    h.run();
    assert!(h.state().1);
    let text = &h.state().0.text;
    assert!(text.contains("text health.hp.text() [size 28];"), "{text}");
    assert!(
        text.contains("Health health = Ui:entity():health();"),
        "{text}"
    );
}

#[test]
fn with_nothing_to_read_ties_say_how_to_get_some() {
    let mut h = harness(vec![0, 0, 0]);
    h.run();
    assert!(h.query_by_label_contains("Give the entity").is_some());
}

/// The review, BUG-29: the square was always the standard blue.
#[test]
fn the_align_grid_marks_the_chosen_square_in_the_chosen_accent() {
    use crate::theme::{self, Accent};
    for accent in [Accent::Purple, Accent::Blue, Accent::Orange] {
        // The stack: centred and in the middle.
        let mut h = harness(vec![0]);
        let look = theme::InterfaceLook {
            accent,
            ..theme::InterfaceLook::default()
        };
        theme::restyle(&h.ctx, look);
        h.run();
        // The square, not the words beside the grid.
        let button = egui::accesskit::Role::Button;
        let square = h.get_by_role_and_label(button, "center, middle").rect();
        let drawn = crate::test_support::rects_drawn(&h);
        let fill = drawn
            .iter()
            .find(|(rect, ..)| rect.center().distance(square.center()) < 0.5)
            .map(|(_, fill, _)| *fill);
        assert_eq!(fill, Some(accent.colors().fill), "{}", accent.name());
        // And no other square.
        let other = h.get_by_role_and_label(button, "left, top").rect();
        let fill = drawn
            .iter()
            .find(|(rect, ..)| rect.center().distance(other.center()) < 0.5)
            .map(|(_, fill, _)| *fill);
        assert_eq!(fill, Some(egui::Color32::TRANSPARENT));
    }
    crate::interface_look::use_look(theme::InterfaceLook::default());
}
