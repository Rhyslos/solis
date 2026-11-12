//! The Library as the design's third screen draws it (milestone M7):
//! the kinds' colours, the ribbon and what a double-click does there.

use crate::library::{LibraryItem, LibraryKind, ProjectInfo};
use crate::picture_support::{drawn, matches_approved, window};
use crate::shell::RibbonTab;
use crate::test_support::*;
use crate::theme::kind;
use crate::tokens::{self, size, text};
use crate::workbench::{Workbench, WorkbenchAction};
use egui_kittest::kittest::Queryable;

#[test]
fn the_kinds_colours_are_option_a_with_three_of_today_s() {
    use LibraryKind as K;
    assert_eq!(K::Model.color(), tokens::KIND_MODEL);
    assert_eq!(K::Assembly.color(), tokens::KIND_ASSEMBLY);
    assert_eq!(K::Texture.color(), tokens::KIND_PICTURE);
    assert_eq!(K::SpriteSheet.color(), tokens::KIND_PICTURE);
    assert_eq!(K::SpriteClips.color(), tokens::KIND_ANIMATION);
    assert_eq!(K::Animator.color(), tokens::KIND_ANIMATION);
    assert_eq!(K::Sound.color(), tokens::KIND_SOUND);
    assert_eq!(K::TileMap.color(), tokens::KIND_MATERIAL);
    // Option B's (today's) for these three.
    assert_eq!(
        K::SoundEvent.color(),
        egui::Color32::from_rgb(0x5d, 0xca, 0xa5)
    );
    assert_eq!(K::Code.color(), egui::Color32::from_rgb(0xe8, 0x87, 0x5a));
    assert_eq!(K::Area.color(), egui::Color32::from_rgb(0x58, 0xc2, 0x6a));
    assert_eq!(K::Other.color(), kind::OTHER);
}

/// Every kind of file with its colour and name: the sheet the project
/// lead checks (answer 21), an approved picture.
#[test]
fn the_kinds_colour_sheet() {
    let rows = LibraryKind::ALL.len();
    let row_h = size::CHIP_H;
    let height = size::PANEL_PAD * 2.0 + rows as f32 * row_h;
    let Some(mut h) = window([260.0, height], 1.0, |ui| {
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, tokens::panel());
        for (i, kind) in LibraryKind::ALL.into_iter().enumerate() {
            let top = size::PANEL_PAD + i as f32 * row_h;
            let dot = egui::pos2(size::PANEL_PAD + size::KIND_DOT, top + row_h / 2.0);
            ui.painter()
                .circle_filled(dot, size::KIND_DOT / 2.0, kind.color());
            ui.painter().text(
                dot + egui::vec2(size::KIND_DOT + size::GAP, 0.0),
                egui::Align2::LEFT_CENTER,
                kind.name(),
                text::ui(),
                kind.color(),
            );
        }
    }) else {
        return;
    };
    h.run();
    h.run();
    let picture = drawn(&mut h);
    let first = size::PANEL_PAD + size::KIND_DOT;
    assert_eq!(
        picture.at([first, size::PANEL_PAD + row_h / 2.0], 1.0),
        LibraryKind::ALL[0].color()
    );
    matches_approved("kind-colours", &picture);
}

fn with_a_model() -> Workbench {
    let mut w = Workbench::new(info());
    w.set_project(Some(ProjectInfo {
        name: "Harbour".into(),
        root: "/projects/harbour".into(),
        assets: "/projects/harbour/assets".into(),
    }));
    w.set_library(vec![LibraryItem {
        id: Some(reflect::AssetId(reflect::Id::random())),
        kind: LibraryKind::Model,
        name: "crate".into(),
        path: "crate.glb".into(),
        thumbnail: None,
    }]);
    w
}

#[test]
fn a_double_click_in_the_library_tab_only_selects_and_place_places() {
    let mut w = with_a_model();
    w.show_tab(RibbonTab::Library);
    let mut h = harness(w);
    h.run();
    h.run();
    h.get_all_by_label("crate").next().unwrap().click();
    h.run();
    let at = h.get_all_by_label("crate").next().unwrap().rect().center();
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    let placed = |h: &egui_kittest::Harness<'_, State>| {
        h.state()
            .1
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::PlaceAsset(_)))
    };
    assert!(!placed(&h), "a double-click selects only");
    assert_eq!(
        h.state().0.open.library_state.selected.as_deref(),
        Some("crate.glb")
    );
    run(&mut h, "Place the picked file");
    h.run();
    assert!(placed(&h));
}

#[test]
fn the_shelf_bar_counts_the_folder_s_files() {
    let mut w = with_a_model();
    w.show_tab(RibbonTab::Library);
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(h.query_by_label("1 items \u{b7} 0 selected").is_some());
}

#[test]
fn the_ribbon_makes_files_and_sets_the_view() {
    let mut w = with_a_model();
    w.show_tab(RibbonTab::Library);
    let mut h = harness(w);
    h.run();
    h.run();
    h.get_by_label("List").click();
    h.run();
    assert!(h.state().0.open.library_state.list);
    h.get_by_label("Type: All").click();
    h.run();
    h.get_by_label("Models").click();
    h.run();
    assert_eq!(
        h.state().0.open.library_state.filter,
        crate::library::Filter::Models
    );
    h.get_by_label("New in the Library: Folder").click();
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::NewFolder("".into()))
    );
}
