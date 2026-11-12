//! Part C's screens (the approved additions, round 3): the 2D editors
//! drawn in the whole editor, against what each screen must show, and
//! written beside the approved mock-ups to look at.

use crate::interface_look::InterfaceLook;
use crate::picture_support::{Picture, drawn, editor_window, repository};
use crate::shell::RibbonTab;
use crate::tokens;
use crate::workbench::Workbench;
use std::sync::Arc;

/// The design's size.
const SIZE: [f32; 2] = [1440.0, 900.0];

/// A tile map placed and selected, the Brush in hand.
fn tile_map() -> Workbench {
    let (mut w, map, _) = crate::workbench_tiles::tests::with_map();
    w.place_tile_map(map, None).unwrap();
    w.viewport.tile_overlay.paint.tool = Some(crate::tile_paint::PaintTool::Brush);
    w
}

/// A tileset open from the Library.
fn tileset() -> Workbench {
    let (mut w, _, _) = crate::workbench_tileset::tests::with_tileset();
    w.open_tileset("farm.tileset.lst");
    w
}

/// A picture's crops open, its picture read, a few crops made.
fn crops() -> Workbench {
    let mut w = crate::workbench_atlas::tests::with_project();
    let mut actions = Vec::new();
    w.library_request(
        crate::library::LibraryRequest::AtlasCrop("sprites/items.png".into()),
        &mut actions,
    );
    let mut rgba = vec![0u8; 64 * 48 * 4];
    for (i, px) in rgba.chunks_mut(4).enumerate() {
        let (x, y) = (i % 64, i / 64);
        if (x / 16 + y / 16) % 3 != 2 {
            px.copy_from_slice(&[0x60 + (x as u8) * 2, 0x90, 0xd0 - (y as u8) * 2, 255]);
        }
    }
    let art = assets::tile_art::TileArt {
        size: [64, 48],
        frames: vec![rgba],
        durations: vec![100],
    };
    Arc::make_mut(&mut w.viewport.sheets).tile_art.insert(
        "sprites/items.png".into(),
        crate::sprite_sheets::ArtState::Read(Arc::new(art)),
    );
    w.atlas_texture_now(&egui::Context::default());
    let editor = w.open.atlas_editor.as_mut().unwrap();
    editor.crop_every_tile([64, 48], &|r| (r[0] / 16 + r[1] / 16) % 3 != 2);
    editor.pick(Some(2));
    w
}

/// A clips file open on Animate, with its Frames tab in front.
fn clips() -> Workbench {
    let (mut w, _, _) = crate::workbench_frames::tests::with_clips();
    w.open_frames("walker.solframes");
    w
}

/// An animator open on Animate, a state picked.
fn animator() -> Workbench {
    let (mut w, _, _) = crate::workbench_animator::tests::with_animator();
    w.open_animator("farmer.solanim");
    w
}

/// The editor as `w` is, drawn at the design's size: the picture, the
/// tab in front, and whether the tile map's tools are.
fn screen(w: Workbench) -> Option<(Picture, RibbonTab, bool)> {
    let mut harness = editor_window(w, InterfaceLook::default(), SIZE)?;
    for _ in 0..4 {
        harness.run();
    }
    let picture = drawn(&mut harness);
    let w = harness.state();
    Some((picture, w.ribbon_tab(), w.map_tools_in_front()))
}

/// The scenes of Part C: a name, the editor, and the mock-up beside it.
fn scenes() -> Vec<(&'static str, Workbench, &'static str)> {
    vec![
        ("tile-map", tile_map(), "01-tile-map-painting-a.png"),
        ("tileset", tileset(), "02-tileset-editor.png"),
        ("crops", crops(), "03-picture-crops.png"),
        ("clips", clips(), "04-sprite-clips-a.png"),
        ("animator", animator(), "05-animator-graph.png"),
    ]
}

#[test]
fn each_2d_editor_is_its_own_tools_tab_with_its_ribbon() {
    for (name, w, _) in scenes() {
        let Some((_, tab, map_tools)) = screen(w) else {
            return;
        };
        match name {
            "tile-map" => assert!(map_tools, "{name}"),
            "tileset" => assert_eq!(tab, RibbonTab::Tileset),
            "crops" => assert_eq!(tab, RibbonTab::Picture),
            _ => assert_eq!(tab, RibbonTab::Animate),
        }
    }
}

/// Writes each of Part C's screens beside its approved mock-up. Run with
/// `--ignored` and `SOLIS_PICTURE_SHEET` set to the folder to write into.
#[test]
#[ignore = "writes pictures to look at; needs SOLIS_PICTURE_SHEET and a graphics driver"]
fn each_2d_screen_beside_its_mock_up() {
    let folder = std::env::var_os("SOLIS_PICTURE_SHEET").expect("SOLIS_PICTURE_SHEET");
    let folder = std::path::Path::new(&folder);
    let round = repository().join("docs/design/g-layout/additions/round-3");
    for (name, w, mock_up) in scenes() {
        let Some((picture, ..)) = screen(w) else {
            return;
        };
        let design = Picture::read(&round.join(mock_up)).expect("the mock-up reads");
        design
            .beside(&picture, 24, tokens::text_meta())
            .write(&folder.join(format!("part-c-{name}.png")));
    }
}
