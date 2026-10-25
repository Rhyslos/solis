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
    let worker = std::thread::Builder::new()
        .name("lust compiler".into())
        .stack_size(COMPILER_STACK)
        .spawn(move || compile_here(&code));
    match worker.map(std::thread::JoinHandle::join) {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err(vec![compiler_stopped()]),
        Err(why) => Err(vec![CodeProblem {
            file: ENTRY.to_string(),
            line: 0,
            column: 0,
            message: format!("the code couldn't be checked: there was no room to start ({why})"),
        }]),
    }
}

/// [`compile`], on this thread.
fn compile_here(code: &Code) -> Result<Option<Compiled>, Vec<CodeProblem>> {
    let mut problems = Vec::new();
    let mut lust = None;
    if !code.files.is_empty() {
        let missing = check_entry(code);
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
