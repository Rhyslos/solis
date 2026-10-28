//! A whole project checked without the editor's window (Phase 8, C4;
//! [decision 0053](../../../docs/decisions/0053-compatibility-before-2d.md)):
//! `solis --check-project <folder>`. Before an update ships, a project is
//! opened as a new build would open it:
//!
//! 1. every file Solis reads is read (an older one upgraded, as opening
//!    it would), and any that fails is reported;
//! 2. the game's code is checked;
//! 3. each recording in `recordings/checks/` is played back in the area it
//!    names, and must match on every tick.
//!
//! The report is plain text; anything that changed is a failure, and the
//! program exits with a failure code, so a workflow can stop the update.

use crate::play_files::{RECORDING_EXTENSION, RECORDINGS};
use reflect::upgrade::Versioned;
use std::path::{Path, PathBuf};

/// The folder of recordings a check plays back, inside `recordings/`.
pub const CHECKS: &str = "checks";

/// The most files a check reads (a project bigger than this is reported).
const MAX_FILES: usize = 100_000;

/// The most ticks a playback runs past its recording's length before it's
/// given up (it should end on its last tick).
const SPARE_TICKS: u64 = 60;

/// What a check found.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    /// Each line of the report, in order.
    pub lines: Vec<String>,
    /// How many things failed.
    pub failed: usize,
}

impl Report {
    fn ok(&mut self, line: impl Into<String>) {
        self.lines.push(format!("  ok    {}", line.into()));
    }

    fn note(&mut self, line: impl Into<String>) {
        self.lines.push(format!("  note  {}", line.into()));
    }

    fn fail(&mut self, line: impl Into<String>) {
        self.failed += 1;
        self.lines.push(format!("  FAIL  {}", line.into()));
    }

    fn heading(&mut self, line: impl Into<String>) {
        self.lines.push(String::new());
        self.lines.push(line.into());
    }

    /// The report as text, ending with a line saying how it went.
    pub fn text(&self) -> String {
        let mut text = self.lines.join("\n");
        text.push_str("\n\n");
        text.push_str(&if self.failed == 0 {
            "The project checks out: nothing changed.\n".to_string()
        } else {
            format!(
                "{} thing{} failed: this build changes the project. Don't ship it until \
                 that's understood.\n",
                self.failed,
                if self.failed == 1 { "" } else { "s" }
            )
        });
        text
    }
}

/// Checks the project at `root` as `made_by` (the program and version
/// the engine's description files name).
pub fn check(root: &Path, made_by: &str) -> Report {
    let mut report = Report::default();
    report.lines.push(format!("Checking {}", root.display()));
    let project = match asset_db::Project::open(root) {
        Ok(project) => project,
        Err(why) => {
            report.fail(format!("it can't be opened as a project: {why}"));
            return report;
        }
    };
    report.ok(format!("the project \"{}\" opens", project.name()));
    report.heading("Files");
    check_files(root, &mut report);
    report.heading("Game code");
    let program = check_code(&project.assets(), made_by, &mut report);
    report.heading("Recordings");
    check_recordings(root, &project, program, made_by, &mut report);
    report
}

/// A reader for a kind of file: its format, and how it's read (an error
/// says why it couldn't be).
type Reader = fn(&str) -> Result<(), String>;

/// The reader for `path`, by its name, and the most of it read (the
/// editor's own limit for that kind of file).
fn reader_for(path: &Path) -> Option<(&'static dyn Versioned, Reader, u64)> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    let extension = name.rsplit_once('.').map_or("", |(_, e)| e);
    Some(match extension {
        "solarea" | "solassembly" => (
            &area::text::FORMAT,
            |text| {
                let registry = area::builtin::registry().map_err(|e| e.to_string())?;
                area::text::read_file(text, &registry)
                    .map(drop)
                    .map_err(|e| e.to_string())
            },
            area::text::MAX_BYTES as u64,
        ),
        "solsound" => (
            &sound::event::FORMAT,
            |text| {
                sound::event::read(text)
                    .map(drop)
                    .map_err(|e| e.to_string())
            },
            sound::event::MAX_BYTES as u64,
        ),
        "solmix" => (
            &sound::mixer::FORMAT,
            |text| {
                sound::mixer::read(text)
                    .map(drop)
                    .map_err(|e| e.to_string())
            },
            sound::mixer::MAX_BYTES as u64,
        ),
        "solinput" => (
            &input::BINDINGS_FORMAT,
            |text| {
                input::Bindings::read(text)
                    .map(drop)
                    .map_err(|e| e.to_string())
            },
            1 << 20,
        ),
        "solrec" => (
            &input::RECORDING_FORMAT,
            |text| {
                input::Recording::read(text)
                    .map(drop)
                    .map_err(|e| e.to_string())
            },
            input::MAX_RECORDING_BYTES as u64,
        ),
        "solmeta" => (
            &asset_db::SIDECAR_FORMAT,
            |text| asset_db::Sidecar::parse(text).map(drop),
            asset_db::sidecar::MAX_BYTES,
        ),
        "solsave" => (
            &editor::SAVE_FORMAT,
            |text| editor::save_read_back(text).map(drop),
            crate::play_files::MAX_SAVE_BYTES,
        ),
        "solis" if name == asset_db::PROJECT_FILE => (
            &asset_db::PROJECT_FORMAT,
            |text| {
                asset_db::Project::read_file(text)
                    .map(drop)
                    .map_err(|e| e.to_string())
            },
            64 << 10,
        ),
        _ => return None,
    })
}

/// Reads every file Solis reads in the project (not its cache, Git's
/// folder or exported games).
fn check_files(root: &Path, report: &mut Report) {
    let mut files = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            report.fail(format!("{} can't be listed", shown(root, &folder)));
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let skipped = folder == root
                && matches!(
                    entry.file_name().to_str(),
                    Some(asset_db::CACHE_FOLDER | ".git" | "builds")
                );
            // Links aren't followed: a project's files are inside it.
            if kind.is_dir() && !skipped {
                folders.push(path);
            } else if kind.is_file() && reader_for(&path).is_some() {
                files.push(path);
            }
        }
        if files.len() > MAX_FILES {
            report.fail(format!("it has more than {MAX_FILES} files to read"));
            return;
        }
    }
    files.sort();
    let (mut read, mut older) = (0, 0);
    for path in &files {
        let Some((format, reader, limit)) = reader_for(path) else {
            continue;
        };
        let text = match asset_db::read_limited(path, limit) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    report.fail(format!("{}: it isn't text", shown(root, path)));
                    continue;
                }
            },
            Err(why) => {
                report.fail(format!("{}: {why}", shown(root, path)));
                continue;
            }
        };
        match reader(&text) {
            Ok(()) => read += 1,
            Err(why) => {
                report.fail(format!("{}: {why}", shown(root, path)));
                continue;
            }
        }
        if let Some(found) = format.version_of(&text)
            && found < format.current()
        {
            older += 1;
            report.note(format!(
                "{} is {} format {found}: it opens, upgraded to format {}",
                shown(root, path),
                format.name(),
                format.current()
            ));
        }
    }
    report.ok(format!(
        "{read} of {} files read{}",
        files.len(),
        match older {
            0 => String::new(),
            n => format!(" ({n} from an older version, upgraded as they're read)"),
        }
    ));
}

/// Checks the game's code, as the editor would on opening. The program,
/// when it has code and that code has no mistakes.
fn check_code(
    assets: &Path,
    made_by: &str,
    report: &mut Report,
) -> Option<std::sync::Arc<script::Compiled>> {
    for note in crate::game_code::GameCode::describe(assets, made_by) {
        report.note(note);
    }
    let code = script::gather(assets);
    for note in &code.notes {
        report.note(note.clone());
    }
    match script::compile(&code) {
        Ok(None) => {
            report.ok("no game code");
            None
        }
        Ok(Some(compiled)) => {
            for old in script::renamed_uses(&code) {
                report.note(old.to_string());
            }
            report.ok(format!(
                "the game's code checks ({} component{})",
                compiled.schemas.len(),
                if compiled.schemas.len() == 1 { "" } else { "s" }
            ));
            Some(std::sync::Arc::new(compiled))
        }
        Err(problems) => {
            for problem in problems {
                report.fail(problem.to_string());
            }
            None
        }
    }
}

/// Plays back each recording in `recordings/checks/`.
fn check_recordings(
    root: &Path,
    project: &asset_db::Project,
    program: Option<std::sync::Arc<script::Compiled>>,
    made_by: &str,
    report: &mut Report,
) {
    let folder = root.join(RECORDINGS).join(CHECKS);
    let mut recordings: Vec<PathBuf> = std::fs::read_dir(&folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        // Ordinary files only: a link could bring a recording in from
        // outside the project.
        .filter(|p| {
            std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file())
                && p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case(RECORDING_EXTENSION))
        })
        .collect();
    recordings.sort();
    if recordings.is_empty() {
        report.note(format!(
            "no recordings to play back (keep some in {RECORDINGS}/{CHECKS}/ to check that the \
             game still plays the same)"
        ));
        return;
    }
    let (db, _, _) = asset_db::AssetDb::open(project.clone(), assets::Limits::default());
    for path in recordings {
        let name = shown(root, &path);
        match play_back(root, &db, program.clone(), made_by, &path) {
            Ok(ticks) => report.ok(format!("{name} plays back the same on all {ticks} ticks")),
            Err(why) => report.fail(format!("{name}: {why}")),
        }
    }
}

/// Plays one recording back in the area it names, with nothing but the
/// project's files. The ticks it matched on, or why it didn't.
fn play_back(
    root: &Path,
    db: &asset_db::AssetDb,
    program: Option<std::sync::Arc<script::Compiled>>,
    made_by: &str,
    path: &Path,
) -> Result<usize, String> {
    let text = asset_db::read_limited(path, input::MAX_RECORDING_BYTES as u64)
        .map_err(|e| e.to_string())
        .and_then(|b| String::from_utf8(b).map_err(|_| "it isn't text".to_string()))?;
    let recording = input::Recording::read(&text).map_err(|e| e.to_string())?;
    let area_path = recording.area.clone().ok_or(
        "it doesn't say which area it was recorded in (it was made before recordings did): \
         record it again",
    )?;
    let mut w = workbench_for(root, db, program, made_by, &area_path)?;
    w.replay(&text, &path.display().to_string());
    // The area must start as it did when recorded: otherwise the build
    // isn't what changed.
    if w.world()
        .is_some_and(|world| world.checksum() != recording.start)
    {
        return Err(format!(
            "its area, {area_path}, isn't as it was when this was recorded (changed since, or \
             recorded with unsaved changes): record it again"
        ));
    }
    let ticks = recording.ticks.len() as u64;
    let mut ran = 0;
    let mut saved = Saves::default();
    while w.replayed().is_none() && ran < ticks + SPARE_TICKS && w.world().is_some() {
        w.play_for(1.0 / 60.0);
        ran += 1;
        saved.answer(&mut w, root)?;
    }
    match w.replayed() {
        Some(Ok(ticks)) => Ok(ticks),
        Some(Err(tick)) => Err(format!(
            "it stopped playing back the same by tick {tick} ({:.2} s in): the game comes out \
             differently from the same input",
            tick as f64 / 60.0
        )),
        None => Err("it didn't play back to its end".into()),
    }
}

/// Saves game code makes while a check plays a recording back: kept here,
/// never written into the project; loads read them, or the project's own.
#[derive(Default)]
pub(crate) struct Saves(std::collections::HashMap<String, String>);

impl Saves {
    /// Answers what the editor asked for since last time: saves are kept,
    /// loads are made from them (or from the project's `saves` folder).
    pub(crate) fn answer(&mut self, w: &mut editor::Workbench, root: &Path) -> Result<(), String> {
        for action in w.take_outbox() {
            match action {
                editor::WorkbenchAction::SaveGame { slot, text } => {
                    self.0.insert(slot, text);
                }
                editor::WorkbenchAction::LoadGame(slot) => {
                    let text = match self.0.get(&slot) {
                        Some(text) => Some(text.clone()),
                        None => crate::play_files::save_path(root, &slot).and_then(|path| {
                            crate::play_files::read_limited(
                                &path,
                                crate::play_files::MAX_SAVE_BYTES,
                            )
                            .ok()
                            .flatten()
                        }),
                    };
                    match text {
                        Some(text) => w.load_game_from_code(&text, &slot),
                        None => return Err(format!("it loads the save {slot}, which isn't there")),
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// A workbench with the project at `root` open as the editor would have
/// it, headless: its code, input actions, assemblies and Library, and the
/// area at `area_path` (inside the assets, `/` between folders).
pub(crate) fn workbench_for(
    root: &Path,
    db: &asset_db::AssetDb,
    program: Option<std::sync::Arc<script::Compiled>>,
    made_by: &str,
    area_path: &str,
) -> Result<editor::Workbench, String> {
    let assets = db.project().assets();
    let area_file = assets.join(area_path);
    if !input::Recording::is_plain_area_path(area_path) || !area_file.starts_with(&assets) {
        return Err(format!("its area, {area_path}, isn't in the assets"));
    }
    let mut w = editor::Workbench::new(
        editor::AppInfo {
            name: crate::branding::APP_NAME.to_string(),
            version: made_by.to_string(),
        },
        None,
    );
    w.set_project(Some(editor::ProjectInfo {
        name: db.project().name().to_string(),
        root: root.to_path_buf(),
        assets: assets.clone(),
    }));
    // The sample's Rust component, in that project only (decision 0029),
    // as the editor gives it.
    w.set_rust_components(crate::sample_game::rust_components(&assets).0);
    w.set_program(program);
    w.set_bindings(crate::play_files::load_bindings(root).0);
    let (items, folders) = crate::project::Projects::library(&db.listing(), |_| None);
    // Every assembly, so game code can spawn any of them.
    for item in &items {
        if item.kind == editor::LibraryKind::Assembly
            && let Some(id) = item.id
            && let Some(file) = db.file(id)
        {
            // One the editor can't read is left out, as the editor does (the
            // Files step reports it); only a game that uses it fails.
            if let Ok(read) = crate::area_file::AreaFiles::read(&file, &w.document().registry) {
                w.set_assembly(id, std::sync::Arc::new(read));
            }
        }
    }
    w.set_library(items);
    w.set_folders(folders);
    let file = crate::area_file::AreaFiles::read(&area_file, &w.document().registry)
        .map_err(|why| format!("its area, {area_path}: {why}"))?;
    let area = w.expand_file(&file).map_err(|e| e.to_string())?;
    w.open_area(area, area_file);
    Ok(w)
}

/// `path` inside `root`, with `/` between folders.
fn shown(root: &Path, path: &Path) -> String {
    crate::project::slashed(path.strip_prefix(root).unwrap_or(path))
}

#[cfg(test)]
#[path = "check_project_tests.rs"]
mod tests;
