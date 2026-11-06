//! The canvas (decision 0089): picking, box picking, moving, sizing and
//! keys, on a screen drawn from the game's code.

use super::*;
use crate::test_support::Picker;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

struct State {
    doc: ScreenDoc,
    picker: Picker,
    fonts: ui::Fonts,
    out: Vec<CanvasOut>,
}

fn harness() -> Harness<'static, State> {
    let mut w = crate::workbench_screens::tests::opened();
    w.refresh_screen();
    let doc = w.screen_doc.take().unwrap();
    Harness::builder()
        .with_size(egui::vec2(1000.0, 600.0))
        .build_ui_state(
            |ui, s: &mut State| {
                let out = canvas_tab(
                    ui,
                    CanvasUi {
                        doc: &mut s.doc,
                        renderer: &mut s.picker,
                        fonts: &mut s.fonts,
                        pictures: None,
                        accent: Color32::from_rgb(0x24, 0x68, 0xd8),
                        picture_files: &[],
                        readable: &[],
                    },
                );
                s.out.push(out);
            },
            State {
                doc,
                picker: Picker::default(),
                fonts: crate::game_screens::editor_fonts(),
                out: Vec::new(),
            },
        )
}

/// Where the piece at file place `file` is drawn, in points.
fn rect_of(h: &Harness<'static, State>, file: &[usize]) -> Rect {
    let s = h.state();
    let full = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 600.0));
    let area = Rect::from_min_max(Pos2::new(0.0, 30.0), full.max);
    let fit = ((area.width() - 40.0) / 1920.0).min((area.height() - 40.0) / 1080.0);
    let rect = Rect::from_center_size(
        area.center() + s.doc.pan,
        Vec2::new(1920.0, 1080.0) * s.doc.zoom.unwrap_or(fit),
    );
    let size_px = [rect.width().round() as u32, rect.height().round() as u32];
    let view = s.doc.view.clone().unwrap();
    let screen = ui::Screen {
        design: ui::view::design_of(&view),
        view: Some(view),
        ..ui::Screen::new("x", [1920.0, 1080.0], ui::ScaleRule::Grow)
    };
    let mut fonts = crate::game_screens::editor_fonts();
    let seen = s
        .doc
        .matches()
        .into_iter()
        .find(|(f, _)| f == file)
        .unwrap()
        .1;
    let (_, r) = ui::layout::boxes(&screen, size_px, None, &mut fonts)
        .into_iter()
        .find(|(p, _)| *p == seen)
        .unwrap();
    Rect::from_min_max(
        rect.min + Vec2::new(r[0], r[1]),
        rect.min + Vec2::new(r[2], r[3]),
    )
}

fn changed(h: &Harness<'static, State>) -> bool {
    h.state().out.iter().any(|o| o.changed)
}

#[test]
fn a_click_picks_the_deepest_piece_and_shift_adds_one() {
    let mut h = harness();
    h.run();
    let quit = rect_of(&h, &[0, 0, 2]);
    h.hover_at(quit.center());
    h.event(egui::Event::PointerButton {
        pos: quit.center(),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: quit.center(),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    assert_eq!(h.state().doc.picked, vec![vec![0, 0, 2]]);
    let title = rect_of(&h, &[0, 0, 0]);
    h.hover_at(title.center());
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: title.center(),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::SHIFT,
        });
    }
    h.run();
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    assert_eq!(h.state().doc.picked.len(), 2);
    // A locked piece can't be picked: its column is.
    h.state_mut().doc.locked.insert(vec![0, 0, 2]);
    h.state_mut().doc.picked.clear();
    h.hover_at(quit.center());
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: quit.center(),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
    assert_eq!(h.state().doc.picked, vec![vec![0, 0]]);
}

/// Drags with the left button from `from` to `to`.
fn drag(h: &mut Harness<'static, State>, from: Pos2, to: Pos2) {
    h.hover_at(from);
    h.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    for t in 1..=5 {
        h.hover_at(from + (to - from) * (t as f32 / 5.0));
        h.run();
    }
    h.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
}

#[test]
fn dragging_a_piece_puts_it_among_its_neighbours_and_its_handle_sizes_it() {
    let mut h = harness();
    h.run();
    // Quit dragged above Paused: first in the column.
    let quit = rect_of(&h, &[0, 0, 2]);
    let title = rect_of(&h, &[0, 0, 0]);
    drag(
        &mut h,
        quit.center(),
        Pos2::new(title.center().x, title.top() + 2.0),
    );
    assert!(changed(&h));
    let text = h.state().doc.text.clone();
    assert!(
        text.find("button \"Quit\";").unwrap() < text.find("text \"Paused\"").unwrap(),
        "{text}"
    );
    assert_eq!(h.state().doc.picked, vec![vec![0, 0, 0]], "still picked");
    // Undo, then size Resume by its right handle.
    assert!(h.state_mut().doc.undo());
    h.state_mut().doc.behind = false;
    h.state_mut().doc.picked = vec![vec![0, 0, 1]];
    h.run();
    let resume = rect_of(&h, &[0, 0, 1]);
    let handle = Pos2::new(resume.right(), resume.center().y);
    let per_point = 1080.0 / (600.0 - 30.0 - 40.0_f32).min((1000.0 - 40.0) / 1920.0 * 1080.0);
    drag(&mut h, handle, handle + Vec2::new(40.0, 0.0));
    let doc = &h.state().doc;
    let piece = doc.piece(&[0, 0, 1]).unwrap();
    let w: f32 = piece
        .style_text(&doc.text, "width")
        .unwrap()
        .parse()
        .unwrap();
    let expected = (resume.width() + 40.0) * per_point;
    assert!((w - expected).abs() < 14.0, "width {w}, about {expected}");
    assert!(
        doc.text.contains("button \"Resume\" [blue, width"),
        "{}",
        doc.text
    );
}

#[test]
fn a_box_picks_what_s_inside_it_and_keys_delete_and_copy() {
    let mut h = harness();
    h.run();
    let column = rect_of(&h, &[0, 0]);
    let resume = rect_of(&h, &[0, 0, 1]);
    let quit = rect_of(&h, &[0, 0, 2]);
    // From outside the screen's pieces (left of the column), round
    // Resume and Quit.
    let from = Pos2::new(column.left() - 30.0, resume.top() - 3.0);
    let to = Pos2::new(column.right() + 30.0, quit.bottom() + 3.0);
    // The stack fills the screen, so start the box on its empty part:
    // lock it first so the press isn't a move.
    h.state_mut().doc.locked.insert(vec![0]);
    h.state_mut().doc.locked.insert(vec![0, 0]);
    drag(&mut h, from, to);
    let mut picked = h.state().doc.picked.clone();
    picked.sort();
    assert_eq!(picked, vec![vec![0, 0, 1], vec![0, 0, 2]]);
    h.hover_at(quit.center());
    h.key_press(egui::Key::Delete);
    h.run();
    assert!(changed(&h));
    assert!(!h.state().doc.text.contains("Resume"));
    assert!(!h.state().doc.text.contains("Quit"));
    assert!(h.state_mut().doc.undo());
    h.state_mut().doc.picked = vec![vec![0, 0, 0]];
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::D);
    h.run();
    assert_eq!(h.state().doc.text.matches("text \"Paused\"").count(), 2);
}

#[test]
fn the_toolbar_picks_a_size_and_the_wheel_zooms() {
    let mut h = harness();
    h.run();
    assert_eq!(h.state().doc.zoom, None, "fits at first");
    h.hover_at(Pos2::new(500.0, 300.0));
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(0.0, 120.0),
        modifiers: egui::Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    });
    h.run();
    h.run();
    assert!(
        h.state().doc.zoom.is_some_and(|z| z > 0.49),
        "{:?}",
        h.state().doc.zoom
    );
    h.get_by_label_contains("%").click();
    h.run();
    assert_eq!(h.state().doc.zoom, None, "back to fit");
    // The drawn picture's size follows the preview size.
    h.state_mut().doc.preset = 3; // 1280 × 720
    h.run();
    let last = h.state().picker.views.last().unwrap();
    assert_eq!(last.image, CANVAS_IMAGE);
    let [w, hgt] = last.size_px;
    assert!((w as f32 / hgt as f32 - 16.0 / 9.0).abs() < 0.02);
}

/// Presses `key` with the pointer over the canvas.
fn press(h: &mut Harness<'static, State>, key: egui::Key) {
    h.hover_at(Pos2::new(500.0, 320.0));
    h.run();
    h.key_press(key);
    h.run();
}

#[test]
fn keys_pick_tools_and_a_dragged_box_goes_among_the_pieces() {
    use crate::canvas_tools::CanvasTool;
    let mut h = harness();
    h.run();
    press(&mut h, egui::Key::Num2);
    assert_eq!(h.state().doc.tool, CanvasTool::Box);
    // Over the column, below Quit: a box goes into the column.
    let quit = rect_of(&h, &[0, 0, 2]);
    let from = quit.center_bottom() + Vec2::new(-20.0, 4.0);
    drag(&mut h, from, from + Vec2::new(60.0, 30.0));
    assert!(changed(&h));
    let doc = &h.state().doc;
    assert!(doc.text.contains("box [width "), "{}", doc.text);
    assert_eq!(doc.picked.len(), 1);
    assert_eq!(doc.picked[0][..2], [0, 0], "in the column");
    // Esc goes back to picking.
    press(&mut h, egui::Key::Escape);
    assert_eq!(h.state().doc.tool, CanvasTool::Select);
}

#[test]
fn a_click_with_text_puts_words_and_the_ring_picks_a_tool() {
    use crate::canvas_tools::CanvasTool;
    let mut h = harness();
    h.run();
    press(&mut h, egui::Key::Num6);
    assert_eq!(h.state().doc.tool, CanvasTool::Text);
    let at = rect_of(&h, &[0, 0, 0]).center_bottom() + Vec2::new(0.0, 2.0);
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
    assert!(
        h.state().doc.text.contains("text \"Text\" [size 24];"),
        "{}",
        h.state().doc.text
    );
    // E opens the ring at the pointer; a click on Box picks it, and puts
    // nothing on the canvas.
    let before = h.state().doc.text.clone();
    let c = Pos2::new(500.0, 320.0);
    press(&mut h, egui::Key::E);
    let n = CanvasTool::ALL.len() as f32;
    let a = std::f32::consts::TAU * (1.0 / n) - std::f32::consts::FRAC_PI_2;
    let box_at = c + Vec2::new(a.cos(), a.sin()) * 52.0;
    h.hover_at(box_at);
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: box_at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
    h.run();
    assert_eq!(h.state().doc.tool, CanvasTool::Box);
    assert_eq!(h.state().doc.text, before, "the ring's click puts nothing");
}

#[test]
fn the_pen_s_corners_become_lines_on_enter() {
    let mut h = harness();
    h.run();
    press(&mut h, egui::Key::Num5);
    for p in [
        Pos2::new(120.0, 100.0),
        Pos2::new(200.0, 140.0),
        Pos2::new(260.0, 90.0),
    ] {
        h.hover_at(p);
        h.run();
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run();
        // Not a double-click.
        std::thread::sleep(std::time::Duration::from_millis(1));
        h.run();
    }
    assert_eq!(h.state().doc.pen.len(), 3);
    h.key_press(egui::Key::Enter);
    h.run();
    let text = &h.state().doc.text;
    assert_eq!(text.matches("line [").count(), 2, "{text}");
    assert!(text.contains("canvas [width"), "{text}");
    assert!(h.state().doc.pen.is_empty());
}
