//! `solis --play` (Fixes 4, T1): a level played without a window, for game
//! makers and a game's own checks. It plays the start level, or the one
//! named, for a number of ticks, with a script of input if one is given
//! (`input::Script`: `at 60 press Jump`), and prints what the Console
//! says with its tick, then the values of the components asked for on
//! every entity that has one. Any error the Console says fails it.

use crate::check_project::{Read, Report, Saves, check_code, workbench_for};
use crate::cli::Play;
use editor::LogLevel;
use std::fmt::Write as _;

/// How long a play with no ticks named and no script lasts: ten seconds.
const DEFAULT_TICKS: u32 = 600;

/// What a play printed, and whether it went without an error.
pub struct Played {
    /// What to print.
    pub text: String,
    /// Whether nothing went wrong (the code checked, the level opened,
    /// and the Console said no error).
    pub ok: bool,
}

/// Plays `asked`, with `made_by` the program's version.
pub fn play(asked: &Play, made_by: &str) -> Played {
    let mut out = String::new();
    let failed = |mut out: String, why: String| {
        out.push_str(&why);
        out.push('\n');
        Played {
            text: out,
            ok: false,
        }
    };
    let project = match asset_db::Project::open(&asked.folder) {
        Ok(p) => p,
        Err(why) => return failed(out, format!("The project can't be opened: {why}")),
    };
    let script = match &asked.inputs {
        None => None,
        Some(file) => match std::fs::read_to_string(file)
            .map_err(|e| e.to_string())
            .and_then(|text| input::Script::read(&text))
        {
            Ok(script) => Some(script),
            Err(why) => {
                return failed(
                    out,
                    format!("The script {} can't be read: {why}", file.display()),
                );
            }
        },
    };
    // The game's code, checked as the project check checks it.
    let mut checked = Report::default();
    let program = check_code(&project, made_by, &mut checked);
    if checked.failed > 0 {
        return failed(out, checked.text());
    }
    let (db, _, _) = asset_db::AssetDb::open(project.clone(), assets::Limits::default());
    let levels = crate::first_level::levels(&db.listing());
    let level = match &asked.level {
        Some(level) => level.clone(),
        None => match crate::first_level::start_level(&asked.folder, &levels) {
            Some(level) => level,
            None => return failed(out, "The project has no level to play.".to_string()),
        },
    };
    let mut w = match workbench_for(
        &asked.folder,
        &db,
        program,
        made_by,
        &level,
        &mut Read::default(),
    ) {
        Ok(w) => w,
        Err(why) => return failed(out, format!("{level} can't be played: {why}")),
    };
    let ticks = asked.ticks.unwrap_or_else(|| {
        script
            .as_ref()
            .map_or(DEFAULT_TICKS, |s| s.last_tick().saturating_add(2))
    });
    let _ = writeln!(
        out,
        "Playing {level} for {ticks} tick{}{}",
        if ticks == 1 { "" } else { "s" },
        asked
            .inputs
            .as_ref()
            .map(|f| format!(", with {}", f.display()))
            .unwrap_or_default()
    );
    let mut seen = 0;
    let mut errors = 0;
    let mut said = |w: &editor::Workbench, tick: u32, out: &mut String, errors: &mut usize| {
        let (lines, now) = w.log_since(seen);
        seen = now;
        for line in lines {
            let kind = match line.level {
                LogLevel::Error => {
                    *errors += 1;
                    "error"
                }
                LogLevel::Warning => "warning",
                _ => "said",
            };
            let _ = writeln!(out, "tick {tick}, {kind}: {}", line.text);
        }
    };
    said(&w, 0, &mut out, &mut errors);
    w.play();
    if let Some(script) = script {
        for unknown in w.play_script(script) {
            let _ = writeln!(
                out,
                "The script names {unknown}, which isn't one of the project's actions: it does \
                 nothing."
            );
        }
    }
    let mut saves = Saves::default();
    let mut ran = 0;
    while ran < ticks && w.world().is_some() {
        w.play_for(1.0 / 60.0);
        ran += 1;
        said(&w, ran, &mut out, &mut errors);
        if let Err(why) = saves.answer(&mut w, &asked.folder) {
            errors += 1;
            let _ = writeln!(out, "tick {ran}, error: {why}");
        }
    }
    if ran < ticks {
        let _ = writeln!(out, "The game stopped by itself after {ran} ticks.");
    }
    print_components(&mut w, &asked.print, ran, &mut out);
    Played {
        text: out,
        ok: errors == 0,
    }
}

/// The values of each component in `names` on every entity that has one,
/// in the level's order, as the game has them now.
fn print_components(w: &mut editor::Workbench, names: &[String], ran: u32, out: &mut String) {
    if names.is_empty() {
        return;
    }
    let _ = writeln!(out, "After {ran} ticks:");
    for name in names {
        let holders: Vec<(reflect::EntityId, String)> = w
            .document()
            .level()
            .iter()
            .filter(|(_, e)| e.components.contains_key(name))
            .map(|(id, e)| (id, e.name.clone()))
            .collect();
        if holders.is_empty() {
            let _ = writeln!(out, "  no entity has a {name}");
        }
        for (id, entity) in holders {
            let values = w
                .values_now(id, name)
                .map(|data| {
                    data.fields
                        .iter()
                        .map(|(field, value)| format!("{field} = {}", shown(value)))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let _ = writeln!(out, "  {name} on \"{entity}\" ({id}): {values}");
        }
    }
}

/// A value as game code would write it.
fn shown(value: &reflect::Value) -> String {
    match value {
        reflect::Value::Bool(b) => b.to_string(),
        reflect::Value::Int(i) => i.to_string(),
        reflect::Value::Float(f) => f.to_string(),
        reflect::Value::Text(t) => format!("{t:?}"),
        reflect::Value::List(items) => {
            format!(
                "[{}]",
                items.iter().map(shown).collect::<Vec<_>>().join(", ")
            )
        }
        reflect::Value::Entity(Some(id)) => id.to_string(),
        reflect::Value::Entity(None) => "nothing".to_string(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
#[path = "play_headless_tests.rs"]
mod tests;
