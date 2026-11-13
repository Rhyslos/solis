//! The Problems panel's lists: the area's problems, found again only
//! when something they depend on changed, and the code's, from its last
//! check, with the notes the compiler gave.

use super::Workbench;
use crate::console::LogLevel;
use reflect::AssetId;
use std::collections::HashSet;

impl Workbench {
    /// Looks for problems in the area again, if anything they depend on
    /// changed.
    pub(crate) fn find_problems(&mut self) {
        use crate::problems::{Fingerprint, Sources};
        let print = Fingerprint::of(&self.open.document, self.sources);
        self.open.problems.refresh_between(
            self.open.document.revision(),
            self.open.pixels.pixels_per_unit,
            self.open.document.area(),
            &self.viewport.area.sprites,
        );
        if self.open.problems.is_from(&print) {
            return;
        }
        let known: HashSet<AssetId> = self.open.library.iter().filter_map(|i| i.id).collect();
        let assemblies: HashSet<AssetId> = self.open.assemblies.keys().copied().collect();
        let tile_gaps = self.viewport.sheets.tile_gaps();
        self.open.problems.refresh(
            print,
            &Sources {
                area: self.open.document.area(),
                registry: &self.open.document.registry,
                models: &self.open.models,
                known: &known,
                assemblies: &assemblies,
                tile_gaps: &tile_gaps,
            },
        );
    }

    /// Takes in the result of checking the game's code (the app checks it
    /// on a worker when a project opens and when a code file changes): its
    /// mistakes go to the Problems panel, and the Console says how it went.
    /// `notes` are files left out (too big, not text).
    pub fn set_code_problems(&mut self, problems: Vec<script::CodeProblem>, notes: &[String]) {
        for note in notes {
            self.log(LogLevel::Warning, note.clone());
        }
        let text = match problems.len() {
            0 if !self.open.code_problems.is_empty() => Some((
                LogLevel::Info,
                "The game's code has no mistakes now.".to_string(),
            )),
            0 => None,
            1 => Some((
                LogLevel::Warning,
                format!("A mistake in the game's code: {}", problems[0]),
            )),
            n => Some((
                LogLevel::Warning,
                format!("{n} mistakes in the game's code; they're listed in Problems."),
            )),
        };
        if let Some((level, text)) = text {
            self.log(level, text);
        }
        self.open.code_problems = problems;
        self.open.code_checked = Some(chrono::Local::now().format("%H:%M:%S").to_string());
    }

    /// Mistakes in the game's code, from the last check.
    pub fn code_problems(&self) -> &[script::CodeProblem] {
        &self.open.code_problems
    }

    /// The code's mistakes, then pictures its screens name that can't be
    /// shown (decision 0089): everything Problems lists from the code.
    pub(crate) fn all_code_problems(&self) -> std::borrow::Cow<'_, [script::CodeProblem]> {
        let pictures = self.picture_problems();
        if pictures.is_empty() {
            std::borrow::Cow::Borrowed(&self.open.code_problems)
        } else {
            std::borrow::Cow::Owned([&self.open.code_problems[..], pictures].concat())
        }
    }

    /// The problems found in the area, for the app and tests.
    pub fn problems(&self) -> &[crate::problems::Problem] {
        &self.open.problems.list
    }
}
