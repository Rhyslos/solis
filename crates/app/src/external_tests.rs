//! The one place files and folders reach the computer's own programs
//! (the security review, SEC-03): what each request comes to, without
//! starting anything.

use super::*;

/// A project in a scratch folder: its root and its assets folder, holding
/// one file of each kind that matters and one beside the assets.
fn project(name: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("solis-external-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let assets = root.join("assets");
    for folder in ["ai", "props", "notes", "areas"] {
        std::fs::create_dir_all(assets.join(folder)).unwrap();
    }
    for file in [
        "main.lst",
        "ai/sparks.lua",
        "ai/armour.rs",
        "props/brick.png",
        "props/brick.png.exe",
        "props/ship.glb",
        "props/hero.aseprite",
        "notes/plan.md",
        "areas/farm.tilemap.lst",
        "areas/farm.solarea",
        "run.bat",
    ] {
        std::fs::write(assets.join(file), b"a project's file").unwrap();
    }
    std::fs::write(root.join("outside.lst"), b"beside the assets").unwrap();
    (root, assets)
}

/// What asking for `file` (inside `assets`) comes to.
fn asked(assets: &Path, file: &str) -> io::Result<Handed> {
    settle(External::InProject {
        path: &assets.join(file),
        assets: Some(assets),
    })
}

#[test]
fn only_pictures_text_and_lust_are_opened_and_the_rest_shown_in_their_folder() {
    // SEC-03: the window opened whatever file it was handed, so a way of
    // asking that skipped the Library's rule ran a project's .lua file in
    // the computer's Lua interpreter, outside the sandbox.
    let (root, assets) = project("kinds");
    let opened = |file: &str| Handed::Opened(assets.join(file));
    let shown = |folder: &str| Handed::Shown(assets.join(folder));
    for (file, comes_to) in [
        ("main.lst", opened("main.lst")),
        ("props/brick.png", opened("props/brick.png")),
        ("notes/plan.md", opened("notes/plan.md")),
        ("ai/sparks.lua", shown("ai")),
        ("ai/armour.rs", shown("ai")),
        ("props/brick.png.exe", shown("props")),
        ("props/ship.glb", shown("props")),
        ("props/hero.aseprite", shown("props")),
        ("areas/farm.tilemap.lst", shown("areas")),
        ("areas/farm.solarea", shown("areas")),
        ("run.bat", Handed::Shown(assets.clone())),
    ] {
        assert_eq!(asked(&assets, file).unwrap(), comes_to, "{file}");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_folder_of_the_project_is_shown_whatever_it_is_called() {
    let (root, assets) = project("folders");
    // A folder named like a file that opens is still a folder.
    std::fs::create_dir_all(assets.join("props/trap.lst")).unwrap();
    for folder in ["props", "props/trap.lst", ""] {
        assert_eq!(
            asked(&assets, folder).unwrap(),
            Handed::Shown(assets.join(folder)),
            "{folder}"
        );
    }
    // What isn't there is an error, never a guess.
    assert!(asked(&assets, "props/gone.png").is_err());
    assert!(asked(&assets, "gone").is_err());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn nothing_outside_the_assets_folder_is_handed_over() {
    let (root, assets) = project("outside");
    // Each of these exists or could: only where it is refuses it.
    let elsewhere = project("elsewhere").1.join("main.lst");
    // Paths a project's file could name, joined to the assets folder as
    // the editor joins them.
    let named = [
        "../outside.lst",
        "props/../../outside.lst",
        "props/../brick.png",
        "//attacker.example/share/a.lst",
        "\\\\attacker\\share\\a.lst",
        "C:/Windows/notepad.exe",
        "C:main.lst",
        "/etc/passwd",
    ];
    let mut outside: Vec<PathBuf> = named.iter().map(|name| assets.join(name)).collect();
    outside.extend([
        root.join("outside.lst"),
        root.clone(),
        elsewhere.clone(),
        PathBuf::from("main.lst"),
        PathBuf::new(),
    ]);
    for path in &outside {
        let why = settle(External::InProject {
            path,
            assets: Some(&assets),
        })
        .unwrap_err();
        assert_eq!(
            why.to_string(),
            "it isn't in the project's assets folder",
            "{}",
            path.display()
        );
    }
    // With no project open there is no assets folder to be inside.
    let no_project = settle(External::InProject {
        path: &assets.join("main.lst"),
        assets: None,
    });
    assert!(no_project.is_err());
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(elsewhere.parent().and_then(Path::parent).unwrap());
}

#[test]
fn a_folder_the_user_chose_is_shown_and_a_file_there_is_never_opened() {
    let (root, assets) = project("chosen");
    // A project's folder from the launcher's list, or an export's.
    assert_eq!(
        settle(External::Folder(&root)).unwrap(),
        Handed::Shown(root.clone())
    );
    // Even a picture: only a folder is ever shown this way.
    for file in ["props/brick.png", "main.lst", "ai/sparks.lua", "run.bat"] {
        let why = settle(External::Folder(&assets.join(file))).unwrap_err();
        assert_eq!(why.to_string(), "it isn't a folder", "{file}");
    }
    assert!(settle(External::Folder(&root.join("gone"))).is_err());
    let _ = std::fs::remove_dir_all(root);
}

/// The endings whose program only shows or edits the file, written out
/// here on their own so the sweep doesn't lean on the rule it checks.
const SAFE: [&str; 10] = [
    "png", "jpg", "jpeg", "md", "txt", "json", "csv", "toml", "ron", "lst",
];

#[test]
fn no_name_makes_another_kind_of_file_open() {
    // Every pairing of endings, in small and capital letters: a file opens
    // only if its last ending is a safe one, and a Lust data file the
    // editor has its own tab for (a tile map, say) doesn't either.
    let (root, assets) = project("names");
    let endings = [
        "png",
        "jpg",
        "jpeg",
        "md",
        "txt",
        "json",
        "csv",
        "toml",
        "ron",
        "lst",
        "lua",
        "rs",
        "exe",
        "bat",
        "cmd",
        "com",
        "scr",
        "lnk",
        "js",
        "vbs",
        "ps1",
        "py",
        "glb",
        "gltf",
        "aseprite",
        "wav",
        "solarea",
        "solassembly",
        "tilemap",
        "tileset",
        "atlas",
        "html",
        "url",
    ];
    let mut tried = 0;
    for first in endings {
        for last in endings {
            for name in [
                format!("file.{last}"),
                format!("file.{first}.{last}"),
                format!("FILE.{}.{}", first.to_uppercase(), last.to_uppercase()),
            ] {
                std::fs::write(assets.join(&name), b"x").unwrap();
                let lower = name.to_lowercase();
                let data = ["tilemap", "tileset", "atlas"]
                    .iter()
                    .any(|kind| lower.ends_with(&format!(".{kind}.lst")));
                let safe = SAFE.contains(&last) && !data;
                let comes_to = if safe {
                    Handed::Opened(assets.join(&name))
                } else {
                    Handed::Shown(assets.clone())
                };
                assert_eq!(asked(&assets, &name).unwrap(), comes_to, "{name}");
                tried += 1;
            }
        }
    }
    assert!(tried > 3_000);
    // A file with no ending, or only an ending, never opens.
    for name in ["lst", "png", ".lst", "file"] {
        std::fs::write(assets.join(name), b"x").unwrap();
        assert_eq!(
            asked(&assets, name).unwrap(),
            Handed::Shown(assets.clone()),
            "{name}"
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn no_path_a_project_could_name_leaves_the_assets_folder() {
    // Every path of up to three of these parts, joined as the editor joins
    // a project's path to the assets folder: whatever comes back is inside
    // the assets, with no step upwards.
    let (root, assets) = project("paths");
    let parts = [
        "..",
        ".",
        "props",
        "main.lst",
        "brick.png",
        "",
        "C:",
        "/",
        "\\",
        "//attacker.example/share",
        "\\\\attacker\\share",
        "/etc",
        "..\\..",
    ];
    let mut tried = 0;
    for a in parts {
        for b in parts {
            for c in parts {
                for path in [
                    a.to_string(),
                    format!("{a}/{b}"),
                    format!("{a}/{b}/{c}"),
                    format!("{a}\\{b}\\{c}"),
                ] {
                    tried += 1;
                    let full = assets.join(&path);
                    let Ok(Handed::Opened(handed) | Handed::Shown(handed)) =
                        settle(External::InProject {
                            path: &full,
                            assets: Some(&assets),
                        })
                    else {
                        continue;
                    };
                    assert!(handed.starts_with(&assets), "{path}: {handed:?}");
                    assert!(
                        handed
                            .components()
                            .all(|part| part != std::path::Component::ParentDir),
                        "{path}: {handed:?}"
                    );
                }
            }
        }
    }
    assert!(tried > 8_000);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_files_kind_comes_from_its_own_name_as_the_library_lists_it() {
    // The Library's kinds come from the same place, so what it says will
    // happen to a file is what happens.
    use editor::LibraryKind as K;
    for (name, kind) in [
        ("brick.png", K::Texture),
        ("main.lst", K::Code),
        ("sparks.lua", K::Code),
        ("plan.md", K::Text),
        ("farm.tilemap.lst", K::TileMap),
        ("ship.glb", K::Model),
        ("brick.png.exe", K::Other),
    ] {
        let found = crate::project::library_kind(asset_db::FileKind::of(Path::new(name)));
        assert_eq!(found, kind, "{name}");
        assert_eq!(
            opens_in_its_program(Path::new(name)),
            kind.opens_in_its_program(name),
            "{name}"
        );
    }
}
