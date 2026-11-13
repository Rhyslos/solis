//! Home's panel layouts used as a person uses them (decision 0109): where
//! each list is drawn in each layout, every border dragged, put back with
//! a double-click and remembered per layout across a restart, the pointer
//! over a border, a layout changed at once, placing, and the lists going
//! back on the shelf when the window is too narrow.

use super::sizes::Edge;
use super::workspaces::ShelfTab;
use super::{PanelLayout, RibbonTab, ShellState};
use crate::panel::Panel;
use crate::test_support::{State, double_click, fonts_ready, in_layout};
use crate::tokens::size;
use crate::workbench::Workbench;
use egui::{Pos2, Rect, vec2};
use egui_kittest::kittest::Queryable;

type Harness = egui_kittest::Harness<'static, State>;

const SIDE: [PanelLayout; 3] = [
    PanelLayout::AreaLeftAssetsBottom,
    PanelLayout::AreaAndAssetsLeft,
    PanelLayout::AreaLeftAssetsRight,
];

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

/// The middle of the border called `name`.
fn border(h: &Harness, name: &str) -> Pos2 {
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

#[test]
fn each_side_layout_puts_the_lists_round_the_area_and_the_shelf_keeps_the_rest() {
    for layout in SIDE {
        let h = window(1440.0, in_layout(layout));
        let (area, properties, assets) = (
            body(&h, Panel::Area),
            body(&h, Panel::Properties),
            body(&h, Panel::Assets),
        );
        let view = body(&h, Panel::Viewport);
        // The Area list at the left of the area, Properties at its right.
        assert!(area.right() < view.left(), "{layout:?}");
        assert!(properties.left() > view.right(), "{layout:?}");
        match layout {
            PanelLayout::AreaLeftAssetsBottom => {
                assert!(assets.top() > view.bottom());
                assert!(assets.left() > area.right() && assets.right() < properties.left());
            }
            PanelLayout::AreaAndAssetsLeft => {
                assert_eq!(assets.left(), area.left());
                assert!(assets.top() > area.bottom());
            }
            _ => {
                assert_eq!(assets.right(), properties.right());
                assert!(assets.top() > properties.bottom());
            }
        }
        // Tall: the columns start where the groups do and reach down to
        // the shelf, past the area.
        assert_eq!(area.top(), view.top(), "{layout:?}");
        // (Properties draws its own header, in its body.)
        assert_eq!(
            properties.top() + size::COLUMN_HEADER_H,
            view.top(),
            "{layout:?}"
        );
        let lowest = [area, properties, assets].map(|r| r.bottom());
        assert!(lowest.iter().all(|b| *b > view.top()), "{layout:?}");
        assert!(lowest.iter().any(|b| *b >= view.bottom()), "{layout:?}");
        // The shelf has no Entity tab; its first after Place is in front.
        let w = &h.state().0;
        assert_eq!(w.shelf_tab(), Some(ShelfTab::Console));
        assert!(h.query_by_label("Entity").is_none(), "{layout:?}");
        let _ = h.get_by_label("Place");
    }
}

#[test]
fn every_side_border_drags_is_held_to_its_least_and_a_double_click_puts_it_back() {
    let cases = [
        (
            PanelLayout::AreaLeftAssetsBottom,
            "Left column's edge",
            Edge::Left,
            vec2(120.0, 0.0),
        ),
        (
            PanelLayout::AreaLeftAssetsBottom,
            "Right column's edge",
            Edge::Right,
            vec2(-120.0, 0.0),
        ),
        (
            PanelLayout::AreaLeftAssetsBottom,
            "Assets strip's edge",
            Edge::Strip,
            vec2(0.0, -80.0),
        ),
        (
            PanelLayout::AreaAndAssetsLeft,
            "Between Area and Assets",
            Edge::LeftShare,
            vec2(0.0, 80.0),
        ),
        (
            PanelLayout::AreaLeftAssetsRight,
            "Between Properties and Assets",
            Edge::RightShare,
            vec2(0.0, -80.0),
        ),
    ];
    for (layout, name, edge, by) in cases {
        let mut h = window(1440.0, in_layout(layout));
        let standard = size_of(&h, edge);
        let line = border(&h, name);
        drag(&mut h, line, by);
        let dragged = size_of(&h, edge);
        assert_ne!(dragged, standard, "{name}");
        let moved = border(&h, name) - line;
        assert!((moved - by).length() <= 2.0, "{name}: {moved:?}");
        // Far past its room: held so everything keeps its least.
        let line = border(&h, name);
        drag(&mut h, line, by * 40.0);
        assert!(
            body(&h, Panel::Viewport).width() >= size::CENTER_MIN_W - 1.0,
            "{name}"
        );
        assert!(body(&h, Panel::Viewport).height() + size::DOC_TAB_H >= size::CENTER_MIN_H - 1.0);
        for panel in [Panel::Area, Panel::Properties] {
            assert!(
                body(&h, panel).width() >= size::COLUMN_MIN_W - 1.0,
                "{name}"
            );
        }
        // A double-click: its standard again, and forgotten.
        let line = border(&h, name);
        double_click(&mut h, line);
        assert_eq!(size_of(&h, edge), standard, "{name}");
        assert_eq!(h.state().0.shell.sizes.kept().count(), 0, "{name}");
    }
}

#[test]
fn the_pointer_shows_a_resize_arrow_over_a_border() {
    let mut h = window(1440.0, in_layout(PanelLayout::AreaLeftAssetsBottom));
    let cursor = |h: &Harness| h.output().platform_output.cursor_icon;
    for (name, icon) in [
        ("Left column's edge", egui::CursorIcon::ResizeHorizontal),
        ("Assets strip's edge", egui::CursorIcon::ResizeVertical),
        ("Shelf's top edge", egui::CursorIcon::ResizeVertical),
    ] {
        h.hover_at(border(&h, name));
        h.run();
        assert_eq!(cursor(&h), icon, "{name}");
    }
    h.hover_at(body(&h, Panel::Area).center());
    h.run();
    assert_ne!(cursor(&h), egui::CursorIcon::ResizeHorizontal);
}

#[test]
fn the_shelf_s_columns_drag_too() {
    let mut h = window(1440.0, in_layout(PanelLayout::Shelf));
    let area = body(&h, Panel::Area).width();
    let at = border(&h, "Between Area and Properties");
    drag(&mut h, at, vec2(100.0, 0.0));
    assert!((body(&h, Panel::Area).width() - (area + 100.0)).abs() <= 2.0);
    let assets = body(&h, Panel::Assets).width();
    let at = border(&h, "Between Properties and Assets");
    drag(&mut h, at, vec2(-60.0, 0.0));
    assert!((body(&h, Panel::Assets).width() - (assets + 60.0)).abs() <= 2.0);
    let edge = Edge::Column(ShelfTab::Entity, 0);
    assert!((size_of(&h, edge) - (size::SCENE_COLUMN_W + 100.0)).abs() <= 2.0);
    let at = border(&h, "Between Area and Properties");
    double_click(&mut h, at);
    assert_eq!(body(&h, Panel::Area).width(), size::SCENE_COLUMN_W);
}

#[test]
fn sizes_are_kept_per_layout_and_across_a_restart() {
    let mut h = window(1440.0, in_layout(PanelLayout::AreaLeftAssetsBottom));
    let at = border(&h, "Left column's edge");
    drag(&mut h, at, vec2(100.0, 0.0));
    let wide = body(&h, Panel::Area).width();
    // Another layout has its own; coming back finds this one's.
    let choose = |h: &mut Harness, layout| {
        let ctx = h.ctx.clone();
        let w = &mut h.state_mut().0;
        let look = crate::interface_look::InterfaceLook {
            layout,
            ..w.interface_look()
        };
        w.choose_interface_look(&ctx, look);
        h.run();
        h.run();
    };
    choose(&mut h, PanelLayout::AreaLeftAssetsRight);
    assert_eq!(body(&h, Panel::Area).width(), size::SCENE_COLUMN_W);
    choose(&mut h, PanelLayout::AreaLeftAssetsBottom);
    assert_eq!(body(&h, Panel::Area).width(), wide);
    // A later run, through the saved text.
    let saved = ron::to_string(&h.state().0.shell_state()).unwrap();
    let back: ShellState = ron::from_str(&saved).unwrap();
    let mut w = in_layout(PanelLayout::AreaLeftAssetsBottom);
    w.restore_shell(Some(&back), false);
    let h = window(1440.0, w);
    assert_eq!(body(&h, Panel::Area).width(), wide);
}

#[test]
fn a_layout_chosen_in_preferences_shows_at_once() {
    let mut h = window(1440.0, in_layout(PanelLayout::Shelf));
    assert_eq!(h.state().0.shelf_tab(), Some(ShelfTab::Entity));
    let shelf_top = body(&h, Panel::Area).top();
    crate::test_support::file_item(&mut h, "Preferences");
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Appearance")
        .click();
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Panel layout")
        .click();
    h.run();
    h.get_by_label(PanelLayout::AreaAndAssetsLeft.name())
        .click();
    h.run();
    assert_eq!(
        h.state().0.interface_look().layout,
        PanelLayout::AreaAndAssetsLeft
    );
    h.state_mut().0.show_tab(RibbonTab::Home);
    h.run();
    h.run();
    let area = body(&h, Panel::Area);
    assert!(area.top() < shelf_top, "a tall column now");
    assert_eq!(body(&h, Panel::Assets).left(), area.left());
}

#[test]
fn placing_works_in_every_layout() {
    for layout in PanelLayout::ALL {
        let mut w = in_layout(layout);
        w.set_project(Some(crate::library::ProjectInfo {
            name: "Farm".into(),
            root: "/projects/farm".into(),
            assets: "/projects/farm/assets".into(),
        }));
        let mut h = window(1440.0, w);
        h.get_by_label("Place").click();
        h.run();
        h.run();
        let w = &h.state().0;
        assert!(w.placing(), "{layout:?}");
        for panel in [Panel::Placement, Panel::Assets, Panel::Area] {
            assert!(w.shell.drawn_at(panel).is_some(), "{layout:?} {panel:?}");
        }
        // The Assets list in its placing form, wherever it is.
        let _ = h.get_by_label("Open Library");
        // Leaving Place for the Console ends placing.
        h.get_by_label("Console").click();
        h.run();
        assert!(!h.state().0.placing(), "{layout:?}");
    }
}

#[test]
fn too_narrow_for_the_columns_the_lists_go_back_on_the_shelf_and_return() {
    let layout = PanelLayout::AreaLeftAssetsBottom;
    let least = super::layouts::least_width(layout);
    // Just wide enough: columns, at their least.
    let h = window(least + 20.0, in_layout(layout));
    assert!(body(&h, Panel::Area).right() < body(&h, Panel::Viewport).left());
    assert!(h.query_by_label("Entity").is_none());
    // Narrower: on the shelf's Entity tab, under the area.
    let mut h = window(least - 20.0, in_layout(layout));
    h.get_by_label("Entity").click();
    h.run();
    h.run();
    assert!(body(&h, Panel::Area).top() > body(&h, Panel::Viewport).bottom());
    // Wider again: beside it.
    h.set_size(vec2(1440.0, 900.0));
    h.run();
    h.run();
    h.run();
    assert!(body(&h, Panel::Area).right() < body(&h, Panel::Viewport).left());
    // Showing Properties from anywhere still finds it.
    h.state_mut().0.show_panel(Panel::Properties);
    h.run();
    assert!(h.state().0.shell.drawn_at(Panel::Properties).is_some());
}

#[test]
fn other_tabs_look_the_same_in_every_layout() {
    for tab in [RibbonTab::Library, RibbonTab::Audio, RibbonTab::Debug] {
        let rects: Vec<Vec<(Panel, Rect)>> = PanelLayout::ALL
            .into_iter()
            .map(|layout| {
                let mut w = in_layout(layout);
                w.show_tab(tab);
                let h = window(1440.0, w);
                h.state().0.shell.drawn.clone()
            })
            .collect();
        for drawn in &rects[1..] {
            assert_eq!(drawn, &rects[0], "{tab:?}");
        }
    }
}
