//! A project's kind in approved pictures (decision 0118, R6; board 08):
//! a 2D project's Home with a tile map and a sprite, Project settings'
//! Kind row, and a 3D project's Home, each drawn for real at 1440 × 900
//! and cropped to what the kind changes; and Both, drawn as the editor
//! was before R6 (the approved `editor` picture, unchanged).

use crate::file_pages::Tab;
use crate::interface_look::InterfaceLook;
use crate::launcher::ProjectKind;
use crate::picture_support::{Picture, drawn, editor_window, matches_approved, sample_project};
use crate::workbench::Workbench;

const SIZE: [f32; 2] = [1440.0, 900.0];

/// A 2D project as it opens for the first time, a tile map placed and a
/// sprite picked, the 2D view at 200 %.
fn farm_2d() -> Workbench {
    let (mut w, map, _) = crate::workbench_tiles::tests::with_map();
    w.place_tile_map(map, None).unwrap();
    let sprite = w.add_sprite();
    w.set_project_kind(ProjectKind::TwoD);
    w.viewport.camera.zoom = 2.0;
    w.select(sprite);
    w
}

/// The look of a computer that has opened a 2D or 3D project once, so
/// Home's note shows.
fn first_time() -> InterfaceLook {
    InterfaceLook {
        kind_note_seen: 1,
        ..InterfaceLook::default()
    }
}

/// Draws `w` and crops the band from its top down to `bottom`.
fn top_of(w: Workbench, bottom: f32) -> Option<Picture> {
    let mut h = editor_window(w, first_time(), SIZE)?;
    for _ in 0..4 {
        h.run();
    }
    let picture = drawn(&mut h);
    Some(picture.cropped([0, 0], [SIZE[0] as u32, bottom as u32]))
}

#[test]
fn a_2d_project_s_home() {
    // The tab bar, the ribbon with the note, and the document tabs (board
    // 08's top): the Level panel's rows below come in random order.
    let Some(picture) = top_of(farm_2d(), 128.0) else {
        return;
    };
    matches_approved("kind-home-2d", &picture);
}

#[test]
fn a_3d_project_s_home() {
    let mut w = sample_project();
    w.set_project_kind(ProjectKind::ThreeD);
    let Some(picture) = top_of(w, 128.0) else {
        return;
    };
    matches_approved("kind-home-3d", &picture);
}

#[test]
fn project_settings_kind_row() {
    let mut w = farm_2d();
    w.show_settings(Tab::Project);
    let Some(mut h) = editor_window(w, first_time(), SIZE) else {
        return;
    };
    for _ in 0..4 {
        h.run();
    }
    let picture = drawn(&mut h);
    // The page's head, its sub-tabs and the Identity section's first
    // rows: the name and Kind with what 2D hides.
    matches_approved(
        "kind-project-settings",
        &picture.cropped([0, 40], [1440, 400]),
    );
}

#[test]
fn both_draws_home_as_before() {
    let mut w = sample_project();
    w.set_project_kind(ProjectKind::Both);
    let Some(mut h) = editor_window(w, InterfaceLook::default(), SIZE) else {
        return;
    };
    h.run();
    h.run();
    matches_approved("editor", &drawn(&mut h));
}

#[test]
fn the_2d_view_s_pills_in_a_narrow_view() {
    // Shared 1f's loose ends: the app's least window with the Library at
    // the right, so the 2D view is narrower than its pills' full words.
    // They take the board's short words and wrap onto a second row,
    // inside the view, with the tool bar above them.
    let w = farm_2d();
    let look = InterfaceLook {
        layout: crate::shell::PanelLayout::LibraryRight,
        ..first_time()
    };
    let Some(mut h) = editor_window(w, look, [960.0, 600.0]) else {
        return;
    };
    for _ in 0..4 {
        h.run();
    }
    let view = h.state().viewport.rect;
    let picture = drawn(&mut h).cropped(
        [view.left() as u32, view.top() as u32],
        [view.width() as u32, view.height() as u32],
    );
    matches_approved("pixel-pills-narrow", &picture);
}
