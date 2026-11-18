//! The ground's approved pictures (Tools 1b, G1): the New ground dialog,
//! and a ground's card in Properties. The sample project drawn for real
//! at 1440 × 900, cropped to the part shown. (Picture tests draw no 3D
//! view, so the ground in the view is checked by the renderer's GPU
//! tests instead: `crates/render/tests/gpu.rs`.)

use crate::interface_look::InterfaceLook;
use crate::panel::Panel;
use crate::picture_support::{drawn, editor_window, matches_approved, sample_project};
use crate::workbench::Workbench;

const SIZE: [f32; 2] = [1440.0, 900.0];

fn picture_of(
    w: Workbench,
    crop: impl FnOnce(&Workbench) -> egui::Rect,
) -> Option<crate::picture_support::Picture> {
    let mut h = editor_window(w, InterfaceLook::default(), SIZE)?;
    for _ in 0..4 {
        h.run();
    }
    let rect = crop(h.state());
    let picture = drawn(&mut h);
    let min = [rect.left().max(0.0) as u32, rect.top().max(0.0) as u32];
    let size = [rect.width() as u32, rect.height() as u32];
    Some(picture.cropped(min, size))
}

#[test]
fn the_new_ground_dialog() {
    let mut w = sample_project();
    w.ask_new_ground("");
    let Some(picture) = picture_of(w, |_| {
        egui::Rect::from_center_size(egui::pos2(720.0, 450.0), egui::vec2(500.0, 320.0))
    }) else {
        return;
    };
    matches_approved("ground-new", &picture);
}

#[test]
fn a_ground_in_properties() {
    let mut w = sample_project();
    let asset = reflect::AssetId(reflect::Id::from_raw(0x6a0d).unwrap());
    let mut items: Vec<crate::library::LibraryItem> = w.open.library.to_vec();
    items.push(crate::library::LibraryItem {
        id: Some(asset),
        kind: crate::library::LibraryKind::Ground,
        name: "Ground".into(),
        path: "Ground.ground.lst".into(),
        thumbnail: None,
    });
    w.set_library(items);
    std::sync::Arc::make_mut(&mut w.viewport.sheets)
        .paths
        .insert(asset, "Ground.ground.lst".into());
    let data = level::EntityData::new("Ground").with(&level::builtin::Ground {
        ground: Some(reflect::AssetRef { asset, part: None }),
        solid: true,
    });
    let id = w.spawn_selected(data);
    w.select(id);
    let Some(picture) = picture_of(w, |w| {
        let right = w.shell.drawn_at(Panel::Properties).expect("Properties");
        right.with_max_y(right.top() + 260.0)
    }) else {
        return;
    };
    matches_approved("ground-properties", &picture);
}
