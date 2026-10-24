//! A project's Lust: every `.lst` file in its assets, read within limits
//! (CONTRIBUTING rule 9), and checked by the Lust compiler. Both are slow
//! enough to run on a worker, never on the frame.

use std::path::Path;

/// The most a code file may hold.
pub const MAX_FILE_BYTES: u64 = 1 << 20;
/// The most all code files together may hold.
pub const MAX_TOTAL_BYTES: u64 = 16 << 20;
/// The most code files read.
pub const MAX_FILES: usize = 10_000;
/// The game's first file, which Lust starts from.
pub const ENTRY: &str = "main.lst";

/// A project's Lust files, read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Code {
    /// Each file's path inside the assets (with `/` between folders) and
    /// its text, sorted by path.
    pub files: Vec<(String, String)>,
    /// Files left out, and why, in plain words.
    pub notes: Vec<String>,
}

impl Code {
    /// Whether there's any Lust at all.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
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

/// Reads every `.lst` file under `assets`, skipping hidden folders and
/// links to folders (which could loop), within the limits above.
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
                    .is_some_and(|e| e.eq_ignore_ascii_case("lst"))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    for path in found {
        let inside = slashed(path.strip_prefix(assets).unwrap_or(&path));
        if code.files.len() >= MAX_FILES {
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
        match std::fs::read(&path).map(String::from_utf8) {
            Ok(Ok(text)) => {
                total += size;
                code.files.push((inside, text));
            }
            Ok(Err(_)) => code
                .notes
                .push(format!("{inside} is left out: it isn't text.")),
            Err(e) => code.notes.push(format!("{inside} couldn't be read ({e}).")),
        }
    }
    code
}

/// Checks the game's Lust, starting from [`ENTRY`]: every mistake found,
/// in order (none when it's sound, or there's no Lust). A crash inside the
/// compiler is reported as a mistake too, never passed on.
pub fn check(code: &Code) -> Vec<CodeProblem> {
    if code.is_empty() {
        return Vec::new();
    }
    let missing = check_entry(code);
    if !missing.is_empty() {
        return missing;
    }
    let files = &code.files;
    match std::panic::catch_unwind(|| lust::check(ENTRY, files)) {
        Ok(Ok(())) => Vec::new(),
        Ok(Err(problems)) => problems.into_iter().map(CodeProblem::from).collect(),
        Err(_) => vec![compiler_stopped()],
    }
}

/// The game's Lust checked and turned into Lua for Luau, starting from
/// [`ENTRY`]: `Ok(None)` when there's no Lust, the Lua when it's sound, or
/// every mistake found. A crash inside the compiler is a mistake too.
pub fn compile(code: &Code) -> Result<Option<lust::LuaChunk>, Vec<CodeProblem>> {
    if code.is_empty() {
        return Ok(None);
    }
    let missing = check_entry(code);
    if !missing.is_empty() {
        return Err(missing);
    }
    let files = &code.files;
    match std::panic::catch_unwind(|| lust::to_lua(ENTRY, files)) {
        Ok(Ok(chunk)) => Ok(Some(chunk)),
        Ok(Err(problems)) => Err(problems.into_iter().map(CodeProblem::from).collect()),
        Err(_) => Err(vec![compiler_stopped()]),
    }
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
            notes: Vec::new(),
        };
        let problems = check(&code);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].message.contains("main.lst"));
        let sound = Code {
            files: vec![("main.lst".into(), "end\n".into())],
            notes: Vec::new(),
        };
        assert!(check(&sound).is_empty());
    }

    #[test]
    fn sound_code_compiles_to_lua_and_unsound_code_gives_its_mistakes() {
        assert!(compile(&Code::default()).unwrap().is_none());
        let sound = Code {
            files: vec![("main.lst".into(), "end\n".into())],
            notes: Vec::new(),
        };
        assert!(compile(&sound).unwrap().is_some());
        let wrong = Code {
            files: vec![("main.lst".into(), "int x = \"y\";\nend\n".into())],
            notes: Vec::new(),
        };
        assert_eq!(compile(&wrong).unwrap_err()[0].line, 1);
        let headless = Code {
            files: vec![("gun.lst".into(), "end\n".into())],
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
                notes: Vec::new(),
            };
            let _ = check(&code);
            let _ = compile(&code);
        }
    }
}
