//! The launcher's side of the window (decision 0118): the one window
//! shows the launcher or the editor, each at its own size and place;
//! New project's folders are looked at on workers and its project made;
//! File › Close project goes back to the launcher; a project's kind is
//! kept in its file. A child module of `window.rs`.
//!
//! **One window, two places.** The launcher and the editor are drawn in
//! the same native window, which takes the launcher's size and place
//! while it shows and the editor's while a project is open (each
//! remembered on this computer). Two real windows would each need their
//! own surface on the graphics card and their own copy of what the
//! editor uploads there; and closing the first window eframe opens ends
//! the program, so the launcher's would have to stay open, hidden, all
//! the time the editor works. One window keeps one GPU device, one sound
//! card, one update being downloaded, with nothing hidden.

use super::App;
use crate::cli::Sample;
use editor::{FolderState, LogLevel, NewProjectPlan, Starter};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};

/// Where a window was last left, on this computer: its inner size, and
/// its outer corner if known (centred otherwise).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(super) struct WindowPlace {
    /// Its inside's size, in points.
    pub(super) size: [f32; 2],
    /// Its top-left corner on the screen, in points.
    #[serde(default)]
    pub(super) at: Option<[f32; 2]>,
}

impl WindowPlace {
    /// A window of `size`, centred.
    pub(super) fn centred(size: [f32; 2]) -> Self {
        Self { size, at: None }
    }

    /// `saved` if it's a sensible place for a window that must be at
    /// least `min` (a damaged or hand-edited settings file can hold
    /// anything); `first` otherwise. A corner that isn't a number is
    /// dropped, and the window centred.
    pub(super) fn checked(saved: Option<Self>, min: [f32; 2], first: [f32; 2]) -> Self {
        let Some(saved) = saved else {
            return Self::centred(first);
        };
        let size_ok = saved
            .size
            .iter()
            .zip(min)
            .all(|(v, min)| v.is_finite() && *v >= min && *v <= 100_000.0);
        if !size_ok {
            return Self::centred(first);
        }
        let at = saved
            .at
            .filter(|at| at.iter().all(|v| v.is_finite() && v.abs() <= 100_000.0));
        Self { at, ..saved }
    }

    /// Where its corner goes on a screen of `monitor` points: where it was
    /// left if the window's top and a good part of it are still on the
    /// screen (a screen unplugged since leaves it off), centred otherwise.
    pub(super) fn corner_on(&self, monitor: egui::Vec2) -> egui::Pos2 {
        let size = egui::vec2(self.size[0], self.size[1]);
        let centred = ((monitor - size) / 2.0).max(egui::Vec2::ZERO);
        let Some([x, y]) = self.at else {
            return centred.to_pos2();
        };
        const SEEN: f32 = 120.0;
        let on_screen =
            y >= 0.0 && y + SEEN <= monitor.y && x + SEEN <= monitor.x && x + size.x >= SEEN;
        if on_screen {
            egui::pos2(x, y)
        } else {
            centred.to_pos2()
        }
    }
}

/// New project's questions about folders, each answered on a worker:
/// what is at the folder its name makes.
pub(super) struct FolderLooks {
    sender: Sender<(PathBuf, FolderState)>,
    answers: Receiver<(PathBuf, FolderState)>,
    /// Questions asked and not yet answered.
    waiting: usize,
}

impl Default for FolderLooks {
    fn default() -> Self {
        let (sender, answers) = channel();
        Self {
            sender,
            answers,
            waiting: 0,
        }
    }
}

impl FolderLooks {
    /// Looks at `folder` on a worker.
    fn ask(&mut self, folder: PathBuf) {
        let sender = self.sender.clone();
        let started = std::thread::Builder::new()
            .name("folder look".into())
            .spawn(move || {
                let state = folder_state(&folder);
                let _ = sender.send((folder, state));
            });
        if started.is_ok() {
            self.waiting += 1;
        }
    }

    /// The answers that have come in.
    fn answered(&mut self) -> Vec<(PathBuf, FolderState)> {
        let answers: Vec<_> = self.answers.try_iter().collect();
        self.waiting = self.waiting.saturating_sub(answers.len());
        answers
    }

    /// Whether an answer is still to come.
    pub(super) fn waiting(&self) -> bool {
        self.waiting > 0
    }
}

/// What is at `folder`: nothing, an empty folder, a folder with things
/// in it, or a file. A link is never taken for an empty folder.
pub(super) fn folder_state(folder: &Path) -> FolderState {
    match std::fs::symlink_metadata(folder) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => FolderState::Free,
        Err(e) => FolderState::Unknown(e.to_string()),
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => FolderState::AFile,
        Ok(_) => match std::fs::read_dir(folder).map(|mut entries| entries.next().is_none()) {
            Ok(true) => FolderState::Empty,
            Ok(false) => FolderState::Taken,
            Err(e) => FolderState::Unknown(e.to_string()),
        },
    }
}

/// What making a sample takes: its maker, the level it opens on and what
/// the Console says once it's ready.
struct SampleParts {
    make: fn(&Path) -> Result<(), asset_db::ProjectError>,
    level: &'static str,
    what: &'static str,
    ready: &'static str,
}

fn sample_parts(sample: Sample) -> SampleParts {
    match sample {
        Sample::Game => SampleParts {
            make: crate::sample_game::make,
            level: crate::sample_game::LEVEL,
            what: "the sample game",
            ready: "The sample game is ready. Press Play: the turret shoots at the drone, and \
                    the Console tells the story. Its code is in the Library's Code tab.",
        },
        Sample::Ui => SampleParts {
            make: crate::ui_sample::make,
            level: crate::ui_sample::LEVEL,
            what: "the UI sample",
            ready: "The UI sample is ready. Press Play, click the Game panel, and press Space to \
                    fire and P to pause. Double-click a screen in the Library's ui folder to see \
                    it on the canvas.",
        },
        Sample::Playground => SampleParts {
            make: crate::playground::make,
            level: crate::playground::LEVEL,
            what: "the playground",
            ready: "The playground is ready. Press Play and click the Game panel: W A S D walk, \
                    the mouse looks, Space jumps, Shift sprints, F6 saves and F7 loads. Esc \
                    gives the editor the mouse back.",
        },
        Sample::Benchmark => SampleParts {
            make: crate::benchmark::make,
            level: crate::benchmark::FIRST_LEVEL,
            what: "the benchmark",
            ready: "The benchmark is ready. Press Play and open the Profiler: target.solarea is \
                    2,000 ships and 20,000 lasers, stress.solarea 10,000 and 100,000, and \
                    ceiling.solarea adds ships until 60 frames a second ends, then says where \
                    in the Console.",
        },
    }
}

/// The sample a starter makes, if it's one.
fn sample_of(starter: Starter) -> Option<Sample> {
    match starter {
        Starter::Empty => None,
        Starter::Playground => Some(Sample::Playground),
        Starter::TurretSample => Some(Sample::Game),
        Starter::UiSample => Some(Sample::Ui),
        Starter::Benchmark => Some(Sample::Benchmark),
    }
}

/// The folder New project first makes projects in: one named after the
/// program in the Documents folder (`branding`, never the name typed).
pub(super) fn first_create_in() -> PathBuf {
    crate::playtest::documents()
        .unwrap_or_default()
        .join(crate::branding::projects_folder())
}

impl App {
    /// A failure the person must see: in the Console, and on the
    /// launcher while it shows (it has no Console).
    pub(super) fn say_failed(&mut self, text: String) {
        if self.workbench.launcher_shown() {
            self.workbench.projects_notice(text.clone());
        }
        self.workbench.log(LogLevel::Error, text);
    }

    /// Makes `sample` in `folder` as a new project and opens it on its
    /// level, or says why it couldn't.
    pub(super) fn make_sample(&mut self, sample: Sample, folder: &Path) {
        let parts = sample_parts(sample);
        if let Err(why) = (parts.make)(folder) {
            self.say_failed(format!(
                "Couldn't make {} in {}: {why}.",
                parts.what,
                folder.display()
            ));
            return;
        }
        self.open_sample(&parts, folder);
    }

    fn open_sample(&mut self, parts: &SampleParts, folder: &Path) {
        self.open_project(folder, false);
        let level = folder.join(asset_db::ASSETS_FOLDER).join(parts.level);
        self.open_level(&level);
        self.workbench.log(LogLevel::Info, parts.ready);
    }

    /// New project's Create: makes the folder and the project in it (an
    /// empty one, or a sample under the name typed), of the kind chosen,
    /// and opens it. A name or folder that won't do is refused with why,
    /// in the dialog, which stays open; nothing already there is ever
    /// written over.
    pub(super) fn create_project(&mut self, plan: NewProjectPlan) {
        if let Some(why) = editor::launcher::name_problem(&plan.name) {
            self.workbench.new_project_failed(why);
            return;
        }
        if let Some(why) = folder_state(&plan.folder).problem() {
            self.workbench.new_project_failed(why);
            return;
        }
        let kind = crate::known_projects::file_kind(plan.kind);
        let made = std::fs::create_dir_all(&plan.folder)
            .map_err(|e| e.to_string())
            .and_then(|()| match sample_of(plan.starter) {
                None => asset_db::Project::create_kind(&plan.folder, &plan.name, kind)
                    .map(drop)
                    .map_err(|e| e.to_string()),
                Some(sample) => (sample_parts(sample).make)(&plan.folder)
                    .and_then(|()| asset_db::tidy::rename(&plan.folder, &plan.name).map(drop))
                    .and_then(|()| asset_db::Project::open(&plan.folder))
                    .and_then(|mut project| project.set_kind(kind).map(drop))
                    .map_err(|e| e.to_string()),
            });
        if let Err(why) = made {
            self.workbench.new_project_failed(format!(
                "Couldn't make it in {}: {why}.",
                plan.folder.display()
            ));
            return;
        }
        // Remembered for the next project.
        self.create_in = plan.create_in.clone();
        self.workbench.set_create_in(plan.create_in);
        match sample_of(plan.starter) {
            None => self.open_project(&plan.folder, false),
            Some(sample) => self.open_sample(&sample_parts(sample), &plan.folder),
        }
    }

    /// File › Close project (or the editor's window closed), once
    /// unsaved work was asked about: everything of the project goes, and
    /// the launcher comes back.
    pub(super) fn close_project(&mut self) {
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.clear_models();
            gpu.clear_sheets();
        }
        self.thumbnails.clear(self.gpu.as_ref());
        self.sounds.clear(self.sound.device());
        self.sprites.clear();
        self.game_code.forget();
        self.to_place.clear();
        self.level_files.forget();
        self.projects.close();
        self.workbench.set_project(None);
        self.workbench.set_sounds(self.sounds.states());
        self.workbench.set_sound_events(self.sounds.events());
        self.workbench.open_launcher();
        self.list_projects();
    }

    /// The launcher's Open folder…: a project's folder opens; any other
    /// folder is looked through for projects to add to the list (what
    /// Scan a folder… did).
    pub(super) fn open_folder(&mut self, folder: PathBuf) {
        match asset_db::Project::summary(&folder) {
            Err(asset_db::ProjectError::NotAProject(_)) if folder.is_dir() => {
                self.scan_for_projects(folder);
            }
            _ => self.open_project(&folder, false),
        }
    }

    /// Project settings › Kind: kept in the project's file.
    pub(super) fn set_project_kind(&mut self, kind: editor::ProjectKind) {
        let file = crate::known_projects::file_kind(kind);
        if let Err(why) = self.projects.set_kind(file) {
            self.workbench.log(
                LogLevel::Error,
                format!("Couldn't keep the project's kind: {why}."),
            );
        }
    }

    /// New project asked what is at `folder`.
    pub(super) fn look_at_folder(&mut self, folder: PathBuf) {
        self.folder_looks.ask(folder);
    }

    /// Hands the editor the answers about folders that have come in.
    pub(super) fn follow_folder_looks(&mut self) {
        for (folder, state) in self.folder_looks.answered() {
            self.workbench.set_folder_state(folder, state);
        }
    }

    /// Gives the window the place of what it shows (the launcher's or the
    /// editor's) whenever that changes, and remembers where each is left
    /// (not while maximized: the place to come back to). Right after a
    /// change the window still reports the old place for a few frames,
    /// so nothing is remembered until it has taken the new size.
    pub(super) fn fit_window(&mut self, ctx: &egui::Context) {
        let launcher = self.workbench.launcher_shown();
        if self.placed_as != Some(launcher) {
            self.placed_as = Some(launcher);
            let (place, min) = if launcher {
                (self.launcher_place, editor::tokens::launcher::MIN_WINDOW)
            } else {
                (self.editor_place, super::MIN_SIZE)
            };
            if ctx.input(|i| i.viewport().maximized) == Some(true) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                min[0], min[1],
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                place.size[0],
                place.size[1],
            )));
            if let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                    place.corner_on(monitor),
                ));
            }
            self.settling = 60;
            return;
        }
        let (inner, outer, maximized) = ctx.input(|i| {
            let v = i.viewport();
            (v.inner_rect, v.outer_rect, v.maximized)
        });
        let Some(inner) = inner else {
            return;
        };
        let place = if launcher {
            &mut self.launcher_place
        } else {
            &mut self.editor_place
        };
        if self.settling > 0 {
            let taken = (inner.width() - place.size[0]).abs() < 2.0
                && (inner.height() - place.size[1]).abs() < 2.0;
            self.settling = if taken { 0 } else { self.settling - 1 };
            ctx.request_repaint();
            return;
        }
        let min = if launcher {
            editor::tokens::launcher::MIN_WINDOW
        } else {
            super::MIN_SIZE
        };
        if maximized != Some(true) && inner.width() >= min[0] && inner.height() >= min[1] {
            place.size = [inner.width(), inner.height()];
            if let Some(outer) = outer {
                place.at = Some([outer.left(), outer.top()]);
            }
        }
    }
}

#[cfg(test)]
#[path = "window_launcher_tests.rs"]
mod tests;
