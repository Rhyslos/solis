//! The game's code, looked after on a worker (CONTRIBUTING rule 10): the
//! files describing the engine to Lust written into a project when it
//! opens, and the project's Lust read and checked whenever something in
//! its assets changes, and turned into Lua for the editor to run. Only a
//! check that found different code reports.

use script::{Code, CodeProblem};
use std::hash::{Hash, Hasher};
use std::io::Read;
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

/// A worker's answer: the code's hash, what was read, and the program or
/// its mistakes.
type Answer = (
    u64,
    Code,
    Result<Option<script::Compiled>, Vec<CodeProblem>>,
);

/// One check at a time, and whether another is wanted after it.
#[derive(Default)]
pub struct GameCode {
    /// The check running, and when it started.
    running: Option<(Receiver<Answer>, Instant)>,
    /// The folder to check again when the running check ends.
    again: Option<PathBuf>,
    /// The code the last reported check found, by its hash.
    last: Option<u64>,
}

impl GameCode {
    /// Writes the files describing the engine (`assets/solis/*.lst`) if
    /// they're missing or different. `made_by` names the program and
    /// version. Returns notes on any that couldn't be written.
    pub fn describe(assets: &Path, made_by: &str) -> Vec<String> {
        let mut notes = Vec::new();
        for (path, text) in script::describe::descriptions(made_by, crate::branding::CODE_FOLDER) {
            let file = assets.join(&path);
            if same_text(&file, &text) {
                continue;
            }
            let written = file
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| asset_db::write_atomically(&file, text.as_bytes()));
            if let Err(why) = written {
                notes.push(format!(
                    "Couldn't write {path}, which describes the engine to game code: {why}."
                ));
            }
        }
        notes
    }

    /// Reads and checks the code in `assets` on a worker; if a check is
    /// running, another starts when it ends.
    pub fn check(&mut self, assets: PathBuf) {
        if self.running.is_some() {
            self.again = Some(assets);
            return;
        }
        let (sender, receiver) = channel();
        let started = std::thread::Builder::new()
            .name("game code".into())
            .spawn(move || {
                let code = script::gather(&assets);
                let compiled = script::compile(&code);
                let _ = sender.send((hash(&code), code, compiled));
            });
        if started.is_ok() {
            self.running = Some((receiver, Instant::now()));
        }
    }

    /// Another project opened: what the last one's checks found is
    /// forgotten, and one still running is left to finish unheard.
    pub fn forget(&mut self) {
        self.running = None;
        self.again = None;
        self.last = None;
    }

    /// Whether a check is running.
    pub fn waiting(&self) -> bool {
        self.running.is_some()
    }

    /// A finished check's findings, if the code changed since the last
    /// one reported. Never waits.
    pub fn poll(&mut self) -> Option<Checked> {
        let (receiver, started) = self.running.as_ref()?;
        let (found, code, compiled) = match receiver.try_recv() {
            Ok(done) => done,
            Err(std::sync::mpsc::TryRecvError::Empty) if started.elapsed() > GIVE_UP => {
                // Stuck: stop waiting (the worker is left to end on its
                // own), say so, and let the next change be checked.
                self.running = None;
                self.last = None;
                if let Some(assets) = self.again.take() {
                    self.check(assets);
                }
                return Some(Checked {
                    problems: vec![CodeProblem {
                        file: script::code::ENTRY.to_string(),
                        line: 0,
                        column: 0,
                        message: format!(
                            "checking the game's code took more than {} s, so Solis stopped \
                             waiting. This is a bug in Lust ({}); please report it with the \
                             code that caused it. Saving a change checks the code again.",
                            GIVE_UP.as_secs(),
                            script::LUST_VERSION
                        ),
                    }],
                    notes: Vec::new(),
                    program: None,
                });
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // The worker stopped without an answer: try again later.
                self.running = None;
                return None;
            }
        };
        self.running = None;
        if let Some(assets) = self.again.take() {
            self.check(assets);
        }
        if self.last == Some(found) {
            return None;
        }
        self.last = Some(found);
        let (problems, program) = match compiled {
            Ok(program) => (Vec::new(), Some(program.map(Arc::new))),
            Err(problems) => (problems, None),
        };
        Some(Checked {
            problems,
            notes: code.notes,
            program,
        })
    }
}

/// Whether `file` holds exactly `text`, reading no more than that.
fn same_text(file: &Path, text: &str) -> bool {
    let Ok(opened) = std::fs::File::open(file) else {
        return false;
    };
    let mut old = Vec::with_capacity(text.len());
    opened
        .take(text.len() as u64 + 1)
        .read_to_end(&mut old)
        .is_ok_and(|_| old == text.as_bytes())
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

    fn folder(name: &str) -> PathBuf {
        let path = std::env::temp_dir()
            .join(format!("solis-game-code-{}", std::process::id()))
            .join(name);
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
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

    #[test]
    fn the_engine_is_described_once_and_the_code_checked_when_it_changes() {
        let assets = folder("check");
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
            "get \"solis/world\":[say];\ncomponent Gun {\n    has {\n        int ammo = \"x\";\n    }\n}\npass Gun;\nend\n",
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
            "get \"solis/world\":[say];\ncomponent Gun {\n    has {\n        int ammo = 3;\n    }\n}\npass Gun;\nend\n",
        )
        .unwrap();
        code.check(assets);
        let sound = finished(&mut code).unwrap();
        assert!(sound.problems.is_empty());
        assert!(matches!(sound.program, Some(Some(_))), "a program to run");
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
        code.running = Some((receiver, long_ago));
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
        let assets = std::env::temp_dir()
            .join(format!("solis-fast-{}", std::process::id()))
            .join("assets");
        let _ = std::fs::remove_dir_all(&assets);
        std::fs::create_dir_all(&assets).unwrap();
        GameCode::describe(&assets, "Test 1.0");
        std::fs::write(assets.join("main.lst"), MOVER).unwrap();
        let mut code = GameCode::default();
        code.check(assets.clone());
        let mut w = editor::Workbench::new(
            editor::AppInfo {
                name: "Test".into(),
                version: "1.0".into(),
            },
            None,
        );
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
