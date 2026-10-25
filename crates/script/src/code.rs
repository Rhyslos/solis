//! A project's Lust: every `.lst` file in its assets, read within limits
//! (CONTRIBUTING rule 9), and checked by the Lust compiler. Both are slow
//! enough to run on a worker, never on the frame.

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
/// The stack the compiler (and the code's start, to read its components)
/// runs with. Lust keeps its own nesting limit and stack since 2.0.0-alpha.35;
/// this is room to spare for Luau compiling what Lust made.
const COMPILER_STACK: usize = 64 << 20;

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
    /// Whether there's any game code at all.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.lua.is_empty()
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
            message: p.message,
        }
    }
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
/// worker. Code nested too deep is a mistake Lust finds.
pub fn compile(code: &Code) -> Result<Option<Compiled>, Vec<CodeProblem>> {
    if code.is_empty() {
        return Ok(None);
    }
    let code = code.clone();
    let (sender, answer) = std::sync::mpsc::channel();
    let worker = std::thread::Builder::new()
        .name("lust compiler".into())
        .stack_size(COMPILER_STACK)
        .spawn(move || {
            let _ = sender.send(compile_here(&code));
        });
    let worker = match worker {
        Ok(worker) => worker,
        Err(why) => {
            return Err(vec![CodeProblem {
                file: ENTRY.to_string(),
                line: 0,
                column: 0,
                message: format!(
                    "the code couldn't be checked: there was no room to start ({why})"
                ),
            }]);
        }
    };
    // A compiler that never finishes (a case no guard here knows yet) is
    // given up on, not waited for forever: the code is reported as not
    // checked, so the editor isn't left waiting. Its thread is left to
    // finish on its own.
    match answer.recv_timeout(COMPILE_LIMIT) {
        // It answered: its thread is ending, and is waited for, so nothing
        // of it is left behind (found by fuzzing).
        Ok(result) => {
            let _ = worker.join();
            result
        }
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(vec![CodeProblem {
            file: ENTRY.to_string(),
            line: 0,
            column: 0,
            message: format!(
                "checking the code took more than {} seconds, so it was given up on. Something \
                 in it is more than the checker can handle: undo the last change, or look for \
                 something unusual (please report it)",
                COMPILE_LIMIT.as_secs()
            ),
        }]),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(vec![compiler_stopped()]),
    }
}

/// The longest checking the code may take before it's given up on.
pub const COMPILE_LIMIT: std::time::Duration = std::time::Duration::from_secs(30);

/// The longest run of `!` (as in `!!!x`) allowed in Lust. The Lust
/// compiler's nesting limit counts `not` and `-` but not `!`, and a long
/// run takes it time that grows with the square of its length (a file of
/// them, hours), so a run longer than any real code has is refused before
/// compiling.
pub const MAX_PREFIX_RUN: usize = 64;

/// Problems for runs of `!` longer than [`MAX_PREFIX_RUN`],
/// outside comments (`--` to the end of the line) and strings.
fn long_prefix_chains(files: &[(String, String)]) -> Vec<CodeProblem> {
    let mut problems = Vec::new();
    for (path, text) in files {
        let mut run = 0usize;
        let mut line = 1u32;
        let mut chars = text.chars().peekable();
        let mut in_string = false;
        while let Some(c) = chars.next() {
            if c == '\n' {
                line += 1;
            }
            if in_string {
                match c {
                    '\\' => {
                        chars.next();
                    }
                    '"' | '\n' => in_string = false,
                    _ => {}
                }
                continue;
            }
            match c {
                '"' => {
                    in_string = true;
                    run = 0;
                }
                '-' if chars.peek() == Some(&'-') => {
                    // A comment, to the end of the line.
                    for c in chars.by_ref() {
                        if c == '\n' {
                            line += 1;
                            break;
                        }
                    }
                    run = 0;
                }
                '!' => {
                    run += 1;
                    if run == MAX_PREFIX_RUN + 1 {
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
                c if c.is_whitespace() => {}
                _ => run = 0,
            }
        }
    }
    problems
}

/// Problems for two components, classes or objects with the same name,
/// in one file or two. The Lust compiler (rev a60cf87) never finishes
/// such code, so it's found here first (by the hardening milestone's
/// fuzzing, decision 0042), with what to do about it. Outside comments and
/// strings, as [`long_prefix_chains`] reads.
fn shared_names(files: &[(String, String)]) -> Vec<CodeProblem> {
    let mut problems = Vec::new();
    let mut seen: std::collections::HashMap<String, (String, u32)> =
        std::collections::HashMap::new();
    for (path, text) in files {
        for (line, word, name) in declarations(text) {
            match seen.get(&name) {
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
                    seen.insert(name, (path.clone(), line));
                }
            }
        }
    }
    problems
}

/// Each `component`, `class` or `object` declared in `text`: its line,
/// the word, and its name. Comments (`--` to the end of the line) and
/// strings are skipped.
fn declarations(text: &str) -> Vec<(u32, &'static str, String)> {
    let mut found = Vec::new();
    let mut line = 1u32;
    let mut word = String::new();
    // The declaring word just read, waiting for its name.
    let mut pending: Option<&'static str> = None;
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    let mut finish = |word: &mut String, pending: &mut Option<&'static str>, line: u32| {
        if word.is_empty() {
            return;
        }
        if let Some(kind) = pending.take() {
            found.push((line, kind, word.clone()));
        } else {
            *pending = match word.as_str() {
                "component" => Some("component"),
                "class" => Some("class"),
                "object" => Some("object"),
                _ => None,
            };
        }
        word.clear();
    };
    while let Some(c) = chars.next() {
        if in_string {
            match c {
                '\\' => {
                    chars.next();
                }
                '"' => in_string = false,
                '\n' => {
                    in_string = false;
                    line += 1;
                }
                _ => {}
            }
            continue;
        }
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
            continue;
        }
        finish(&mut word, &mut pending, line);
        match c {
            '"' => {
                in_string = true;
                pending = None;
            }
            '-' if chars.peek() == Some(&'-') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        line += 1;
                        break;
                    }
                }
            }
            '\n' => line += 1,
            c if c.is_whitespace() => {}
            // Anything else between the word and a name: not a
            // declaration (`component:` in a comment-free odd place, say).
            _ => pending = None,
        }
    }
    finish(&mut word, &mut pending, line);
    found
}

/// [`compile`], on this thread.
fn compile_here(code: &Code) -> Result<Option<Compiled>, Vec<CodeProblem>> {
    let mut problems = Vec::new();
    let mut lust = None;
    if !code.files.is_empty() {
        let mut missing = check_entry(code);
        missing.extend(long_prefix_chains(&code.files));
        missing.extend(shared_names(&code.files));
        if missing.is_empty() {
            let files = &code.files;
            match std::panic::catch_unwind(|| lust::to_lua(ENTRY, files)) {
                Ok(Ok(chunk)) => lust = Some(chunk),
                Ok(Err(found)) => problems.extend(found.into_iter().map(CodeProblem::from)),
                Err(_) => problems.push(compiler_stopped()),
            }
        } else {
            problems.extend(missing);
        }
    }
    if !code.lua.is_empty() {
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
    let compiler = mlua::Compiler::new();
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
    };
    // Starting it here reads its components, so the editor needn't on the
    // frame.
    let started = crate::Runtime::new().and_then(|mut runtime| {
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
fn compiler_stopped() -> CodeProblem {
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

    fn folder(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir()
            .join(format!("solis-script-{}", std::process::id()))
            .join(name);
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
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
        // or objects) with one name never finished checking.
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
    fn a_long_run_of_prefix_operators_is_refused_quickly() {
        // The third security audit: `!!!!…true` isn't counted by the Lust
        // compiler's nesting limit and took it minutes.
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
        let _ = std::fs::remove_dir_all(&assets);
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
        let _ = std::fs::remove_dir_all(&assets);
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
        // Declarations: after a string with an escaped quote; across a
        // line break between the word and its name; not after a comment.
        let decls = declarations(
            "s = \"\\\"component Fake\";\ncomponent\n  Real {}\n-- component Gone\nclass Kept {}\n",
        );
        let names: Vec<(u32, &str, &str)> =
            decls.iter().map(|(l, w, n)| (*l, *w, n.as_str())).collect();
        assert_eq!(names, [(3, "component", "Real"), (5, "class", "Kept")]);
        // A word that only starts like one isn't a declaration.
        assert!(declarations("components X {}\nmy_class Y {}\n").is_empty());
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
