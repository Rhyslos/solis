//! The game's code, looked after on a worker (CONTRIBUTING rule 10):
//! whenever something in the project's assets changes, the files
//! describing the engine and the project's names to Lust are written again
//! (walking the assets for its assemblies, sound events and screens), and
//! then its Lust is read, checked and turned into Lua for the editor to
//! run. Only a check that found different code reports. Code that couldn't
//! be checked, because an earlier check never finished and is still
//! running (`script::try_compile`), is checked by itself when that one
//! ends.

use script::{Code, CodeProblem};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

/// How long a check may take before Solis stops waiting for it: only a
/// bug in the compiler could take this long.
const GIVE_UP: Duration = Duration::from_secs(60);

/// What a finished check found.
pub struct Checked {
    /// The mistakes, in order.
    pub problems: Vec<CodeProblem>,
    /// Files left out, and why.
    pub notes: Vec<String>,
    /// With no mistakes, the program to run (`None`: there's no code).
    /// With mistakes, nothing: the last program that worked stays.
    pub program: Option<Option<Arc<script::Compiled>>>,
}

/// What became of a check: the program or its mistakes, or that it
/// wasn't started (`script::try_compile`).
type Attempt = Result<Result<Option<script::Compiled>, Vec<CodeProblem>>, script::NotStarted>;

/// A worker's answer: the notes describing the engine gave, the code's
/// hash, what was read, and what became of its check (nothing when the
/// code is what the last reported check found: it isn't compiled again).
/// Nothing at all when the worker crashed.
type Answer = Option<(Vec<String>, u64, Code, Option<Attempt>)>;

/// How the code is checked, and whether a check that was given up on is
/// still running: `script`'s own, but for tests, which mustn't start a
/// check that never finishes.
#[derive(Clone, Copy)]
struct Checker {
    check: fn(&Code) -> Attempt,
    unfinished: fn() -> bool,
}

impl Default for Checker {
    fn default() -> Self {
        Self {
            check: script::try_compile,
            unfinished: script::check_unfinished,
        }
    }
}

/// One check at a time, and whether another is wanted after it.
#[derive(Default)]
pub struct GameCode {
    /// The check running, when it started, and the folder it checks.
    running: Option<(Receiver<Answer>, Instant, PathBuf)>,
    /// The folder to check again when the running check ends.
    again: Option<PathBuf>,
    /// The code the last reported check found, by its hash.
    last: Option<u64>,
    /// The code (by its hash) that wasn't checked because a check that
    /// was given up on is still running, and its folder: checked when
    /// that one ends.
    held: Option<(u64, PathBuf)>,
    checker: Checker,
    /// The program and version the engine's files are written by.
    made_by: String,
    /// The notes describing the engine last gave, and those not yet taken
    /// ([`GameCode::take_notes`]): each is said only when they change.
    told: Vec<String>,
    to_say: Vec<String>,
    /// The sprites' clips (`sheet/clip` paths) and cues, for the project
    /// file, and the animators, as the editor last read them.
    animation: Animation,
}

/// What the editor read of the project for game code's names: the sprites'
/// clips (`sheet/clip` paths), their cues, the animators and the tile maps.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Animation {
    /// The sprites' clips, as `sheet/clip` paths.
    pub clips: Vec<String>,
    /// The clips' events' names.
    pub cues: Vec<String>,
    /// Each animator, for its own file.
    pub animators: Vec<script::animator_code::AnimatorNames>,
    /// Each tile map, for its own file.
    pub maps: Vec<script::map_code::MapNames>,
    /// The pictures cut by Atlas Crop (`sprites/tools/Axe`).
    pub pictures: Vec<String>,
}

impl Animation {
    /// The names in the editor's sheets.
    pub fn of(sheets: &editor::sprite_sheets::SpriteSheets) -> Self {
        let (clips, cues) = sheets.clip_names();
        Self {
            clips,
            cues,
            animators: sheets.animator_names(),
            maps: sheets.map_names(),
            pictures: sheets.picture_names(),
        }
    }
}

/// Takes away the animators' or maps' files in `assets/solis/<kind>/` that
/// `files` no longer has (one deleted or renamed). Only `.lst` files, never
/// through a link.
fn forget_animator_files(assets: &Path, files: &[(String, String)], kind: &str) {
    let folder = assets.join(crate::branding::CODE_FOLDER).join(kind);
    let wanted: std::collections::HashSet<std::path::PathBuf> =
        files.iter().map(|(p, _)| assets.join(p)).collect();
    let mut folders = vec![folder];
    while let Some(folder) = folders.pop() {
        let Ok(listing) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in listing.filter_map(Result::ok) {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                folders.push(path);
            } else if kind.is_file()
                && path.extension().is_some_and(|e| e == "lst")
                && !wanted.contains(&path)
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

/// The project's files with extension `ext` (assemblies, sound events),
/// as paths in `assets` without it (`ships/fighter`), sorted: hidden
/// folders and the engine's own are skipped, and links to folders aren't
/// followed. At most [`MAX_ASSEMBLY_NAMES`].
fn asset_paths(assets: &Path, ext: &str) -> Vec<String> {
    let ext = format!(".{ext}");
    let mut found = Vec::new();
    let mut folders = vec![assets.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let Ok(listing) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in listing.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if name.starts_with('.') {
                continue;
            }
            if kind.is_dir() {
                if folder != assets || name != crate::branding::CODE_FOLDER {
                    folders.push(path);
                }
            } else if kind.is_file()
                && name.to_lowercase().ends_with(&ext)
                && let Ok(inside) = path.strip_prefix(assets)
            {
                let inside = inside.to_string_lossy().replace('\\', "/");
                found.push(inside[..inside.len() - ext.len()].to_string());
                if found.len() >= MAX_ASSEMBLY_NAMES {
                    folders.clear();
                    break;
                }
            }
        }
    }
    found.sort();
    found
}

/// The most of each kind of file named in `solis/project.lst`: past it,
/// the rest can't be named from game code (a guard against a folder of
/// millions).
const MAX_ASSEMBLY_NAMES: usize = 10_000;

impl GameCode {
    /// The game code of the program and version `made_by`, which writes
    /// the files describing the engine.
    pub fn new(made_by: impl Into<String>) -> Self {
        Self {
            made_by: made_by.into(),
            ..Self::default()
        }
    }

    /// Writes the files describing the engine (`assets/solis/*.lst`) if
    /// they're missing or different. `made_by` names the program and
    /// version. Returns notes on any that couldn't be written.
    ///
    /// They include `solis/project.lst`, the project's names (its
    /// assemblies, and the input actions in its `input.solinput`, beside
    /// `assets`) as states, so call this again when they change;
    /// the notes then also say which names game code can't use.
    pub fn describe(assets: &Path, made_by: &str) -> Vec<String> {
        Self::describe_with(assets, made_by, &Animation::default())
    }

    /// [`GameCode::describe`], with the sprites' clips (`sheet/clip`
    /// paths) and cues, and each animator's own file
    /// (`solis/animators/…`; those no longer wanted are taken away).
    pub fn describe_with(assets: &Path, made_by: &str, animation: &Animation) -> Vec<String> {
        let Animation {
            clips,
            cues,
            animators,
            maps,
            pictures,
        } = animation;
        let root = assets.parent();
        let names = script::project_names::Names {
            assemblies: asset_paths(assets, asset_db::ASSEMBLY_EXTENSION),
            sounds: asset_paths(assets, sound::event::EXTENSION),
            buses: root
                .map_or_else(sound::Mixer::default, |root| {
                    crate::play_files::load_mixer(root).0
                })
                .buses
                .into_iter()
                .map(|b| b.name)
                .collect(),
            clips: clips.to_vec(),
            cues: cues.to_vec(),
            pictures: pictures.to_vec(),
            // ui/pauseMenu.lst is the screen PauseMenu (as the screens'
            // program finds them).
            screens: asset_paths(&assets.join(script::screens::FOLDER), "lst")
                .iter()
                .filter_map(|p| script::screens::screen_name(&format!("{p}.lst")))
                .collect(),
            actions: assets
                .parent()
                .map_or_else(input::Bindings::default, |root| {
                    crate::play_files::load_bindings(root).0
                })
                // A play adds the menu actions a project's file lacks, so
                // game code can name them.
                .with_menu_actions()
                .actions
                .into_iter()
                .map(|a| a.name)
                .collect(),
        };
        let (mut files, mut notes) =
            script::describe::descriptions_for(made_by, crate::branding::CODE_FOLDER, &names);
        let mut classes = std::collections::HashSet::new();
        for animator in animators {
            let (file, said) = script::animator_code::animator_file(
                made_by,
                crate::branding::CODE_FOLDER,
                animator,
            );
            notes.extend(said);
            // Two animators of the same name in different folders would
            // be the same class: the first keeps it.
            if let Some((path, text)) = file {
                match script::animator_code::class_names(&animator.path) {
                    Some((class, _)) if !classes.insert(class.clone()) => notes.push(format!(
                        "The animator {} can't be reached from game code: another is called \
                         {class} too. Rename one.",
                        animator.path
                    )),
                    _ => files.push((path, text)),
                }
            }
        }
        for map in maps {
            let (file, said) =
                script::map_code::map_file(made_by, crate::branding::CODE_FOLDER, map);
            notes.extend(said);
            if let Some((path, text)) = file {
                let class = map
                    .path
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if classes.insert(format!("map:{class}")) {
                    files.push((path, text));
                } else {
                    notes.push(format!(
                        "The tile map {} can't be reached from game code: another has the same file \
                         name. Rename one.",
                        map.path
                    ));
                }
            }
        }
        forget_animator_files(assets, &files, "animators");
        forget_animator_files(assets, &files, "maps");
        for (path, text) in files {
            let file = assets.join(&path);
            if same_text(&file, &text) {
                continue;
            }
            // The folders are Solis's own: never written through a link
            // a project planted in their place.
            let written = path
                .rsplit_once('/')
                .map_or(Ok(()), |(folder, _)| {
                    asset_db::own_folder(assets, folder).map(|_| ())
                })
                .and_then(|()| asset_db::write_atomically(&file, text.as_bytes()));
            if let Err(why) = written {
                notes.push(format!(
                    "Couldn't write {path}, which describes the engine to game code: {why}."
                ));
            }
        }
        notes
    }

    /// What describing the engine said since last asked, when it changed
    /// (the names game code can't use, files that couldn't be written).
    pub fn take_notes(&mut self) -> Vec<String> {
        std::mem::take(&mut self.to_say)
    }

    /// Takes the sprites' clips and cues as the editor has read them;
    /// whether they changed (then check the code again).
    pub fn set_animation(&mut self, animation: Animation) -> bool {
        let changed = self.animation != animation;
        self.animation = animation;
        changed
    }

    /// Describes the engine and the project's names to Lust in `assets`
    /// (they may have changed with them: an assembly added, the input
    /// actions saved), then reads and checks the code there, all on a
    /// worker; if a check is running, another starts when it ends.
    pub fn check(&mut self, assets: PathBuf) {
        if self.running.is_some() {
            self.again = Some(assets);
            return;
        }
        let (sender, receiver) = channel();
        let last = self.last;
        let check = self.checker.check;
        let made_by = self.made_by.clone();
        let animation = self.animation.clone();
        let folder = assets.clone();
        let started = std::thread::Builder::new()
            .name("game code".into())
            .spawn(move || {
                // A crash is answered too, so it's said and the next
                // change is checked (the review, BUG-33).
                let answer = std::panic::catch_unwind(|| {
                    let described = Self::describe_with(&assets, &made_by, &animation);
                    let code = script::gather(&assets);
                    let found = hash(&code);
                    // Most changes in the assets aren't code (a model, the
                    // area saved): then there's nothing to compile.
                    let compiled = (last != Some(found)).then(|| check(&code));
                    (described, found, code, compiled)
                });
                let _ = sender.send(answer.ok());
            });
        if started.is_ok() {
            self.running = Some((receiver, Instant::now(), folder));
        }
    }

    /// Another project opened: what the last one's checks found is
    /// forgotten, and one still running is left to finish unheard. The
    /// next check's notes on describing the engine are all said.
    pub fn forget(&mut self) {
        self.running = None;
        self.again = None;
        self.last = None;
        self.held = None;
        self.animation = Animation::default();
        self.told.clear();
        self.to_say.clear();
    }

    /// Whether a check is running.
    pub fn waiting(&self) -> bool {
        self.running.is_some()
    }

    /// A finished check's findings, if the code changed since the last
    /// one reported. Never waits.
    pub fn poll(&mut self) -> Option<Checked> {
        // The check that was given up on has ended: the code that
        // couldn't be checked while it ran is checked now, whatever the
        // last report was about.
        if self.running.is_none() && self.held.is_some() && !(self.checker.unfinished)() {
            self.last = None;
            if let Some((_, assets)) = self.held.take() {
                self.check(assets);
            }
        }
        let (receiver, started, _) = self.running.as_ref()?;
        let answer = match receiver.try_recv() {
            Ok(answer) => answer,
            // Stuck: stop waiting (the worker is left to end on its own).
            Err(std::sync::mpsc::TryRecvError::Empty) if started.elapsed() > GIVE_UP => {
                return Some(self.stopped(format!(
                    "checking the game's code took more than {} s, so {} stopped waiting. \
                     This is a bug in Lust ({}); please report it with the code that caused \
                     it. Saving a change checks the code again.",
                    GIVE_UP.as_secs(),
                    crate::branding::APP_NAME,
                    script::LUST_VERSION
                )));
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            // The worker ended without a word: it crashed, as below.
            Err(std::sync::mpsc::TryRecvError::Disconnected) => None,
        };
        let Some((described, found, code, compiled)) = answer else {
            return Some(self.stopped(format!(
                "checking the game's code failed unexpectedly, so it wasn't checked. This is \
                 a bug in {} or in Lust ({}); please report it with the code that caused it. \
                 Saving a change checks the code again.",
                crate::branding::APP_NAME,
                script::LUST_VERSION
            )));
        };
        let (_, _, checked_folder) = self.running.take()?;
        if described != self.told {
            self.to_say.extend(described.iter().cloned());
            self.told = described;
        }
        if let Some(assets) = self.again.take() {
            self.check(assets);
        }
        let compiled = match compiled.filter(|_| self.last != Some(found))? {
            Ok(compiled) => compiled,
            // Not checked: a check that was given up on is still running,
            // and no other starts until it ends (`script::try_compile`).
            // Said once for this code, which is checked when it has ended.
            Err(script::NotStarted(waiting)) => {
                let told = self.held.as_ref().is_some_and(|(held, _)| *held == found);
                self.held = Some((found, checked_folder));
                return (!told).then(|| Checked {
                    problems: vec![waiting],
                    notes: code.notes,
                    program: None,
                });
            }
        };
        self.held = None;
        self.last = Some(found);
        let (problems, program) = match compiled {
            // Code that runs, but calls a native by an old name: warned
            // about in Problems (Phase 8, C3).
            Ok(program) => (script::renamed_uses(&code), Some(program.map(Arc::new))),
            Err(problems) => (problems, None),
        };
        Some(Checked {
            problems,
            notes: code.notes,
            program,
        })
    }

    /// The check running ended without an answer (`why`): it's said as the
    /// code's problem, the last program that worked stays, and the next
    /// change is checked (a change already waiting starts now).
    fn stopped(&mut self, why: String) -> Checked {
        self.running = None;
        self.last = None;
        if let Some(assets) = self.again.take() {
            self.check(assets);
        }
        Checked {
            problems: vec![CodeProblem {
                file: script::code::ENTRY.to_string(),
                line: 0,
                column: 0,
                message: why,
            }],
            notes: Vec::new(),
            program: None,
        }
    }
}

/// Whether `file` is an ordinary file holding exactly `text`, reading no
/// more than that.
fn same_text(file: &Path, text: &str) -> bool {
    reflect::ordinary_file::read_start(file, text.len() as u64 + 1)
        .is_ok_and(|old| old == text.as_bytes())
}

/// A hash of the files read and those left out, to tell a change.
fn hash(code: &Code) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    code.files.hash(&mut hasher);
    code.lua.hash(&mut hasher);
    code.notes.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A project's assets folder of its own: checking the code reads the
    /// input actions and buses beside it, which another test may write.
    /// The folder holding it goes when the first part is dropped.
    fn folder(name: &str) -> (reflect::test_support::TempFolder, PathBuf) {
        let root = reflect::test_support::TempFolder::new(&format!("game-code-{name}")).unwrap();
        let path = root.join("assets");
        std::fs::create_dir_all(&path).unwrap();
        (root, path)
    }

    fn finished(code: &mut GameCode) -> Option<Checked> {
        let start = std::time::Instant::now();
        while code.waiting() && start.elapsed().as_secs() < 30 {
            if let Some(done) = code.poll() {
                return Some(done);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        None
    }

    /// Checks the code in `assets` and waits for the check: what
    /// describing the engine said on its way, if it's new.
    fn notes_of_a_check(code: &mut GameCode, assets: &Path) -> Vec<String> {
        code.check(assets.to_path_buf());
        finished(code);
        assert!(!code.waiting());
        code.take_notes()
    }

    #[test]
    fn a_file_is_the_same_text_only_if_it_is_an_ordinary_file_holding_it() {
        let (_temp, dir) = folder("same-text");
        let file = dir.join("names.lst");
        std::fs::write(&file, "enum Clip {}").unwrap();
        assert!(same_text(&file, "enum Clip {}"));
        assert!(!same_text(&file, "enum Clip {"), "the file holds more");
        assert!(!same_text(&file, "enum Clip {};"), "the file holds less");
        assert!(!same_text(&dir.join("none.lst"), ""));
        // A folder, a device or a pipe in its place: never the same, and
        // answered at once (it's asked on every check of the game's code).
        for odd in crate::odd_files(&dir, "names.lst") {
            assert!(!same_text(&odd, ""), "{odd:?}");
            assert!(!same_text(&odd, "enum Clip {}"), "{odd:?}");
        }
    }

    #[test]
    fn the_project_s_assemblies_become_names_and_its_notes_are_said_once() {
        let (_temp, assets) = folder("names");
        std::fs::create_dir_all(assets.join("ships")).unwrap();
        std::fs::create_dir_all(assets.join(".hidden")).unwrap();
        std::fs::create_dir_all(assets.join("solis")).unwrap();
        for file in [
            "ships/fighter.solassembly",
            "crate.SOLASSEMBLY",
            "big-ship.solassembly",
            ".hidden/x.solassembly",
            "solis/y.solassembly",
            "ships/notes.txt",
        ] {
            std::fs::write(assets.join(file), "").unwrap();
        }
        // The review, BUG-53: the engine is described on the check's
        // worker, not on the window's thread, and what it says comes back
        // with the check.
        let mut code = GameCode::new("Test 1.0");
        let notes = notes_of_a_check(&mut code, &assets);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("big-ship"));
        let project = std::fs::read_to_string(assets.join("solis/project.lst")).unwrap();
        assert!(
            project.contains("state Assembly = [Crate, Ships [Fighter]];"),
            "{project}"
        );
        // The same again: nothing new to say. A new assembly joins.
        assert!(notes_of_a_check(&mut code, &assets).is_empty());
        std::fs::write(assets.join("ships/freighter.solassembly"), "").unwrap();
        assert!(
            notes_of_a_check(&mut code, &assets).is_empty(),
            "the same note isn't said again"
        );
        let project = std::fs::read_to_string(assets.join("solis/project.lst")).unwrap();
        assert!(project.contains("Ships [Fighter, Freighter]"), "{project}");
        assert!(
            project.contains("state Action = [Move, Look, Jump,"),
            "the ready-made actions: {project}"
        );
        // The project's own input actions, beside its assets.
        let mut bindings = input::Bindings::default();
        bindings
            .actions
            .push(input::Action::button("OpenMap", &["M"]));
        bindings
            .actions
            .push(input::Action::button("openMap2", &["N"]));
        std::fs::write(assets.parent().unwrap().join(input::FILE), bindings.write()).unwrap();
        let notes = notes_of_a_check(&mut code, &assets);
        assert!(
            notes.iter().any(|n| n.contains("\"openMap2\"")),
            "{notes:?}"
        );
        let project = std::fs::read_to_string(assets.join("solis/project.lst")).unwrap();
        assert!(project.contains("Pause, OpenMap];"), "{project}");
        // Sound events and the Mixer's buses.
        std::fs::create_dir_all(assets.join("sfx")).unwrap();
        std::fs::write(assets.join("sfx/footstep.solsound"), "").unwrap();
        notes_of_a_check(&mut code, &assets);
        let project = std::fs::read_to_string(assets.join("solis/project.lst")).unwrap();
        assert!(
            project.contains("state Sound = [Sfx [Footstep]];"),
            "{project}"
        );
        assert!(
            project.contains("state Bus = [Master, Music, Effects, Voice, Ambience, Ui];"),
            "{project}"
        );
        // Another project: its notes are all said, even the same ones.
        code.forget();
        assert_eq!(notes_of_a_check(&mut code, &assets).len(), 2);
    }

    #[test]
    fn each_animator_gets_its_own_file_and_a_gone_one_s_is_taken_away() {
        use script::animator_code::{AnimatorNames, ParamKind};
        let (_temp, assets) = folder("animators");
        let farmer = AnimatorNames {
            path: "animals/farmer".into(),
            parameters: vec![("moving".into(), ParamKind::Switch)],
            states: vec!["Idle".into(), "Walk".into()],
        };
        let twin = AnimatorNames {
            path: "people/farmer".into(),
            ..farmer.clone()
        };
        let notes = GameCode::describe_with(
            &assets,
            "Test 1.0",
            &Animation {
                animators: vec![farmer, twin],
                ..Animation::default()
            },
        );
        let file = assets.join("solis/animators/animals/farmer.lst");
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("class FarmerAnimator"), "{text}");
        assert!(
            notes.iter().any(|n| n.contains("people/farmer")),
            "the same class twice: {notes:?}"
        );
        assert!(!assets.join("solis/animators/people/farmer.lst").exists());
        GameCode::describe(&assets, "Test 1.0");
        assert!(!file.exists(), "a gone animator's file is taken away");
    }

    #[test]
    fn the_standard_names_have_a_new_project_s_buses() {
        let buses: Vec<String> = sound::Mixer::default()
            .buses
            .into_iter()
            .map(|b| b.name)
            .collect();
        assert_eq!(buses, script::project_names::Names::standard().buses);
    }

    #[test]
    fn the_engine_is_described_once_and_the_code_checked_when_it_changes() {
        let (_temp, assets) = folder("check");
        assert!(GameCode::describe(&assets, "Test 1.0").is_empty());
        let written = std::fs::read_to_string(assets.join("solis/world.lst")).unwrap();
        assert!(written.contains("method say(string words) native;"));
        let stamp = std::fs::metadata(assets.join("solis/world.lst"))
            .unwrap()
            .modified()
            .unwrap();
        GameCode::describe(&assets, "Test 1.0");
        assert_eq!(
            std::fs::metadata(assets.join("solis/world.lst"))
                .unwrap()
                .modified()
                .unwrap(),
            stamp,
            "the same text isn't written again"
        );
        let mut code = GameCode::default();
        std::fs::write(
            assets.join("main.lst"),
            "fetch \"solis/world\":World;\ncomponent Gun {\n    has {\n        int ammo = \"x\";\n    }\n}\npass Gun;\nend\n",
        )
        .unwrap();
        code.check(assets.clone());
        let found = finished(&mut code).unwrap();
        assert_eq!(found.problems.len(), 1, "{:?}", found.problems.first());
        assert_eq!(found.problems[0].line, 4);
        assert!(found.program.is_none(), "the last program stays");
        // The same code again: nothing new to say.
        code.check(assets.clone());
        assert!(finished(&mut code).is_none());
        std::fs::write(
            assets.join("main.lst"),
            "fetch \"solis/world\":World;\ncomponent Gun {\n    has {\n        int ammo = 3;\n    }\n}\npass Gun;\nend\n",
        )
        .unwrap();
        code.check(assets);
        let sound = finished(&mut code).unwrap();
        assert!(sound.problems.is_empty());
        assert!(matches!(sound.program, Some(Some(_))), "a program to run");
    }
}

#[cfg(test)]
mod unfinished {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    /// Whether the stand-in's check that was given up on is still running.
    static STILL_RUNNING: AtomicBool = AtomicBool::new(true);
    /// How often the stand-in was asked to check.
    static ASKED: AtomicUsize = AtomicUsize::new(0);

    /// A stand-in for `script::try_compile` after a check was given up on:
    /// nothing is started while that one runs (nothing here really hangs).
    fn stand_in(_code: &Code) -> Attempt {
        ASKED.fetch_add(1, Ordering::SeqCst);
        if STILL_RUNNING.load(Ordering::SeqCst) {
            Err(script::NotStarted(CodeProblem {
                file: script::code::ENTRY.to_string(),
                line: 0,
                column: 0,
                message: "the code wasn't checked".to_string(),
            }))
        } else {
            Ok(Ok(None))
        }
    }

    fn still_running() -> bool {
        STILL_RUNNING.load(Ordering::SeqCst)
    }

    /// The answer of the check running, if it has anything to report.
    fn finished(code: &mut GameCode) -> Option<Checked> {
        let start = Instant::now();
        while code.waiting() && start.elapsed().as_secs() < 30 {
            if let Some(done) = code.poll() {
                return Some(done);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        None
    }

    #[test]
    fn code_not_checked_while_a_check_never_finishes_is_checked_by_itself_when_it_ends() {
        let temp = reflect::test_support::TempFolder::new("game-code-unfinished").unwrap();
        let assets = temp.join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(assets.join("main.lst"), "end\n").unwrap();
        let asked = || ASKED.load(Ordering::SeqCst);
        let mut code = GameCode {
            checker: Checker {
                check: stand_in,
                unfinished: still_running,
            },
            ..GameCode::default()
        };
        // A save while the check that was given up on runs: said, and the
        // last program that worked stays.
        code.check(assets.clone());
        let told = finished(&mut code).expect("told why");
        assert_eq!(told.problems.len(), 1);
        assert_eq!(told.problems[0].message, "the code wasn't checked");
        assert!(told.program.is_none());
        // Frame after frame, nothing more is asked for while it runs.
        for _ in 0..100 {
            assert!(code.poll().is_none());
            assert!(!code.waiting());
        }
        assert_eq!(asked(), 1);
        // Something else in the assets changes: the same code is asked
        // about, and nothing is said a second time.
        code.check(assets.clone());
        assert!(finished(&mut code).is_none());
        assert_eq!(asked(), 2);
        // The code changes: said for the new code.
        std::fs::write(assets.join("main.lst"), "\nend\n").unwrap();
        code.check(assets.clone());
        assert_eq!(finished(&mut code).expect("told again").problems.len(), 1);
        assert_eq!(asked(), 3);
        // The check ends: the next frame starts checking the code, with
        // nothing saved, and its findings are reported.
        STILL_RUNNING.store(false, Ordering::SeqCst);
        let checked = code
            .poll()
            .or_else(|| finished(&mut code))
            .expect("checked at last");
        assert!(checked.problems.is_empty());
        assert!(matches!(checked.program, Some(None)));
        assert_eq!(asked(), 4);
        // And that's all: nothing is held any more.
        for _ in 0..100 {
            assert!(code.poll().is_none());
            assert!(!code.waiting());
        }
        assert_eq!(asked(), 4);
        // Another project: nothing of this one is checked later.
        STILL_RUNNING.store(true, Ordering::SeqCst);
        std::fs::write(assets.join("main.lst"), "\n\nend\n").unwrap();
        code.check(assets.clone());
        assert!(finished(&mut code).is_some());
        code.forget();
        STILL_RUNNING.store(false, Ordering::SeqCst);
        assert!(code.poll().is_none());
        assert!(!code.waiting());
        assert_eq!(asked(), 5);
    }
}

#[cfg(test)]
mod stuck {
    use super::*;

    #[test]
    fn a_check_that_never_ends_is_given_up_on_and_said() {
        let mut code = GameCode::default();
        let (sender, receiver) = channel::<Answer>();
        let long_ago = Instant::now()
            .checked_sub(GIVE_UP + Duration::from_secs(1))
            .unwrap();
        code.running = Some((receiver, long_ago, PathBuf::new()));
        let found = code.poll().expect("given up on");
        assert!(found.problems[0].message.contains("stopped waiting"));
        assert!(found.program.is_none(), "the last program stays");
        assert!(!code.waiting(), "the next change is checked");
        drop(sender);
    }
}

#[cfg(test)]
mod fast_changes {
    use super::*;

    const MOVER: &str =
        "component Mover {\n    has {\n        float speed = 1;\n    }\n}\npass Mover;\nend\n";
    const FASTER: &str = "component Mover {\n    has {\n        float speed = 1;\n        bool fast = true;\n    }\n}\npass Mover;\nend\n";

    #[test]
    #[ignore = "scale check: run by tools/scale/scale-check.sh"]
    fn scale_check_a_lust_edit_reaches_the_running_game() {
        // The gate (ROADMAP Phase 4): saving a Lust file while the game
        // runs, to the running game having the new code, within 2 s. The
        // whole path: the folder's watcher, reading and compiling on the
        // worker, and the editor swapping it in.
        const TARGET_MS: f64 = 2000.0;
        let temp = reflect::test_support::TempFolder::new("fast").unwrap();
        let assets = temp.join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        GameCode::describe(&assets, "Test 1.0");
        std::fs::write(assets.join("main.lst"), MOVER).unwrap();
        let mut code = GameCode::default();
        code.check(assets.clone());
        let mut w = editor::Workbench::new(editor::AppInfo {
            name: "Test".into(),
            version: "1.0".into(),
        });
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < 30 {
            if let Some(checked) = code.poll() {
                w.set_program(checked.program.flatten());
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let mut area = area::builtin::starter_area();
        let mut cart = area::EntityData::new("Cart");
        cart.components
            .insert("Mover".into(), reflect::ComponentData::default());
        area.spawn(cart);
        w.open_area(area, "cart.solarea".into());
        w.play();
        let watcher = asset_db::Watcher::new(&assets).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(800));
        // Anything left over from setting up isn't this edit.
        let _ = watcher.changed();
        let saved = std::time::Instant::now();
        std::fs::write(assets.join("main.lst"), FASTER).unwrap();
        let mut took = None;
        while saved.elapsed().as_secs() < 10 {
            if watcher.changed() {
                code.check(assets.clone());
            }
            if let Some(checked) = code.poll()
                && let Some(program) = checked.program
            {
                w.set_program(program);
            }
            if w.log_lines()
                .iter()
                .any(|l| l.text.contains("took the new code"))
            {
                took = Some(saved.elapsed());
                break;
            }
            // A frame.
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        let ms = took.expect("the new code arrived").as_secs_f64() * 1000.0;
        let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
        println!(
            "Scale check: a Lust edit saved to running in play mode {ms:.0} ms (target {TARGET_MS} ms) {verdict}"
        );
        assert!(ms <= TARGET_MS, "a Lust edit took {ms:.0} ms");
    }
}

#[cfg(test)]
mod crashed {
    use super::*;

    /// A stand-in for the compiler with a bug that panics.
    fn crashes(_code: &Code) -> Attempt {
        panic!("a bug in the compiler (a stand-in for the test)")
    }

    fn checking(code: &mut GameCode) -> Option<Checked> {
        let start = Instant::now();
        while code.waiting() && start.elapsed().as_secs() < 30 {
            if let Some(done) = code.poll() {
                return Some(done);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        None
    }

    /// The review, BUG-33: a panic on the game code's worker was silent.
    /// The editor showed no problem and kept the old program, and an
    /// exported game, with no program and no mistakes, said "Starting"
    /// for ever. Now it's said as the code's problem, and the next change
    /// is checked.
    #[test]
    fn a_crash_while_checking_is_said_and_the_next_change_is_checked() {
        let temp = reflect::test_support::TempFolder::new("game-code-crashed").unwrap();
        let assets = temp.join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(assets.join("main.lst"), "end\n").unwrap();
        let mut code = GameCode {
            checker: Checker {
                check: crashes,
                unfinished: || false,
            },
            ..GameCode::default()
        };
        code.check(assets.clone());
        let said = checking(&mut code).expect("the crash is said");
        assert_eq!(said.problems.len(), 1);
        assert!(
            said.problems[0].message.contains("failed unexpectedly"),
            "{}",
            said.problems[0].message
        );
        assert!(said.program.is_none(), "the last program that worked stays");
        assert!(!code.waiting());
        // The same code again is checked again (it was never checked).
        code.check(assets.clone());
        assert!(checking(&mut code).is_some(), "said again");
    }

    /// A worker that ends without a word is a crash too, and a change
    /// that waited behind its check is checked at once.
    #[test]
    fn a_worker_gone_without_an_answer_is_said_and_what_waited_starts() {
        let temp = reflect::test_support::TempFolder::new("game-code-gone").unwrap();
        let assets = temp.join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        let mut code = GameCode::default();
        let (sender, receiver) = channel::<Answer>();
        drop(sender);
        code.running = Some((receiver, Instant::now(), PathBuf::new()));
        code.again = Some(assets.clone());
        let said = code.poll().expect("said");
        assert!(said.problems[0].message.contains("failed unexpectedly"));
        assert!(code.waiting(), "the change that waited is being checked");
        assert!(code.again.is_none());
        checking(&mut code);
    }
}
