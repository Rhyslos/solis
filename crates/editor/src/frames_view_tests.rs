//! A clips file's shelf tabs: the time ruler's marks.

use super::*;
use animation::{Clip, ClipFrame, Direction};

/// A clip of frames lasting `ms` each, played `direction`.
fn clip_of(ms: &[u32], direction: Direction) -> Clip {
    let frames = ms
        .iter()
        .enumerate()
        .map(|(i, &ms)| ClipFrame {
            frame: i as u32,
            ms,
            events: Vec::new(),
        })
        .collect();
    Clip::new("walk", frames, direction, 0, 1.0, false)
}

/// How wide a frame's tile is at `zoom`, as the ruler has it.
fn width_at(zoom: f32) -> impl Fn(u32) -> f32 {
    move |ms: u32| (ms as f32 / 1000.0 * zoom).clamp(TILE_WIDTH.0, TILE_WIDTH.1)
}

/// The ruler's marks for `clip` at `zoom`, from 0, with `seen` on screen.
fn marks(clip: &Clip, zoom: f32, seen: egui::Rangef) -> Vec<Mark> {
    let order: Vec<usize> = clip.pass_order().collect();
    ruler_marks(&order, clip, &width_at(zoom), 0.0, seen)
}

#[test]
fn the_ruler_marks_every_50_ms_and_writes_every_100() {
    // As it always has, wherever tiles are wide enough: a frame of 100,
    // 150 and 70 ms at the standard zoom and the smallest.
    for zoom in [
        crate::frames_editor::STANDARD_ZOOM,
        crate::frames_editor::ZOOM_RANGE.0,
    ] {
        let clip = clip_of(&[100, 150, 70], Direction::Forward);
        let width_of = width_at(zoom);
        let order: Vec<usize> = clip.pass_order().collect();
        let all = egui::Rangef::new(-1e6, 1e6);
        let expected: Vec<Mark> = (0..=320 / 50)
            .map(|k| k * 50)
            .map(|ms| Mark {
                x: x_of_ms(&order, &clip, &width_of, 0.0, ms),
                ms,
                time: ms % 100 == 0,
            })
            .collect();
        assert_eq!(marks(&clip, zoom, all), expected, "zoom {zoom}");
    }
    // A pass ending on a mark has its mark at the last tile's edge.
    let clip = clip_of(&[100, 100], Direction::Forward);
    let all = marks(&clip, 1000.0, egui::Rangef::new(-1e6, 1e6));
    let last = all.last().unwrap();
    assert_eq!((last.ms, last.time), (200, true));
    assert_eq!(last.x, 100.0 + 4.0 + 100.0);
}

/// BUG-21: a mark every 50 ms over the whole pass, each placed by walking
/// every frame before it, whatever was on screen. A frame may last a
/// minute and a clip have 1,024 frames: 1.2 million marks a frame.
#[test]
fn a_long_clip_has_marks_only_where_seen_and_a_few_points_apart() {
    // 100 frames of 10 s: 20,000 marks in 60,400 points.
    let clip = clip_of(&[10_000; 100], Direction::Forward);
    let seen = egui::Rangef::new(1_000.0, 2_500.0);
    let shown = marks(&clip, 1000.0, seen);
    assert!(!shown.is_empty());
    assert!(
        shown.len() <= (seen.span() / MARK_GAP) as usize + 1,
        "{} marks in {} points",
        shown.len(),
        seen.span()
    );
    assert!(shown.iter().all(|m| seen.contains(m.x)));
    for pair in shown.windows(2) {
        assert!(pair[1].x - pair[0].x >= MARK_GAP - 0.01, "{pair:?}");
    }
    // Times are written far enough apart to read.
    let times: Vec<&Mark> = shown.iter().filter(|m| m.time).collect();
    assert!(!times.is_empty());
    for pair in times.windows(2) {
        assert!(pair[1].x - pair[0].x >= TIME_GAP - 0.01, "{pair:?}");
    }
    // Each where its time falls, as the playhead is placed.
    let width_of = width_at(1000.0);
    let order: Vec<usize> = clip.pass_order().collect();
    for m in &shown {
        assert!((m.x - x_of_ms(&order, &clip, &width_of, 0.0, m.ms)).abs() < 0.01);
    }
}

#[test]
fn the_longest_clip_the_file_allows_has_a_screenful_of_marks() {
    // 1,024 frames of a minute each, there and back: 2,046 tiles.
    let clip = clip_of(&[60_000; 1_024], Direction::PingPong);
    let width = 1_920.0;
    for start in [0.0, 300_000.0, 1_235_000.0] {
        let seen = egui::Rangef::new(start, start + width);
        let shown = marks(&clip, crate::frames_editor::ZOOM_RANGE.0, seen);
        assert!(
            shown.len() <= (width / MARK_GAP) as usize + 1,
            "{}",
            shown.len()
        );
        assert!(shown.iter().all(|m| seen.contains(m.x)));
    }
    // Nothing in view: no marks.
    assert!(marks(&clip, 1000.0, egui::Rangef::new(-500.0, -100.0)).is_empty());
}

#[test]
fn a_clip_with_no_frames_has_its_start_marked() {
    let clip = clip_of(&[], Direction::Forward);
    let all = marks(&clip, 1000.0, egui::Rangef::new(-1e6, 1e6));
    assert_eq!(
        all,
        [Mark {
            x: -4.0,
            ms: 0,
            time: true
        }]
    );
}
