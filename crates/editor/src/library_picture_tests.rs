//! The Library's approved pictures (R4, board 04): the right-click menu
//! on empty space, a name being renamed, and two folder trees side by
//! side on Home's left. Each is the sample project drawn for real at
//! 1440 × 900 and cropped to the part that changed, so it stays small.
//! The browser itself on both tabs is in `tab-library.png` and the
//! editor's own pictures (`shell/middle_picture_tests.rs`).

use crate::interface_look::InterfaceLook;
use crate::library::LeftShows;
use crate::panel::Panel;
use crate::picture_support::{drawn, editor_window, matches_approved, sample_project};
use crate::shell::RibbonTab;
use crate::workbench::Workbench;

const SIZE: [f32; 2] = [1440.0, 900.0];

/// Draws `workbench` with `act` done to it after two frames, and gives
/// the picture cropped to the rectangle `crop` finds.
fn picture_of(
    workbench: Workbench,
    act: impl FnOnce(&mut egui_kittest::Harness<'static, Workbench>),
    crop: impl FnOnce(&Workbench) -> egui::Rect,
) -> Option<crate::picture_support::Picture> {
    let mut h = editor_window(workbench, InterfaceLook::default(), SIZE)?;
    h.run();
    h.run();
    act(&mut h);
    h.run();
    h.run();
    let rect = crop(h.state());
    let picture = drawn(&mut h);
    let min = [rect.left().max(0.0) as u32, rect.top().max(0.0) as u32];
    let size = [rect.width() as u32, rect.height() as u32];
    Some(picture.cropped(min, size))
}

#[test]
fn the_right_click_menu_on_empty_space() {
    let mut w = sample_project();
    w.show_tab(RibbonTab::Library);
    let Some(picture) = picture_of(
        w,
        |h| {
            let body = h
                .state()
                .shell
                .drawn_at(Panel::Library)
                .expect("the browser");
            let at = egui::pos2(body.center().x, body.bottom() - 330.0);
            h.hover_at(at);
            h.run();
            for pressed in [true, false] {
                h.event(egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
                h.run();
            }
        },
        |w| {
            let body = w.shell.drawn_at(Panel::Library).expect("the browser");
            egui::Rect::from_min_max(
                egui::pos2(body.center().x - 20.0, body.bottom() - 350.0),
                egui::pos2(body.center().x + 260.0, body.bottom()),
            )
        },
    ) else {
        return;
    };
    matches_approved("library-menu", &picture);
}

#[test]
fn a_name_being_renamed() {
    let mut w = sample_project();
    w.show_tab(RibbonTab::Library);
    w.open.library_state.pick("notes.md");
    w.open.library_state.start_renaming("notes.md", "notes.md");
    let Some(picture) = picture_of(
        w,
        |_| {},
        |w| {
            let body = w.shell.drawn_at(Panel::Library).expect("the browser");
            egui::Rect::from_min_size(body.min, egui::vec2(body.width(), 260.0))
        },
    ) else {
        return;
    };
    matches_approved("library-rename", &picture);
}

#[test]
fn two_folder_trees_side_by_side() {
    let mut w = sample_project();
    w.open.library_state.left = LeftShows::Folders;
    w.open.library_state.trees.list[0]
        .open
        .insert("props".into());
    w.open.library_state.trees.open_another(0);
    w.open.library_state.trees.list[1].top = "props".into();
    let Some(picture) = picture_of(
        w,
        |_| {},
        |w| {
            let trees = w.shell.drawn_at(Panel::Folders).expect("the trees");
            // With the slot's header, its toggle.
            egui::Rect::from_min_max(
                trees.min - egui::vec2(0.0, crate::tokens::size::COLUMN_HEADER_H),
                egui::pos2(trees.right(), trees.top() + 360.0),
            )
        },
    ) else {
        return;
    };
    matches_approved("folder-trees", &picture);
}
