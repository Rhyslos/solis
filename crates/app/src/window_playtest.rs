//! The window as an exported game ([decision 0093](../../../docs/decisions/0093-the-playtest-export.md)):
//! the project beside the program opened, its area played as soon as its
//! code is ready, only the game drawn, the window following the player's
//! settings, saves and settings kept in the player's Saved Games folder,
//! and what the game said written to a log there. And, in the editor,
//! File > Export for playtesting… carried out on a thread of its own.

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
    /// How many of the Console's lines were written to the log.
    logged: usize,
    /// The last lines said, for a crash log.
    pub(super) recent: Arc<Mutex<Vec<String>>>,
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
            self.open_area(&area);
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
        // Starts once the code is read; with mistakes in it, says so.
        let checking = self.game_code.waiting();
        let waiting = match self.workbench.code_mistakes() {
            Some(why) if !checking => NotPlaying::Cannot(format!(
                "The game can't start: its code has mistakes.\n\n{why}"
            )),
            _ => NotPlaying::Starting,
        };
        let ready = !checking && self.workbench.code_ready() && !self.importer.waiting();
        if let Some(run) = self.playtest.as_mut()
            && !run.started
            && ready
        {
            run.started = true;
            self.workbench.start_play_only(&ctx);
        }
        let renderer: &mut dyn editor::ViewportRenderer = match self.gpu.as_mut() {
            Some(gpu) => gpu,
            None => &mut self.no_gpu,
        };
        let actions = self.workbench.show_play_only(ui, renderer, &waiting);
        self.game_audio
            .frame(&mut self.workbench, &self.sounds, self.sound.device(), &ctx);
        self.queued.extend(actions);
        self.run_queue(&ctx);
        self.follow_settings(&ctx);
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
            // points.
            let ppp = ctx.pixels_per_point().max(0.25);
            let [w, h] = settings.resolution;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                w as f32 / ppp,
                h as f32 / ppp,
            )));
        }
        if was.is_some()
            && let Some(saved) = &run.saved
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
        let lines = self.workbench.log_lines();
        if run.logged > lines.len() {
            // The Console let its oldest lines go.
            run.logged = lines.len().saturating_sub(1);
        }
        if run.logged == lines.len() {
            return;
        }
        let new: Vec<String> = lines[run.logged..]
            .iter()
            .map(|l| format!("{} {:?}: {}", l.time, l.level, l.text))
            .collect();
        run.logged = lines.len();
        if let Ok(mut recent) = run.recent.lock() {
            recent.extend(new.iter().cloned());
            let extra = recent.len().saturating_sub(KEPT_LINES);
            recent.drain(..extra);
        }
        let Some(saved) = &run.saved else {
            return;
        };
        let worth: Vec<&String> = new.iter().filter(|l| !l.contains(" Info: ")).collect();
        if worth.is_empty() {
            return;
        }
        use std::io::Write;
        let _ = std::fs::create_dir_all(saved).and_then(|()| {
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

    /// File > Export for playtesting…'s Export: copies on a thread of its
    /// own, so the editor keeps drawing.
    pub(super) fn export_playtest(&mut self, request: editor::export_dialog::ExportRequest) {
        if self.exporting.is_some() {
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
            let outcome = playtest::export(&root, &program, &request.playtest, &request.folder);
            if outcome.is_ok() {
                let _ = playtest::remember_export(&root, &request.playtest);
            }
            let _ = sender.send((request.playtest, outcome));
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
                editor::export_dialog::Playtest::default(),
                Err("the export stopped unexpectedly".into()),
            ),
        };
        self.exporting = None;
        let (playtest, outcome) = outcome;
        if outcome.is_ok() {
            self.workbench.set_last_export(Some(playtest));
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

/// Opens the exported game `found` in a window of its own, and runs until
/// it closes. A crash writes a crash log in the player's folder.
pub fn run_playtest(info: editor::AppInfo, found: Found) -> Result<(), String> {
    super::start_jobs();
    let name = found.playtest.name.clone();
    let saved = playtest::saved_games(&name);
    let settings = playtest::load_settings(saved.as_deref(), &found.playtest);
    let crash_folder = saved.clone();
    let program = format!("{} {}", info.name, info.version);
    let game = name.clone();
    let earlier = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        if let Some(folder) = &crash_folder {
            let said = RECENT.lock().map(|r| r.clone()).unwrap_or_default();
            let about = format!(
                "{game} stopped unexpectedly. Please send this file to the game's maker.\n\n\
                 What happened: {panic}\nMade with: {program}"
            );
            let _ = playtest::write_crash_log(folder, &about, &said);
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
                    Vec::new(),
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
                why.push_str(&format!(
                    " A crash log is in {}.",
                    folder.join(playtest::CRASH_FILE).display()
                ));
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
