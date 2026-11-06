//! The Elements panel (decision 0089).

use super::*;
use crate::screen_doc::tests::MENU;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn harness() -> Harness<'static, (ScreenDoc, Vec<ElementsOut>)> {
    let doc = ScreenDoc::new("ui/pauseMenu.lst".into(), "PauseMenu".into(), MENU.into());
    Harness::builder()
        .with_size(egui::vec2(320.0, 500.0))
        .build_ui_state(
            |ui, (doc, out): &mut (ScreenDoc, Vec<ElementsOut>)| {
                out.push(elements_tab(ui, doc, Color32::BLUE));
            },
            (doc, Vec::new()),
        )
}

#[test]
fn the_tree_lists_pieces_in_order_with_code_greyed_and_picks_on_click() {
    let mut h = harness();
    h.run();
    for name in [
        "stack",
        "column",
        "text \"Paused\"",
        "button \"Resume\"",
        "button \"Quit\"",
    ] {
        h.get_by_label(name);
    }
    assert!(h.query_by_label_contains("‹code› loop 2").is_some());
    h.get_by_label("button \"Quit\"").click();
    h.run();
    assert_eq!(h.state().0.picked, vec![vec![0, 0, 2]]);
    // Ctrl-click adds; Delete over the panel takes both out.
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::COMMAND));
    h.get_by_label("text \"Paused\"").click();
    h.run();
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    assert_eq!(h.state().0.picked.len(), 2);
    h.key_press(egui::Key::Delete);
    h.run();
    assert!(h.state().1.iter().any(|o| o.changed));
    assert!(!h.state().0.text.contains("Quit") && !h.state().0.text.contains("Paused"));
}

#[test]
fn a_row_dragged_onto_another_moves_there() {
    let mut h = harness();
    h.run();
    let quit = h.get_by_label("button \"Quit\"").rect();
    let title = h.get_by_label("text \"Paused\"").rect();
    let from = quit.center();
    let to = egui::pos2(title.center().x, title.top() + 3.0);
    h.hover_at(from);
    h.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    for t in 1..=6 {
        h.hover_at(from + (to - from) * (t as f32 / 6.0));
        h.run();
    }
    h.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    h.run();
    let text = &h.state().0.text;
    assert!(
        text.find("button \"Quit\";").unwrap() < text.find("text \"Paused\"").unwrap(),
        "{text}"
    );
}
