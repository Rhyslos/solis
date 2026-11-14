//! Every Lust file the editor writes checks on its own: a game needn't
//! fetch anything for a file the editor made to be sound (the fix of
//! 2026-11-14). An animator's or a tile map's file (`solis/animators/…`,
//! `solis/maps/…`) fetches the engine's `Animators` or `Maps` itself, but
//! Lust keeps an engine object only once the game reaches its file, while
//! every file in the engine's folder comes with the program: so a game
//! that fetched neither was told "there's nothing called Maps here" in a
//! file it never wrote (`reach::for_lust`). The screens' program, whose
//! entry is Solis's own and reaches only the screens and the components
//! they read, is checked the same way.

use crate::animator_code::{AnimatorNames, ParamKind, animator_file};
use crate::code::{Code, compile};
use crate::map_code::{MapNames, ValueNames, map_file};
use crate::project_names::Names;

const MADE_BY: &str = "Test 1.0";
const FOLDER: &str = "solis";

/// A project with one of every name the editor writes into
/// `solis/project.lst`.
fn every_name() -> Names {
    Names {
        assemblies: vec!["ships/fighter".into()],
        sounds: vec!["sfx/footstep".into()],
        clips: vec!["sprites/farmer/walk".into()],
        cues: vec!["footstep".into()],
        screens: vec!["Hud".into()],
        pictures: vec!["sprites/tools/Axe".into()],
        ..Names::standard()
    }
}

fn farmer() -> AnimatorNames {
    AnimatorNames {
        path: "people/farmer".into(),
        parameters: vec![
            ("moving".into(), ParamKind::Switch),
            ("speed".into(), ParamKind::Number),
            ("facing".into(), ParamKind::Direction),
            ("swing".into(), ParamKind::Trigger),
        ],
        states: vec!["Idle".into(), "Walk".into(), "Use tool".into()],
    }
}

fn farm() -> MapNames {
    MapNames {
        path: "maps/farm".into(),
        layers: vec!["ground".into(), "crops".into()],
        terrains: vec![(b'g', "grass".into()), (b's', "tilled soil".into())],
        values: vec![
            ("growth".into(), ValueNames::Number),
            ("watered".into(), ValueNames::Switch),
            (
                "crop".into(),
                ValueNames::Choice(vec!["parsnip".into(), "corn".into()]),
            ),
        ],
    }
}

/// An animator and a map with nothing in them yet, as when first made.
fn empty_ones() -> (AnimatorNames, MapNames) {
    (
        AnimatorNames {
            path: "cow".into(),
            parameters: Vec::new(),
            states: Vec::new(),
        },
        MapNames {
            path: "cave".into(),
            layers: Vec::new(),
            terrains: Vec::new(),
            values: Vec::new(),
        },
    )
}

/// Every kind of Lust file the editor writes into the engine's folder:
/// the engine's descriptions, the project's names, the data files'
/// classes, the kit, and each animator's and tile map's own file.
fn editor_files() -> Vec<(String, String)> {
    let (mut files, _) = crate::describe::descriptions_for(MADE_BY, FOLDER, &every_name());
    let (cow, cave) = empty_ones();
    for animator in [farmer(), cow] {
        files.push(animator_file(MADE_BY, FOLDER, &animator).0.unwrap());
    }
    for map in [farm(), cave] {
        files.push(map_file(MADE_BY, FOLDER, &map).0.unwrap());
    }
    files
}

/// A screen as the UI editor saves one: an assembly with a view.
fn hud() -> (String, String) {
    (
        "ui/hud.lst".into(),
        "fetch \"solis/screen\":Ui;\n\nassembly Hud {\n    view {\n        text \"Hello\" [size 28, white];\n    }\n}\n\npass Hud;\nend\n"
            .into(),
    )
}

/// The mistakes in a game of `files`, said one to a line.
fn problems(files: Vec<(String, String)>) -> String {
    let code = Code {
        files,
        ..Code::default()
    };
    match compile(&code) {
        Ok(_) => String::new(),
        Err(found) => found
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

#[test]
fn every_file_the_editor_writes_checks_in_an_empty_game() {
    let mut files = editor_files();
    files.push(("main.lst".into(), "end\n".into()));
    assert_eq!(problems(files), "");
}

#[test]
fn every_file_the_editor_writes_checks_beside_a_screen() {
    // The screens' program starts from Solis's own entry, which reaches
    // only the screens: the editor's files must check there too.
    let mut files = editor_files();
    files.push(("main.lst".into(), "end\n".into()));
    files.push(hud());
    assert_eq!(problems(files), "");
}

#[test]
fn a_game_that_fetches_some_of_them_checks_with_a_screen() {
    let mut files = editor_files();
    files.push((
        "main.lst".into(),
        "fetch \"solis/maps/maps/farm\":[FarmMap, FarmLayer, FarmTerrain];\n\
         fetch \"solis/animators/people/farmer\":FarmerAnimator;\n\n\
         component Farmer {\n    has {\n        FarmMap farm;\n        FarmerAnimator anim;\n    }\n\
         \x20   method update(float dt) {\n\
         \x20       is farm:terrain(FarmLayer.Ground, 1, 2) == FarmTerrain.TilledSoil? then {\n\
         \x20           anim:swing();\n        }\n    }\n}\n\npass Farmer;\nend\n"
            .into(),
    ));
    files.push(hud());
    assert_eq!(problems(files), "");
}

#[test]
fn a_game_that_reaches_one_through_another_file_still_has_it() {
    // main fetches a file of the game's own, which fetches the map: the
    // map's file is reached, so it's kept and its names are known.
    let mut files = editor_files();
    files.push((
        "main.lst".into(),
        "fetch \"farming/field\":Field;\nend\n".into(),
    ));
    files.push((
        "farming/field.lst".into(),
        "-- The map's file: \"solis/maps/cave\" is only a comment here.\n\
         fetch \"solis/maps/maps/farm\":FarmMap;\n\n\
         component Field {\n    has {\n        FarmMap farm;\n    }\n\
         \x20   method update(float dt) {\n        int across = farm:width();\n    }\n}\n\npass Field;\nend\n"
            .into(),
    ));
    assert_eq!(problems(files), "");
}

#[test]
fn a_game_that_fetches_a_name_its_file_lacks_is_still_told() {
    // Leaving the unreached files out never hides a real mistake.
    let mut files = editor_files();
    files.push((
        "main.lst".into(),
        "fetch \"solis/maps/maps/farm\":BarnMap;\nend\n".into(),
    ));
    let found = problems(files);
    assert!(found.contains("BarnMap"), "{found}");
}

#[test]
fn the_screens_program_checks_every_file_the_editor_writes() {
    // Its entry reaches only the screens and the components they read.
    let mut files = editor_files();
    files.push(("main.lst".into(), "end\n".into()));
    files.push(hud());
    let engine = lust::Engine {
        folders: crate::describe::engine_folders(),
    };
    match crate::screens::compile(&files, &engine, &[], &[]) {
        Ok(screens) => assert!(screens.is_some()),
        Err(found) => panic!(
            "{}",
            found
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        ),
    }
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(48))]
    /// Whatever an animator or a map holds, its file checks in an empty
    /// game: names that can't be Lust are left out with a note, never a
    /// mistake in a file the game didn't write.
    #[test]
    fn any_animator_or_map_checks_in_an_empty_game(
        path in "[a-zA-Z][a-zA-Z0-9_]{0,5}(/[a-zA-Z][a-zA-Z0-9_]{0,5}){0,2}",
        names in proptest::collection::vec("[a-zA-Z0-9_ -]{1,8}", 0..6),
        kinds in proptest::collection::vec(0u8..4, 0..6),
    ) {
        let parameters = names
            .iter()
            .zip(&kinds)
            .map(|(name, kind)| {
                let kind = [ParamKind::Number, ParamKind::Switch, ParamKind::Trigger, ParamKind::Direction][usize::from(*kind)];
                (name.clone(), kind)
            })
            .collect();
        let values = names
            .iter()
            .zip(&kinds)
            .map(|(name, kind)| {
                let value = match kind {
                    0 => ValueNames::Number,
                    1 => ValueNames::Switch,
                    _ => ValueNames::Choice(names.clone()),
                };
                (name.clone(), value)
            })
            .collect();
        let animator = AnimatorNames { path: path.clone(), parameters, states: names.clone() };
        let map = MapNames {
            path,
            layers: names.clone(),
            terrains: names.iter().enumerate().map(|(at, n)| (b'a' + at as u8, n.clone())).collect(),
            values,
        };
        let (mut files, _) = crate::describe::descriptions_for(MADE_BY, FOLDER, &Names::standard());
        files.extend(animator_file(MADE_BY, FOLDER, &animator).0);
        files.extend(map_file(MADE_BY, FOLDER, &map).0);
        files.push(("main.lst".into(), "end\n".into()));
        let found = problems(files);
        proptest::prop_assert!(found.is_empty(), "{}", found);
    }
}
