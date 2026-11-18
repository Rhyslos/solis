//! The frame used as a person uses it (decision 0118, boards 03, 05, 06
//! and 09): each tab in the same four slots, each layout's slots where
//! its card draws them, every edge dragged, held to its least, put back
//! with a double-click and remembered per tab (or shared), a slot hidden
//! from its ⋯ menu and shown again, the bottom panel folded to one line
//! and opened at its last height with its counts, the command line only
//! on the Console tab, a layout chosen from its card with Undo, placing
//! from the Library, and the window resized.

use super::sizes::Edge;
use super::workspaces::BottomTab;
use super::{PanelLayout, RibbonTab};
use crate::console::LogLevel;
use crate::panel::Panel;
use crate::test_support::{State, double_click, fonts_ready, in_layout, info};
use crate::tokens::size;
use crate::workbench::Workbench;
use egui::{Pos2, Rect, vec2};
use egui_kittest::kittest::Queryable;

type Harness = egui_kittest::Harness<'static, State>;

fn window(width: f32, w: Workbench) -> Harness {
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(width, 900.0))
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

fn body(h: &Harness, panel: Panel) -> Rect {
    h.state()
        .0
        .shell
        .drawn_at(panel)
        .unwrap_or_else(|| panic!("{panel:?} is in view"))
}

fn size_of(h: &Harness, edge: Edge) -> f32 {
    let w = &h.state().0;
    w.shell.size(w.ribbon_tab(), w.open_now().layout, edge)
}

/// The middle of the edge called `name`.
fn edge(h: &Harness, name: &str) -> Pos2 {
    h.get_by_label(name).rect().center()
}

/// Drags from `from` by `by`, a few steps at a time, as a hand does.
fn drag(h: &mut Harness, from: Pos2, by: egui::Vec2) {
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

/// A tab with something open in every slot it has.
fn with_everything(tab: RibbonTab) -> Workbench {
    let mut w = Workbench::new(info());
    w.show_tab(tab);
    w
}

#[test]
fn each_tab_lays_out_in_the_same_four_slots() {
    for (tab, centre, left, right) in [
        (
            RibbonTab::Home,
            Panel::Viewport,
            Some(Panel::Level),
            Some(Panel::Properties),
        ),
        (
            RibbonTab::Library,
            Panel::Library,
            None,
            Some(Panel::Properties),
        ),
        (
            RibbonTab::Ui,
            Panel::Canvas,
            Some(Panel::Elements),
            Some(Panel::Properties),
        ),
        (
            RibbonTab::Audio,
            Panel::Mixer,
            Some(Panel::SoundEvents),
            Some(Panel::Bus),
        ),
        (RibbonTab::Debug, Panel::Profiler, None, None),
    ] {
        let h = window(1440.0, with_everything(tab));
        let centre = body(&h, centre);
        let margin = h.get_by_label("File tab").rect().bottom() - size::TAB_BAR_H;
        let foot = 900.0 - margin - size::STATUS_BAR_H;
        if let Some(left) = left {
            let left = body(&h, left);
            assert_eq!(left.left(), margin, "{tab:?}");
            assert_eq!(left.bottom(), foot, "{tab:?}: full height");
            assert!(left.right() < centre.left(), "{tab:?}");
        }
        if let Some(right) = right {
            let right = body(&h, right);
            assert_eq!(right.right(), 1440.0 - margin, "{tab:?}");
            assert_eq!(right.bottom(), foot, "{tab:?}: full height");
            assert!(right.left() > centre.right(), "{tab:?}");
        }
        // The bottom panel, open or folded, is under the centre only.
        let bar = h.state().0.shell.drawn.iter().map(|(_, r)| *r);
        for rect in bar.filter(|r| r.top() > centre.bottom()) {
            assert!(rect.left() >= centre.left() - 0.5, "{tab:?}");
            assert!(rect.right() <= centre.right() + 0.5, "{tab:?}");
        }
    }
    // Debug's left slot is still to come, drawn as an unfinished part.
    let h = window(1440.0, with_everything(RibbonTab::Debug));
    assert!(h.query_all_by_label("Running").next().is_some());
}

#[test]
fn each_layout_puts_the_slots_where_its_card_draws_them() {
    for layout in PanelLayout::ALL {
        let h = window(1440.0, in_layout(layout));
        let view = body(&h, Panel::Viewport);
        let shown = |panel| h.state().0.shell.drawn_at(panel);
        match layout {
            PanelLayout::Standard | PanelLayout::TwoViews => {
                assert!(shown(Panel::Level).unwrap().right() < view.left());
                assert!(shown(Panel::Properties).unwrap().left() > view.right());
                assert!(shown(Panel::Assets).unwrap().top() > view.bottom());
            }
            PanelLayout::WideView => {
                let (level, properties) = (
                    shown(Panel::Level).unwrap(),
                    shown(Panel::Properties).unwrap(),
                );
                assert!(level.left() > view.right() && level.bottom() < properties.top());
            }
            PanelLayout::LibraryLeft => {
                let (level, assets) = (shown(Panel::Level).unwrap(), shown(Panel::Assets).unwrap());
                assert_eq!(assets.left(), level.left());
                assert!(assets.top() > level.bottom());
            }
            PanelLayout::LibraryRight => {
                let (properties, assets) = (
                    shown(Panel::Properties).unwrap(),
                    shown(Panel::Assets).unwrap(),
                );
                assert_eq!(assets.right(), properties.right());
            }
            PanelLayout::TallProperties => {
                let (level, assets) = (shown(Panel::Level).unwrap(), shown(Panel::Assets).unwrap());
                assert!(assets.left() <= level.left() + 0.5, "under Level too");
            }
            // Chosen in Preferences it hides them all; as a setting alone
            // it is the standard frame until it is chosen.
            PanelLayout::JustTheView => {}
        }
    }
}

#[test]
fn every_edge_drags_is_held_to_its_least_and_a_double_click_puts_it_back() {
    let cases = [
        (
            PanelLayout::Standard,
            "Left column's edge",
            Edge::Left,
            vec2(120.0, 0.0),
        ),
        (
            PanelLayout::Standard,
            "Right column's edge",
            Edge::Right,
            vec2(-120.0, 0.0),
        ),
        (
            PanelLayout::Standard,
            "Bottom panel's edge",
            Edge::Bottom,
            vec2(0.0, -80.0),
        ),
        (
            PanelLayout::LibraryLeft,
            "Between Level and Library",
            Edge::LeftShare,
            vec2(0.0, 80.0),
        ),
        (
            PanelLayout::WideView,
            "Between Level and Properties",
            Edge::RightShare,
            vec2(0.0, -80.0),
        ),
    ];
    for (layout, name, which, by) in cases {
        let mut h = window(1440.0, in_layout(layout));
        let before = size_of(&h, which);
        {
            let at = edge(&h, name);
            drag(&mut h, at, by);
        }
        assert_ne!(size_of(&h, which), before, "{name} moved");
        // Dragged far, it stops at its least, and the centre keeps its own.
        {
            let at = edge(&h, name);
            drag(&mut h, at, by * -40.0);
        }
        let view = body(&h, Panel::Viewport);
        assert!(view.width() >= size::CENTER_MIN_W - 1.0, "{name}");
        assert!(
            view.height() + size::DOC_TAB_H >= size::CENTER_MIN_H - 1.0,
            "{name}"
        );
        for panel in [Panel::Level, Panel::Properties] {
            assert!(
                body(&h, panel).width() >= size::COLUMN_MIN_W - 1.0,
                "{name}"
            );
        }
        let at = edge(&h, name);
        double_click(&mut h, at);
        assert_eq!(size_of(&h, which), before, "{name} is its standard again");
    }
}

#[test]
fn the_pointer_shows_a_resize_arrow_over_an_edge() {
    let mut h = window(1440.0, Workbench::new(info()));
    let cursor = |h: &Harness| h.output().platform_output.cursor_icon;
    for (name, icon) in [
        ("Left column's edge", egui::CursorIcon::ResizeHorizontal),
        ("Right column's edge", egui::CursorIcon::ResizeHorizontal),
        ("Bottom panel's edge", egui::CursorIcon::ResizeVertical),
    ] {
        let at = edge(&h, name);
        h.hover_at(at);
        h.run();
        assert_eq!(cursor(&h), icon, "{name}");
    }
    h.hover_at(body(&h, Panel::Level).center());
    h.run();
    assert_ne!(cursor(&h), egui::CursorIcon::ResizeHorizontal);
}

#[test]
fn sizes_are_kept_per_tab_or_shared_when_the_switch_is_off() {
    let mut h = window(1440.0, Workbench::new(info()));
    {
        let at = edge(&h, "Right column's edge");
        drag(&mut h, at, vec2(-100.0, 0.0));
    }
    let home = size_of(&h, Edge::Right);
    crate::test_support::ribbon_tab(&mut h, RibbonTab::Ui);
    h.run();
    assert_eq!(
        size_of(&h, Edge::Right),
        Edge::Right.standard(RibbonTab::Ui, PanelLayout::Standard),
        "each tab keeps its own"
    );
    // Off: the size dragged on one tab is used on all of them.
    let look = h.state().0.interface_look();
    let ctx = h.ctx.clone();
    h.state_mut().0.choose_interface_look(
        &ctx,
        crate::interface_look::InterfaceLook {
            sizes_per_tab: false,
            ..look
        },
    );
    h.run();
    h.run();
    crate::test_support::ribbon_tab(&mut h, RibbonTab::Home);
    h.run();
    assert_eq!(size_of(&h, Edge::Right), home);
    crate::test_support::ribbon_tab(&mut h, RibbonTab::Ui);
    h.run();
    assert_eq!(size_of(&h, Edge::Right), home);
    let width = body(&h, Panel::Properties).width();
    crate::test_support::ribbon_tab(&mut h, RibbonTab::Home);
    h.run();
    assert_eq!(body(&h, Panel::Properties).width(), width);
}

#[test]
fn a_slot_hides_from_its_menu_and_comes_back() {
    let mut h = window(1440.0, Workbench::new(info()));
    let view = body(&h, Panel::Viewport);
    h.get_by_label("More of Properties").click();
    h.run();
    h.get_by_label("Hide Properties").click();
    h.run();
    h.run();
    assert!(h.state().0.shell.drawn_at(Panel::Properties).is_none());
    assert!(
        body(&h, Panel::Viewport).width() > view.width(),
        "the view has its room"
    );
    // The bottom panel's bar offers it back.
    h.get_by_label("Show Properties").click();
    h.run();
    h.run();
    assert!(h.state().0.shell.drawn_at(Panel::Properties).is_some());
    // The Level panel's own menu is in its ⋯ too, over Hide.
    h.get_by_label("More of Level").click();
    h.run();
    assert!(
        h.query_all_by_label("Fold everything").count() >= 2,
        "its own item too"
    );
    h.get_by_label("Hide Level").click();
    h.run();
    h.run();
    assert!(h.state().0.shell.drawn_at(Panel::Level).is_none());
    // The search's Show hidden panels, and showing the panel itself,
    // bring it back too.
    crate::test_support::run(&mut h, "Show hidden panels");
    h.run();
    assert!(h.state().0.shell.drawn_at(Panel::Level).is_some());
    h.state_mut()
        .0
        .shell
        .arrangement_mut(RibbonTab::Home)
        .hidden
        .left = true;
    h.state_mut().0.show_panel(Panel::Level);
    h.run();
    assert!(h.state().0.shell.drawn_at(Panel::Level).is_some());
}

#[test]
fn the_bottom_panel_folds_to_one_line_and_opens_at_its_last_height() {
    let mut h = window(1440.0, Workbench::new(info()));
    {
        let at = edge(&h, "Bottom panel's edge");
        drag(&mut h, at, vec2(0.0, -90.0));
    }
    let height = body(&h, Panel::Assets).height();
    let view = body(&h, Panel::Viewport);
    h.get_by_label("Fold the bottom panel").click();
    h.run();
    h.run();
    let w = &h.state().0;
    assert!(w.bottom_folded());
    assert!(w.shell.drawn_at(Panel::Assets).is_none());
    // One thin line over the status line: the view runs down to it.
    let margin = h.get_by_label("File tab").rect().bottom() - size::TAB_BAR_H;
    let foot = 900.0 - margin - size::STATUS_BAR_H;
    assert_eq!(
        body(&h, Panel::Viewport).bottom(),
        foot - size::BOTTOM_BAR_H
    );
    let _ = h.get_by_label("Click a name to open it");
    // A name opens its tab at its last height.
    h.get_by_label("Library").click();
    h.run();
    h.run();
    assert!(!h.state().0.bottom_folded());
    assert_eq!(body(&h, Panel::Assets).height(), height);
    assert_eq!(body(&h, Panel::Viewport), view);
    // Clicking the open tab again folds it too.
    h.get_by_label("Library").click();
    h.run();
    assert!(h.state().0.bottom_folded());
}

#[test]
fn the_tabs_counts_follow_the_console_and_problems() {
    let mut w = Workbench::new(info());
    w.log(LogLevel::Warning, "No sound: couldn't open the sound card");
    let mut h = window(1440.0, w);
    let _ = h.get_by_label("Console");
    let _ = h.get_by_label("1 warning");
    h.state_mut().0.log(LogLevel::Error, "A model didn't load");
    h.state_mut().0.log(LogLevel::Info, "Saved.");
    h.run();
    let _ = h.get_by_label("1 error");
    // Folded, the newest warning or error shows faintly in the line.
    h.state_mut().0.fold_bottom(true);
    h.run();
    h.run();
    let _ = h.get_by_label_contains("A model didn't load");
    // A problem in the level is counted on Problems.
    h.state_mut().0.set_code_problems(
        vec![script::CodeProblem {
            file: "main.lst".into(),
            line: 3,
            message: "an unknown name".into(),
            column: 1,
        }],
        &[],
    );
    h.run();
    let _ = h.get_by_label("1 problem");
    // Cleared, the counts go.
    h.state_mut().0.console.clear();
    h.run();
    assert!(h.query_by_label("1 warning").is_none());
}

#[test]
fn the_command_line_shows_only_on_the_console_tab() {
    let mut h = window(1440.0, Workbench::new(info()));
    let line = |h: &Harness| {
        h.ctx
            .read_response(crate::console::command_line_id())
            .is_some()
    };
    assert_eq!(h.state().0.bottom_tab(), Some(BottomTab::Library));
    assert!(!line(&h), "not on the Library");
    h.get_by_label("Console").click();
    h.run();
    h.run();
    assert!(line(&h), "on the Console");
    h.get_by_label("Problems").click();
    h.run();
    h.run();
    assert!(!line(&h), "not on Problems");
    // And the Console's filter sits in the panel's bar.
    h.get_by_label("Console").click();
    h.run();
    let _ = h.get_by_label("Search the Console");
}

#[test]
fn a_layout_chosen_from_its_card_applies_at_once_with_undo() {
    let mut w = Workbench::new(info());
    w.show_settings(crate::file_pages::Tab::Layout);
    let mut h = window(1440.0, w);
    // Every layout has its card; the one in use is marked.
    for layout in PanelLayout::ALL {
        assert!(
            h.query_all_by_label(layout.name()).next().is_some(),
            "{layout:?}"
        );
    }
    let _ = h.get_by_label("In use");
    let _ = h.get_by_label("Preview");
    // Pointing at one shows it in the Preview box.
    let card = |h: &Harness, name: &str| {
        h.get_all_by_label(name)
            .max_by(|a, b| a.rect().height().total_cmp(&b.rect().height()))
            .expect("its card")
            .rect()
            .center()
    };
    let wide = card(&h, "Wide view");
    h.hover_at(wide);
    h.run();
    let _ = h.get_by_label("while you point at it");
    crate::test_support::click(&mut h, wide, egui::Modifiers::NONE);
    h.run();
    h.run();
    assert_eq!(h.state().0.layout(), PanelLayout::WideView);
    let _ = h.get_by_label("Layout is now");
    h.get_by_label("Undo").click();
    h.run();
    h.run();
    assert_eq!(h.state().0.layout(), PanelLayout::Standard);
    assert!(h.query_by_label("Layout is now").is_none());
    // Back in the editor, Just the view leaves only the view and the
    // bottom line.
    let just = card(&h, "Just the view");
    crate::test_support::click(&mut h, just, egui::Modifiers::NONE);
    h.run();
    h.state_mut().0.show_tab(RibbonTab::Home);
    h.run();
    h.run();
    let w = &h.state().0;
    assert!(w.shell.drawn_at(Panel::Level).is_none());
    assert!(w.shell.drawn_at(Panel::Properties).is_none());
    assert!(w.bottom_folded());
    let _ = h.get_by_label("Show Level");
}

#[test]
fn the_window_resized_keeps_the_panels_in_proportion() {
    let dragged = || {
        let mut w = Workbench::new(info());
        w.shell
            .sizes
            .set(RibbonTab::Home, PanelLayout::Standard, Edge::Left, 0.25);
        w
    };
    let small = window(1200.0, dragged());
    let large = window(1800.0, dragged());
    let share = |h: &Harness, width: f32| (body(h, Panel::Level).width() + 1.0) / width;
    let (a, b) = (share(&small, 1200.0), share(&large, 1800.0));
    assert!((a - b).abs() < 0.01, "{a} and {b}");
    assert!((a - 0.25).abs() < 0.01, "{a}");
}

#[test]
fn placing_starts_from_the_library_in_every_layout() {
    for layout in [
        PanelLayout::Standard,
        PanelLayout::LibraryLeft,
        PanelLayout::LibraryRight,
    ] {
        let mut h = window(1440.0, in_layout(layout));
        h.get_by_label("Place").click();
        h.run();
        h.run();
        let w = &h.state().0;
        assert!(w.placing(), "{layout:?}");
        assert!(w.shell.drawn_at(Panel::Placement).is_some(), "{layout:?}");
        h.get_by_label("Place").click();
        h.run();
        assert!(!h.state().0.placing(), "{layout:?}");
    }
}

#[test]
fn too_narrow_the_right_column_goes_then_the_left_and_they_come_back() {
    let least = 2.0 * size::COLUMN_MIN_W + size::CENTER_MIN_W + 2.0;
    let h = window(least - 20.0, Workbench::new(info()));
    let w = &h.state().0;
    assert!(w.shell.drawn_at(Panel::Properties).is_none());
    assert!(w.shell.drawn_at(Panel::Level).is_some());
    let h = window(1440.0, Workbench::new(info()));
    assert!(h.state().0.shell.drawn_at(Panel::Properties).is_some());
}
