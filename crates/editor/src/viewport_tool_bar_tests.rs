//! The tool bar floating at the foot of the view (decision 0114): where
//! it goes and how it shrinks, its buttons clicked and lit by the keys,
//! Points, Edges and Faces while a brush is edited, Show › Tools, and
//! nothing it meets overlapped at the narrowest views.

use super::{Placed, place};
use crate::brushing::edit::Mode;
use crate::interface_look::InterfaceLook;
use crate::panel::Panel;
use crate::shell::PanelLayout;
use crate::test_support::*;
use crate::tokens::size;
use crate::viewport::{Overlays, Tool, ViewLayout};
use crate::workbench::Workbench;
use egui::{Key, Rect, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable};

type H = egui_kittest::Harness<'static, State>;

const TOOLS: [&str; 5] = [
    "Select (Q)",
    "Move (W)",
    "Rotate (R)",
    "Scale (S)",
    "Transform (T)",
];
const MODES: [&str; 3] = ["Points (1)", "Edges (2)", "Faces (3)"];

/// A view 860 wide and 360 tall, and the bottom-right pills' place in
/// it, about as wide as Lock X Y Z, World and the camera's speed.
fn view_and_corner() -> (Rect, Rect) {
    let view = Rect::from_min_size(pos2(240.0, 130.0), vec2(860.0, 360.0));
    let corner = Rect::from_min_max(
        pos2(view.right() - 8.0 - 226.0, view.bottom() - 32.0),
        pos2(view.right() - 8.0, view.bottom() - 8.0),
    );
    (view, corner)
}

#[test]
fn the_bar_is_centred_at_the_view_s_foot_at_the_mock_up_s_size() {
    let (view, corner) = view_and_corner();
    let Placed { frame, button } = place(view, Some(corner), 5, 0);
    assert_eq!(button, size::TOOL_BAR_BUTTON);
    assert_eq!(frame.center().x, view.center().x);
    assert_eq!(frame.bottom(), view.bottom() - size::TOOL_BAR_MARGIN);
    // Five 30 px buttons a point apart, 4 round them: the mock-up's 163
    // by 39, near enough.
    assert_eq!(frame.size(), vec2(162.0, 38.0));
    // With the line and three more, still centred and clear.
    let Placed { frame, .. } = place(view, Some(corner), 8, 3);
    assert_eq!(frame.center().x, view.center().x);
    assert_eq!(frame.width(), 8.0 * 30.0 + 6.0 + 21.0 + 8.0);
    assert!(frame.right() < corner.left());
}

#[test]
fn the_bar_moves_left_of_the_corner_then_shrinks_then_goes_above_it() {
    let (view, _) = view_and_corner();
    let pills = |left: f32| {
        view.with_min_x(view.left() + left)
            .with_min_y(view.bottom() - 32.0)
            .with_max_y(view.bottom() - 8.0)
    };
    // A wide corner: centred, the bar would meet it, so it moves left.
    let corner = pills(view.width() / 2.0);
    let Placed { frame, button } = place(view, Some(corner), 5, 0);
    assert_eq!(button, size::TOOL_BAR_BUTTON);
    assert!(frame.right() <= corner.left() - size::PILL_GAP);
    assert!(frame.left() >= view.left() + size::PILL_MARGIN);
    assert_eq!(frame.bottom(), view.bottom() - size::TOOL_BAR_MARGIN);
    // A shorter row: its buttons shrink to stay on it.
    let corner = pills(150.0);
    let Placed { frame, button } = place(view, Some(corner), 5, 0);
    assert!((size::TOOL_BAR_LEAST_BUTTON..size::TOOL_BAR_BUTTON).contains(&button));
    assert!(frame.left() >= view.left() + size::PILL_MARGIN - 0.01);
    assert!(frame.right() <= corner.left() - size::PILL_GAP + 0.01);
    assert_eq!(frame.bottom(), view.bottom() - size::TOOL_BAR_MARGIN);
    // No room on the row even then: above the corner, full size and
    // centred again.
    let corner = pills(60.0);
    let Placed { frame, button } = place(view, Some(corner), 5, 0);
    assert_eq!(button, size::TOOL_BAR_BUTTON);
    assert_eq!(frame.bottom(), corner.top() - size::PILL_GAP);
    assert_eq!(frame.center().x, view.center().x);
    // A view narrower than the bar: its buttons shrink to fit it.
    let narrow = Rect::from_min_size(view.min, vec2(size::CENTER_MIN_W, 300.0));
    let Placed { frame, button } = place(narrow, None, 8, 3);
    assert!(button < size::TOOL_BAR_BUTTON);
    assert!(frame.left() >= narrow.left() + size::PILL_MARGIN - 0.01);
    assert!(frame.right() <= narrow.right() - size::PILL_MARGIN + 0.01);
    // A view too short to go above the pills: on the row, from the left,
    // never over the pills at the top (it can't miss both).
    let short = Rect::from_min_size(view.min, vec2(300.0, 80.0));
    let corner = short
        .with_min_x(short.left() + 60.0)
        .with_min_y(short.bottom() - 32.0)
        .with_max_y(short.bottom() - 8.0);
    let Placed { frame, button } = place(short, Some(corner), 8, 3);
    assert_eq!(button, size::TOOL_BAR_BUTTON);
    assert_eq!(frame.left(), short.left() + size::PILL_MARGIN);
    assert_eq!(frame.bottom(), short.bottom() - size::TOOL_BAR_MARGIN);
    // Nothing but numbers, however small.
    let tiny = Rect::from_min_size(view.min, vec2(30.0, 30.0));
    let Placed { frame, button } = place(tiny, Some(tiny), 8, 3);
    assert_eq!(button, size::TOOL_BAR_SMALLEST_BUTTON);
    assert!(frame.is_finite());
    let nothing = place(Rect::NOTHING, None, 5, 0);
    assert!(!nothing.button.is_nan());
}

/// The rect of the widget labelled `label`.
fn at(h: &H, label: &str) -> Rect {
    h.get_by_label(label).rect()
}

/// Whether the button labelled `label` is lit.
fn lit(h: &H, label: &str) -> bool {
    h.get_by_label(label).accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
}

/// The rect round every tool button.
fn bar(h: &H) -> Rect {
    TOOLS
        .into_iter()
        .chain(MODES)
        .filter_map(|label| h.query_by_label(label).map(|n| n.rect()))
        .fold(Rect::NOTHING, |all, r| all.union(r))
}

#[test]
fn in_the_editor_the_bar_floats_at_the_view_s_foot_in_place_of_the_column() {
    let (w, _) = with_a_crate();
    let mut h = harness(w);
    h.run();
    h.run();
    let viewport = h.state().0.shell.drawn_at(Panel::Viewport).unwrap();
    // The column went: the view fills the panel.
    assert_eq!(h.state().0.viewport.full, viewport);
    let view = h.state().0.viewport.rect;
    let mut left = view.left();
    for name in TOOLS {
        let tool = at(&h, name);
        assert_eq!(tool.size(), vec2(30.0, 30.0), "{name}");
        assert!(tool.left() > left, "{name}: right of the one before");
        left = tool.left();
        assert!(tool.bottom() < view.bottom() && tool.top() > view.center().y);
    }
    let all = bar(&h);
    assert!((all.center().x - view.center().x).abs() < 1.0, "centred");
    assert!(h.query_by_label("Points (1)").is_none(), "no brush edited");
    assert!(!all.intersects(at(&h, "Lock X")), "clear of the corner");
}

#[test]
fn a_click_on_each_tool_picks_it_and_the_keys_light_it() {
    let (w, _) = with_a_crate();
    let mut h = rest_over(harness(w), Panel::Viewport);
    for (name, tool) in TOOLS.into_iter().zip(Tool::ALL) {
        h.get_by_label(name).click();
        h.run();
        h.run();
        assert_eq!(h.state().0.tool(), tool, "{name}");
        for other in TOOLS {
            assert_eq!(lit(&h, other), other == name, "{other} after {name}");
        }
    }
    let view = h.state().0.viewport.rect;
    h.hover_at(view.center());
    h.run();
    for (key, name) in [Key::W, Key::R, Key::S, Key::T, Key::Q].into_iter().zip([
        "Move (W)",
        "Rotate (R)",
        "Scale (S)",
        "Transform (T)",
        "Select (Q)",
    ]) {
        h.key_press(key);
        h.run();
        h.run();
        assert!(lit(&h, name), "{key:?}");
        assert_eq!(TOOLS.iter().filter(|t| lit(&h, t)).count(), 1);
    }
}

/// An editor with a 2 m box brush picked, its insides being edited.
fn editing() -> H {
    let mut w = Workbench::new(info());
    w.spawn_selected(
        area::EntityData::new("Box_03")
            .with(&area::builtin::Transform::default())
            .with(&area::builtin::Brush::default()),
    );
    let mut h = rest_over(harness(w), Panel::Viewport);
    h.key_press(Key::Tab);
    h.run();
    h.run();
    assert!(h.state().0.viewport.brushing.edit.is_some());
    h
}

fn mode(h: &H) -> Mode {
    h.state().0.viewport.brushing.edit.as_ref().unwrap().mode
}

#[test]
fn while_a_brush_is_edited_points_edges_and_faces_follow_and_a_click_sets_the_mode() {
    let mut h = editing();
    let view = h.state().0.viewport.rect;
    let mut left = at(&h, "Transform (T)").right();
    for name in MODES {
        let button = at(&h, name);
        assert!(button.left() > left, "{name} after the tools");
        left = button.left();
    }
    // The line between them: more room than between two tools.
    let gap = at(&h, "Points (1)").left() - at(&h, "Transform (T)").right();
    assert!(gap > size::TOOL_BAR_DIVIDER[0] - 2.0, "{gap}");
    assert!((bar(&h).center().x - view.center().x).abs() < 1.0);
    assert!(lit(&h, "Faces (3)"), "faces first");
    for (name, m) in MODES.into_iter().zip(Mode::ALL) {
        h.get_by_label(name).click();
        h.run();
        h.run();
        assert_eq!(mode(&h), m, "{name}");
        assert!(lit(&h, name));
        assert!(
            h.state().0.viewport.brushing.edit.is_some(),
            "still editing"
        );
    }
    // The keys light them too.
    h.hover_at(view.center());
    h.run();
    for (key, name) in [(Key::Num2, "Edges (2)"), (Key::Num3, "Faces (3)")] {
        h.key_press(key);
        h.run();
        h.run();
        assert!(lit(&h, name), "{key:?}");
        assert_eq!(MODES.iter().filter(|m| lit(&h, m)).count(), 1);
    }
    // The keys' strip sits just above the bar, and Tab takes them away.
    let strip = h.get_by_label("box pick").rect();
    assert!(strip.bottom() <= bar(&h).top(), "{strip:?} {:?}", bar(&h));
    h.key_press(Key::Tab);
    h.run();
    h.run();
    for name in MODES {
        assert!(h.query_by_label(name).is_none(), "{name} gone");
    }
    assert!(h.query_by_label("Move (W)").is_some());
}

#[test]
fn show_tools_hides_the_bar_and_a_saved_off_stays_off() {
    let mut h = harness(Workbench::new(info()));
    h.run();
    h.get_by_label("Show ▾").click();
    h.run();
    h.get_by_label("Tools").click();
    h.run();
    h.run();
    assert!(!h.state().0.overlays().tools);
    for name in TOOLS {
        assert!(h.query_by_label(name).is_none(), "{name}");
    }
    // Saved off, as the app keeps it (the name `tools` is the one older
    // builds wrote), it stays off.
    let saved: Overlays =
        ron::from_str("(tools:false,snapping:true,stats:true,preview_animation:false)").unwrap();
    assert!(!saved.tools);
    let mut h = harness(Workbench::new(info()).with_overlays(saved));
    h.run();
    h.run();
    assert!(h.query_by_label("Move (W)").is_none());
    // And back on through the same menu.
    h.get_by_label("Show ▾").click();
    h.run();
    h.get_by_label("Tools").click();
    h.run();
    h.run();
    assert!(h.query_by_label("Move (W)").is_some());
    assert_eq!(
        ron::from_str::<Overlays>(&ron::to_string(&h.state().0.overlays()).unwrap()).unwrap(),
        Overlays::default()
    );
}

#[test]
fn with_several_views_the_bar_is_in_the_view_in_use_only() {
    let (mut w, _) = with_a_crate();
    w.viewport.layout = ViewLayout::SideBySide;
    let mut h = harness(w);
    h.run();
    h.run();
    assert_eq!(h.query_all_by_label("Move (W)").count(), 1);
    let shown = {
        let v = &h.state().0.viewport;
        v.shown(v.full)
    };
    let (_, first) = shown[0];
    let (_, second) = shown[1];
    assert!(first.contains_rect(bar(&h)));
    click(&mut h, second.center(), egui::Modifiers::NONE);
    h.run();
    h.run();
    assert!(second.contains_rect(bar(&h)), "it follows the view in use");
}

/// The rects of the pills the bar must not meet, drawn last frame.
fn pills(h: &H) -> Vec<(String, Rect)> {
    ["Lock X", "Show ▾", "Perspective ▾", "box pick"]
        .into_iter()
        .chain(["World ▾", "Object ▾"])
        .flat_map(|label| {
            h.query_all_by_label_contains(label)
                .map(|n| (label.to_owned(), n.rect()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// How tall a view must be for the pills at its top, the bar and the
/// pills at its foot to be three rows.
const THREE_ROWS: f32 = 2.0 * (size::PILL_MARGIN + size::PILL_H + size::PILL_GAP)
    + size::TOOL_BAR_BUTTON
    + 2.0 * size::TOOL_BAR_PAD;

#[test]
fn at_the_narrowest_views_the_layouts_allow_nothing_overlaps() {
    // The standard window and the app's least, each panel layout, each
    // way of splitting the views, with a brush being edited (the widest
    // bar, and the keys' strip over it).
    let mut checked = 0;
    for window in [vec2(1440.0, 900.0), vec2(960.0, 600.0)] {
        for layout in PanelLayout::ALL {
            for views in ViewLayout::ALL {
                let case = format!("{window:?}, {layout:?}, {views:?}");
                let h = narrow_case(window, layout, views);
                let view = h.state().0.viewport.rect;
                let all = bar(&h);
                // Always inside the view, its buttons never past the least.
                assert!(
                    all.left() >= view.left() && all.right() <= view.right(),
                    "{case}: {all:?} in {view:?}"
                );
                assert!(all.bottom() <= view.bottom(), "{case}");
                let button = at(&h, "Move (W)");
                assert!(button.width() >= size::TOOL_BAR_SMALLEST_BUTTON - 0.01);
                // Where three rows fit (every view of the standard window
                // does), it meets nothing. At the least window some views
                // are shorter than that (90 points, or 44 split in four),
                // and something must give.
                if view.height() < THREE_ROWS {
                    assert_ne!(window, vec2(1440.0, 900.0), "{case}");
                    continue;
                }
                checked += 1;
                assert!(all.top() >= view.top(), "{case}");
                for (label, pill) in pills(&h) {
                    let pill = pill.intersect(view);
                    assert!(
                        !pill.is_positive() || !pill.intersects(all.shrink(0.5)),
                        "{case}: the bar {all:?} meets {label} {pill:?}"
                    );
                }
            }
        }
    }
    assert!(checked >= 24, "{checked}");
}

/// A box brush's insides edited in a window of `window` points, Home's
/// lists in `layout` and the views split as `views`, the pointer off
/// the views.
fn narrow_case(window: egui::Vec2, layout: PanelLayout, views: ViewLayout) -> H {
    let look = InterfaceLook {
        layout,
        ..Default::default()
    };
    let mut w = Workbench::new(info()).with_interface_look(look);
    w.spawn_selected(
        area::EntityData::new("Box_03")
            .with(&area::builtin::Transform::default())
            .with(&area::builtin::Brush::default()),
    );
    w.viewport.layout = views;
    let mut h = harness(w);
    h.set_size(window);
    let mut h = rest_over(h, Panel::Viewport);
    h.key_press(Key::Tab);
    h.run();
    // The pointer off the views, so no button's name shows.
    h.hover_at(pos2(2.0, window.y - 2.0));
    h.run();
    h.run();
    assert!(h.state().0.viewport.brushing.edit.is_some());
    h
}

/// The sample project in a test window of `size`, Home's lists in
/// `layout`, with `views` views,
/// and (when `editing`) a box brush's insides being edited; `None` where
/// nothing can draw.
fn drawn_editor(
    size: [f32; 2],
    (layout, views): (PanelLayout, ViewLayout),
    editing: bool,
) -> Option<egui_kittest::Harness<'static, Workbench>> {
    use crate::picture_support::{editor_window, sample_project};
    let mut w = sample_project();
    w.viewport.layout = views;
    if editing {
        w.spawn_selected(
            area::EntityData::new("Box_03")
                .with(&area::builtin::Transform::default())
                .with(&area::builtin::Brush::default()),
        );
    } else {
        w.open.document.selection.clear();
    }
    w.viewport.tool = Tool::Move;
    let look = InterfaceLook {
        layout,
        ..Default::default()
    };
    let mut h = editor_window(w, look, size)?;
    h.run();
    h.run();
    let view = h.state().viewport.rect;
    h.hover_at(view.center());
    h.run();
    if editing {
        h.key_press(Key::Tab);
        h.run();
        assert!(h.state().viewport.brushing.edit.is_some());
    }
    // The pointer off the view, so nothing is lit under it.
    h.hover_at(pos2(2.0, size[1] - 2.0));
    h.run();
    h.run();
    Some(h)
}

/// The foot of the view in use in the frame `h` last drew: from a little
/// above the tool bar (and the keys' strip over it) to the view's foot,
/// the view's whole width.
fn foot_of_the_view(
    h: &mut egui_kittest::Harness<'static, Workbench>,
) -> crate::picture_support::Picture {
    let view = h.state().viewport.rect;
    let picture = crate::picture_support::drawn(h);
    let top = (view.bottom() - 90.0).max(view.top());
    picture.cropped(
        [view.left() as u32, top as u32],
        [view.width() as u32, (view.bottom() - top) as u32],
    )
}

#[test]
fn the_tool_bar_alone_is_its_approved_picture() {
    if let Some(mut h) = drawn_editor(
        [1440.0, 900.0],
        (PanelLayout::default(), ViewLayout::One),
        false,
    ) {
        crate::picture_support::matches_approved("tool-bar", &foot_of_the_view(&mut h));
    }
}

#[test]
fn the_tool_bar_while_a_brush_is_edited_is_its_approved_picture() {
    if let Some(mut h) = drawn_editor(
        [1440.0, 900.0],
        (PanelLayout::default(), ViewLayout::One),
        true,
    ) {
        assert_eq!(
            h.state().viewport.brushing.edit.as_ref().map(|e| e.mode),
            Some(Mode::Faces)
        );
        crate::picture_support::matches_approved("tool-bar-editing", &foot_of_the_view(&mut h));
    }
}

#[test]
fn the_tool_bar_in_the_narrowest_view_is_its_approved_picture() {
    // The app's least window, Properties and Assets at the right, two
    // views side by side, a brush edited: the widest bar in the narrowest
    // view the layouts allow that is tall enough for three rows (180 by
    // 236 points).
    if let Some(mut h) = drawn_editor(
        [960.0, 600.0],
        (PanelLayout::AreaLeftAssetsRight, ViewLayout::SideBySide),
        true,
    ) {
        let view = h.state().viewport.rect;
        let picture = crate::picture_support::drawn(&mut h).cropped(
            [view.left() as u32, view.top() as u32],
            [view.width() as u32, view.height() as u32],
        );
        crate::picture_support::matches_approved("tool-bar-narrow", &picture);
    }
}
