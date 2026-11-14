//! The window as an exported game ([decision 0093](../../../docs/decisions/0093-the-playtest-export.md)):
//! the project beside the program opened, its area played as soon as its
//! code is ready, only the game drawn, the window following the player's
//! settings, saves and settings kept in the player's Saved Games folder,
//! and what the game said written to a log there. And, in the editor,
//! Build > Export carried out on a thread of its own.

use super::{App, file_name};
use crate::playtest::{self, Found};
use editor::LogLevel;
use editor::play_only::NotPlaying;
use eframe::egui;
use script::settings::PlayerSettings;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// The most lines kept for the crash log.
const KEPT_LINES: usize = 40;

/// The log of what the game said, beside the player's saves.
pub(crate) const LOG_FILE: &str = "log.txt";
/// The log before, once the log grows past [`MAX_LOG_BYTES`].
const OLD_LOG_FILE: &str = "log.old.txt";
/// How big the log grows before it's started again.
const MAX_LOG_BYTES: u64 = 1 << 20;

/// An export that finished: the project it came from, what it was, and
/// how it went.
pub(super) type ExportDone = (
    PathBuf,
    editor::export_dialog::Playtest,
    Result<playtest::Exported, String>,
);

/// An exported game, as it runs.
pub(super) struct Run {
    found: Found,
    /// The player's own folder: saves, settings, the log.
    saved: Option<PathBuf>,
    /// Whether the project and area were opened.
    opened: bool,
    /// Whether play started.
    started: bool,
    /// The settings the window was last made to follow.
    applied: Option<PlayerSettings>,
    /// How many of the Console's lines were written to the log (counting
    /// every line ever added, `Workbench::log_since`).
    logged: u64,
    /// The last lines said, for a crash log.
    pub(super) recent: Arc<Mutex<Vec<String>>>,
    /// Why the game can't go on, once it can't (its files can't be read,
    /// or play stopped without the game asking to quit).
    failed: Option<String>,
    /// Whether the settings changed since they were last kept, and when
    /// that was.
    unsaved: bool,
    last_saved: Option<std::time::Instant>,
    /// Log lines not written yet, and when the log was last written.
    log_waiting: Vec<String>,
    log_written: Option<std::time::Instant>,
}

impl App {
    /// Becomes the exported game `found`.
    pub(super) fn start_playtest(&mut self, found: Found) {
        let saved = playtest::saved_games(&found.playtest.name);
        let settings = playtest::load_settings(saved.as_deref(), &found.playtest);
        self.workbench.set_play_only();
        self.workbench.set_player_settings(settings);
        self.playtest = Some(Run {
            found,
            saved,
            opened: false,
            started: false,
            applied: None,
            logged: 0,
            recent: RECENT.clone(),
            failed: None,
            unsaved: false,
            last_saved: None,
            log_waiting: Vec::new(),
            log_written: None,
        });
    }

    /// Where save games go: the player's Saved Games folder for an
    /// exported game, or the open project.
    pub(super) fn saves_root(&self) -> Option<PathBuf> {
        match &self.playtest {
            Some(run) => run.saved.clone(),
            None => self.projects.root().map(std::path::Path::to_path_buf),
        }
    }

    /// One frame of an exported game.
    pub(super) fn playtest_frame(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let Some(run) = self.playtest.as_mut() else {
            return;
        };
        if !run.opened {
            run.opened = true;
            let root = run.found.root.clone();
            let area = root
                .join(asset_db::ASSETS_FOLDER)
                .join(&run.found.playtest.area);
            self.open_project(&root, false);
            if self.projects.root().is_some() {
                self.open_area(&area);
            }
            // Either failing is said in the Console, which the player
            // can't see: the window says it instead.
            let opened = self.projects.root().is_some()
                && self.workbench.document().file() == Some(area.as_path());
            if !opened {
                let why = self.last_problem();
                if let Some(run) = self.playtest.as_mut() {
                    run.failed = Some(format!("The game's files can't be read. {why}"));
                }
            }
        }
        let (changes, notes) = self.projects.poll();
        self.follow_changes(changes, notes);
        if self.projects.take_changed() {
            self.refresh_library();
        }
        if let Some(checked) = self.game_code.poll() {
            self.workbench
                .set_code_problems(checked.problems, &checked.notes);
            if let Some(program) = checked.program {
                self.workbench.set_program(program);
            }
        }
        for note in self.game_code.take_notes() {
            self.workbench.log(LogLevel::Warning, note);
        }
        self.collect_imports();
        if self.sounds.poll() {
            self.workbench.set_sounds(self.sounds.states());
            self.workbench.set_sound_events(self.sounds.events());
        }
        if let Some(wanted) = self.workbench.take_wanted_pictures()
            && let Some(assets) = self.projects.db().map(|db| db.project().assets())
        {
            self.sprites.want_pictures(wanted, &assets);
        }
        self.follow_sprites();
        self.workbench.set_gamepad(self.gamepads.read());
        // Starts once the code is read, and the tile maps and pictures
        // read on workers have arrived (game code's names come from them);
        // with mistakes in the code, says so.
        let checking = self.game_code.waiting() || self.sprites.waiting();
        let failed = self.playtest.as_ref().and_then(|r| r.failed.clone());
        let waiting = match (failed, self.workbench.code_mistakes()) {
            (Some(why), _) => NotPlaying::Cannot(why),
            (None, Some(why)) if !checking => NotPlaying::Cannot(format!(
                "The game can't start: its code has mistakes.\n\n{why}"
            )),
            _ => NotPlaying::Starting,
        };
        let ready = !checking && self.workbench.code_ready() && !self.importer.waiting();
        if let Some(run) = self.playtest.as_mut()
            && !run.started
            && run.failed.is_none()
            && ready
        {
            run.started = true;
            self.workbench.start_play_only(&ctx);
        }
        // Play stopped without the game asking to quit: say why.
        let stopped = self.workbench.play_state() == editor::PlayState::Editing
            && !self.workbench.quit_asked();
        if stopped
            && self
                .playtest
                .as_ref()
                .is_some_and(|r| r.started && r.failed.is_none())
        {
            let why = self.last_problem();
            if let Some(run) = self.playtest.as_mut() {
                run.failed = Some(format!("The game stopped. {why}"));
            }
        }
        let renderer: &mut dyn editor::ViewportRenderer = match self.gpu.as_mut() {
            Some(gpu) => {
                gpu.begin_frame();
                gpu
            }
            None => &mut self.no_gpu,
        };
        let actions = self.workbench.show_play_only(ui, renderer, &waiting);
        self.game_audio
            .frame(&mut self.workbench, &self.sounds, self.sound.device(), &ctx);
        self.queued.extend(actions);
        self.run_queue(&ctx);
        self.follow_settings(&ctx);
        let closing = self.quitting || ctx.input(|i| i.viewport().close_requested());
        self.keep_settings(closing);
        if closing {
            self.flush_log(true);
        }
        self.write_log();
        let title = self
            .playtest
            .as_ref()
            .map(|r| r.found.playtest.name.clone())
            .unwrap_or_default();
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            self.quitting = true;
        }
        if !self.workbench.wants_frames(&ctx) {
            ctx.request_repaint_after(super::WAITING_FRAME);
        }
    }

    /// The last warning or mistake the Console has, for a player to pass
    /// on, and where the log is.
    fn last_problem(&self) -> String {
        let said = self
            .workbench
            .log_lines()
            .iter()
            .rev()
            .find(|l| l.level != LogLevel::Info)
            .map(|l| l.text.clone())
            .unwrap_or_default();
        let log = self
            .playtest
            .as_ref()
            .and_then(|r| r.saved.as_ref())
            .map(|s| format!("\n\nMore in {}.", s.join(LOG_FILE).display()))
            .unwrap_or_default();
        format!("{said}{log}")
    }

    /// Makes the window follow the player's settings, and keeps them in
    /// their folder, when they change.
    fn follow_settings(&mut self, ctx: &egui::Context) {
        let settings = self.workbench.player_settings();
        let Some(run) = self.playtest.as_mut() else {
            return;
        };
        if run.applied == Some(settings) {
            return;
        }
        let was = run.applied.replace(settings);
        let fullscreen_changed = was.is_none_or(|w| w.fullscreen != settings.fullscreen);
        if fullscreen_changed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(settings.fullscreen));
        }
        let size_changed = was.is_none_or(|w| w.resolution != settings.resolution);
        if !settings.fullscreen && (size_changed || fullscreen_changed) {
            // The setting is in the screen's pixels; the window's size in
            // points, and never bigger than the screen.
            let ppp = ctx.pixels_per_point().max(0.25);
            let [w, h] = settings.resolution;
            let mut size = egui::vec2(w as f32 / ppp, h as f32 / ppp);
            if let Some(screen) = ctx.input(|i| i.viewport().monitor_size)
                && screen.x > 0.0
                && screen.y > 0.0
            {
                size = size.min(screen);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }
        if was.is_some() {
            run.unsaved = true;
        }
    }

    /// Keeps the player's settings in their folder, at most once a second
    /// (game code changing them every tick doesn't write every frame), or
    /// now when `now` (the game closing).
    pub(super) fn keep_settings(&mut self, now: bool) {
        let settings = self.workbench.player_settings();
        let Some(run) = self.playtest.as_mut() else {
            return;
        };
        let due = run
            .last_saved
            .is_none_or(|t| t.elapsed() >= std::time::Duration::from_secs(1));
        if !run.unsaved || !(now || due) {
            return;
        }
        run.unsaved = false;
        run.last_saved = Some(std::time::Instant::now());
        if let Some(saved) = &run.saved
            && let Err(why) = playtest::save_settings(saved, &settings)
        {
            let note = format!("The player's settings couldn't be kept: {why}.");
            self.workbench.log(LogLevel::Warning, note);
        }
    }

    /// Writes what the game said since last time (warnings and mistakes)
    /// to its log, and keeps the last lines for a crash log.
    fn write_log(&mut self) {
        let Some(run) = self.playtest.as_mut() else {
            return;
        };
        let (lines, seen) = self.workbench.log_since(run.logged);
        if seen == run.logged {
            return;
        }
        let new: Vec<String> = lines
            .iter()
            .map(|l| format!("{} {:?}: {}", l.time, l.level, l.text))
            .collect();
        run.logged = seen;
        if let Ok(mut recent) = run.recent.lock() {
            recent.extend(new.iter().cloned());
            let extra = recent.len().saturating_sub(KEPT_LINES);
            recent.drain(..extra);
        }
        run.log_waiting
            .extend(new.into_iter().filter(|l| !l.contains(" Info: ")));
        self.flush_log(false);
    }

    /// Writes the waiting log lines to the log: at most once a second (a
    /// game that warns every tick doesn't touch the disk every frame; the
    /// phase review), or now when `now` (the game closing).
    pub(super) fn flush_log(&mut self, now: bool) {
        let Some(run) = self.playtest.as_mut() else {
            return;
        };
        let due = run
            .log_written
            .is_none_or(|t| t.elapsed() >= std::time::Duration::from_secs(1));
        if run.log_waiting.is_empty() || !(now || due) {
            return;
        }
        run.log_written = Some(std::time::Instant::now());
        let worth = std::mem::take(&mut run.log_waiting);
        let Some(saved) = &run.saved else {
            return;
        };
        use std::io::Write;
        let _ = std::fs::create_dir_all(saved).and_then(|()| {
            // Kept small: a game that warns every tick doesn't fill the
            // disk. The one before is kept as log.old.txt.
            let path = saved.join(LOG_FILE);
            if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_LOG_BYTES) {
                std::fs::rename(&path, saved.join(OLD_LOG_FILE))?;
            }
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(saved.join(LOG_FILE))?;
            for line in worth {
                writeln!(file, "{line}")?;
            }
            Ok(())
        });
    }

    /// Build > Export's Export: copies on a thread of its
    /// own, so the editor keeps drawing.
    pub(super) fn export_playtest(&mut self, request: editor::export_dialog::ExportRequest) {
        if self.exporting.is_some() {
            self.workbench.export_finished(Err(
                "an export is already running: wait for it to finish".into(),
            ));
            return;
        }
        let Some(root) = self.projects.root().map(std::path::Path::to_path_buf) else {
            self.workbench
                .export_finished(Err("no project is open".into()));
            return;
        };
        let program = match std::env::current_exe() {
            Ok(p) => p,
            Err(why) => {
                self.workbench
                    .export_finished(Err(format!("the program's own file can't be found: {why}")));
                return;
            }
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // The first look at the folder, now that Export is pressed.
            let folder = playtest::folder_for(&request);
            let outcome = playtest::export(&root, &program, &request.playtest, &folder);
            if outcome.is_ok() {
                let _ = playtest::remember_export(&root, &request.playtest);
            }
            let _ = sender.send((root, request.playtest, outcome));
        });
        self.exporting = Some(receiver);
    }

    /// An export that finished: said in the Console and the window.
    pub(super) fn follow_export(&mut self) {
        let Some(receiver) = &self.exporting else {
            return;
        };
        let outcome = match receiver.try_recv() {
            Ok(outcome) => outcome,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => (
                PathBuf::new(),
                editor::export_dialog::Playtest::default(),
                Err("the export stopped unexpectedly".into()),
            ),
        };
        self.exporting = None;
        let (root, playtest, outcome) = outcome;
        // Remembered by the project it was exported from, if that's still
        // the open one (on disk, it's remembered either way). Where it
        // went is this computer's to remember, in the program's settings.
        let same = self
            .projects
            .root()
            .is_some_and(|open| super::same_folder(open, &root));
        if let Ok(made) = &outcome {
            self.projects.known_mut().exported(&root, &made.folder);
            if same {
                self.workbench.set_last_export(Some(playtest));
                self.workbench
                    .set_last_export_folder(Some(made.folder.clone()));
            }
        }
        if let Ok(made) = &outcome {
            self.workbench.log(
                LogLevel::Info,
                format!(
                    "The export holds {} of the project's files and the program, {} MB in all, in \
                     {}.",
                    made.files,
                    made.bytes >> 20,
                    file_name(&made.folder)
                ),
            );
        }
        self.workbench.export_finished(outcome.map(|m| m.folder));
    }
}

/// The last lines an exported game said, for its crash log: shared with
/// the panic hook.
static RECENT: std::sync::LazyLock<Arc<Mutex<Vec<String>>>> =
    std::sync::LazyLock::new(Arc::default);

/// The last panic on the window's thread, for the crash log.
static LAST_PANIC: Mutex<Option<String>> = Mutex::new(None);

/// Opens the exported game `found` in a window of its own, and runs until
/// it closes. A crash writes a crash log in the player's folder.
pub fn run_playtest(info: editor::AppInfo, found: Found) -> Result<(), String> {
    super::start_jobs();
    let name = found.playtest.name.clone();
    let saved = playtest::saved_games(&name);
    let settings = playtest::load_settings(saved.as_deref(), &found.playtest);
    let program = format!("{} {}", info.name, info.version);
    // Panics that are caught and carried on from (a model that can't be
    // read, say) aren't crashes: the hook only notes the window's thread's
    // last one, and the crash log is written below if the window stopped.
    let earlier = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        if std::thread::current().name() == Some("main")
            && let Ok(mut last) = LAST_PANIC.try_lock()
        {
            *last = Some(panic.to_string());
        }
        earlier(panic);
    }));
    let [w, h] = settings.resolution;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(name.clone())
            .with_app_id(id_of(&name))
            .with_inner_size([w as f32, h as f32])
            .with_min_inner_size([320.0, 240.0])
            .with_fullscreen(settings.fullscreen)
            .with_icon(super::window_icon()),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: super::gpu_options(),
        // The window opens as the player's settings say, not as eframe
        // last saw it.
        persist_window: false,
        ..Default::default()
    };
    let pending = crate::updates::PendingSlot::default();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        eframe::run_native(
            &id_of(&name),
            options,
            Box::new(move |cc| {
                Ok(Box::new(App::new(
                    cc,
                    info,
                    crate::cli::Launch::default(),
                    pending,
                    Some(found),
                )))
            }),
        )
    }));
    match outcome {
        Ok(result) => result.map_err(|err| err.to_string()),
        Err(payload) => {
            let mut why = super::explain_panic(payload.as_ref());
            if let Some(folder) = &saved {
                let said = RECENT.try_lock().map(|r| r.clone()).unwrap_or_default();
                let what = LAST_PANIC
                    .try_lock()
                    .ok()
                    .and_then(|mut l| l.take())
                    .unwrap_or_else(|| why.clone());
                let about = format!(
                    "{name} stopped unexpectedly. Please send this file to the game's maker.\n\n\
                     What happened: {what}\nMade with: {program}"
                );
                if let Ok(path) = playtest::write_crash_log(folder, &about, &said) {
                    why.push_str(&format!(" A crash log is in {}.", path.display()));
                }
            }
            Err(why)
        }
    }
}

/// The game's name as a window and settings identifier: small letters and
/// digits, `-` between words.
pub(super) fn id_of(name: &str) -> String {
    let id: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let id = id.trim_matches('-').to_owned();
    if id.is_empty() { "game".into() } else { id }
}

#[cfg(test)]
#[path = "window_playtest_tests.rs"]
mod tests;
