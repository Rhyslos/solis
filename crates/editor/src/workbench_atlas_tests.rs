//! The Atlas Crop tab's workbench side (decision 0089).

use crate::library::{LibraryKind, LibraryRequest};
use crate::panel::Panel;
use crate::sprite_sheets::{ArtState, AtlasState};
use crate::test_support::*;
use crate::workbench::Workbench;
use egui_kittest::kittest::Queryable;
use std::sync::Arc;

pub(crate) fn with_project() -> Workbench {
    let mut w = Workbench::new(info());
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    w
}

fn saved(w: &Workbench) -> Vec<(String, String)> {
    saved_files(&w.open.outbox, |f| matches!(f, crate::OwnFile::Atlas(_)))
}

#[test]
fn atlas_crop_makes_the_file_beside_the_picture_and_saves_each_change() {
    let mut w = with_project();
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::AtlasCrop("sprites/tools.png".into()),
        &mut actions,
    );
    assert!(has_panel(&w, Panel::AtlasCrop), "the tab opened");
    let made = saved(&w);
    assert_eq!(made.len(), 1, "made at once, so the picture is read");
    assert_eq!(made[0].0, "sprites/tools.atlas.lst");
    assert!(made[0].1.contains("picture = \"tools.png\""));
    // The picture arrives: 32 × 16, the right half drawn.
    let mut rgba = vec![0u8; 32 * 16 * 4];
    for y in 0..16 {
        for x in 16..32 {
            rgba[(y * 32 + x) * 4 + 3] = 255;
        }
    }
    let art = assets::tile_art::TileArt {
        size: [32, 16],
        frames: vec![rgba],
        durations: vec![100],
    };
    Arc::make_mut(&mut w.viewport.sheets)
        .tile_art
        .insert("sprites/tools.png".into(), ArtState::Read(Arc::new(art)));
    w.atlas_texture_now(&egui::Context::default());
    let (picture, problem) = w.atlas_picture();
    assert!(picture.is_some() && problem.is_none());
    let editor = w.open.atlas_editor.as_mut().unwrap();
    assert_eq!(
        editor.crop_every_tile([32, 16], &|r| r[0] >= 16),
        1,
        "only the drawn tile"
    );
    w.save_open_atlas();
    let last = saved(&w).pop().unwrap();
    assert!(
        last.1
            .contains("name = \"Tile_1_0\", box = [16, 0, 16, 16]"),
        "{}",
        last.1
    );
    // Undo over the tab saves again, without the crop.
    w.atlas_undo(false);
    assert!(!saved(&w).pop().unwrap().1.contains("Tile_1_0"));
    w.close_atlas();
    assert!(!has_panel(&w, Panel::AtlasCrop));
}

#[test]
fn an_atlas_file_opens_its_picture_s_crops_and_a_change_on_disk_is_taken_in() {
    let mut w = with_project();
    let mut atlas = animation::atlas::Atlas::new("hero.aseprite");
    atlas.crops.push(animation::atlas::Crop {
        name: "Face".into(),
        rect: [0, 0, 8, 8],
    });
    Arc::make_mut(&mut w.viewport.sheets).atlases.insert(
        "hero.atlas.lst".into(),
        AtlasState::Read(Arc::new(atlas.clone()), Vec::new()),
    );
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::Open(LibraryKind::Atlas, "hero.atlas.lst".into()),
        &mut actions,
    );
    assert_eq!(w.open.atlas_editor.as_ref().unwrap().atlas, atlas);
    assert!(saved(&w).is_empty(), "nothing new to make");
    // No picture: the tab says why.
    w.open.library = Default::default();
    let (_, problem) = w.atlas_picture();
    assert!(problem.unwrap().contains("hero.aseprite"));
    // Changed by hand: taken in, one undo step.
    atlas.grid = [8, 8];
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.atlases.insert(
        "hero.atlas.lst".into(),
        AtlasState::Read(Arc::new(atlas.clone()), Vec::new()),
    );
    w.set_sprite_sheets(sheets.clone());
    let editor = w.open.atlas_editor.as_mut().unwrap();
    assert_eq!(editor.atlas.grid, [8, 8]);
    assert!(editor.undo(), "one undo step");
    assert!(!editor.undo());
    // Gone: the tab closes.
    sheets.atlases.clear();
    w.set_sprite_sheets(sheets);
    assert!(w.open.atlas_editor.is_none());
}

/// The × on the Atlas Crop tab closes the atlas file.
#[test]
fn the_x_on_its_tab_closes_the_atlas_file() {
    let mut w = with_project();
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::AtlasCrop("sprites/tools.png".into()),
        &mut actions,
    );
    assert!(w.open.atlas_editor.is_some());
    let mut h = harness(w);
    click_the_x(&mut h, Panel::AtlasCrop);
    let w = &h.state().0;
    assert!(w.open.atlas_editor.is_none());
    assert!(!has_panel(w, Panel::AtlasCrop));
}

/// BUG-20: the grid's size dragged for ten frames in the Atlas Crop tab is
/// one undo step and one save, made when the mouse is let go.
#[test]
fn a_dragged_grid_size_is_one_step_and_one_save() {
    let mut w = with_project();
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::AtlasCrop("sprites/tools.png".into()),
        &mut actions,
    );
    let grid = w.open.atlas_editor.as_ref().unwrap().atlas.grid;
    let mut h = harness(w);
    h.run();
    h.run();
    h.state_mut().1.actions.clear();
    let saves = |h: &egui_kittest::Harness<'static, State>| {
        saved_files(&h.state().1.actions, |f| {
            matches!(f, crate::OwnFile::Atlas(_))
        })
        .len()
    };
    let field = number_field_beside(&h, "Grid");
    h.hover_at(field.center());
    h.run();
    h.drag_at(field.center());
    h.run();
    for frame in 1..=10 {
        h.hover_at(field.center() + egui::vec2(5.0, 0.0) * frame as f32);
        h.run();
        assert_eq!(saves(&h), 0, "nothing saved mid-drag");
    }
    h.drop_at(field.center() + egui::vec2(50.0, 0.0));
    h.run();
    assert_eq!(saves(&h), 1, "saved once, when let go");
    let editor = h.state_mut().0.open.atlas_editor.as_mut().unwrap();
    assert!(
        editor.atlas.grid[0] > grid[0] + 20,
        "{:?}",
        editor.atlas.grid
    );
    assert_eq!(editor.atlas.grid[1], grid[1]);
    assert!(editor.undo());
    assert_eq!(editor.atlas.grid, grid);
    assert!(!editor.undo(), "the drag was one step");
}

/// The scale case's picture: 1024 × 1024, cut into 4,096 crops of 16 × 16
/// (the most a file holds), on the Picture tab with its Crops shelf tab.
fn atlas_of_4096_crops() -> Workbench {
    let mut w = with_project();
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::AtlasCrop("sprites/big.png".into()),
        &mut actions,
    );
    let art = assets::tile_art::TileArt {
        size: [1024, 1024],
        frames: vec![vec![255u8; 1024 * 1024 * 4]],
        durations: vec![100],
    };
    Arc::make_mut(&mut w.viewport.sheets)
        .tile_art
        .insert("sprites/big.png".into(), ArtState::Read(Arc::new(art)));
    let editor = w.open.atlas_editor.as_mut().unwrap();
    assert_eq!(editor.crop_every_tile([1024, 1024], &|_| true), 4096);
    editor.pick(Some(2000));
    w
}

/// The second review pass: the Crops list made a row for every crop,
/// every frame. Now only the rows in view are made, and the list is as
/// tall as before, so scrolling reaches the last crop.
#[test]
fn the_crops_list_makes_only_the_rows_in_view() {
    let mut h = harness(atlas_of_4096_crops());
    h.run();
    h.run();
    let first = h.get_by_label("Tile_0_0").rect();
    assert!(
        h.query_by_label("Tile_63_63").is_none(),
        "the last isn't made"
    );
    let made = (0..4096)
        .filter(|i| {
            let name = format!("Tile_{}_{}", i % 64, i / 64);
            h.query_all_by_label(&name).next().is_some()
        })
        .count();
    assert!(made < 40, "{made} rows made");
    // The wheel down to the end (scrolling eases, a frame at a time).
    h.hover_at(first.center());
    for _ in 0..300 {
        if h.query_by_label("Tile_63_63").is_some() {
            break;
        }
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -10_000.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        h.step();
    }
    h.get_by_label("Tile_63_63").click();
    h.step();
    h.step();
    let editor = h.state().0.open.atlas_editor.as_ref().unwrap();
    assert_eq!(
        editor.picked,
        Some(4095),
        "the last row picks the last crop"
    );
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
#[allow(clippy::print_stdout)]
fn scale_check_the_picture_tab_with_4096_crops() {
    // The second review pass: every crop was drawn every frame, in the
    // list and on the picture, seen or not.
    const TARGET_MS: f64 = 8.0;
    let ms = crate::shell::layout_scale_tests::frame_ms(atlas_of_4096_crops());
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: editor frame on the Picture tab, 4,096 crops listed and drawn \
         {ms:.2} ms (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "{ms:.2} ms a frame");
}
