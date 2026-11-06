//! The canvas's drawing tools (decision 0089, M5).

use super::*;
use crate::screen_doc::tests::MENU;

fn doc() -> ScreenDoc {
    ScreenDoc::new("ui/pauseMenu.lst".into(), "PauseMenu".into(), MENU.into())
}

#[test]
fn numbers_are_whole_when_they_can_be() {
    assert_eq!(num(12.0), "12");
    assert_eq!(num(12.345), "12.35");
    assert_eq!(num(-3.0), "-3");
    assert_eq!(num(0.004), "0");
}

#[test]
fn each_tool_writes_its_piece_in_a_canvas_or_among_others() {
    let (a, b) = ([10.0, 20.0], [110.0, 70.0]);
    assert_eq!(
        piece_text(CanvasTool::Box, a, b, true, None).unwrap(),
        "box [x 10, y 20, width 100, height 50];"
    );
    assert_eq!(
        piece_text(CanvasTool::Box, b, a, false, None).unwrap(),
        "canvas [width 100, height 50] {\n    box [width 100, height 50];\n}",
        "dragged either way; outside a canvas, in one of its own"
    );
    assert_eq!(
        piece_text(CanvasTool::Ellipse, a, b, false, None).unwrap(),
        "canvas [width 50, height 50] {\n    circle [x 25, y 25, radius 25];\n}"
    );
    assert_eq!(
        piece_text(CanvasTool::Box, [10.4, 20.6], [110.2, 70.7], true, None).unwrap(),
        "box [x 10, y 21, width 100, height 50];",
        "whole pixels"
    );
    assert_eq!(
        piece_text(CanvasTool::Ellipse, a, b, true, None).unwrap(),
        "circle [x 60, y 45, radius 25];"
    );
    assert_eq!(
        piece_text(CanvasTool::Line, a, b, true, None).unwrap(),
        "line [x 10, y 20, toX 110, toY 70];"
    );
    let line = piece_text(CanvasTool::Line, a, b, false, None).unwrap();
    assert!(
        line.starts_with("canvas [width 100, height 50] {"),
        "{line}"
    );
    assert!(line.contains("line [x 0, y 0, toX 100, toY 50];"), "{line}");
    assert_eq!(
        piece_text(CanvasTool::Text, a, a, true, None).unwrap(),
        "text \"Text\" [x 10, y 20, size 24];"
    );
    assert_eq!(
        piece_text(CanvasTool::Text, a, a, false, None).unwrap(),
        "text \"Text\" [size 24];"
    );
    assert_eq!(
        piece_text(CanvasTool::Image, a, a, false, Some("art/coin.png")).unwrap(),
        "image \"art/coin.png\" [width 128];"
    );
    assert_eq!(
        piece_text(CanvasTool::Button, a, a, false, None).unwrap(),
        "button \"Button\" {\n}"
    );
    assert_eq!(
        piece_text(CanvasTool::Button, a, a, true, None).unwrap(),
        "button \"Button\" [x 10, y 20] {\n}"
    );
}

#[test]
fn every_piece_written_is_lust_a_screen_can_run() {
    let (a, b) = ([10.0, 20.0], [110.0, 70.0]);
    for tool in CanvasTool::ALL {
        for on_canvas in [false, true] {
            let Some(t) = piece_text(tool, a, b, on_canvas, Some("art/coin.png"))
                .or_else(|| pen_text(&[a, b, [5.0, 5.0]], on_canvas))
            else {
                continue;
            };
            let body = if on_canvas {
                format!("canvas [width 400, height 400] {{\n{t}\n}}")
            } else {
                format!("column {{\n{t}\n}}")
            };
            let file = format!(
                "fetch \"solis/screen\":Ui;\nassembly A {{\n    view {{\n{body}\n    }}\n}}\npass A;\nend\n"
            );
            let code = script::Code {
                files: vec![
                    ("main.lst".into(), "end\n".into()),
                    ("ui/a.lst".into(), file.clone()),
                ],
                lua: Vec::new(),
                notes: Vec::new(),
            };
            if let Err(p) = script::compile(&code) {
                panic!("{tool:?} on_canvas {on_canvas}: {}\n{file}", p[0]);
            }
        }
    }
}

#[test]
fn too_small_or_nothing_to_put_writes_nothing() {
    let a = [10.0, 10.0];
    for tool in [CanvasTool::Box, CanvasTool::Ellipse, CanvasTool::Line] {
        assert_eq!(
            piece_text(tool, a, [11.0, 11.0], true, None),
            None,
            "{tool:?}"
        );
    }
    assert_eq!(
        piece_text(CanvasTool::Image, a, a, true, None),
        None,
        "no picture"
    );
    assert_eq!(piece_text(CanvasTool::Select, a, a, true, None), None);
    assert_eq!(piece_text(CanvasTool::Pen, a, a, true, None), None);
    assert_eq!(pen_text(&[a], true), None, "one corner is no line");
    assert_eq!(pen_text(&[], false), None);
}

#[test]
fn a_picture_s_name_is_quoted_safely() {
    let t = piece_text(
        CanvasTool::Image,
        [0.0; 2],
        [0.0; 2],
        false,
        Some("a \"b\".png"),
    )
    .unwrap();
    assert!(
        ui::source::parse(&format!("assembly A {{ view {{ {t} }} }}\npass A;\nend\n")).is_ok(),
        "{t}"
    );
}

#[test]
fn the_pen_makes_a_line_per_corner_to_corner() {
    let pts = [[0.0, 0.0], [50.0, 10.0], [80.0, 60.0]];
    assert_eq!(
        pen_text(&pts, true).unwrap(),
        "line [x 0, y 0, toX 50, toY 10];\nline [x 50, y 10, toX 80, toY 60];"
    );
    let pts = [[100.0, 200.0], [150.0, 210.0]];
    let t = pen_text(&pts, false).unwrap();
    assert!(t.starts_with("canvas [width 50, height 10] {"), "{t}");
    assert!(t.contains("    line [x 0, y 0, toX 50, toY 10];"), "{t}");
}

#[test]
fn every_tool_has_a_name_a_hint_and_keys_one_to_seven() {
    let keys: Vec<_> = CanvasTool::ALL.iter().filter_map(|t| t.key()).collect();
    assert_eq!(keys.len(), 7);
    let mut seen = keys.clone();
    seen.dedup();
    assert_eq!(seen.len(), 7, "no two the same");
    for t in CanvasTool::ALL {
        assert!(!t.name().is_empty() && !t.hint().is_empty());
    }
    assert!(CanvasTool::Box.drags() && !CanvasTool::Text.drags());
}

#[test]
fn a_piece_put_in_is_one_undo_step_and_picked() {
    let mut d = doc();
    // Into the column, second.
    assert!(d.insert(&[0, 0], 1, "box [width 40, height 40];"));
    assert_eq!(d.picked, vec![vec![0, 0, 1]]);
    assert!(d.text.contains("box [width 40, height 40];"));
    let col = d
        .block()
        .unwrap()
        .at(&[0, 0])
        .unwrap()
        .children
        .clone()
        .unwrap();
    assert_eq!(col.pieces.len(), 5, "four and the box");
    assert!(d.undo());
    assert_eq!(d.text, MENU);
    // Nowhere: a place that isn't there.
    assert!(!d.insert(&[9], 0, "box;"));
    // Past the end goes last.
    assert!(d.insert(&[0, 0], 99, "text \"End\";"));
    assert_eq!(d.picked, vec![vec![0, 0, 4]]);
    // Something unreadable is refused.
    assert!(!d.insert(&[0, 0], 0, "box [width"));
}
