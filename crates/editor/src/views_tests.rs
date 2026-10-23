//! Several views: layouts, which view is in use, flat views, maximising,
//! and saving them.

use super::*;
use crate::test_support::*;
use egui_kittest::kittest::Queryable;

fn wheel(h: &egui_kittest::Harness<'_, State>, notches: f32) {
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, notches),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
}

fn mouse(
    h: &egui_kittest::Harness<'_, State>,
    button: egui::PointerButton,
    pressed: bool,
    at: egui::Pos2,
) {
    h.event(egui::Event::PointerButton {
        pos: at,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    });
}

/// A harness showing four views; returns where each view is.
fn four() -> (egui_kittest::Harness<'static, State>, Vec<(usize, Rect)>) {
    let (mut w, _) = with_a_crate();
    w.viewport.layout = ViewLayout::Four;
    let mut h = harness(w);
    h.run();
    let rects = {
        let v = &h.state().0.viewport;
        v.shown(v.full)
    };
    (h, rects)
}

/// The views drawn in the last frame, by image.
fn drawn(h: &egui_kittest::Harness<'_, State>, count: usize) -> Vec<ViewportView> {
    let views = &h.state().1.views;
    let mut last: Vec<ViewportView> = views[views.len().saturating_sub(count)..].to_vec();
    last.sort_by_key(|v| v.image);
    last
}

#[test]
fn each_layout_splits_the_viewport_without_overlaps() {
    let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), vec2(1000.0, 600.0));
    for (layout, count) in [
        (ViewLayout::One, 1),
        (ViewLayout::SideBySide, 2),
        (ViewLayout::Stacked, 2),
        (ViewLayout::Four, 4),
    ] {
        let views = layout.rects(rect);
        assert_eq!(views.len(), count, "{layout:?}");
        let numbers: Vec<usize> = views.iter().map(|(i, _)| *i).collect();
        assert_eq!(numbers, (0..count).collect::<Vec<_>>());
        for (a, (_, ra)) in views.iter().enumerate() {
            assert!(rect.contains_rect(*ra), "{layout:?}");
            assert!(ra.width() > 100.0 && ra.height() > 100.0);
            for (_, rb) in &views[a + 1..] {
                assert!(!ra.intersects(rb.shrink(0.5)), "{layout:?} overlaps");
            }
        }
        let area: f32 = views.iter().map(|(_, r)| r.area()).sum();
        assert!(area > rect.area() * 0.98, "{layout:?} leaves little out");
    }
}

#[test]
fn four_views_draw_four_images_each_with_its_own_kind() {
    let (h, rects) = four();
    assert_eq!(rects.len(), 4);
    let views = drawn(&h, 4);
    let images: Vec<usize> = views.iter().map(|v| v.image).collect();
    assert_eq!(images, [0, 1, 2, 3]);
    let kinds: Vec<ViewKind> = views.iter().map(|v| v.kind).collect();
    assert_eq!(
        kinds,
        [
            ViewKind::Perspective,
            ViewKind::Top,
            ViewKind::Front,
            ViewKind::Right
        ]
    );
    assert!(views.iter().all(|v| v.editor_lines));
    // Only the view in use has the tool bar.
    assert_eq!(h.query_all_by_label("Move (W)").count(), 1);
    assert_eq!(h.query_all_by_label("Maximise").count(), 4);
}

#[test]
fn a_click_in_a_view_makes_it_the_one_in_use_and_f_frames_there() {
    let (mut h, rects) = four();
    let (_, front) = rects[2];
    let perspective_before = h.state().0.viewport.view_settings(0).unwrap().camera;
    click(&mut h, front.center(), egui::Modifiers::NONE);
    assert_eq!(h.state().0.viewport.active(), 2);
    assert_eq!(h.state().0.camera().kind, ViewKind::Front);
    // The tool bar moved there.
    let bar = h.get_by_label("Move (W)").rect();
    assert!(front.contains(bar.center()), "{bar:?} in {front:?}");
    // F (with the crate selected) frames the view in use only.
    let id = h
        .state()
        .0
        .document
        .area()
        .iter()
        .find(|(_, e)| e.name == "Crate")
        .unwrap()
        .0;
    h.state_mut().0.document.selection.select_only(id);
    h.key_press(egui::Key::F);
    h.run();
    let front_camera = h.state().0.camera();
    assert!(front_camera.flat_height < 5.0, "framed the 1 m crate");
    assert_eq!(
        h.state().0.viewport.view_settings(0).unwrap().camera,
        perspective_before,
        "the other views stay as they were"
    );
}

#[test]
fn a_click_in_a_flat_view_asks_its_own_image_what_is_there() {
    let (mut h, rects) = four();
    let (_, top) = rects[1];
    h.state_mut().1.views.clear();
    click(&mut h, top.center(), egui::Modifiers::NONE);
    h.run();
    let asked = h
        .state()
        .1
        .views
        .iter()
        .find(|v| v.pick.is_some())
        .expect("a pick was asked for");
    assert_eq!(asked.image, 1, "the top view's own image");
    assert_eq!(asked.kind, ViewKind::Top);
}

#[test]
fn a_flat_view_zooms_with_the_wheel_and_slides_with_the_right_button() {
    let (mut h, rects) = four();
    let (_, top) = rects[1];
    let at = top.center() + vec2(40.0, 20.0);
    h.hover_at(at);
    h.run();
    let start = h.state().0.viewport.view_settings(1).unwrap().camera;
    wheel(&h, 2.0);
    h.run();
    assert_eq!(h.state().0.viewport.active(), 1, "the wheel picks the view");
    let zoomed = h.state().0.camera();
    assert!(zoomed.flat_height < start.flat_height);
    // Right drag slides it; no flying in a flat view.
    mouse(&h, egui::PointerButton::Secondary, true, at);
    h.run();
    assert!(!h.state().0.viewport.is_flying());
    h.event(egui::Event::PointerMoved(at + vec2(50.0, 0.0)));
    h.run();
    mouse(
        &h,
        egui::PointerButton::Secondary,
        false,
        at + vec2(50.0, 0.0),
    );
    h.run();
    let slid = h.state().0.camera();
    assert!(
        slid.position.x < zoomed.position.x,
        "the world follows the mouse"
    );
    assert_eq!(slid.kind, ViewKind::Top);
}

#[test]
fn maximise_shows_one_view_and_all_views_brings_them_back() {
    let (mut h, rects) = four();
    let (_, right) = rects[3];
    click(&mut h, right.center(), egui::Modifiers::NONE);
    let maximise = h
        .query_all_by_label("Maximise")
        .find(|n| right.contains(n.rect().center()))
        .unwrap();
    maximise.click();
    h.run();
    h.run();
    assert_eq!(h.state().0.viewport.maximized, Some(3));
    assert_eq!(
        h.state().0.viewport.shown(h.state().0.viewport.full).len(),
        1
    );
    h.state_mut().1.views.clear();
    h.run();
    assert!(h.state().1.views.iter().all(|v| v.image == 3));
    h.get_by_label("All views").click();
    h.run();
    assert_eq!(h.state().0.viewport.maximized, None);
}

#[test]
fn the_menus_choose_the_layout_and_each_views_kind() {
    let (w, _) = with_a_crate();
    let mut h = harness(w);
    h.run();
    // The viewport's ≡ (the panels have their own).
    let full = h.state().0.viewport.full;
    h.query_all_by_label("≡")
        .find(|n| full.contains(n.rect().center()))
        .unwrap()
        .click();
    h.run();
    h.get_by_label("Two side by side").click();
    h.run();
    assert_eq!(h.state().0.viewport.layout, ViewLayout::SideBySide);
    // The first view becomes a flat one from its own menu.
    let menu = h.query_all_by_label("Perspective ▴").next().unwrap();
    menu.click();
    h.run();
    h.get_by_label("Front").click();
    h.run();
    let first = h.state().0.viewport.view_settings(0).unwrap().camera;
    assert_eq!(first.kind, ViewKind::Front);
}

#[test]
fn views_are_saved_and_read_back_and_damaged_ones_are_put_right() {
    let mut v = Viewport {
        layout: ViewLayout::Stacked,
        maximized: Some(1),
        ..Viewport::default()
    };
    v.activate(2);
    v.camera = v.camera.as_kind(ViewKind::Right);
    v.look.kind = crate::look::LookKind::Unlit;
    let saved = v.views();
    assert_eq!(saved.views.len(), VIEWS);
    let text = ron::to_string(&saved).unwrap();
    let back: Views = ron::from_str(&text).unwrap();
    assert_eq!(back, saved);
    let mut w = Viewport::default();
    w.set_views(&back);
    assert_eq!(w.views(), saved, "read back the same");
    assert_eq!(w.active(), 0);
    // An old save, or nothing: one perspective view.
    let empty: Views = ron::from_str("()").unwrap();
    let mut fresh = Viewport::default();
    fresh.set_views(&empty);
    assert_eq!(fresh.layout, ViewLayout::One);
    assert_eq!(fresh.camera.kind, ViewKind::Perspective);
    // Damaged numbers and too many views are put right.
    let mut damaged = saved.clone();
    damaged.views[0].camera.flat_height = f32::NAN;
    damaged.views[0].camera.position = Vec3::splat(f32::INFINITY);
    damaged.views.extend(saved.views.clone());
    damaged.maximized = Some(99);
    let mut fixed = Viewport::default();
    fixed.set_views(&damaged);
    assert!(fixed.camera.position.is_finite());
    assert!(fixed.camera.flat_height.is_finite());
    assert_eq!(fixed.maximized, None);
}

#[test]
fn reading_saved_views_never_panics() {
    // A small fuzz: a real save cut short and corrupted in many ways.
    let text = ron::to_string(&Viewport::default().views()).unwrap();
    let bytes = text.as_bytes();
    let mut seed = 0xFACE_FEEDu64;
    for _ in 0..2_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut broken = bytes.to_vec();
        broken.truncate(1 + (seed as usize) % bytes.len());
        let at = (seed >> 20) as usize % broken.len();
        broken[at] = b"(),:.-0123456789ePerTopFronOn[]x"[(seed >> 40) as usize % 32];
        let Ok(text) = std::str::from_utf8(&broken) else {
            continue;
        };
        if let Ok(views) = ron::from_str::<Views>(text) {
            let mut v = Viewport::default();
            v.set_views(&views);
            assert!(v.camera.view_projection(1.5).is_finite());
        }
    }
}
