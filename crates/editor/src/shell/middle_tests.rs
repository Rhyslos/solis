//! The frame's middle used as a person uses it: the bands' measured
//! layout at two window sizes, the grip, Ctrl + Space, the divider, the
//! document tabs and their menus, groups and columns at narrow widths,
//! and what each tab remembers across a restart.

use super::sizes::Edge;
use super::workspaces::ShelfTab;
use super::{PanelLayout, RibbonTab, ShellState};
use crate::panel::Panel;
use crate::test_support::{State, fonts_ready, in_layout, info};
use crate::tokens::size;
use crate::workbench::Workbench;
use egui::{Key, Modifiers, Rect, vec2};
use egui_kittest::kittest::Queryable;

type Harness = egui_kittest::Harness<'static, State>;

/// The editor in a window `width` × `height`, drawn.
fn window(width: f32, height: f32, w: Workbench) -> Harness {
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(width, height))
        .build_ui_state(
            |ui, state: &mut State| {
                if !fonts_ready(ui.ctx()) {
                    return;
                }
                let (workbench, picker) = state;
                let actions = workbench.show(ui, picker);
                picker.actions.extend(actions);
            },
            (w, Default::default()),
        );
    h.run();
    h.run();
    h
}

/// The size the tab in front keeps for `edge` in the layout drawn.
fn size_of(h: &Harness, tab: RibbonTab, edge: Edge) -> f32 {
    let w = &h.state().0;
    w.shell.size(tab, w.open_now().layout, edge)
}

fn body(h: &Harness, panel: Panel) -> Rect {
    h.state()
        .0
        .shell
        .drawn_at(panel)
        .unwrap_or_else(|| panic!("{panel:?} is in view"))
}

/// A document tab: the node of that name as tall as a tab strip.
fn doc_tab(h: &Harness, name: &str) -> Rect {
    h.get_all_by_label(name)
        .map(|n| n.rect())
        .find(|r| (r.height() - size::DOC_TAB_H).abs() < 0.5)
        .unwrap_or_else(|| panic!("{name}'s document tab"))
}

/// Drags from `from` by `by`, a few steps at a time, as a hand does.
fn drag(h: &mut Harness, from: egui::Pos2, by: egui::Vec2) {
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    for step in 1..=5 {
        h.hover_at(from + by * step as f32 / 5.0);
        h.run();
    }
    h.drop_at(from + by);
    h.run();
}

#[test]
fn the_five_bands_have_the_design_s_heights_at_both_sizes() {
    for (width, height) in [(1440.0, 900.0), (1100.0, 700.0)] {
        let h = window(width, height, in_layout(PanelLayout::Shelf));
        // The window's margin: the tabs sit at the foot of the tab bar.
        let margin = h.get_by_label("File tab").rect().bottom() - size::TAB_BAR_H;
        let viewport = body(&h, Panel::Viewport);
        let area = body(&h, Panel::Area);
        let library = body(&h, Panel::Assets);
        let properties = body(&h, Panel::Properties);
        let ribbon_foot = margin + size::TAB_BAR_H + size::RIBBON_H;
        assert_eq!(viewport.top(), ribbon_foot + size::DOC_TAB_H, "{width}");
        assert_eq!(viewport.left(), margin);
        assert_eq!(viewport.right(), width - margin);
        // The shelf: its bar, then each column's header, then the bodies.
        let shelf = viewport.bottom();
        assert_eq!(
            area.top(),
            shelf + size::SHELF_BAR_H + size::COLUMN_HEADER_H
        );
        assert_eq!(area.bottom() - shelf - size::SHELF_BAR_H, size::SHELF_H);
        // The status bar under it all.
        assert_eq!(height - margin - area.bottom(), size::STATUS_BAR_H);
        // The columns: Area and the Library at their widths, Properties
        // between, 1 px apart.
        assert_eq!(area.width(), size::SCENE_COLUMN_W);
        assert_eq!(properties.left(), area.right() + 1.0);
        assert_eq!(library.left(), properties.right() + 1.0);
        assert_eq!(library.right(), width - margin);
        assert!((library.width() - size::ASSETS_COLUMN_W).abs() <= 1.0);
    }
}

#[test]
fn the_grip_resizes_the_shelf_within_limits() {
    let mut h = window(1440.0, 900.0, in_layout(PanelLayout::Shelf));
    let grip = h.get_by_label("Shelf grip").rect().center();
    let before = body(&h, Panel::Viewport).bottom();
    drag(&mut h, grip, vec2(0.0, -100.0));
    let height = size_of(&h, RibbonTab::Home, Edge::ShelfHeight);
    assert!((height - (size::SHELF_H + 100.0)).abs() <= 1.0);
    assert!((body(&h, Panel::Viewport).bottom() - (before - 100.0)).abs() <= 1.0);
    // Far up: the center keeps its least height.
    let grip = h.get_by_label("Shelf grip").rect().center();
    drag(&mut h, grip, vec2(0.0, -2000.0));
    assert!(body(&h, Panel::Viewport).height() + size::DOC_TAB_H >= size::CENTER_MIN_H - 1.0);
    // Far down: the shelf keeps its least.
    let grip = h.get_by_label("Shelf grip").rect().center();
    drag(&mut h, grip, vec2(0.0, 2000.0));
    assert_eq!(
        size_of(&h, RibbonTab::Home, Edge::ShelfHeight),
        size::SHELF_MIN_H
    );
    // Another tab's shelf is its own.
    assert_eq!(
        size_of(&h, RibbonTab::Debug, Edge::ShelfHeight),
        size::SHELF_H
    );
    // A double-click on the grip puts the standard height back.
    let grip = h.get_by_label("Shelf grip").rect().center();
    crate::test_support::double_click(&mut h, grip);
    assert_eq!(
        size_of(&h, RibbonTab::Home, Edge::ShelfHeight),
        size::SHELF_H
    );
}

#[test]
fn ctrl_space_folds_the_shelf_and_works_while_typing() {
    let mut h = window(1440.0, 900.0, Workbench::new(info()));
    let area = body(&h, Panel::Area);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Space);
    h.run();
    assert!(h.state().0.shelf_folded());
    assert!(h.state().0.shell.drawn_at(Panel::Console).is_none());
    // The bar stays, on the status bar; the columns beside the area have
    // the room.
    let _ = h.get_by_label("Place");
    assert!(body(&h, Panel::Area).height() > area.height());
    h.key_press_modifiers(Modifiers::COMMAND, Key::Space);
    h.run();
    assert!(!h.state().0.shelf_folded());
    // While typing in a field, too, and nothing is typed.
    h.get_by_label("Search the area by name").click();
    h.run();
    h.get_by_label("Search the area").focus();
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND, Key::Space);
    h.run();
    assert!(h.state().0.shelf_folded());
    assert!(h.state().0.open.library_state.search.is_empty());
    // A click on a shelf tab opens it again.
    h.get_by_label("Console").click();
    h.run();
    h.run();
    assert!(!h.state().0.shelf_folded());
    assert_eq!(h.state().0.shelf_tab(), Some(ShelfTab::Console));
}

#[test]
fn two_groups_side_by_side_share_the_room_and_stack_when_narrow() {
    let mut w = Workbench::new(info());
    w.show_tab(RibbonTab::Audio);
    let mut h = window(1440.0, 900.0, w);
    let mixer = body(&h, Panel::Mixer);
    let bus = body(&h, Panel::Bus);
    assert_eq!(bus.left(), mixer.right() + 1.0, "a 1 px divider");
    assert_eq!(mixer.top(), bus.top());
    // The divider's drag moves the split, within its limits.
    let divider = h.get_by_label("Group divider").rect().center();
    drag(&mut h, divider, vec2(-300.0, 0.0));
    let split = size_of(&h, RibbonTab::Audio, Edge::Split);
    assert!(split < 0.4, "{split}");
    let divider = h.get_by_label("Group divider").rect().center();
    drag(&mut h, divider, vec2(-2000.0, 0.0));
    let split = size_of(&h, RibbonTab::Audio, Edge::Split);
    assert_eq!(split, super::workspaces::SPLIT_LIMIT);
    // Too narrow for two side by side: they stack.
    let mut w = Workbench::new(info());
    w.show_tab(RibbonTab::Audio);
    let h = window(2.0 * size::GROUP_MIN_W - 40.0, 900.0, w);
    let mixer = body(&h, Panel::Mixer);
    let bus = body(&h, Panel::Bus);
    assert_eq!(bus.top(), mixer.bottom() + 1.0 + size::DOC_TAB_H);
    assert_eq!(bus.left(), mixer.left());
}

#[test]
fn columns_wrap_under_each_other_when_narrow() {
    // Too narrow for the side layout, and for the three columns at their
    // least side by side: they go on the shelf, under each other.
    let h = window(
        3.0 * size::COLUMN_MIN_W - 20.0,
        900.0,
        Workbench::new(info()),
    );
    let area = body(&h, Panel::Area);
    let properties = body(&h, Panel::Properties);
    let library = body(&h, Panel::Assets);
    assert_eq!(area.left(), properties.left());
    assert!(properties.top() > area.bottom(), "{area:?} {properties:?}");
    assert!(library.top() > properties.bottom());
    assert_eq!(library.width(), area.width());
}

#[test]
fn the_area_s_tab_has_its_plus_and_its_menu() {
    let mut w = Workbench::new(info());
    w.add_point_light();
    let mut h = window(1440.0, 900.0, w);
    h.get_by_label("New or open an area").click();
    h.run();
    let _ = h.get_by_label("Open area\u{2026}");
    h.get_by_label("New area").click();
    h.run();
    h.run();
    // A new area, asked about the unsaved light first.
    assert!(h.state().0.open.unsaved_prompt.is_some());
    h.state_mut().0.open.unsaved_prompt = None;
    let tab = doc_tab(&h, "Untitled area");
    h.hover_at(tab.center());
    h.run();
    h.event(egui::Event::PointerButton {
        pos: tab.center(),
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: tab.center(),
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run();
    for item in ["Close", "Close others"] {
        let _ = h.get_by_label(item);
    }
    let _ = h.get_by_label_contains("Save as\u{2026}");
    h.get_by_label("Show in the Library").click();
    h.run();
    assert_eq!(h.state().0.ribbon_tab(), RibbonTab::Library);
}

#[test]
fn the_game_tab_comes_to_the_front_on_play_and_goes_on_stop() {
    let mut h = window(1440.0, 900.0, Workbench::new(info()));
    assert!(h.state().0.shell.drawn_at(Panel::Game).is_none());
    h.state_mut().0.play();
    h.run();
    h.run();
    assert!(
        h.state().0.shell.drawn_at(Panel::Game).is_some(),
        "in front"
    );
    let game = body(&h, Panel::Game);
    // The area's tab beside it brings the viewport back.
    let tab = doc_tab(&h, "Untitled area");
    h.get_all_by_label("Untitled area")
        .find(|n| n.rect() == tab)
        .unwrap()
        .click();
    h.run();
    assert_eq!(body(&h, Panel::Viewport), game, "the same group");
    h.state_mut().0.stop();
    h.run();
    h.run();
    assert!(h.query_by_label("Game").is_none(), "gone with the game");
}

#[test]
fn each_tab_remembers_its_shelf_its_height_and_its_split_across_a_restart() {
    let mut h = window(1440.0, 900.0, in_layout(PanelLayout::Shelf));
    h.get_by_label("Console").click();
    h.run();
    let grip = h.get_by_label("Shelf grip").rect().center();
    drag(&mut h, grip, vec2(0.0, -60.0));
    crate::test_support::ribbon_tab(&mut h, RibbonTab::Audio);
    h.run();
    let divider = h.get_by_label("Group divider").rect().center();
    drag(&mut h, divider, vec2(200.0, 0.0));
    h.state_mut().0.set_ribbon_collapsed(true);
    let saved = ron::to_string(&h.state().0.shell_state()).unwrap();
    // A later run.
    let back: ShellState = ron::from_str(&saved).unwrap();
    let mut w = Workbench::new(info());
    w.restore_shell(Some(&back), false);
    let before = &h.state().0.shell;
    for tab in [RibbonTab::Home, RibbonTab::Audio] {
        let (a, b) = (before.arrangement(tab), w.shell.arrangement(tab));
        assert_eq!(a.shelf_tab, b.shelf_tab, "{tab:?}");
    }
    assert_eq!(before.sizes, w.shell.sizes);
    assert!(w.shell.sizes.kept().count() >= 2);
    assert_eq!(w.ribbon_tab(), RibbonTab::Audio);
    assert!(w.ribbon_collapsed());
    assert_eq!(
        w.shell.arrangement(RibbonTab::Home).shelf_tab,
        ShelfTab::Console
    );
}

#[test]
fn a_dragged_tab_splits_to_a_half_joins_on_a_strip_and_esc_puts_it_back() {
    let mut w = Workbench::new(info());
    w.show_tab(RibbonTab::Debug);
    let mut h = window(1440.0, 900.0, w);
    let groups = |h: &Harness| {
        let w = &h.state().0;
        w.shell
            .arrangement(RibbonTab::Debug)
            .arranged(RibbonTab::Debug, w.open_now())
    };
    assert_eq!(groups(&h), [vec![Panel::Profiler, Panel::Game]]);
    // The Game's tab to the right half: alone in a group there.
    let game = doc_tab(&h, "Game").center();
    let center = body(&h, Panel::Profiler);
    let right = egui::pos2(center.right() - 100.0, center.center().y);
    drag(&mut h, game, right - game);
    assert_eq!(groups(&h), [vec![Panel::Profiler], vec![Panel::Game]]);
    // Onto the Profiler's strip: one group again.
    let game = doc_tab(&h, "Game").center();
    let profiler = doc_tab(&h, "Profiler").center();
    drag(&mut h, game, profiler - game);
    assert_eq!(groups(&h), [vec![Panel::Profiler, Panel::Game]]);
    // Esc while dragging puts it back where it was.
    let game = doc_tab(&h, "Game").center();
    h.hover_at(game);
    h.run();
    h.drag_at(game);
    h.run();
    for step in 1..=5 {
        h.hover_at(game + (right - game) * step as f32 / 5.0);
        h.run();
    }
    h.key_press(egui::Key::Escape);
    h.run();
    h.drop_at(right);
    h.run();
    assert_eq!(groups(&h), [vec![Panel::Profiler, Panel::Game]]);
}
