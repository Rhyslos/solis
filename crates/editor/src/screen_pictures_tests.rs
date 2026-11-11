//! Pictures on the game's screens (decision 0089).

use crate::library::{LibraryItem, LibraryKind};
use crate::sprite_sheets::{ArtState, AtlasState};
use crate::test_support::*;
use crate::workbench::Workbench;
use area::EntityData;
use area::builtin::Screen;
use reflect::EntityId;
use std::sync::Arc;

fn items(icons: &str) -> String {
    format!(
        "assembly Items {{\n    has {{ string chosen = \"sprites/tools.png#Hoe\"; }}\n    view {{\n        row {{\n            {icons}\n        }}\n    }}\n}}\npass Items;\nend\n"
    )
}

fn program(screen: &str) -> Arc<script::Compiled> {
    let code = script::Code {
        files: vec![
            ("main.lst".to_string(), "end\n".to_string()),
            ("ui/items.lst".to_string(), screen.to_string()),
        ],
        lua: Vec::new(),
        notes: Vec::new(),
    };
    match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    }
}

fn picture(path: &str) -> LibraryItem {
    LibraryItem {
        id: None,
        kind: LibraryKind::Texture,
        name: path.rsplit('/').next().unwrap().into(),
        path: path.into(),
        thumbnail: None,
    }
}

/// `sprites/tools.png` (32 × 16) read, with `crops` (name, box) cut out
/// of it when given.
fn with_tools(w: &mut Workbench, crops: Option<&[(&str, [u32; 4])]>) {
    w.set_library(vec![picture("sprites/tools.png")]);
    let sheets = Arc::make_mut(&mut w.viewport.sheets);
    sheets.tile_art.insert(
        "sprites/tools.png".into(),
        ArtState::Read(Arc::new(assets::tile_art::TileArt {
            size: [32, 16],
            frames: vec![vec![200; 32 * 16 * 4]],
            durations: vec![100],
        })),
    );
    if let Some(crops) = crops {
        let mut atlas = animation::atlas::Atlas::new("tools.png");
        for (name, rect) in crops {
            atlas.crops.push(animation::atlas::Crop {
                name: (*name).into(),
                rect: *rect,
            });
        }
        sheets.atlases.insert(
            "sprites/tools.atlas.lst".into(),
            AtlasState::Read(Arc::new(atlas), Vec::new()),
        );
    }
}

#[test]
fn a_picture_named_in_a_screen_that_isn_t_there_is_a_problem_at_its_line() {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(&items("image \"sprites/tools.png\";"))));
    w.refresh_screen_pictures();
    let problems = w.picture_problems();
    assert_eq!(problems.len(), 1);
    assert_eq!(
        (problems[0].file.as_str(), problems[0].line),
        ("ui/items.lst", 5)
    );
    assert!(
        problems[0].message.contains("no picture sprites/tools.png"),
        "{}",
        problems[0]
    );
    assert_eq!(w.all_code_problems().len(), 1, "listed in Problems");
    assert_eq!(w.status().problems, 1);
    assert_eq!(w.take_wanted_pictures(), None, "nothing to read");
    // In the project now: it's read, and shown.
    with_tools(&mut w, None);
    w.refresh_screen_pictures();
    assert!(w.picture_problems().is_empty());
    let wanted = w.take_wanted_pictures().unwrap();
    assert_eq!(
        wanted.into_iter().collect::<Vec<_>>(),
        ["sprites/tools.png"]
    );
    assert_eq!(w.take_wanted_pictures(), None, "taken once");
    let pictures = w.open.screen_pictures.pictures.clone().unwrap();
    assert_eq!(pictures.get("sprites/tools.png").unwrap().size, [32, 16]);
}

#[test]
fn a_crop_shows_its_part_and_a_missing_one_names_those_there_are() {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(&items(
        "image \"sprites/tools.png#Axe\";\n            image \"sprites/tools.png#Spade\";",
    ))));
    with_tools(&mut w, None);
    w.refresh_screen_pictures();
    let problems = w.picture_problems();
    assert_eq!(problems.len(), 2);
    assert!(
        problems[0].message.contains("no crops yet"),
        "{}",
        problems[0]
    );
    with_tools(
        &mut w,
        Some(&[("Axe", [0, 0, 16, 16]), ("Hoe", [16, 0, 8, 8])]),
    );
    w.refresh_screen_pictures();
    let problems = w.picture_problems();
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].line, 6);
    assert!(
        problems[0]
            .message
            .contains("no crop called Spade (its crops: Axe, Hoe)"),
        "{}",
        problems[0]
    );
    let pictures = w.open.screen_pictures.pictures.clone().unwrap();
    assert_eq!(
        pictures.get("sprites/tools.png#Axe").unwrap().size,
        [16, 16]
    );
    assert!(pictures.get("sprites/tools.png#Spade").is_none());
}

#[test]
fn a_name_worked_out_as_the_game_runs_is_read_shown_and_said_once_if_wrong() {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(&items("image chosen;"))));
    w.open
        .document
        .edit(
            "Add",
            vec![area::Edit::Spawn {
                entities: vec![(
                    EntityId::random(),
                    EntityData::new("Shower").with(&Screen {
                        screen: "Items".into(),
                        shown: true,
                        order: 0,
                        ..Default::default()
                    }),
                )],
            }],
            None,
            None,
        )
        .unwrap();
    with_tools(&mut w, Some(&[("Axe", [0, 0, 16, 16])]));
    w.refresh_screen_pictures();
    assert!(w.picture_problems().is_empty(), "nothing written out");
    w.play();
    w.run_game(1.0 / 60.0);
    w.refresh_screen_pictures();
    assert!(
        w.open
            .screen_pictures
            .seen
            .contains("sprites/tools.png#Hoe"),
        "seen as the screen ran"
    );
    let wrong = |w: &Workbench| {
        w.log_lines()
            .iter()
            .filter(|l| l.text.contains("\"sprites/tools.png#Hoe\""))
            .count()
    };
    assert_eq!(wrong(&w), 1, "said in the Console");
    assert!(w.picture_problems().is_empty(), "not a problem in the code");
    // The screens carry the pictures, so they're drawn.
    w.run_game(1.0 / 60.0);
    w.refresh_screen_pictures();
    assert!(w.open.game_screens.pictures[0].pictures.is_some());
    // The crop's cut now: shown, and nothing said again.
    with_tools(
        &mut w,
        Some(&[("Axe", [0, 0, 16, 16]), ("Hoe", [16, 0, 8, 8])]),
    );
    w.refresh_screen_pictures();
    w.run_game(1.0 / 60.0);
    w.refresh_screen_pictures();
    let on_screen = w.open.game_screens.pictures[0].pictures.clone().unwrap();
    assert!(on_screen.get("sprites/tools.png#Hoe").is_some());
    assert_eq!(wrong(&w), 1);
}

#[test]
fn image_names_are_gathered_from_a_view_within_limits() {
    let mut deep = ui::view::Node {
        kind: "image".into(),
        text: "a.png".into(),
        ..Default::default()
    };
    for _ in 0..200 {
        deep = ui::view::Node {
            kind: "column".into(),
            children: vec![deep],
            ..Default::default()
        };
    }
    let mut out = std::collections::BTreeSet::new();
    crate::screen_pictures::names_in(&deep, &mut out);
    assert!(out.is_empty(), "deeper than any layout goes");
    let wide = ui::view::Node {
        kind: "row".into(),
        children: (0..2000)
            .map(|i| ui::view::Node {
                kind: "image".into(),
                text: format!("{i}.png"),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    crate::screen_pictures::names_in(&wide, &mut out);
    assert_eq!(out.len(), crate::screen_pictures::MAX_SEEN);
}

/// The review, BUG-10: the screens' pictures went by where the project's
/// sheets were in memory. Sheets handed over again (a picture read) could
/// be put where the last were, and the old pictures stayed.
#[test]
fn sheets_handed_over_again_get_the_pictures_made_again() {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(&items("image \"sprites/tools.png\";"))));
    with_tools(&mut w, None);
    w.refresh_screen_pictures();
    let first = w.open.screen_pictures.pictures.clone().unwrap();
    w.refresh_screen_pictures();
    let kept = w.open.screen_pictures.pictures.clone().unwrap();
    assert!(Arc::ptr_eq(&first, &kept), "kept while nothing changes");
    // The same sheets, handed over again by the app: other sheets, though
    // all in them is the same.
    let sheets = (*w.viewport.sheets).clone();
    w.set_sprite_sheets(sheets);
    w.refresh_screen_pictures();
    let again = w.open.screen_pictures.pictures.clone().unwrap();
    assert!(!Arc::ptr_eq(&first, &again), "made again");
    assert_eq!(again.get("sprites/tools.png").unwrap().size, [32, 16]);
}
