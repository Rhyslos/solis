//! The user guide (decision 0112): which program shows it, and asking
//! the app to open it. The guide itself is a file the app finds beside
//! the program; the editor only says when and where to open it.
//!
//! Which browser is the user's choice (File › Preferences › General ›
//! Open the guide in), saved by the app: the default browser, one the
//! app found installed, or a program the user picked.

use crate::workbench::Workbench;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The program the guide opens in.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GuideBrowser {
    /// The browser Windows opens web pages in (the standard).
    #[default]
    Default,
    /// A browser Windows lists as installed: its name and its program.
    Installed {
        /// Its name, as Windows lists it.
        name: String,
        /// Its program file.
        program: PathBuf,
    },
    /// A program the user picked.
    Chosen(PathBuf),
}

/// A browser the app found installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundBrowser {
    /// Its name, as Windows lists it.
    pub name: String,
    /// Its program file.
    pub program: PathBuf,
}

/// What the editor knows about the guide's browser.
#[derive(Debug, Clone, Default)]
pub(crate) struct GuideSettings {
    /// The choice, as saved.
    pub(crate) browser: GuideBrowser,
    /// The browsers found installed, once the app has looked.
    pub(crate) found: Vec<FoundBrowser>,
}

/// One line of the Open the guide in list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuideChoice {
    /// The default browser.
    Default,
    /// A browser of the list, by its place in it.
    Listed(usize),
    /// The program picked before.
    Chosen,
    /// Pick a program.
    Pick,
}

/// The words for picking a program.
pub(crate) const PICK_WORDS: &str = "A program you choose…";

impl GuideSettings {
    /// The browsers to list: those found, and the one chosen from the
    /// list before even if it wasn't found this time (it may still be
    /// there; opening it says if not).
    pub(crate) fn listed(&self) -> Vec<FoundBrowser> {
        let mut listed = self.found.clone();
        if let GuideBrowser::Installed { name, program } = &self.browser
            && !listed.iter().any(|b| &b.program == program)
        {
            listed.push(FoundBrowser {
                name: name.clone(),
                program: program.clone(),
            });
        }
        listed
    }

    /// The list's lines with their words, and the line chosen now.
    pub(crate) fn choices(&self) -> (Vec<(GuideChoice, String)>, GuideChoice) {
        let listed = self.listed();
        let mut choices = vec![(GuideChoice::Default, "Default browser".to_string())];
        choices.extend(
            listed
                .iter()
                .enumerate()
                .map(|(i, b)| (GuideChoice::Listed(i), b.name.clone())),
        );
        if let GuideBrowser::Chosen(program) = &self.browser {
            choices.push((GuideChoice::Chosen, program_name(program)));
        }
        choices.push((GuideChoice::Pick, PICK_WORDS.to_string()));
        let now = match &self.browser {
            GuideBrowser::Default => GuideChoice::Default,
            GuideBrowser::Installed { program, .. } => listed
                .iter()
                .position(|b| &b.program == program)
                .map_or(GuideChoice::Default, GuideChoice::Listed),
            GuideBrowser::Chosen(_) => GuideChoice::Chosen,
        };
        (choices, now)
    }

    /// Takes the line chosen; returns whether the app must ask for a
    /// program instead.
    pub(crate) fn choose(&mut self, choice: GuideChoice) -> bool {
        match choice {
            GuideChoice::Default => self.browser = GuideBrowser::Default,
            GuideChoice::Listed(i) => {
                if let Some(b) = self.listed().get(i) {
                    self.browser = GuideBrowser::Installed {
                        name: b.name.clone(),
                        program: b.program.clone(),
                    };
                }
            }
            GuideChoice::Chosen => {}
            GuideChoice::Pick => return true,
        }
        false
    }
}

/// A program's name to show: its file's name without `.exe`, whichever
/// way its path's folders are written.
fn program_name(program: &Path) -> String {
    let text = program.to_string_lossy();
    let file = text.rsplit(['/', '\\']).next().unwrap_or(&text);
    let name = match file.len().checked_sub(4) {
        Some(at) if file.is_char_boundary(at) && file[at..].eq_ignore_ascii_case(".exe") => {
            &file[..at]
        }
        _ => file,
    };
    if name.is_empty() {
        text.into_owned()
    } else {
        name.to_string()
    }
}

impl Workbench {
    /// The program the guide opens in, as chosen (the app saves it).
    pub fn guide_browser(&self) -> &GuideBrowser {
        &self.guide.browser
    }

    /// Sets the program the guide opens in: as saved, or as just picked.
    pub fn set_guide_browser(&mut self, browser: GuideBrowser) {
        self.guide.browser = browser;
    }

    /// The browsers the app found installed, for Preferences' list.
    pub fn set_found_browsers(&mut self, found: Vec<FoundBrowser>) {
        self.guide.found = found;
    }

    /// Says `words` in the status bar for a moment, as for a click on an
    /// unfinished part: what became of opening the guide, say.
    pub fn say_in_status(&self, ctx: &egui::Context, words: impl Into<String>) {
        crate::shell::status::tell(ctx, words);
    }
}

#[cfg(test)]
#[path = "guide_tests.rs"]
mod tests;
