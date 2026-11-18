//! The Library's parts on their own: what's shown, picking, back and up,
//! names, moves, the trees. The browser drawn in the editor is tested in
//! `library_screen_tests.rs` and `library_browser_tests.rs`.

use super::menus::MenuAction;
use super::*;

fn item(path: &str, kind: LibraryKind) -> LibraryItem {
    LibraryItem {
        id: kind.placeable().then(AssetId::random),
        kind,
        name: last_part(path)
            .split('.')
            .next()
            .unwrap_or_default()
            .to_string(),
        path: path.to_string(),
        thumbnail: None,
    }
}

fn names(entries: &[Entry<'_>]) -> Vec<String> {
    entries.iter().map(|e| e.name().to_owned()).collect()
}

#[test]
fn an_item_knows_its_folder() {
    assert_eq!(
        item("props/old/crate.glb", LibraryKind::Model).folder(),
        "props/old"
    );
    assert_eq!(item("crate.glb", LibraryKind::Model).folder(), "");
    assert_eq!(
        item("props/crate.glb", LibraryKind::Model).file_name(),
        "crate.glb"
    );
    assert_eq!(join("", "a"), "a");
    assert_eq!(join("x/y", "a"), "x/y/a");
    assert_eq!(parent_of("x/y/a"), "x/y");
    assert_eq!(parent_of("a"), "");
}

#[test]
fn the_browser_shows_a_folder_s_folders_then_its_files_or_every_match() {
    let items = LibraryFiles::from(vec![
        item("crate.glb", LibraryKind::Model),
        item("props/rock.glb", LibraryKind::Model),
        item("props/rocket.solassembly", LibraryKind::Assembly),
        item("turret.lst", LibraryKind::Code),
        item("solis/project.lst", LibraryKind::Code),
    ]);
    let folders: Vec<String> = ["assets", "props", "props/old", "solis", "solis/data"]
        .map(String::from)
        .to_vec();
    let mut state = LibraryState::default();
    let at_top = shown(&items, &folders, &state);
    assert_eq!(
        names(&at_top),
        ["assets", "props", "crate", "turret"],
        "folders first; an existing assets folder is an ordinary one; the engine's is left out"
    );
    state.folder = "props".into();
    assert_eq!(
        names(&shown(&items, &folders, &state)),
        ["old", "rock", "rocket"]
    );
    state.search = "ROCK".into();
    state.folder.clear();
    assert_eq!(
        names(&shown(&items, &folders, &state)),
        ["rock", "rocket"],
        "from every folder"
    );
    state.search = "o".into();
    let found = names(&shown(&items, &folders, &state));
    assert!(found.contains(&"old".to_owned()) && !found.contains(&"project".to_owned()));
    state.search.clear();
    state.filter = Filter::Models;
    assert_eq!(
        names(&shown(&items, &folders, &state)),
        ["assets", "props", "crate"],
        "the kind filter keeps the folders, to go through them"
    );
}

/// The review, BUG-30: no kind's menu names an action twice, and every
/// kind's offers something.
#[test]
fn a_kind_s_menu_offers_open_or_place_or_both_each_once() {
    use LibraryKind as K;
    use MenuAction::{Open, Place};
    for (kind, offered) in [
        (K::Model, &[Place][..]),
        (K::Assembly, &[Place]),
        (K::Level, &[Open]),
        (K::Texture, &[Open, Place]),
        (K::Code, &[Open]),
        (K::Sound, &[Open]),
        (K::SoundEvent, &[Open]),
        (K::SpriteSheet, &[Place]),
        (K::SpriteClips, &[Open, Place]),
        (K::Animator, &[Open]),
        (K::Tileset, &[Open]),
        (K::TileMap, &[Place]),
        (K::Atlas, &[Open]),
        (K::Text, &[Open]),
        (K::Other, &[Open]),
    ] {
        assert_eq!(kind.menu_actions(), offered, "{kind:?}");
        let mut labels: Vec<&str> = offered.iter().map(|a| a.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), offered.len(), "{kind:?}");
        assert!(!labels.is_empty(), "{kind:?}");
        assert_eq!(offered.contains(&Place), kind.placeable(), "{kind:?}");
        if kind.opens_on_double_click() {
            assert!(offered.contains(&Open), "{kind:?}");
        }
    }
}

#[test]
fn the_kind_filter_shows_one_kind_of_file() {
    let items = LibraryFiles::from(vec![
        item("crate.glb", LibraryKind::Model),
        item("farmer.aseprite", LibraryKind::SpriteSheet),
        item("farm.tilemap.lst", LibraryKind::TileMap),
        item("birdsong.ogg", LibraryKind::Sound),
        item("main.lst", LibraryKind::Code),
    ]);
    let mut state = LibraryState::default();
    let mut only = |filter| -> Vec<String> {
        state.filter = filter;
        names(&shown(&items, &[], &state))
    };
    assert_eq!(only(Filter::All).len(), 5);
    assert_eq!(only(Filter::Models), ["crate"]);
    assert_eq!(only(Filter::Sprites), ["farmer"]);
    assert_eq!(only(Filter::Maps), ["farm"]);
    assert_eq!(only(Filter::Sounds), ["birdsong"]);
    assert_eq!(only(Filter::Code), ["main"]);
    for kind in LibraryKind::ALL
        .into_iter()
        .filter(|k| !matches!(k, LibraryKind::Atlas | LibraryKind::Other))
    {
        assert!(Filter::ALL[1..].iter().any(|f| f.passes(kind)), "{kind:?}");
    }
}

#[test]
fn back_and_up_go_where_they_say() {
    let mut state = LibraryState::default();
    state.go_back();
    state.go_up();
    assert_eq!(state.folder, "", "nothing to go back or up to");
    state.open_folder("farm");
    state.open_folder("farm/animals");
    state.go_up();
    assert_eq!(state.folder, "farm");
    state.go_back();
    assert_eq!(state.folder, "farm/animals", "back to where up came from");
    state.go_back();
    state.go_back();
    assert_eq!(state.folder, "");
    state.search = "x".into();
    state.open_folder("farm");
    assert!(state.search.is_empty(), "opening a folder shows its files");
    for n in 0..200 {
        state.open_folder(&format!("f{n}"));
    }
    assert!(state.back.len() <= 64, "back keeps a few");
}

#[test]
fn ctrl_and_shift_pick_several_and_a_click_one() {
    let order = ["a", "b", "c", "d"];
    let mut state = LibraryState::default();
    state.pick("b");
    state.pick_also("d");
    assert_eq!(state.picked(), ["d", "b"]);
    state.pick_also("d");
    assert_eq!(state.picked(), ["b"], "Ctrl again takes it out");
    state.pick_range("d", &order);
    assert_eq!(
        state.picked(),
        ["b", "c", "d"],
        "Shift: from the last picked"
    );
    assert!(state.is_picked("c") && !state.is_picked("a"));
    state.pick("a");
    assert_eq!(state.picked(), ["a"]);
    state.pick_also("a");
    assert!(state.picked().is_empty());
    state.pick_range("c", &order);
    assert_eq!(state.picked(), ["c"], "nothing picked: Shift picks the one");
}

#[test]
fn a_name_that_won_t_do_says_why() {
    let none = |_: &str| false;
    assert_eq!(checked_name("  Crops ", none).unwrap(), "Crops");
    for (bad, why) in [
        ("", "empty"),
        ("   ", "empty"),
        (".hidden", "dot"),
        ("a/b", "Windows forbids"),
        ("what?", "Windows forbids"),
        ("a:b", "Windows forbids"),
        ("tab\there", "control"),
        ("dots.", "end with a dot"),
        ("CON", "keeps for itself"),
        ("com1.png", "keeps for itself"),
    ] {
        let said = checked_name(bad, none).unwrap_err();
        assert!(said.contains(why), "{bad:?}: {said}");
    }
    assert!(checked_name(&"x".repeat(101), none).is_err());
    let taken = |n: &str| n == "barn";
    assert!(checked_name("Barn", taken).unwrap_err().contains("already"));
}

#[test]
fn renaming_keeps_a_file_s_ending_and_refuses_a_name_taken_in_its_folder() {
    let project = ProjectInfo {
        name: "Farm".into(),
        root: PathBuf::new(),
        assets: PathBuf::new(),
    };
    let items = LibraryFiles::from(vec![
        item("farm/grass.png", LibraryKind::Texture),
        item("farm/hay.png", LibraryKind::Texture),
        item("farm/notes.md", LibraryKind::Text),
    ]);
    let folders = vec!["farm".to_owned(), "farm/barn".to_owned()];
    let states = HashMap::new();
    let events = HashMap::new();
    let sheets = crate::sprite_sheets::SpriteSheets::default();
    let pictures = HashMap::new();
    let uses = BTreeMap::new();
    let mixer = sound::Mixer::default();
    let files = Files {
        project: &project,
        items: &items,
        folders: &folders,
        accent: Accent::default(),
        sounds: Sounds {
            states: &states,
            events: &events,
            preview: None,
            mixer: &mixer,
            sheets: &sheets,
            sheet_pictures: &pictures,
            code_uses: &uses,
        },
    };
    assert_eq!(
        rename_to(files, "farm/grass.png", "meadow").unwrap(),
        "farm/meadow.png"
    );
    assert_eq!(
        rename_to(files, "farm/grass.png", "grass").unwrap(),
        "farm/grass.png",
        "its own name isn't taken"
    );
    assert!(
        rename_to(files, "farm/grass.png", "HAY").is_err(),
        "hay.png is there"
    );
    assert!(
        rename_to(files, "farm/grass.png", "barn").is_ok(),
        "barn.png isn't"
    );
    assert!(
        rename_to(files, "farm/barn", "grass.png").is_err(),
        "a folder can't take a file's name"
    );
    assert_eq!(
        rename_to(files, "farm/barn", "stable").unwrap(),
        "farm/stable"
    );
}

#[test]
fn dropping_onto_a_folder_moves_what_isn_t_there_already() {
    let moving = vec![
        "farm/hay.png".to_owned(),
        "barn.png".to_owned(),
        "farm".to_owned(),
        "farm/animals".to_owned(),
    ];
    assert_eq!(
        moves_into("farm", &moving),
        [LibraryRequest::Move(
            "barn.png".into(),
            "farm/barn.png".into()
        )],
        "not what's in it already, not a folder into itself"
    );
    assert_eq!(
        moves_into("farm/animals", &["farm".to_owned()]),
        [],
        "never a folder into one inside it"
    );
    assert_eq!(
        moves_into("", &["farm/hay.png".to_owned()]),
        [LibraryRequest::Move(
            "farm/hay.png".into(),
            "hay.png".into()
        )]
    );
}

#[test]
fn the_folder_tree_keeps_each_folder_under_its_own() {
    let folders: Vec<String> = ["farm", "farm 2", "farm/animals", "farm/b", "solis", "x"]
        .map(String::from)
        .to_vec();
    let mut folded = std::collections::BTreeSet::new();
    let rows = tree::visible_folders(&folders, &folded, "");
    assert_eq!(
        rows,
        [
            ("farm", 0),
            ("farm/animals", 1),
            ("farm/b", 1),
            ("farm 2", 0),
            ("x", 0)
        ]
    );
    folded.insert("farm".to_owned());
    let rows = tree::visible_folders(&folders, &folded, "");
    assert_eq!(rows, [("farm", 0), ("farm 2", 0), ("x", 0)]);
    let inside = tree::visible_folders(&folders, &Default::default(), "farm");
    assert_eq!(inside, [("farm/animals", 0), ("farm/b", 0)]);
}

#[test]
fn trees_open_beside_each_other_close_and_keep_their_room() {
    let mut trees = Trees::default();
    trees.list[0].top = "farm".into();
    trees.close(0);
    assert_eq!(trees.list.len(), 1, "the last stays");
    trees.open_another(0);
    assert_eq!(trees.list.len(), 2);
    assert_eq!(trees.list[1].top, "farm", "it opens where the first is");
    assert_eq!(trees.edges, [500]);
    for _ in 0..10 {
        trees.open_another(0);
    }
    assert_eq!(trees.list.len(), trees::MOST_TREES);
    trees.close(1);
    trees.close(1);
    assert_eq!(trees.edges, [500]);
}
