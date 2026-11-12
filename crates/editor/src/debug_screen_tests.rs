//! The Debug tab as round 3's `09-debug-profiler.png` draws it (M14):
//! the Profiler fills the middle, a click on a bar of the frame-time
//! graph picks that frame (the project lead's answer 25), and the
//! frame shown is the one picked while it is kept.

use crate::Workbench;
use crate::panel::Panel;
use crate::profiler::{KEPT, Profiler, Stage};
use crate::shell::RibbonTab;
use crate::test_support::*;
use crate::viewport::DrawReport;
use egui_kittest::kittest::Queryable;
use std::time::Duration;

/// A profiler that has kept `frames` frames, the n-th taking n ms of
/// game code.
fn kept(frames: usize) -> Profiler {
    let mut p = Profiler::default();
    for n in 1..=frames {
        p.begin();
        p.add(Stage::GameCode, Duration::from_millis(n as u64));
        p.end(DrawReport::default(), 0, 0);
    }
    p
}

#[test]
fn the_frame_shown_is_the_one_picked_while_it_is_kept() {
    let mut p = kept(3);
    assert_eq!(p.number_of(0), 1);
    assert_eq!(p.shown().map(|(n, _)| n), Some(3), "the newest");
    p.picked = Some(2);
    let (n, frame) = p.shown().unwrap();
    assert_eq!(n, 2);
    assert_eq!(frame.cpu[Stage::GameCode as usize], 2.0);
    // Once it is no longer kept, the newest is shown again.
    for _ in 0..KEPT {
        p.begin();
        p.end(DrawReport::default(), 0, 0);
    }
    assert_eq!(p.shown().map(|(n, _)| n), Some(3 + KEPT as u64));
    assert!(Profiler::default().shown().is_none());
}

#[test]
fn the_profiler_fills_debug_s_middle_and_a_click_on_a_bar_picks_its_frame() {
    let mut w = Workbench::new(info());
    w.show_tab(RibbonTab::Debug);
    let mut h = harness(w);
    for _ in 0..4 {
        h.run();
    }
    assert!(in_view(&h.state().0, Panel::Profiler));
    let graph = h.get_by_label("Frame-time graph").rect();
    // The newest frame stands at the graph's right edge.
    click(
        &mut h,
        egui::pos2(graph.right() - 1.0, graph.bottom() - 1.0),
        egui::Modifiers::NONE,
    );
    let p = &h.state().0.profiler;
    let picked = p.picked.expect("a frame is picked");
    assert!(picked >= 1);
    // A click left of every kept frame picks nothing new.
    click(
        &mut h,
        egui::pos2(graph.left() + 1.0, graph.bottom() - 1.0),
        egui::Modifiers::NONE,
    );
    assert_eq!(h.state().0.profiler.picked, Some(picked));
}
