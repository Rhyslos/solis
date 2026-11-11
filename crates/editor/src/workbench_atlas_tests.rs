//! The Atlas Crop tab's workbench side (decision 0089).

use crate::library::{LibraryKind, LibraryRequest};
use crate::panel::Panel;
use crate::sprite_sheets::{ArtState, AtlasState};
use crate::test_support::*;
use crate::workbench::{Workbench, WorkbenchAction};
use std::sync::Arc;

fn with_project() -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    w
}

fn saved(w: &Workbench) -> Vec<(String, String)> {
    w.open
        .outbox
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::SaveAtlas { path, text } => {
                Some((path.to_string_lossy().replace('\\', "/"), text.clone()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn atlas_crop_makes_the_file_beside_the_picture_and_saves_each_change() {
    let mut w = with_project();
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::AtlasCrop("sprites/tools.png".into()),
        &mut actions,
    );
    assert!(
        w.dock.find_tab(&Panel::AtlasCrop).is_some(),
        "the tab opened"
    );
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
    assert!(w.dock.find_tab(&Panel::AtlasCrop).is_none());
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
    w.open.library.clear();
    let (_, problem) = w.atlas_picture();
    assert!(problem.unwrap().contains("hero.aseprite"));
    // Changed by hand: taken in, one undo step.
    atlas.grid = [8, 8];
    Arc::make_mut(&mut w.viewport.sheets).atlases.insert(
        "hero.atlas.lst".into(),
        AtlasState::Read(Arc::new(atlas.clone()), Vec::new()),
    );
    w.atlas_read_again();
    assert_eq!(w.open.atlas_editor.as_ref().unwrap().atlas.grid, [8, 8]);
    // Gone: the tab closes.
    Arc::make_mut(&mut w.viewport.sheets).atlases.clear();
    w.atlas_read_again();
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
    assert!(w.dock.find_tab(&Panel::AtlasCrop).is_none());
}
