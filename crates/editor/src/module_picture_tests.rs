//! Modules' approved pictures (R5, board 07): the Make module dialog, a
//! copy's Properties with a setting of its own, and a module's own page.
//! Each is the sample project drawn for real at 1440 × 900 with the lamp
//! post known and placed, cropped to the part that changed.

use crate::interface_look::InterfaceLook;
use crate::panel::Panel;
use crate::picture_support::{drawn, editor_window, matches_approved, sample_project};
use crate::properties::PropertyRequest;
use crate::test_support::lamp_post_file;
use crate::workbench::Workbench;
use reflect::{EntityId, Value};

const SIZE: [f32; 2] = [1440.0, 900.0];

/// The sample project with the lamp post known, one copy placed and
/// picked, its lamp brighter than the module's.
fn with_a_changed_copy() -> (Workbench, reflect::AssetId, EntityId) {
    let mut w = sample_project();
    let (asset, file) = lamp_post_file();
    let mut items: Vec<crate::library::LibraryItem> = w.open.library.iter().cloned().collect();
    items.push(crate::library::LibraryItem {
        id: Some(asset),
        kind: crate::library::LibraryKind::Module,
        name: "Lamp post".into(),
        path: "props/Lamp post.module.lst".into(),
        thumbnail: None,
    });
    w.set_library(items);
    w.set_module(asset, file);
    w.place_module(asset);
    let root = w.selection().expect("the copy is picked");
    let lamp = level::module::derive(root, &[EntityId(reflect::Id::from_raw(2).unwrap())]);
    w.apply_property(
        PropertyRequest::SetField {
            entity: lamp,
            component: "Light".into(),
            field: "strength".into(),
            value: Value::Float(9.0),
        },
        1,
    );
    w.select(lamp);
    (w, asset, lamp)
}

/// Draws `w` (after `act`, between frames) and crops to what `crop` finds.
fn picture_of(
    mut w: Workbench,
    act: impl FnOnce(&mut Workbench),
    crop: impl FnOnce(&Workbench) -> egui::Rect,
) -> Option<crate::picture_support::Picture> {
    act(&mut w);
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
fn the_make_module_dialog() {
    let (w, ..) = with_a_changed_copy();
    let Some(picture) = picture_of(
        w,
        |w| {
            let post = w.add_entity();
            w.select(post);
            w.ask_make_module();
        },
        |_| egui::Rect::from_center_size(egui::pos2(720.0, 450.0), egui::vec2(500.0, 260.0)),
    ) else {
        return;
    };
    matches_approved("make-module", &picture);
}

#[test]
fn a_copy_s_properties_with_a_setting_of_its_own() {
    let (w, ..) = with_a_changed_copy();
    let Some(picture) = picture_of(
        w,
        |_| {},
        |w| {
            let right = w.shell.drawn_at(Panel::Properties).expect("Properties");
            right.with_max_y(right.top() + 420.0)
        },
    ) else {
        return;
    };
    matches_approved("module-copy-properties", &picture);
}

#[test]
fn a_module_s_own_page() {
    let (w, asset, _) = with_a_changed_copy();
    let Some(picture) = picture_of(
        w,
        |w| w.open_module_page(asset),
        |w| {
            let level = w.shell.drawn_at(Panel::Level).expect("the Level panel");
            let view = w.shell.drawn_at(Panel::ModulePage).expect("the page");
            egui::Rect::from_min_max(
                level.min - egui::vec2(0.0, 40.0),
                egui::pos2(view.right(), view.top() + 260.0),
            )
        },
    ) else {
        return;
    };
    matches_approved("module-page", &picture);
}
