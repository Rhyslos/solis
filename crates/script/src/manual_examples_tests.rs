//! The manual's game-code examples, checked against the engine (decision
//! 0113): every Lust example is checked by Lust with the engine's own
//! description files, so it may call only what the engine has today, and
//! every Lua example is compiled and started as Solis starts a Lua file,
//! with no world, so its syntax and its components' fields are right.
//! (Each example's Rust and the names its Lua calls are checked by the
//! user guide's tests, `tools/user-guide/tests/manual.rs`, which also
//! fail when an example lacks one of the three languages.)
//!
//! An example is a whole file. One whose first line names it (`-- hero.lst:
//! the player.`, `-- ui/pauseMenu.lst`) is checked under that name, with
//! the page's other named files beside it; any other is `main.lst`, or a
//! screen in `ui/` when it is one. The project's names they use (an
//! assembly, a sound, a clip, an animator, a tile map) come from the
//! stand-in project below.

use crate::animator_code::{AnimatorNames, ParamKind, animator_file};
use crate::code::{Code, compile};
use crate::map_code::{MapNames, ValueNames, map_file};
use crate::project_names::Names;
use std::path::{Path, PathBuf};

const MADE_BY: &str = "Test 1.0";

fn manual_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual")
}

/// One fenced block of game code: its page, language and text.
struct Fenced {
    page: String,
    language: String,
    code: String,
}

/// Every `lust`, `rust` and `lua` block of every page of the manual.
fn fenced() -> Vec<Fenced> {
    let mut found = Vec::new();
    let mut pages: Vec<PathBuf> = std::fs::read_dir(manual_folder())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    pages.sort();
    for path in pages {
        let page = path.file_stem().unwrap().to_string_lossy().into_owned();
        if page == "_template" {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .unwrap()
            .replace("\r\n", "\n");
        let mut open: Option<(String, usize, Vec<String>)> = None;
        for line in text.lines() {
            let indent = line.len() - line.trim_start().len();
            let trimmed = line.trim_start().trim_start_matches('>').trim_start();
            let fence = trimmed.starts_with("```");
            match open.take() {
                None if fence => {
                    let language = trimmed.trim_start_matches('`').trim().to_string();
                    open = Some((language, indent, Vec::new()));
                }
                None => {}
                Some((language, _, lines)) if fence && trimmed.trim_matches('`').is_empty() => {
                    if ["lust", "rust", "lua"].contains(&language.as_str()) {
                        found.push(Fenced {
                            page: page.clone(),
                            language,
                            code: lines.join("\n"),
                        });
                    }
                }
                Some((language, at, mut lines)) => {
                    let quoted = line.trim_start().starts_with('>');
                    let body = if quoted {
                        trimmed
                    } else {
                        line.get(at.min(indent)..).unwrap_or("")
                    };
                    lines.push(body.to_string());
                    open = Some((language, at, lines));
                }
            }
        }
    }
    found
}

/// The names a project holding the manual's examples would have.
fn names() -> Names {
    let mut names = Names::with_assemblies(&["ships/fighter", "ships/bolt", "drones/drone"])
        .and_sounds(&["sfx/footstep", "sfx/hoe", "sfx/engine", "music/theme"])
        .and_clips(
            &[
                "sprites/farmer/idle",
                "sprites/farmer/walk",
                "sprites/farmer/hoe",
                "sprites/hero/idle",
                "sprites/hero/run",
                "door/open",
            ],
            &["footstep", "hoeHits"],
        )
        .and_screens(&["PauseMenu", "Hud", "DroneHealth", "Where", "Hotbar"])
        .and_actions(&["Interact", "Use", "QuickSave", "QuickLoad", "Pause", "Fire"]);
    names.pictures = vec![
        "sprites/tools/Axe".into(),
        "sprites/tools/Hoe".into(),
        "sprites/tools/Can".into(),
    ];
    names
}

/// The engine's files and the stand-in project's names, with the farm's
/// animator and tile map (which the Animators and Tile maps pages use)
/// when `code` fetches them. (Only then: with the pinned Lust, an
/// animator's or map's file that nothing in the game fetches fails the
/// check, unless the game fetches `solis/animation` or `solis/tiles`
/// itself.)
fn engine_files(code: &str) -> Vec<(String, String)> {
    let (mut files, _) = crate::describe::descriptions_for(MADE_BY, "solis", &names());
    let farmer = AnimatorNames {
        path: "farmer".into(),
        parameters: vec![
            ("moving".into(), ParamKind::Switch),
            ("speed".into(), ParamKind::Number),
            ("facing".into(), ParamKind::Direction),
            ("swing".into(), ParamKind::Trigger),
        ],
        states: vec!["Idle".into(), "Walk".into(), "UseTool".into()],
    };
    if code.contains("\"solis/animators/farmer\"") {
        files.extend(animator_file(MADE_BY, "solis", &farmer).0);
    }
    let farm = MapNames {
        path: "maps/farm".into(),
        layers: vec!["Ground".into()],
        terrains: vec![
            (b'.', "grass".into()),
            (b't', "tilled".into()),
            (b's', "soil".into()),
        ],
        values: vec![
            ("watered".into(), ValueNames::Switch),
            ("growth".into(), ValueNames::Number),
            (
                "crop".into(),
                ValueNames::Choice(vec!["Parsnip".into(), "Wheat".into()]),
            ),
        ],
    };
    if code.contains("\"solis/maps/maps/farm\"") {
        files.extend(map_file(MADE_BY, "solis", &farm).0);
    }
    files
}

/// The file an example names on its first line (`-- hero.lst: …`).
fn named(code: &str) -> Option<String> {
    let first = code.lines().next()?.trim().strip_prefix("--")?.trim();
    let name = first.split([':', ' ']).next()?;
    name.ends_with(".lst").then(|| name.to_string())
}

/// The screen's name, when the example is a screen (`assembly Hud {`).
fn screen(code: &str) -> Option<String> {
    code.lines()
        .find_map(|l| l.trim_start().strip_prefix("assembly "))
        .and_then(|rest| rest.split([' ', '{']).next())
        .map(str::to_string)
}

/// The game the Game screens page's screens read: its clock, the drone's
/// health, the pause menu's events, and Sparks, a Lua component.
const SCREENS_GAME: &str = "\
object Clock {
    has {
        int day = 1;
        float hour = 6;
        float energy = 100;
    }

    method update(float dt) {
        hour = hour + dt;
    }
}

component Health {
    has {
        float hp = 20;
    }

    method float fraction() {
        return hp / 20;
    }
}

event Resumed();
event Quit();
pass Clock, Health, Resumed, Quit;
end
";

const SCREENS_LUA: &str =
    "local Sparks = component(\"Sparks\", {\n    { \"shown\", \"int\", 0 },\n})\n";

/// The names a file passes (`pass Hero, Coin;`).
fn passed(code: &str) -> Vec<String> {
    code.lines()
        .filter_map(|l| l.trim().strip_prefix("pass "))
        .flat_map(|rest| rest.trim_end_matches(';').split(','))
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .collect()
}

/// A Lust example as a program: the engine's files and the example under
/// its own name, fetched from a `main.lst` of one line when it isn't
/// `main.lst`; a named one with the page's other named files beside it.
/// A screen is put in `ui/`, with a stand-in game for it to read.
fn program(example: &Fenced, page: &[&Fenced]) -> Code {
    let screen = screen(&example.code);
    let own = named(&example.code).unwrap_or_else(|| match &screen {
        Some(name) => format!("ui/{}{}.lst", name[..1].to_ascii_lowercase(), &name[1..]),
        None => "main.lst".into(),
    });
    let mut files = engine_files(&example.code);
    let mut lua = Vec::new();
    if screen.is_some() {
        files.push(("main.lst".into(), SCREENS_GAME.into()));
        lua.push(("sparks.lua".into(), SCREENS_LUA.into()));
    } else if named(&example.code).is_some() {
        // A named file goes with the page's other named files; a later
        // one of the same name is the same file grown.
        for other in page {
            if let Some(name) = named(&other.code).filter(|n| *n != own) {
                files.retain(|(p, _)| *p != name);
                files.push((name, other.code.clone()));
            }
        }
    }
    files.retain(|(p, _)| *p != own);
    files.push((own.clone(), example.code.clone()));
    if !files.iter().any(|(p, _)| p == "main.lst") {
        let main = match own.strip_suffix(".lst") {
            Some(module) => format!(
                "fetch \"{module}\":[{}];\nend\n",
                passed(&example.code).join(", ")
            ),
            None => "end\n".to_string(),
        };
        files.push(("main.lst".into(), main));
    }
    files.sort();
    Code {
        files,
        lua,
        notes: Vec::new(),
    }
}

fn first_line(code: &str) -> &str {
    code.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
}

#[test]
fn the_manual_has_examples_in_each_language() {
    let all = fenced();
    for language in ["lust", "rust", "lua"] {
        let count = all.iter().filter(|f| f.language == language).count();
        assert!(count >= 20, "only {count} {language} examples were found");
    }
}

#[test]
fn every_lust_example_in_the_manual_is_checked_by_lust() {
    let all = fenced();
    let lust: Vec<&Fenced> = all.iter().filter(|f| f.language == "lust").collect();
    let mut wrong = Vec::new();
    for example in &lust {
        let page: Vec<&Fenced> = lust
            .iter()
            .copied()
            .filter(|f| f.page == example.page)
            .collect();
        if let Err(problems) = compile(&program(example, &page)) {
            wrong.push(format!(
                "{}.md, the Lust example starting \"{}\": {}",
                example.page,
                first_line(&example.code),
                problems[0]
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_lua_example_in_the_manual_compiles_and_starts() {
    let mut wrong = Vec::new();
    for example in fenced().iter().filter(|f| f.language == "lua") {
        let code = Code {
            files: engine_files(""),
            lua: vec![("example.lua".into(), example.code.clone())],
            notes: Vec::new(),
        };
        if let Err(problems) = compile(&code) {
            wrong.push(format!(
                "{}.md, the Lua example starting \"{}\": {}",
                example.page,
                first_line(&example.code),
                problems[0]
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_lust_example_with_a_mistake_is_caught() {
    let wrong = Fenced {
        page: "test".into(),
        language: "lust".into(),
        code: "component Mover {\n    method update(float dt) {\n        moveTo(position() + 1);\n    }\n}\npass Mover;\nend".into(),
    };
    assert!(compile(&program(&wrong, &[])).is_err());
    let unknown = Fenced {
        code: "fetch \"solis/world\":World;\ncomponent A {\n    method update(float dt) {\n        World:fly();\n    }\n}\npass A;\nend".into(),
        ..wrong
    };
    assert!(compile(&program(&unknown, &[])).is_err());
}

#[test]
fn a_lua_example_with_a_mistake_is_caught() {
    for broken in [
        "function A:update(dt)\n  if then\nend",
        "local A = component(\"A\", { { \"x\", \"nope\", 1 } })",
    ] {
        let code = Code {
            files: engine_files(""),
            lua: vec![("example.lua".into(), broken.into())],
            notes: Vec::new(),
        };
        assert!(compile(&code).is_err(), "{broken}");
    }
}

#[test]
fn a_named_file_is_checked_under_its_name_with_main_fetching_it() {
    let hero = Fenced {
        page: "p".into(),
        language: "lust".into(),
        code: "-- hero.lst: the player.\ncomponent Hero {\n}\npass Hero;\nend".into(),
    };
    let code = program(&hero, &[&hero]);
    let main = code.files.iter().find(|(p, _)| p == "main.lst").unwrap();
    assert_eq!(main.1, "fetch \"hero\":[Hero];\nend\n");
    assert!(code.files.iter().any(|(p, _)| p == "hero.lst"));
    assert!(compile(&code).is_ok());
}
