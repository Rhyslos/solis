use super::*;
use crate::test_support::{harness, info};
use egui_kittest::kittest::Queryable;

fn fresh() -> Workbench {
    Workbench::new(info(), None)
}

#[test]
fn unpinning_takes_the_library_out_and_pinning_puts_it_back() {
    let mut w = fresh();
    assert!(w.library_pinned());
    w.set_library_pinned(false);
    assert!(w.dock.find_tab(&Panel::Library).is_none());
    w.set_library_pinned(true);
    assert!(w.dock.find_tab(&Panel::Library).is_some());
    let path = w.dock.find_tab(&Panel::Library).unwrap();
    assert_eq!(path.tab.0, 0, "first along the bottom");
}

#[test]
fn ctrl_space_pops_the_drawer_up_and_tucks_it_away() {
    let mut w = fresh();
    w.set_library_pinned(false);
    let mut h = harness(w);
    h.run();
    assert!(
        h.query_by_label("Search the Library").is_none(),
        "tucked away"
    );
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Space);
    h.run();
    assert!(h.state().0.drawer_open());
    assert!(
        h.query_by_label("Search the Library").is_some(),
        "popped up"
    );
    let _ = h.get_by_label("Pin the Library");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Space);
    h.run();
    assert!(!h.state().0.drawer_open());
    // Esc tucks it away too.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Space);
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(!h.state().0.drawer_open());
}

#[test]
fn a_click_outside_tucks_the_drawer_away() {
    let mut w = fresh();
    w.set_library_pinned(false);
    w.drawer_open = true;
    let mut h = harness(w);
    h.run();
    // The top of the editor is outside the drawer.
    let at = egui::pos2(700.0, 150.0);
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
    assert!(!h.state().0.drawer_open());
}

#[test]
fn the_pin_buttons_dock_and_undock_it() {
    let mut h = harness(fresh());
    h.run();
    h.get_by_label("Unpin the Library").click();
    h.run();
    assert!(!h.state().0.library_pinned());
    assert!(h.state().0.dock.find_tab(&Panel::Library).is_none());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Space);
    h.run();
    h.get_by_label("Pin the Library").click();
    h.run();
    assert!(h.state().0.library_pinned());
    assert!(!h.state().0.drawer_open());
    assert!(h.state().0.dock.find_tab(&Panel::Library).is_some());
}

#[test]
fn with_the_library_pinned_ctrl_space_brings_it_to_the_front() {
    let mut w = fresh();
    crate::layout::bring_to_front(&mut w.dock, Panel::Console);
    let mut h = harness(w);
    h.run();
    assert!(!h.state().0.in_front.contains(&Panel::Library));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Space);
    h.run();
    h.run();
    assert!(h.state().0.in_front.contains(&Panel::Library));
}

#[test]
fn chips_narrow_the_library() {
    let mut h = harness(fresh());
    h.run();
    for chip in ["All", "Models", "Sprites", "Maps", "Sounds", "Code"] {
        let _ = h.get_by_label(&format!("{chip} files"));
    }
    h.get_by_label("Sprites files").click();
    h.run();
    assert_eq!(
        h.state().0.library_state.filter,
        crate::library::Filter::Sprites
    );
}
