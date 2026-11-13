//! A project's Lust: every `.lst` file in its assets, read within limits
//! (CONTRIBUTING rule 9), and checked by the Lust compiler. Both are slow
//! enough to run on a worker, never on the frame.

pub use crate::check_turns::NotStarted;
use crate::check_turns::{Checked, Turns};
use area::lust_guard::{Found, MAX_PREFIX_RUN};
use reflect::ComponentSchema;
use std::io::Read;
use std::path::Path;

/// The most a code file may hold.
pub const MAX_FILE_BYTES: u64 = 1 << 20;
/// The most all code files together may hold.
pub const MAX_TOTAL_BYTES: u64 = 16 << 20;
/// The most code files read.
pub const MAX_FILES: usize = 10_000;
/// The game's first file, which Lust starts from.
pub const ENTRY: &str = "main.lst";

/// Whose turn it is to check the game's code, in this program: every check
/// takes it ([`compile`], so the editor, an exported game and
/// `--check-project` alike).
static TURNS: Turns = Turns::new();

/// A project's Lust files, read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Code {
    /// Each Lust file's path inside the assets (with `/` between folders)
    /// and its text, sorted by path.
    pub files: Vec<(String, String)>,
    /// Each Lua file's, likewise.
    pub lua: Vec<(String, String)>,
    /// Files left out, and why, in plain words.
    pub notes: Vec<String>,
}

impl Code {
    /// Whether there's no game code at all. The files in the engine's own
    /// folder ([`crate::describe::in_engine_folder`]) only describe the
    /// engine and the project to game code, and are written into every
    /// project: they alone aren't a game's code. Counted as code, a
    /// project with none was told its `main.lst` was missing, and an
    /// exported game of it never started (the full review's NEW-10).
    pub fn is_empty(&self) -> bool {
        self.lua.is_empty() && !self.has_own_lust()
    }

    /// Whether the game has Lust files of its own, outside the engine's
    /// folder. Only then is it a Lust game, which starts at [`ENTRY`]: a
    /// game written only in Lua was told its `main.lst` was missing (the
    /// second review pass's N-01).
    fn has_own_lust(&self) -> bool {
        self.files
            .iter()
            .any(|(path, _)| !crate::describe::in_engine_folder(path))
    }
}

/// The game's code, ready to run: the Lust turned into Lua, and the Lua
/// components' files, checked.
#[derive(Debug, Clone)]
pub struct Compiled {
    /// The Lust, as Lua for Luau, if there's any Lust.
    pub lust: Option<lust::LuaChunk>,
    /// The Lua files (path inside the assets, and text), sorted by path.
    pub lua: Vec<(String, String)>,
    /// Its components (Lust and Lua) as the editor shows them, sorted by
    /// name: read by starting the code on its own, with no world.
    pub schemas: Vec<ComponentSchema>,
    /// Where each component is written: its file, and line (0 when not
    /// known), by name.
    pub places: std::collections::BTreeMap<String, (String, u32)>,
    /// The Lust and Lua compiled for Luau on the worker, so loading them
    /// on the frame is quick (empty: they're compiled when loaded).
    pub bytecode: Bytecode,
    /// The game's screens, compiled into a program of their own
    /// (decision 0087), if it has any.
    pub screens: Option<crate::screens::ScreenCode>,
}

/// Luau's own compiled form of the game's code (see
/// [`Compiled::bytecode`]). Made only by [`compile`], from code it just
/// checked: it's never read from a file.
#[derive(Debug, Clone, Default)]
pub struct Bytecode {
    /// The Lust's chunk.
    pub(crate) lust: Option<Vec<u8>>,
    /// Each Lua file's, in the order of [`Compiled::lua`].
    pub(crate) lua: Vec<Vec<u8>>,
}

/// A mistake in the game's code: where it is, and what's wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeProblem {
    /// The file, inside the assets, with `/` between folders.
    pub file: String,
    /// Its line, from 1 (0 when the mistake isn't at a place in a file).
    pub line: u32,
    /// Its column, from 1 (0 likewise).
    pub column: u32,
    /// What's wrong, in plain words.
    pub message: String,
}

impl std::fmt::Display for CodeProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.column) {
            _ if self.file.is_empty() => write!(f, "{}", self.message),
            (0, _) => write!(f, "{}: {}", self.file, self.message),
            (line, 0) => write!(f, "{}, line {line}: {}", self.file, self.message),
            (line, column) => write!(f, "{}, line {line}:{column}: {}", self.file, self.message),
        }
    }
}

impl From<lust::Problem> for CodeProblem {
    fn from(p: lust::Problem) -> Self {
        Self {
            file: p.file,
            line: p.line,
            column: p.column,
            message: moved_message(p.message),
        }
    }
}

/// Lust's message for a call that moved (decision 0065: a straight
/// switch), saying how it's written now: "there's no method called
/// positionOf." becomes "positionOf is now written target:position() …".
/// A moved call that kept its name (`moveTo`) given the entity first
/// gets the same help after Lust's own words.
fn moved_message(message: String) -> String {
    for moved in crate::describe::MOVED {
        if message == format!("there's no method called {}.", moved.old) {
            return format!("{} is now written {}.", moved.old, moved.lust);
        }
        if message.ends_with(&format!(" has no {}.", moved.old)) {
            return format!("{message} {} is now written {}.", moved.old, moved.lust);
        }
        if message.starts_with(&format!("{} takes ", moved.old)) {
            return format!("{message} {} is now written {}.", moved.old, moved.lust);
        }
    }
    message
}

/// Lua's uses of calls that moved (`solis.positionOf(e)`), as mistakes
/// saying how they're written now: Lua would only find out as it ran.
pub(crate) fn moved_lua_uses(code: &Code) -> Vec<CodeProblem> {
    let mut found = Vec::new();
    for (file, text) in &code.lua {
        for (n, line) in text.lines().enumerate() {
            let code_part = line.split("--").next().unwrap_or_default();
            for moved in crate::describe::MOVED {
                let old = format!("solis.{}", moved.old);
                let mut from = 0;
                while let Some(at) = code_part[from..].find(&old) {
                    let start = from + at;
                    let end = start + old.len();
                    from = end;
                    let word = |c: char| c.is_alphanumeric() || c == '_';
                    let before = code_part[..start].chars().next_back();
                    if before.is_some_and(|c| word(c) || c == '.')
                        || code_part[end..].chars().next().is_some_and(word)
                    {
                        continue;
                    }
                    found.push(CodeProblem {
                        file: file.clone(),
                        line: u32::try_from(n + 1).unwrap_or(u32::MAX),
                        column: u32::try_from(code_part[..start].chars().count() + 1)
                            .unwrap_or(u32::MAX),
                        message: format!("{old} is now written {}.", moved.lua),
                    });
                }
            }
        }
    }
    found
}

/// Reads every `.lst` and `.lua` file under `assets`, skipping hidden
/// folders and links to folders (which could loop), within the limits
/// above (shared by both).
pub fn gather(assets: &Path) -> Code {
    let mut code = Code::default();
    let mut total = 0u64;
    let mut folders = vec![assets.to_path_buf()];
    let mut found = Vec::new();
    while let Some(folder) = folders.pop() {
        let Ok(listing) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in listing.filter_map(Result::ok) {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                folders.push(path);
            } else if kind.is_file()
                && path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("lst") || e.eq_ignore_ascii_case("lua"))
                // What Solis makes (a tile map's farm.tilemap.lst) is
                // data, not part of the game's program (decision 0081).
                && reflect::data_files::kind_of(&path).is_none()
            {
                found.push(path);
            }
        }
    }
    found.sort();
    for path in found {
        let inside = slashed(path.strip_prefix(assets).unwrap_or(&path));
        if code.files.len() + code.lua.len() >= MAX_FILES {
            code.notes.push(format!(
                "Only the first {MAX_FILES} code files are read; {inside} and the rest are left out."
            ));
            break;
        }
        let size = std::fs::metadata(&path).map_or(0, |m| m.len());
        if size > MAX_FILE_BYTES {
            code.notes.push(format!(
                "{inside} is left out: it's {} KB, and a code file may be at most {} KB.",
                size >> 10,
                MAX_FILE_BYTES >> 10
            ));
            continue;
        }
        if total + size > MAX_TOTAL_BYTES {
            code.notes.push(format!(
                "{inside} is left out: the code files together may be at most {} MB.",
                MAX_TOTAL_BYTES >> 20
            ));
            continue;
        }
        match read_within(&path, MAX_FILE_BYTES.min(MAX_TOTAL_BYTES - total)) {
            Ok(None) => code.notes.push(format!(
                "{inside} is left out: it grew past the limits while it was read."
            )),
            Ok(Some(Err(_))) => code
                .notes
                .push(format!("{inside} is left out: it isn't text.")),
            Ok(Some(Ok(text))) => {
                total += text.len() as u64;
                let is_lua = path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("lua"));
                if is_lua {
                    code.lua.push((inside, text));
                } else {
                    code.files.push((inside, text));
                }
            }
            Err(e) => code.notes.push(format!("{inside} couldn't be read ({e}).")),
        }
    }
    code
}

/// A file's text, reading at most `most` bytes: `None` if it holds more
/// (it may have grown since its size was looked at).
fn read_within(
    path: &Path,
    most: u64,
) -> std::io::Result<Option<Result<String, std::string::FromUtf8Error>>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(most + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > most {
        return Ok(None);
    }
    Ok(Some(String::from_utf8(bytes)))
}

/// Checks the game's code: every mistake found, in order (none when it's
/// sound, or there's no code). See [`compile`].
pub fn check(code: &Code) -> Vec<CodeProblem> {
    compile(code).err().unwrap_or_default()
}

/// The game's code made ready to run: the Lust checked and turned into
/// Lua for Luau (starting from [`ENTRY`]), the Lua files checked for
/// mistakes in how they're written, and the whole started on its own to
/// read its components. `Ok(None)` when there's no code; otherwise the
/// code, or every mistake found. A crash inside the Lust compiler is a
/// mistake too.
///
/// Runs on a thread of its own with a large stack, so call it from a
/// worker. Code nested too deep is a mistake Lust finds. A check that
/// takes longer than [`COMPILE_LIMIT`] is given up on, and until its
/// thread ends no other is started: the one mistake then says so (see
/// [`try_compile`]).
pub fn compile(code: &Code) -> Result<Option<Compiled>, Vec<CodeProblem>> {
    compile_within(code, crate::luau::Limits::default())
}

/// [`compile`], with the code's own start and the whole of the checking
/// given `limits` (a game's, but for tests that must come out the same on
/// a busy machine: see [`crate::luau::Limits`]).
pub(crate) fn compile_within(
    code: &Code,
    limits: crate::luau::Limits,
) -> Result<Option<Compiled>, Vec<CodeProblem>> {
    attempt(code, limits).unwrap_or_else(|NotStarted(waiting)| Err(vec![waiting]))
}

/// [`compile`], saying apart the check that wasn't started because one
/// given up on earlier is still running (`Err`): then the caller can check
/// again when [`check_unfinished`] says it has ended (the editor does).
pub fn try_compile(code: &Code) -> Result<Result<Option<Compiled>, Vec<CodeProblem>>, NotStarted> {
    attempt(code, crate::luau::Limits::default())
}

/// Whether a check that was given up on is still running: while it is,
/// no code is checked.
pub fn check_unfinished() -> bool {
    TURNS.unfinished()
}

fn attempt(code: &Code, limits: crate::luau::Limits) -> Result<Checked, NotStarted> {
    if code.is_empty() {
        return Ok(Ok(None));
    }
    TURNS.check(code, limits, compile_here)
}

/// The longest checking the code may take before it's given up on.
pub const COMPILE_LIMIT: std::time::Duration = std::time::Duration::from_secs(30);

/// Problems for runs of `!` longer than [`MAX_PREFIX_RUN`], refused
/// before compiling. To the Lust compiler (2.0.0-alpha.67, rev ee12407;
/// first measured at rev 930afae) each `!` is a mistake of its own ("Lust
/// writes not, not !"), and its time grows with the square of the number
/// of mistakes: 64,000 in a row take it about 3 seconds and 128,000 more
/// than 8, so a file full of them (1 MB) would take about ten minutes,
/// well past [`COMPILE_LIMIT`]. Any symbol Lust doesn't use costs it the
/// same (`@` was measured alike), and those aren't looked for here: they
/// meet the limit. Found as Lust reads the text, outside comments and
/// strings ([`area::lust_guard`]).
fn long_prefix_chains(files: &[(String, String)]) -> Vec<CodeProblem> {
    let mut problems = Vec::new();
    for (path, text) in files {
        for (line, found) in area::lust_guard::scan(text) {
            if found == Found::LongRun {
                problems.push(CodeProblem {
                    file: path.clone(),
                    line,
                    column: 0,
                    message: format!(
                        "more than {MAX_PREFIX_RUN} `!` in a row; write it more simply"
                    ),
                });
            }
        }
    }
    problems
}

/// Problems for two components, classes, objects, nodes or schemas with
/// the same name (two of one kind, or one of each), in one file or two.
/// The Lust compiler (2.0.0-alpha.67, rev ee12407, as at rev 930afae)
/// never finishes such code (it orders classes by name), so it's found
/// here first (by the hardening milestone's fuzzing, decision 0042), with
/// what to do about it. Two things sharing a name any other way (an
/// assembly, a state, an event) Lust says itself. Found as
/// [`long_prefix_chains`] finds its runs.
fn shared_names(files: &[(String, String)]) -> Vec<CodeProblem> {
    let mut problems = Vec::new();
    let mut seen: std::collections::HashMap<&str, (&str, u32)> = std::collections::HashMap::new();
    for (path, text) in files {
        for (line, found) in area::lust_guard::scan(text) {
            let Found::Declared(
                word @ ("component" | "class" | "object" | "node" | "schema"),
                name,
            ) = found
            else {
                continue;
            };
            match seen.get(name) {
                Some((first, at)) => problems.push(CodeProblem {
                    file: path.clone(),
                    line,
                    column: 0,
                    message: format!(
                        "there's already something called {name} ({first}, line {at}); a \
                         {word} needs a name of its own"
                    ),
                }),
                None => {
                    seen.insert(name, (path, line));
                }
            }
        }
    }
    problems
}

/// [`compile_within`], on this thread.
fn compile_here(
    code: &Code,
    limits: crate::luau::Limits,
) -> Result<Option<Compiled>, Vec<CodeProblem>> {
    let mut problems = Vec::new();
    let mut lust = None;
    let own_lust = code.has_own_lust();
    if own_lust {
        let mut missing = check_entry(code);
        missing.extend(long_prefix_chains(&code.files));
        missing.extend(shared_names(&code.files));
        if missing.is_empty() {
            let files = &code.files;
            // The engine's own folder (solis/): Lust's host folder, whose files give entity
            // its methods and come with every program.
            let engine = lust::Engine {
                folders: crate::describe::engine_folders(),
            };
            match std::panic::catch_unwind(|| lust::to_lua_with(&engine, ENTRY, files)) {
                Ok(Ok(chunk)) => lust = Some(chunk),
                Ok(Err(found)) => problems.extend(found.into_iter().map(CodeProblem::from)),
                Err(_) => problems.push(compiler_stopped()),
            }
        } else {
            problems.extend(missing);
        }
    }
    if !code.lua.is_empty() {
        problems.extend(moved_lua_uses(code));
        let lua = crate::luau::sandboxed_lua().map_err(|_| vec![compiler_stopped()])?;
        for (path, text) in &code.lua {
            let chunk = crate::luau::as_text(lua.load(text));
            if let Err(err) = chunk.set_name(format!("={path}")).into_function() {
                problems.push(crate::lua_components::lua_problem(path, &err));
            }
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    let compiler = mlua::chunk::Compiler::new();
    let bytecode = Bytecode {
        lust: lust
            .as_ref()
            .and_then(|chunk: &lust::LuaChunk| compiler.compile(&chunk.code).ok()),
        lua: code
            .lua
            .iter()
            .map_while(|(_, text)| compiler.compile(text).ok())
            .collect(),
    };
    let mut compiled = Compiled {
        lust,
        lua: code.lua.clone(),
        schemas: Vec::new(),
        places: std::collections::BTreeMap::new(),
        bytecode,
        screens: None,
    };
    // Starting it here reads its components, so the editor needn't on the
    // frame.
    let started = crate::Runtime::new().and_then(|mut runtime| {
        runtime.set_limits(limits);
        runtime
            .load(&compiled, &mut crate::NoHost)
            .map(|()| runtime)
    });
    let runtime = match started {
        Ok(runtime) => runtime,
        Err(failure) => {
            let mut at = failure.at;
            at.message = format!("the code stopped while starting: {}", at.message);
            return Err(vec![at]);
        }
    };
    compiled.schemas = runtime.schemas();
    for schema in &compiled.schemas {
        let place = match runtime.lua_file(&schema.name) {
            Some(path) => Some((path.to_string(), 0)),
            None => lust_place(&code.files, &schema.name),
        };
        if let Some(place) = place {
            compiled.places.insert(schema.name.clone(), place);
        }
    }
    // The screens, once the game's code is sound and its components known
    // (decision 0087): each Lust component readable from them.
    if own_lust {
        let components: Vec<(String, String)> = compiled
            .places
            .iter()
            .filter(|(_, (file, _))| file.ends_with(".lst"))
            .map(|(name, (file, _))| (name.clone(), file.clone()))
            .collect();
        let engine = lust::Engine {
            folders: crate::describe::engine_folders(),
        };
        // And the game's Lua and Rust components and Solis's own, described
        // to screens as Lust components of their fields.
        let mut described: Vec<reflect::ComponentSchema> = compiled
            .schemas
            .iter()
            .filter(|s| !components.iter().any(|(n, _)| *n == s.name))
            .cloned()
            .collect();
        if let Ok(builtin) = area::builtin::registry() {
            described.extend(builtin.iter().map(|s| (**s).clone()));
        }
        compiled.screens = crate::screens::compile(&code.files, &engine, &components, &described)?;
    }
    Ok(Some(compiled))
}

/// Where the Lust component `name` is written (`component Name`): the
/// first file and line saying so.
fn lust_place(files: &[(String, String)], name: &str) -> Option<(String, u32)> {
    files.iter().find_map(|(path, text)| {
        text.lines().enumerate().find_map(|(i, line)| {
            let rest = line.trim_start().strip_prefix("component")?;
            let word = rest.trim_start();
            let named = word.strip_prefix(name)?;
            let whole = rest.len() > word.len()
                && !named.starts_with(|c: char| c.is_alphanumeric() || c == '_');
            whole.then(|| (path.clone(), u32::try_from(i + 1).unwrap_or(0)))
        })
    })
}

/// The mistake of having Lust but no [`ENTRY`] (none if it's there).
fn check_entry(code: &Code) -> Vec<CodeProblem> {
    if code.files.iter().any(|(path, _)| path == ENTRY) {
        return Vec::new();
    }
    vec![CodeProblem {
        file: ENTRY.to_string(),
        line: 0,
        column: 0,
        message: format!(
            "the game's code starts at {ENTRY} in the assets folder, and there isn't one. \
             Make it, and fetch the other files from it."
        ),
    }]
}

/// What's said when the compiler itself crashed.
pub(crate) fn compiler_stopped() -> CodeProblem {
    CodeProblem {
        file: ENTRY.to_string(),
        line: 0,
        column: 0,
        message: format!(
            "the Lust compiler ({}) stopped unexpectedly. This is a bug in Lust; please \
             report it with the code that caused it.",
            lust::VERSION
        ),
    }
}

/// A path with `/` between folders, as Lust takes it on every system.
fn slashed(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A new, empty folder for one test, removed when it's dropped.
    fn folder(name: &str) -> reflect::test_support::TempFolder {
        reflect::test_support::TempFolder::new(&format!("script-{name}")).unwrap()
    }

    #[test]
    fn every_lust_file_is_read_with_slashed_paths_and_hidden_folders_skipped() {
        let assets = folder("gather");
        std::fs::create_dir_all(assets.join("weapons")).unwrap();
        std::fs::create_dir_all(assets.join(".hidden")).unwrap();
        std::fs::write(assets.join("main.lst"), "end\n").unwrap();
        std::fs::write(assets.join("weapons/gun.lst"), "end\n").unwrap();
        std::fs::write(assets.join(".hidden/x.lst"), "end\n").unwrap();
        std::fs::write(assets.join("notes.txt"), "not code").unwrap();
        // A tile map is data, never part of the program (decision 0081).
        std::fs::write(
            assets.join("weapons/farm.tilemap.lst"),
            "lock int version = 3;\nend\n",
        )
        .unwrap();
        std::fs::write(assets.join("binary.lst"), [0xff, 0xfe, 0x00]).unwrap();
        std::fs::write(
            assets.join("huge.lst"),
            vec![b'-'; MAX_FILE_BYTES as usize + 1],
        )
        .unwrap();
        let code = gather(&assets);
        let paths: Vec<&str> = code.files.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(paths, ["main.lst", "weapons/gun.lst"]);
        assert_eq!(code.notes.len(), 2, "{:?}", code.notes);
        assert!(gather(&assets.join("missing")).is_empty());
    }

    #[test]
    fn two_things_with_one_name_are_a_mistake_never_a_hang() {
        // The hardening milestone's fuzzing: two components (or classes,
        // or objects) with one name never finished checking. Nor do nodes
        // and schemas, alone or with one of the others (BUG-50).
        let check = |files: Vec<(&str, &str)>| {
            let start = std::time::Instant::now();
            let got = compile(&Code {
                files: files
                    .into_iter()
                    .map(|(p, t)| (p.to_string(), t.to_string()))
                    .collect(),
                lua: Vec::new(),
                notes: Vec::new(),
            });
            assert!(start.elapsed() < std::time::Duration::from_secs(5));
            got
        };
        for text in [
            "component Health {}\ncomponent Health {}\nend\n",
            "component h{}component h{}",
            "class Health {}\n\ncomponent Health {}\nend\n",
            "object Ship {}\nobject Ship {}\nend\n",
            "node Cave {}\nnode Cave {}\nend\n",
            "schema NewUser {}\nschema NewUser {}\nend\n",
            "node Health {}\nclass Health {}\nend\n",
            "component Health {}\n\nschema Health {}\nend\n",
        ] {
            let problems = check(vec![("main.lst", text)]).unwrap_err();
            assert!(
                problems[0].message.contains("already something called"),
                "{problems:?}"
            );
        }
        let problems = check(vec![
            (
                "main.lst",
                "fetch \"other\":[Health];\ncomponent Health {}\nend\n",
            ),
            ("other.lst", "\n\ncomponent Health {}\nend\n"),
        ])
        .unwrap_err();
        assert_eq!(
            (problems[0].file.as_str(), problems[0].line),
            ("other.lst", 3)
        );
        // In comments and strings, and other names, it's fine.
        let fine = vec![(
            "main.lst".to_string(),
            "-- component Health\nstring s = \"component Health\";\ncomponent Health {}\n\
             component Armour {}\nend\n"
                .to_string(),
        )];
        assert!(shared_names(&fine).is_empty());
    }

    #[test]
    fn an_assembly_sharing_a_name_is_a_mistake_lust_finds_by_itself() {
        // Measured against the pinned Lust (BUG-50): an assembly with the
        // name of another thing is checked in no time, so the guard
        // leaves it to Lust, whose words say what's wrong.
        for text in [
            "assembly Loot {}\nassembly Loot {}\nend\n",
            "assembly Loot {}\nclass Loot {}\nend\n",
            "component Loot {}\nassembly Loot {}\nend\n",
        ] {
            let code = Code {
                files: vec![("main.lst".into(), text.into())],
                lua: Vec::new(),
                notes: Vec::new(),
            };
            assert!(shared_names(&code.files).is_empty(), "{text}");
            let problems = compile(&code).unwrap_err();
            assert_eq!(problems[0].line, 2, "{problems:?}");
            assert!(
                problems[0]
                    .message
                    .contains("Two things can't share a name"),
                "{problems:?}"
            );
        }
    }

    #[test]
    fn a_long_run_of_prefix_operators_is_refused_quickly() {
        // The third security audit: `!!!!…true` wasn't counted by the Lust
        // compiler's nesting limit and took it minutes. (The pinned Lust
        // takes each `!` for a mistake, and is as slow over many mistakes.)
        let bangs = "!".repeat(100_000);
        let main = format!("bool b = {bangs}true;\nend\n");
        let start = std::time::Instant::now();
        let problems = compile(&Code {
            files: vec![("main.lst".into(), main)],
            lua: Vec::new(),
            notes: Vec::new(),
        })
        .unwrap_err();
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
        assert!(problems[0].message.contains("in a row"), "{problems:?}");
        assert_eq!(
            (problems[0].file.as_str(), problems[0].line),
            ("main.lst", 1)
        );
        // Comments, strings and ordinary code are fine.
        let fine = vec![(
            "main.lst".to_string(),
            format!(
                "-- {dashes}\nstring s = \"{bangs}\";\nfloat f = - -1 - 2;\nend\n",
                dashes = "-".repeat(500),
                bangs = "!".repeat(500)
            ),
        )];
        assert!(long_prefix_chains(&fine).is_empty());
        let spaced = vec![(
            "a.lst".to_string(),
            "\n\nx = ".to_string() + &"! ".repeat(70) + "1",
        )];
        assert_eq!(long_prefix_chains(&spaced)[0].line, 3);
    }

    #[test]
    fn the_code_limits_hold_exactly_at_their_edges() {
        // The hardening milestone's mutation testing: the limits on code
        // files weren't tried at their edges.
        let assets = folder("edges");
        std::fs::write(assets.join("main.lst"), "end\n").unwrap();
        // A file exactly at the limit is read; one byte over isn't.
        std::fs::write(assets.join("at.lst"), vec![b'-'; MAX_FILE_BYTES as usize]).unwrap();
        std::fs::write(
            assets.join("over.lst"),
            vec![b'-'; MAX_FILE_BYTES as usize + 1],
        )
        .unwrap();
        let code = gather(&assets);
        let paths: Vec<&str> = code.files.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(paths, ["at.lst", "main.lst"]);
        assert!(
            code.notes.iter().any(|n| n.contains("over.lst")),
            "{:?}",
            code.notes
        );
        // All together: up to the total, exactly; the next is left out.
        let assets = folder("edges-total");
        let full = (MAX_TOTAL_BYTES / MAX_FILE_BYTES) as usize;
        for n in 0..full {
            std::fs::write(
                assets.join(format!("f{n:03}.lst")),
                vec![b'-'; MAX_FILE_BYTES as usize],
            )
            .unwrap();
        }
        std::fs::write(assets.join("z.lst"), "end\n").unwrap();
        let code = gather(&assets);
        assert_eq!(code.files.len(), full, "{:?}", code.notes);
        assert!(
            code.notes.iter().any(|n| n.contains("z.lst")),
            "{:?}",
            code.notes
        );
        // At most so many files: the next is left out, and said.
        let assets = folder("edges-count");
        for n in 0..=MAX_FILES {
            std::fs::write(assets.join(format!("f{n:05}.lst")), "").unwrap();
        }
        let code = gather(&assets);
        assert_eq!(code.files.len(), MAX_FILES);
        assert!(
            code.notes[0].contains(&format!("f{MAX_FILES:05}.lst")),
            "{:?}",
            code.notes
        );
    }

    #[test]
    fn code_without_main_is_a_mistake_saying_so() {
        let problems = compile(&Code {
            files: vec![("other.lst".into(), "end\n".into())],
            lua: Vec::new(),
            notes: Vec::new(),
        })
        .unwrap_err();
        assert_eq!(problems[0].file, "main.lst");
        assert!(
            problems[0].message.contains("starts at main.lst"),
            "{problems:?}"
        );
    }

    #[test]
    fn the_guards_read_past_strings_comments_and_escapes() {
        // Mutation testing: the scanners' handling of escapes, line breaks
        // in strings and single dashes wasn't tried.
        let bangs = "!".repeat(70);
        let one = |text: String| vec![("main.lst".to_string(), text)];
        // A string with an escaped quote, then a long run after it: found,
        // on the right line.
        let found = long_prefix_chains(&one(format!("s = \"a\\\"b\";\nb = {bangs}x;\n")));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 2);
        // A string left open at the end of its line ends there.
        let found = long_prefix_chains(&one(format!("s = \"open\n{bangs}x;\n")));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 2);
        // A single dash is subtraction, not a comment: what follows counts.
        let found = long_prefix_chains(&one(format!("x = 1 - 2; {bangs}x;\n")));
        assert_eq!(found.len(), 1);
        // Spaces between the !s don't hide them; anything else breaks the run.
        let spaced = "! ".repeat(70);
        assert_eq!(long_prefix_chains(&one(format!("{spaced}x\n"))).len(), 1);
        let broken = "!x".repeat(70);
        assert!(long_prefix_chains(&one(broken)).is_empty());
    }

    #[test]
    fn a_file_is_read_only_as_far_as_the_limit() {
        let assets = folder("within");
        std::fs::write(assets.join("main.lst"), "end\n").unwrap();
        let text = read_within(&assets.join("main.lst"), 4).unwrap();
        assert_eq!(text.unwrap().unwrap(), "end\n");
        assert!(read_within(&assets.join("main.lst"), 3).unwrap().is_none());
    }

    #[test]
    fn mistakes_come_back_with_file_line_and_column() {
        let code = Code {
            files: vec![
                (
                    "main.lst".into(),
                    "fetch \"parts/gun\":Gun;\nend\n".into(),
                ),
                (
                    "parts/gun.lst".into(),
                    "component Gun {\n    has {\n        int ammo = \"many\";\n    }\n}\npass Gun;\nend\n"
                        .into(),
                ),
            ],
            lua: Vec::new(),
            notes: Vec::new(),
        };
        let problems = check(&code);
        assert!(!problems.is_empty());
        let first = &problems[0];
        assert_eq!((first.file.as_str(), first.line), ("parts/gun.lst", 3));
        assert!(first.column > 0);
        assert!(
            first.to_string().starts_with("parts/gun.lst, line 3:"),
            "{first}"
        );
    }

    #[test]
    fn no_lust_is_no_mistake_but_lust_without_main_is_one() {
        assert!(check(&Code::default()).is_empty());
        let code = Code {
            files: vec![("gun.lst".into(), "end\n".into())],
            lua: Vec::new(),
            notes: Vec::new(),
        };
        let problems = check(&code);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].message.contains("main.lst"));
        let sound = Code {
            files: vec![("main.lst".into(), "end\n".into())],
            lua: Vec::new(),
            notes: Vec::new(),
        };
        assert!(check(&sound).is_empty());
    }

    /// NEW-10: the files describing the engine, which every project gets
    /// in the engine's folder, aren't game code: alone they're no code
    /// (not a missing `main.lst`), and any file of the game's own is.
    #[test]
    fn the_engine_s_own_files_alone_are_no_code() {
        let (engine, _) = crate::describe::descriptions_for(
            "Test 1.0",
            "solis",
            &crate::project_names::Names::standard(),
        );
        assert!(engine.len() > 1 && engine.iter().all(|(p, _)| p.starts_with("solis/")));
        let only_the_engine = Code {
            files: engine.clone(),
            lua: Vec::new(),
            notes: Vec::new(),
        };
        assert!(only_the_engine.is_empty());
        assert!(compile(&only_the_engine).unwrap().is_none());
        // One file of the game's own, Lust or Lua, is code.
        let mut gun = only_the_engine.clone();
        gun.files.push(("gun.lst".into(), "end\n".into()));
        assert!(!gun.is_empty());
        assert!(check(&gun)[0].message.contains("main.lst"));
        let mut lua = only_the_engine.clone();
        lua.lua.push(("gun.lua".into(), "return {}\n".into()));
        assert!(!lua.is_empty());
        // N-14: the engine's folder is the branding's, so a file elsewhere
        // saying it describes the engine is the game's own code (it made
        // its folder the engine's), and one in the engine's folder isn't.
        let marked = format!("-- Made by me{}\nend\n", crate::describe::ENGINE_FILE);
        let elsewhere = Code {
            files: vec![("tools/gun.lst".into(), marked.clone())],
            lua: Vec::new(),
            notes: Vec::new(),
        };
        assert!(!elsewhere.is_empty());
        let inside = Code {
            files: vec![("solis/gun.lst".into(), marked)],
            lua: Vec::new(),
            notes: Vec::new(),
        };
        assert!(inside.is_empty());
    }

    /// N-01: a game written only in Lua has the engine's Lust files beside
    /// its own, as every project does; they don't make it a Lust game
    /// missing its `main.lst`, and it compiles with no Lust at all.
    #[test]
    fn a_game_only_in_lua_is_no_lust_game_missing_its_main() {
        let (engine, _) = crate::describe::descriptions_for(
            "Test 1.0",
            "solis",
            &crate::project_names::Names::standard(),
        );
        let lua_only = Code {
            files: engine,
            lua: vec![(
                "spin.lua".into(),
                "local Spin = component(\"Spin\", {})\n".into(),
            )],
            notes: Vec::new(),
        };
        assert_eq!(check(&lua_only), Vec::new());
        let compiled = compile(&lua_only).unwrap().unwrap();
        assert!(compiled.lust.is_none(), "no Lust of its own to compile");
        assert!(compiled.schemas.iter().any(|s| s.name == "Spin"));
        // A Lust file of its own, beside the Lua, still needs main.lst.
        let mut both = lua_only.clone();
        both.files.push(("gun.lst".into(), "end\n".into()));
        assert!(check(&both)[0].message.contains("main.lst"));
    }

    #[test]
    fn sound_code_compiles_to_lua_and_unsound_code_gives_its_mistakes() {
        assert!(compile(&Code::default()).unwrap().is_none());
        let sound = Code {
            files: vec![("main.lst".into(), "end\n".into())],
            lua: Vec::new(),
            notes: Vec::new(),
        };
        assert!(compile(&sound).unwrap().is_some());
        let wrong = Code {
            files: vec![("main.lst".into(), "int x = \"y\";\nend\n".into())],
            lua: Vec::new(),
            notes: Vec::new(),
        };
        assert_eq!(compile(&wrong).unwrap_err()[0].line, 1);
        let headless = Code {
            files: vec![("gun.lst".into(), "end\n".into())],
            lua: Vec::new(),
            notes: Vec::new(),
        };
        assert!(
            compile(&headless).unwrap_err()[0]
                .message
                .contains("main.lst")
        );
    }

    #[test]
    fn damaged_code_never_crashes_the_checker() {
        // Thousands of randomly damaged programs (CONTRIBUTING rule 9).
        let base = "state Mode = [Single, Burst];\nevent Hit(entity by, float damage);\n\
                    component Gun {\n    has {\n        int ammo = 30;\n        Mode mode = Mode.Single;\n    }\n\
                    \x20   method update(float dt) {\n        ammo = ammo - 1;\n        send Hit(me, 2.5) to me;\n    }\n\
                    \x20   on Hit(entity by, float damage) {\n        ammo = ammo + 1;\n    }\n}\npass Gun;\nend\n";
        let mut seed: u64 = 0x5eed;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..2_000 {
            let mut bytes = base.as_bytes().to_vec();
            for _ in 0..(next() % 6 + 1) {
                let at = (next() as usize) % bytes.len();
                match next() % 3 {
                    0 => bytes[at] = b" {}()[];=\"-+*/.,:abc0"[(next() % 21) as usize],
                    1 => {
                        bytes.remove(at);
                    }
                    _ => bytes.insert(at, b"{}();\"[]"[(next() % 8) as usize]),
                }
            }
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let code = Code {
                files: vec![("main.lst".into(), text)],
                lua: Vec::new(),
                notes: Vec::new(),
            };
            let _ = check(&code);
            let _ = compile(&code);
        }
    }

    #[test]
    fn code_nested_ever_so_deep_is_a_mistake_never_a_crash() {
        // Each of these overflowed the compiler's stack once, which ends
        // the whole program (it can't be caught). Lust's own limit makes
        // them mistakes.
        let deep = |text: String| Code {
            files: vec![("main.lst".into(), text)],
            lua: Vec::new(),
            notes: Vec::new(),
        };
        let cases = [
            format!(
                "int x = {}1{};\nend\n",
                "(".repeat(100_000),
                ")".repeat(100_000)
            ),
            format!(
                "int x = 1;\nint y = {}1{};\nend\n",
                "[".repeat(5_000),
                "]".repeat(5_000)
            ),
            format!("bool x = {}true;\nend\n", "not ".repeat(200_000)),
            format!("int x = {}1;\nend\n", "- ".repeat(200_000)),
            format!("int x = 1{};\nend\n", " + 1".repeat(200_000)),
            format!(
                "component A {{\n    has {{\n        int x = 1;\n    }}\n    method update(float dt) {{\n{}x = 2;\n{}    }}\n}}\npass A;\nend\n",
                "is x == 1? then {\n".repeat(5_000),
                "}\n".repeat(5_000)
            ),
        ];
        for text in cases {
            let problems = compile(&deep(text)).unwrap_err();
            assert!(
                problems[0].message.contains("nested too deeply"),
                "{}",
                problems[0]
            );
        }
        // Deep, but not too deep, compiles, and the editor loads it on its
        // own thread, with an ordinary stack.
        let inside = format!(
            "int x = {}1{};\nint y = 1{};\nend\n",
            "(".repeat(140),
            ")".repeat(140),
            " + 1".repeat(140)
        );
        let compiled = compile(&deep(inside)).unwrap().unwrap();
        crate::Runtime::new()
            .unwrap()
            .load(&compiled, &mut crate::NoHost)
            .unwrap();
    }
}

/// Each use of a renamed native's old name in the game's code
/// ([`crate::describe::RENAMED`]): still working, but a warning for the
/// Problems panel, with the new name and when the old one stops (Phase 8,
/// C3). Words in comments are left alone.
pub fn renamed_uses(code: &Code) -> Vec<CodeProblem> {
    uses_of(crate::describe::RENAMED, code)
}

/// [`renamed_uses`], for a given list of renamed natives.
fn uses_of(renamed: &[crate::describe::Renamed], code: &Code) -> Vec<CodeProblem> {
    let mut found = Vec::new();
    // Nothing renamed (as now): nothing to look for.
    if renamed.is_empty() {
        return found;
    }
    // Each name to look for, worked out once: in Lust, and in Lua.
    let needles: Vec<[(String, String); 2]> = renamed
        .iter()
        .map(|r| {
            [
                (r.old.to_owned(), r.new.to_owned()),
                (format!("solis.{}", r.old), format!("solis.{}", r.new)),
            ]
        })
        .collect();
    for (lua, (file, text)) in code
        .files
        .iter()
        .map(|f| (false, f))
        .chain(code.lua.iter().map(|f| (true, f)))
    {
        for (n, line) in text.lines().enumerate() {
            // Not in a comment (`--` in Lust and Lua alike).
            let code_part = line.split("--").next().unwrap_or_default();
            for (r, names) in renamed.iter().zip(&needles) {
                let (old, new) = &names[usize::from(lua)];
                let (old, new) = (old.as_str(), new.as_str());
                let mut from = 0;
                while let Some(at) = code_part[from..].find(old) {
                    let start = from + at;
                    let end = start + old.len();
                    from = end;
                    let word = |c: char| c.is_alphanumeric() || c == '_';
                    let before = code_part[..start].chars().next_back();
                    let after = code_part[end..].chars().next();
                    if before.is_some_and(|c| word(c) || (!lua && c == '.'))
                        || after.is_some_and(word)
                    {
                        continue;
                    }
                    found.push(CodeProblem {
                        file: file.clone(),
                        line: u32::try_from(n + 1).unwrap_or(u32::MAX),
                        column: u32::try_from(code_part[..start].chars().count() + 1)
                            .unwrap_or(u32::MAX),
                        message: format!(
                            "{old} is now {new}; the old name stops working in {} (it still \
                             works until then)",
                            r.until
                        ),
                    });
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod renamed_tests {
    use super::*;
    use crate::describe::Renamed;

    const SPAWN_AT: Renamed = Renamed {
        file: "world",
        old: "spawnAt",
        new: "spawn",
        until: "Phase 12",
    };

    #[test]
    fn each_use_of_an_old_name_is_found_where_it_is() {
        let code = Code {
            files: vec![(
                "main.lst".into(),
                "get \"solis/world\":[spawnAt, say];\n\
                 -- spawnAt used to be the name\n\
                 component A {\n    method update(float dt) {\n        entity e = spawnAt(\"ships/fighter\", [0, 0, 0]); -- spawnAt\n        say(\"respawnAt\");\n        notspawnAt(1);\n    }\n}\n"
                    .into(),
            )],
            lua: vec![(
                "ships.lua".into(),
                "local e = solis.spawnAt(\"ships/fighter\", {0, 0, 0})\nlocal f = mysolis.spawnAt\n"
                    .into(),
            )],
            notes: Vec::new(),
        };
        let found = uses_of(&[SPAWN_AT], &code);
        let at: Vec<(&str, u32, u32)> = found
            .iter()
            .map(|p| (p.file.as_str(), p.line, p.column))
            .collect();
        assert_eq!(
            at,
            [
                ("main.lst", 1, 20),
                ("main.lst", 5, 20),
                ("ships.lua", 1, 11)
            ]
        );
        assert!(found[0].message.contains("spawnAt is now spawn"));
        assert!(found[0].message.contains("Phase 12"));
        assert!(
            found[2]
                .message
                .contains("solis.spawnAt is now solis.spawn")
        );
        assert!(renamed_uses(&code).is_empty(), "nothing is renamed yet");
    }

    #[test]
    fn a_call_that_moved_is_a_mistake_that_says_the_new_way() {
        let mut files = crate::describe::descriptions("Test Engine 9.9.9", "solis");
        files.push((
            "main.lst".into(),
            "component A {\n    method update(float dt) {\n        vec3 at = positionOf(this);\n        moveTo(this, at);\n    }\n}\npass A;\nend\n"
                .into(),
        ));
        let code = Code {
            files,
            lua: vec![(
                "b.lua".into(),
                "-- solis.say(\"in a comment\")\nlocal at = solis.positionOf(self.me) solis.entity.position(self.me)\n".into(),
            )],
            notes: Vec::new(),
        };
        let problems = check(&code);
        let said: Vec<String> = problems.iter().map(ToString::to_string).collect();
        assert!(
            said.iter().any(|p| p.starts_with(
                "b.lua, line 2:12: solis.positionOf is now written solis.entity.position(target)"
            )),
            "{said:?}"
        );
        assert_eq!(
            said.iter().filter(|p| p.starts_with("b.lua")).count(),
            1,
            "not in a comment, nor the new way: {said:?}"
        );
        let lust: Vec<&String> = said.iter().filter(|p| p.starts_with("main.lst")).collect();
        assert!(
            lust.iter()
                .any(|p| p.contains("positionOf is now written target:position()")),
            "{lust:?}"
        );
        assert!(
            lust.iter()
                .any(|p| p.contains("moveTo is now written target:moveTo(place)")),
            "{lust:?}"
        );
    }
}
