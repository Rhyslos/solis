//! Opening the user guide (decision 0112): the page the Windows build
//! puts beside the program as `user-guide/index.html`, in the program
//! chosen in File › Preferences.
//!
//! What to do is worked out first ([`plan`]: which program, which
//! address, what to say), with whether a file exists asked through a
//! function, so every case is tested anywhere; [`open`] then does it.
//! The default browser is reached the way the editor opens every web
//! address (egui's `open_url`); a chosen program is started with the
//! guide's address.

use editor::{GuideBrowser, LogLevel, Workbench};
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};

/// The guide's folder beside the program.
pub(crate) const FOLDER: &str = "user-guide";
/// The guide's file in it.
pub(crate) const FILE: &str = "index.html";
/// The command that makes the guide, for a build of one's own.
const MAKE: &str = "cargo run -p user-guide -- --out";

/// Where the guide should be for the program at `program`.
pub(crate) fn beside(program: &Path) -> Option<PathBuf> {
    Some(program.parent()?.join(FOLDER).join(FILE))
}

/// How the guide opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Opening {
    /// In the default browser, at this address.
    Default(String),
    /// In this program, given this address.
    Program(PathBuf, String),
}

/// What opening the guide comes to: how, and anything to say in the
/// status bar about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Plan {
    /// How it opens.
    pub(crate) opening: Opening,
    /// Words for the status bar: why it isn't the browser chosen.
    pub(crate) note: Option<String>,
}

/// Why the guide can't open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Missing {
    /// Where it was looked for.
    pub(crate) looked: PathBuf,
}

/// Works out how to open the guide at `guide` (where it should be), at
/// `chapter` (a manual page's name), in `browser`. `is_file` says whether
/// a file is there.
pub(crate) fn plan(
    guide: &Path,
    chapter: Option<&str>,
    browser: &GuideBrowser,
    is_file: impl Fn(&Path) -> bool,
) -> Result<Plan, Missing> {
    if !is_file(guide) {
        return Err(Missing {
            looked: guide.to_path_buf(),
        });
    }
    let address = address(guide, chapter);
    let (program, name) = match browser {
        GuideBrowser::Default => {
            return Ok(Plan {
                opening: Opening::Default(address),
                note: None,
            });
        }
        GuideBrowser::Installed { name, program } => (program, name.clone()),
        GuideBrowser::Chosen(program) => (program, program_name(program)),
    };
    if usable_program(program) && is_file(program) {
        Ok(Plan {
            opening: Opening::Program(program.clone(), address),
            note: None,
        })
    } else {
        Ok(Plan {
            opening: Opening::Default(address),
            note: Some(format!(
                "{name} isn't there any more: the guide opened in the default browser"
            )),
        })
    }
}

/// Whether a saved program can be started at all: a full path to a
/// program file, on this computer's own drives (a network share is never
/// asked about: even looking at one sends it the user's sign-in).
pub(crate) fn usable_program(program: &Path) -> bool {
    let text = program.to_string_lossy();
    let local = program.is_absolute() && !text.starts_with("\\\\") && !text.starts_with("//");
    let is_program = cfg!(not(windows))
        || program
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("exe"));
    local && is_program && text.len() <= 1024 && !text.chars().any(char::is_control)
}

/// A program's name to say: its file's name without `.exe`.
fn program_name(program: &Path) -> String {
    program
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "The program chosen".to_string())
}

/// The guide's address: a `file:` address of `guide`, with `#chapter`.
/// A chapter is a manual page's name; anything else is left out.
pub(crate) fn address(guide: &Path, chapter: Option<&str>) -> String {
    let path = guide.to_string_lossy().replace('\\', "/");
    let path = if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    };
    let mut out = String::from("file://");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~:".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    if let Some(chapter) = chapter.filter(|c| {
        !c.is_empty()
            && c.len() <= 100
            && c.chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    }) {
        out.push('#');
        out.push_str(chapter);
    }
    out
}

/// Opens the guide at `chapter` in the browser the workbench has chosen,
/// and says in the status bar (and the Console) what went wrong.
pub(crate) fn open(ctx: &egui::Context, workbench: &mut Workbench, chapter: Option<&str>) {
    let Some(guide) = std::env::current_exe().ok().as_deref().and_then(beside) else {
        workbench.log(
            LogLevel::Warning,
            "Couldn't find where this program is, to open the user guide beside it.",
        );
        return;
    };
    match plan(&guide, chapter, workbench.guide_browser(), Path::is_file) {
        Ok(plan) => {
            if let Some(note) = &plan.note {
                workbench.say_in_status(ctx, note.clone());
                workbench.log(LogLevel::Warning, format!("{note}."));
            }
            start(ctx, workbench, plan.opening);
        }
        Err(missing) => {
            workbench.say_in_status(
                ctx,
                "The user guide isn't beside this build: the Console says how to make it",
            );
            workbench.log(
                LogLevel::Warning,
                format!(
                    "The user guide isn't beside this copy of the program ({}), as with a build \
                     of your own. Make it with: {MAKE} \"{}\"",
                    missing.looked.display(),
                    missing.looked.display()
                ),
            );
        }
    }
}

fn start(ctx: &egui::Context, workbench: &mut Workbench, opening: Opening) {
    match opening {
        Opening::Default(address) => ctx.open_url(egui::OpenUrl::new_tab(address)),
        Opening::Program(program, address) => {
            if let Err(why) = std::process::Command::new(&program).arg(&address).spawn() {
                let note = format!(
                    "{} didn't start ({why}): the guide opened in the default browser",
                    program_name(&program)
                );
                workbench.say_in_status(ctx, note.clone());
                workbench.log(LogLevel::Warning, format!("{note}."));
                ctx.open_url(egui::OpenUrl::new_tab(address));
            }
        }
    }
}

/// Settings are untrusted (rule 9): a saved choice that couldn't be a
/// program on this computer is the default again.
pub(crate) fn checked(browser: GuideBrowser) -> GuideBrowser {
    match &browser {
        GuideBrowser::Default => browser,
        GuideBrowser::Installed { name, program }
            if usable_program(program)
                && !name.is_empty()
                && name.len() <= 200
                && !name.chars().any(char::is_control) =>
        {
            browser
        }
        GuideBrowser::Chosen(program) if usable_program(program) => browser,
        _ => GuideBrowser::Default,
    }
}

/// What the app does for the guide besides opening it, on threads of
/// their own so the editor keeps drawing: looking for the installed
/// browsers once, and the Open window for choosing a program.
#[derive(Default)]
pub(crate) struct GuideWork {
    looking: Option<Receiver<Vec<editor::FoundBrowser>>>,
    picking: Option<Receiver<Option<PathBuf>>>,
}

impl GuideWork {
    /// Starts looking for the browsers Windows lists as installed.
    pub(crate) fn start_looking(&mut self) {
        self.looking = Some(crate::browsers::start_looking());
    }

    /// Shows the Open window for a program, unless it is already open.
    pub(crate) fn ask_for_program(&mut self) {
        if self.picking.is_some() {
            return;
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut window =
                rfd::FileDialog::new().set_title("Choose the program to open the user guide in");
            if cfg!(windows) {
                window = window.add_filter("Program", &["exe"]);
            }
            let _ = sender.send(window.pick_file());
        });
        self.picking = Some(receiver);
    }

    /// Hands the workbench what has come back: the browsers found, or the
    /// program chosen (one that can't be used is said, not kept).
    pub(crate) fn take_in(&mut self, ctx: &egui::Context, workbench: &mut Workbench) {
        if let Some(found) = arrived(&mut self.looking) {
            workbench.set_found_browsers(found.unwrap_or_default());
        }
        if let Some(Some(Some(program))) = arrived(&mut self.picking) {
            if usable_program(&program) {
                workbench.set_guide_browser(GuideBrowser::Chosen(program));
            } else {
                workbench.say_in_status(
                    ctx,
                    "That can't open the guide: choose a program on this computer's own drives",
                );
            }
        }
    }
}

/// What a thread sent, once it has: `Some(None)` if it ended without
/// sending.
fn arrived<T>(slot: &mut Option<Receiver<T>>) -> Option<Option<T>> {
    let receiver = slot.as_ref()?;
    match receiver.try_recv() {
        Ok(value) => {
            *slot = None;
            Some(Some(value))
        }
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => {
            *slot = None;
            Some(None)
        }
    }
}

#[cfg(test)]
#[path = "user_guide_tests.rs"]
mod tests;
