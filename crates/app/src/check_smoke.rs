//! The project check's `--smoke <ticks>` (Fixes 1, T2): the game's start
//! level ([`crate::first_level::start_level`]) played headless for a
//! number of ticks with nothing pressed, as Play would play it, its
//! screens too. Anything the Console says that's an error or a warning
//! fails the check, with the tick it was said on: a game whose screens
//! can't start, or whose code stops on its first tick, is found before
//! anyone presses Play.

use crate::check_project::{Read, Report, Saves, workbench_for};
use editor::LogLevel;
use std::path::Path;

/// The most different messages listed: the rest are counted.
const MAX_LISTED: usize = 50;

/// Plays the project at `root`'s start level for `ticks` ticks into
/// `report`.
pub(crate) fn smoke(
    root: &Path,
    project: &asset_db::Project,
    program: Option<std::sync::Arc<script::Compiled>>,
    made_by: &str,
    ticks: u32,
    report: &mut Report,
) {
    let (db, _, _) = asset_db::AssetDb::open(project.clone(), assets::Limits::default());
    let levels = crate::first_level::levels(&db.listing());
    let Some(level) = crate::first_level::start_level(root, &levels) else {
        report.note("no level to play: the project has none");
        return;
    };
    let mut w = match workbench_for(root, &db, program, made_by, &level, &mut Read::default()) {
        Ok(w) => w,
        Err(why) => {
            report.fail(format!("{level} can't be played: {why}"));
            return;
        }
    };
    let mut said = Said::default();
    // What opening the level said, then Play.
    let mut seen = said.take(&w, 0, 0);
    w.play();
    let mut saves = Saves::default();
    let mut ran = 0;
    while ran < ticks && w.world().is_some() {
        w.play_for(1.0 / 60.0);
        ran += 1;
        seen = said.take(&w, seen, ran);
        if let Err(why) = saves.answer(&mut w, root) {
            said.add(ran, LogLevel::Error, &why);
        }
    }
    if said.lines.is_empty() {
        report.ok(format!(
            "{level} plays for {ran} tick{} with nothing wrong in the Console",
            if ran == 1 { "" } else { "s" }
        ));
    }
    for line in said.listed() {
        report.fail(line);
    }
    if ran < ticks {
        report.note(format!("play stopped by itself after {ran} of {ticks} ticks"));
    }
}

/// The errors and warnings the Console said, each different one once with
/// the first tick it was said on and how often.
#[derive(Default)]
struct Said {
    /// (tick, level, message, times), in the order first said.
    lines: Vec<(u32, LogLevel, String, usize)>,
}

impl Said {
    /// Takes what the Console said since `seen` messages, on `tick`; the
    /// count to pass next time.
    fn take(&mut self, w: &editor::Workbench, seen: u64, tick: u32) -> u64 {
        let (lines, now) = w.log_since(seen);
        for line in lines {
            if line.level != LogLevel::Info {
                self.add(tick, line.level, &line.text);
            }
        }
        now
    }

    fn add(&mut self, tick: u32, level: LogLevel, text: &str) {
        match self.lines.iter_mut().find(|l| l.1 == level && l.2 == text) {
            Some(line) => line.3 += 1,
            None => self.lines.push((tick, level, text.to_string(), 1)),
        }
    }

    /// The report's lines: each message with its tick.
    fn listed(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .lines
            .iter()
            .take(MAX_LISTED)
            .map(|(tick, level, text, times)| {
                let when = if *tick == 0 {
                    "as the level opened".to_string()
                } else {
                    format!("tick {tick}")
                };
                let kind = if *level == LogLevel::Error {
                    "error"
                } else {
                    "warning"
                };
                let again = match times {
                    1 => String::new(),
                    n => format!(" (said {n} times)"),
                };
                format!("{when}, {kind}: {text}{again}")
            })
            .collect();
        if self.lines.len() > MAX_LISTED {
            out.push(format!(
                "and {} more different messages",
                self.lines.len() - MAX_LISTED
            ));
        }
        out
    }
}

#[cfg(test)]
#[path = "check_smoke_tests.rs"]
mod tests;
