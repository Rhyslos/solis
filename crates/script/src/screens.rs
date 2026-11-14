//! Game screens ([decision 0087](../../../docs/decisions/0087-screens-through-the-engine.md)):
//! each `assets/ui/<name>.lst` passes an assembly with a `view`, named
//! after its file. They're compiled into a program of their own (an entry
//! Solis writes, opening every screen) and run in a Luau of their own,
//! never the game's: Lust's `LUST_HOST.ui_open` hands over each screen's
//! view, which is run after each tick and read as a tree for `ui` to lay
//! out. The game-wide objects are copied in, as every core gets them; a
//! screen changing one stops with Lust's message, and anything else it
//! changes stays in its own Luau. `Ui:entity()` is the entity a screen is
//! shown for, and a reading method on entities for each Lust component
//! (`solis/screen.lst`, which only screens fetch) gives a copy of it as the
//! last tick left it. Nothing else of the engine's is lent.

use crate::code::CodeProblem;
use crate::runtime::{Failure, Runtime};
use crate::ticking::plain;
use mlua::{Function, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;
use ui::view::{Node, Style};

/// The folder screens are in, inside the assets.
pub const FOLDER: &str = "ui/";
/// The entry Solis writes for the screens' program: beside `main.lst`,
/// with a space in its name so no file of the game's can be called it.
pub const ENTRY: &str = "solis screens.lst";
/// The most screens a project has.
pub const MAX_SCREENS: usize = 64;
/// The most pieces read from one screen's view.
pub const MAX_PIECES: usize = ui::screen::MAX_ELEMENTS;
/// The most styles read from one screen's view, on all its pieces
/// together, those inside others (`hover:[…]`) counted too.
pub const MAX_STYLES: usize = 200_000;
/// The most text copied out of one screen's view, in bytes: its pieces'
/// kinds and words and their styles' names and words together.
pub const MAX_VIEW_BYTES: usize = 4 * 1024 * 1024;
/// The longest words of one piece or one style, in bytes: the rest is cut.
const MAX_WORDS: usize = 16 * 1024;
/// The longest a screen's view may take to run, each time.
pub const VIEW_LIMIT: std::time::Duration = std::time::Duration::from_millis(50);

/// The screens' program, compiled.
#[derive(Debug, Clone)]
pub struct ScreenCode {
    /// The Lua for Luau.
    pub chunk: lust::LuaChunk,
    /// Each screen's name (`Hud`) and file (`ui/hud.lst`), in the order
    /// they're opened.
    pub screens: Vec<(String, String)>,
    /// The components screens can read, each with its reading method's
    /// name (`Health`, `health`), as `solis/screen.lst` offers them.
    pub readers: Vec<(String, String)>,
    /// What's worth saying about them: components left out, and why.
    pub notes: Vec<String>,
    /// Every picture a screen names in its code (decision 0089), where.
    pub pictures: Vec<PictureUse>,
    /// The screens' files as they were compiled (path, text), for the UI
    /// editor's canvas to read and change.
    pub sources: Vec<(String, String)>,
    /// The game's events (`event Pause();`) and the files they're
    /// written in, for a button to send (decision 0089).
    pub events: Vec<(String, String)>,
}

/// A picture named in a screen's code: `image "sprites/tools.png#Axe"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureUse {
    /// The screen's file.
    pub file: String,
    /// Its line and column, from 1.
    pub line: u32,
    /// Its column, from 1 (at the name's opening quote).
    pub column: u32,
    /// The name, as written: a picture's path inside the assets, perhaps
    /// with `#` and a crop's name.
    pub name: String,
}

/// The most picture names read from the screens' code.
pub const MAX_PICTURE_USES: usize = 4096;

/// Every `image "…"` with its name written out in `files`' screens (those
/// in [`FOLDER`]), skipping comments; a name worked out as the game runs is
/// found then. At most [`MAX_PICTURE_USES`].
pub fn pictures_named(files: &[(String, String)]) -> Vec<PictureUse> {
    let mut found = Vec::new();
    for (path, text) in files.iter().filter(|(p, _)| p.starts_with(FOLDER)) {
        for (n, line) in text.lines().enumerate() {
            let code = line.split("--").next().unwrap_or("");
            let mut rest = code;
            let mut gone = 0;
            while let Some(at) = rest.find("image") {
                let before = code[..gone + at].chars().next_back();
                let after = &rest[at + "image".len()..];
                let word_start = before.is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
                let trimmed = after.trim_start();
                if word_start
                    && after.len() != trimmed.len()
                    && let Some(quoted) = trimmed.strip_prefix('"')
                    && let Some(end) = quoted.find('"')
                    && !quoted[..end].contains('\\')
                {
                    if found.len() >= MAX_PICTURE_USES {
                        return found;
                    }
                    let quote_at = gone + at + "image".len() + (after.len() - trimmed.len());
                    found.push(PictureUse {
                        file: path.clone(),
                        line: u32::try_from(n + 1).unwrap_or(u32::MAX),
                        column: u32::try_from(code[..quote_at].chars().count() + 1)
                            .unwrap_or(u32::MAX),
                        name: quoted[..end].to_string(),
                    });
                }
                gone += at + "image".len();
                rest = &code[gone..];
            }
        }
    }
    found
}

/// The module only screens fetch (decision 0087), `solis/screen`: `Ui`,
/// and a reading method on entities for each component. In the engine's
/// folder ([`crate::describe::in_engine_folder`]), as the screen editor
/// writes it into a screen's `fetch` lines.
pub fn readers_module() -> String {
    format!("{}/screen", area::lust_data::ENGINE_FOLDER)
}

/// The reading method for a component called `name` (`Health` gives
/// `health`).
pub fn reader_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// Lust's own words (LUST.md, section 22), its built-ins and its
/// built-in objects: no field Solis writes into a screen's Lust may be
/// called one.
const LUST_WORDS: &[&str] = &[
    "class",
    "object",
    "node",
    "assembly",
    "create",
    "has",
    "method",
    "hook",
    "return",
    "like",
    "new",
    "nothing",
    "schema",
    "route",
    "view",
    "style",
    "keep",
    "pass",
    "fetch",
    "get",
    "lua",
    "is",
    "then",
    "or",
    "either",
    "not",
    "true",
    "false",
    "loop",
    "stop",
    "skip",
    "in",
    "try",
    "error",
    "state",
    "save",
    "load",
    "saved",
    "end",
    "database",
    "int",
    "float",
    "string",
    "bool",
    "array",
    "cMap",
    "map",
    "native",
    "component",
    "check",
    "when",
    "lock",
    "print",
    "wait",
    "ask",
    "Maths",
    "Files",
    "Time",
    "Json",
    "Web",
    "Random",
    "this",
    "me",
];

/// Whether `name` can be a name in Lust: a letter, then letters, digits
/// and `_`, not all capitals, and not one of Lust's own words.
fn lust_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !(name.len() > 1 && name.chars().all(|c| !c.is_ascii_lowercase()))
        && !LUST_WORDS.contains(&name)
}

/// A field's Lust type and starting value, for a component Solis
/// describes to screens; `None` for a kind screens can't hold (an asset).
fn lust_field(kind: &reflect::FieldKind) -> Option<(&'static str, &'static str)> {
    use reflect::FieldKind;
    Some(match kind {
        FieldKind::Bool => ("bool", "false"),
        FieldKind::Int => ("int", "0"),
        FieldKind::Float => ("float", "0"),
        FieldKind::Text | FieldKind::Choice(_) => ("string", "\"\""),
        // Solis's own colours reach screens as they always have: as vec3s.
        FieldKind::Vec3 | FieldKind::Color => ("vec3", "[0, 0, 0]"),
        FieldKind::Vec2 => ("vec2", "[0, 0]"),
        FieldKind::Rotation => ("rotation", "[0, 0, 0]"),
        FieldKind::Entity => ("entity", "nothing"),
        FieldKind::Asset | FieldKind::Numbers => return None,
    })
}

/// `solis/screen.lst` (decision 0087): `Ui:entity()`, and a reading method
/// for each of the game's Lust components (`lust`: each one's name and
/// file) and each other component screens can read (`described`: the
/// game's Lua and Rust components and Solis's own, by their schemas),
/// except one whose method name an engine call on entities has already,
/// or that isn't a Lust name (a note says so, unless it's one of Solis's
/// own, which nobody can rename). A described one is written into the file as
/// a Lust component of its fields (those Lust can hold), so a screen
/// fetches it from `solis/screen` and gets a copy as Lust components do.
/// Gives the file, the readers kept, and notes on those left out.
pub fn readers_file(
    lust: &[(String, String)],
    described: &[reflect::ComponentSchema],
) -> (String, Vec<(String, String)>, Vec<String>) {
    let taken: Vec<&str> = crate::describe::NATIVES
        .iter()
        .filter(|n| n.owner == "entity")
        .map(|n| n.name)
        .collect();
    let mut text = String::from(
        "-- Made for the screens (decision 0087): what a screen can read. Only\n\
         -- screens fetch it.\n\n",
    );
    let mut readers = Vec::new();
    let mut notes = Vec::new();
    let readable = |name: &str, notes: &mut Vec<String>| {
        let reader = reader_name(name);
        if taken.contains(&reader.as_str()) {
            notes.push(format!(
                "Screens can't read the component {name}: entities already have a {reader}(). \
                 Give the component another name to read it from a screen."
            ));
            return None;
        }
        if !lust_name(name) || crate::describe::PROJECT_STATES.contains(&name) {
            notes.push(format!(
                "Screens can't read the component {name}: its name can't be a Lust name."
            ));
            return None;
        }
        Some(reader)
    };
    for (name, file) in lust {
        let Some(reader) = readable(name, &mut notes) else {
            continue;
        };
        let module = file.strip_suffix(".lst").unwrap_or(file);
        text.push_str(&format!("fetch \"{module}\":{name};\n"));
        readers.push((name.clone(), reader));
    }
    let mut written = Vec::new();
    // Solis's own components (Screen, Transform…) left out aren't worth a
    // note: nobody can rename them.
    let own = area::builtin::registry().ok();
    for schema in described {
        if lust.iter().any(|(n, _)| *n == schema.name) {
            continue;
        }
        let mut said = Vec::new();
        let reader = readable(&schema.name, &mut said);
        if own.as_ref().is_none_or(|r| r.get(&schema.name).is_none()) {
            notes.extend(said);
        }
        let Some(reader) = reader else {
            continue;
        };
        let mut fields = String::new();
        for field in &schema.fields {
            if let Some((kind, start)) = lust_field(&field.kind).filter(|_| lust_name(&field.name))
            {
                fields.push_str(&format!("        {kind} {} = {start};\n", field.name));
            }
        }
        text.push_str(&format!(
            "\n-- {name}'s fields, as the last tick left them, for screens to read.\n\
             component {name} {{\n    has {{\n{fields}    }}\n}}\n",
            name = schema.name
        ));
        written.push(schema.name.clone());
        readers.push((schema.name.clone(), reader));
    }
    text.push_str(
        "\nobject Ui {\n    -- The entity this screen is shown for: the one whose Screen \
         shows it.\n    method entity entity() native;\n}\n\n",
    );
    for (name, reader) in &readers {
        text.push_str(&format!(
            "-- A copy of the entity's {name} as the last tick left it; the screen stops if it \
             has none.\nmethod {name} entity:{reader}() native;\n\n"
        ));
    }
    let passed: Vec<&str> = std::iter::once("Ui")
        .chain(written.iter().map(String::as_str))
        .collect();
    text.push_str(&format!("pass {};\nend\n", passed.join(", ")));
    (text, readers, notes)
}

/// The name a screen file's assembly has: its file's name, first letter
/// made a capital (`ui/pauseMenu.lst` passes `PauseMenu`). None if it
/// can't be a name.
pub fn screen_name(path: &str) -> Option<String> {
    let stem = path.rsplit('/').next()?.strip_suffix(".lst")?;
    let mut chars = stem.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() || !chars.clone().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(first.to_ascii_uppercase().to_string() + chars.as_str())
}

/// The screens in `files` (by path), with their names, and mistakes for
/// files that can't be screens or names used twice.
pub fn find(files: &[(String, String)]) -> (Vec<(String, String)>, Vec<CodeProblem>) {
    let mut screens: Vec<(String, String)> = Vec::new();
    let mut problems = Vec::new();
    for (path, _) in files.iter().filter(|(p, _)| p.starts_with(FOLDER)) {
        let problem = |message: String| CodeProblem {
            file: path.clone(),
            line: 0,
            column: 0,
            message,
        };
        match screen_name(path) {
            None => problems.push(problem(
                "a screen's file name is its assembly's name, so it starts with a letter and \
                 holds only letters and numbers (ui/hud.lst passes Hud)"
                    .into(),
            )),
            Some(name) if screens.iter().any(|(n, _)| *n == name) => problems.push(problem(
                format!("another screen is called {name} too: give one file another name"),
            )),
            Some(_) if screens.len() >= MAX_SCREENS => problems.push(problem(format!(
                "a project has at most {MAX_SCREENS} screens"
            ))),
            Some(name) => screens.push((name, path.clone())),
        }
    }
    (screens, problems)
}

/// The events declared in the game's Lust files (`event Pause();` at a
/// line's start), with their files: Solis's own files left out.
pub fn events_named(files: &[(String, String)]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (path, text) in files {
        if crate::describe::in_engine_folder(path) || !path.ends_with(".lst") {
            continue;
        }
        for line in text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("event ") else {
                continue;
            };
            let name: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if name.starts_with(|c: char| c.is_ascii_alphabetic())
                && !out.iter().any(|(n, _)| *n == name)
                && out.len() < 1024
            {
                out.push((name, path.clone()));
            }
        }
    }
    out.sort();
    out
}

/// The entry that opens every screen.
pub fn entry(screens: &[(String, String)]) -> String {
    let mut text = String::from("-- Made for the screens: never saved.\n");
    // The readers' file always, so the components it describes are known
    // to Lust even when no screen fetches it.
    text.push_str(&format!("fetch \"{}\":Ui;\n", readers_module()));
    for (name, path) in screens {
        let module = path.strip_suffix(".lst").unwrap_or(path);
        text.push_str(&format!("fetch \"{module}\":{name};\n"));
    }
    for (name, _) in screens {
        text.push_str(&format!("{name}:open();\n"));
    }
    text.push_str("end\n");
    text
}

/// Compiles the screens in `files` (none: nothing to do), with the
/// engine's folders and a reading method for each of `components` (each
/// Lust component's name and file) and each of `described` (see
/// [`readers_file`]). Mistakes name the screen's file and line.
pub fn compile(
    files: &[(String, String)],
    engine: &lust::Engine,
    components: &[(String, String)],
    described: &[reflect::ComponentSchema],
) -> Result<Option<ScreenCode>, Vec<CodeProblem>> {
    let (screens, mut problems) = find(files);
    if screens.is_empty() {
        return if problems.is_empty() {
            Ok(None)
        } else {
            Err(problems)
        };
    }
    let (readers_text, readers, notes) = readers_file(components, described);
    let readers_path = format!("{}.lst", readers_module());
    let mut with_entry = files.to_vec();
    with_entry.retain(|(p, _)| p != ENTRY && *p != readers_path);
    with_entry.push((ENTRY.to_string(), entry(&screens)));
    with_entry.push((readers_path, readers_text));
    // The readers' file is the engine's, like the engine folder's others.
    let mut engine = lust::Engine {
        folders: engine.folders.clone(),
    };
    for folder in crate::describe::engine_folders() {
        if !engine.folders.contains(&folder) {
            engine.folders.push(folder);
        }
    }
    let sources: Vec<(String, String)> = files
        .iter()
        .filter(|(p, _)| screens.iter().any(|(_, f)| f == p))
        .cloned()
        .collect();
    match std::panic::catch_unwind(|| lust::to_lua_with(&engine, ENTRY, &with_entry)) {
        Ok(Ok(chunk)) if problems.is_empty() => Ok(Some(ScreenCode {
            chunk,
            screens,
            readers,
            notes,
            pictures: pictures_named(files),
            sources,
            events: events_named(files),
        })),
        Ok(Ok(_)) => Err(problems),
        Ok(Err(found)) => {
            problems.extend(found.into_iter().map(CodeProblem::from).map(|mut p| {
                if p.file == ENTRY {
                    // The entry is Solis's: what it fetches is missing.
                    p.message = format!(
                        "{} (each file in ui/ passes the assembly named after it: ui/hud.lst \
                         passes Hud, with pass Hud;)",
                        p.message
                    );
                }
                p
            }));
            Err(problems)
        }
        Err(_) => Err(vec![CodeProblem {
            file: ENTRY.into(),
            line: 0,
            column: 0,
            message: "the screens couldn't be checked: the checker stopped".into(),
        }]),
    }
}

/// The screens, running: a Luau of their own with the screens' program.
pub struct Screens {
    runtime: Runtime,
    /// Each screen's view, by its name, in the order opened.
    views: Vec<(String, Function)>,
    /// Where each screen is written.
    files: Vec<String>,
    /// The components screens read, with their readers' names.
    readers: Vec<(String, String)>,
    /// Each screen's buttons from its last view, by the screen's name and
    /// the entity it was shown for, in their numbers' order.
    actions: RefCell<Buttons>,
}

/// Each screen's buttons, by its name and the entity it's shown for.
type Buttons = std::collections::BTreeMap<(String, Option<reflect::EntityId>), Vec<Function>>;

/// The most buttons one view keeps.
pub const MAX_ACTIONS: usize = 1024;

/// Where screens read components' values from: the running game.
pub trait ScreenReader {
    /// The values of the component `name` on `entity` as the last tick
    /// left them, if it has one.
    fn values(&mut self, entity: reflect::EntityId, name: &str) -> Option<reflect::ComponentData>;

    /// The player's settings, for `Screens:textSize()` and the rest
    /// (decision 0091).
    fn settings(&self) -> crate::settings::PlayerSettings {
        crate::settings::PlayerSettings::default()
    }
}

/// No game to read: every entity has no components.
pub struct NoReader;

impl ScreenReader for NoReader {
    fn values(&mut self, _: reflect::EntityId, _: &str) -> Option<reflect::ComponentData> {
        None
    }
}

impl std::fmt::Debug for Screens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Screens")
            .field("views", &self.views.len())
            .finish_non_exhaustive()
    }
}

impl Screens {
    /// Starts `code`'s screens: their program's own start runs, and each
    /// screen hands over its view.
    pub fn start(code: &ScreenCode) -> Result<Self, Failure> {
        let opened: Rc<RefCell<Vec<Function>>> = Rc::default();
        let mut runtime = Runtime::new()?;
        runtime.open_views(Rc::clone(&opened));
        let compiled = crate::Compiled {
            lust: Some(code.chunk.clone()),
            lua: Vec::new(),
            schemas: Vec::new(),
            places: Default::default(),
            bytecode: Default::default(),
            screens: None,
        };
        runtime.load(&compiled, &mut crate::NoHost)?;
        let opened = opened.borrow().clone();
        let views = code
            .screens
            .iter()
            .map(|(name, _)| name.clone())
            .zip(opened)
            .collect();
        Ok(Self {
            runtime,
            views,
            files: code.screens.iter().map(|(_, f)| f.clone()).collect(),
            readers: code.readers.clone(),
            actions: RefCell::default(),
        })
    }

    /// The screens' names, in the order opened.
    pub fn names(&self) -> Vec<String> {
        self.views.iter().map(|(n, _)| n.clone()).collect()
    }

    /// Copies the game's objects in (as every core gets them).
    pub fn set_objects(&self, values: &crate::ObjectValues) {
        let _ = self.runtime.set_object_values(values);
    }

    /// Runs the screen `name`'s view, shown for `entity` (`Ui:entity()`),
    /// reading components through `reader`, and reads its tree; or says
    /// why not: it failed (at its file and line), it took too long, its
    /// tree holds more than a view may ([`MAX_STYLES`], [`MAX_VIEW_BYTES`];
    /// its buttons stay as its last view left them), or there's no such
    /// screen. Pieces past [`MAX_PIECES`] are left out.
    pub fn view(
        &self,
        name: &str,
        entity: Option<reflect::EntityId>,
        reader: &mut dyn ScreenReader,
    ) -> Result<Node, CodeProblem> {
        let Some(at) = self.views.iter().position(|(n, _)| n == name) else {
            return Err(CodeProblem {
                file: String::new(),
                line: 0,
                column: 0,
                message: format!("there's no screen called {name}"),
            });
        };
        let in_file = |mut p: CodeProblem| {
            if p.file.is_empty() {
                p.file = self.files.get(at).cloned().unwrap_or_default();
            }
            p
        };
        let tree = self
            .runtime
            .run_view(&self.views[at].1, VIEW_LIMIT, entity, &self.readers, reader)
            .map_err(in_file)?;
        let mut reading = Reading::whole();
        let node = read_node(&tree, 0, &mut reading).map_err(in_file)?;
        self.actions
            .borrow_mut()
            .insert((name.to_string(), entity), reading.actions);
        Ok(node)
    }

    /// Lets go of the buttons of every view not in `shown` (screen names
    /// and their entities): those of entities gone, or screens hidden, so
    /// a game spawning entities with screens doesn't keep their functions
    /// for ever (the stability review).
    pub fn keep_buttons_of(&self, shown: &[(String, Option<reflect::EntityId>)]) {
        self.actions
            .borrow_mut()
            .retain(|key, _| shown.iter().any(|s| s.0 == key.0 && s.1 == key.1));
    }

    /// How many views' buttons are kept.
    pub fn kept_buttons(&self) -> usize {
        self.actions.borrow().len()
    }

    /// Runs the button `action` (its number, [`Node::action`]) of the
    /// screen `name` as last viewed for `entity`, giving back what it sent
    /// (for the next tick), or why it stopped. A button not there (the
    /// view changed) does nothing.
    pub fn press(
        &self,
        name: &str,
        entity: Option<reflect::EntityId>,
        action: usize,
        reader: &mut dyn ScreenReader,
    ) -> Result<Vec<crate::Event>, CodeProblem> {
        let Some(f) = self
            .actions
            .borrow()
            .get(&(name.to_string(), entity))
            .and_then(|a| a.get(action).cloned())
        else {
            return Ok(Vec::new());
        };
        let file = self
            .views
            .iter()
            .position(|(n, _)| n == name)
            .and_then(|at| self.files.get(at).cloned())
            .unwrap_or_default();
        self.runtime
            .run_action(&f, VIEW_LIMIT, entity, &self.readers, reader)
            .map_err(|mut p| {
                if p.file.is_empty() {
                    p.file = file;
                }
                p
            })
    }
}

/// What's left of what one view may hold, and the buttons found in it.
/// The tree is game code's and is read after the view's time limit has
/// ended, so everything looked at in it or copied out of it is taken from
/// here, each time it's met: a table used in many places (or holding
/// itself) is paid for at every one, and can't multiply the work.
struct Reading {
    /// The pieces still to read: past them the rest are left out.
    pieces: usize,
    /// The styles still to read: past them the view is refused.
    styles: usize,
    /// The bytes of text still to copy: past them the view is refused.
    bytes: usize,
    /// The buttons found, in their numbers' order: at most [`MAX_ACTIONS`].
    actions: Vec<Function>,
}

impl Reading {
    /// All a view may hold.
    fn whole() -> Self {
        Self {
            pieces: MAX_PIECES,
            styles: MAX_STYLES,
            bytes: MAX_VIEW_BYTES,
            actions: Vec::new(),
        }
    }

    /// A copy of `words`, cut to `most` bytes before it's made (Luau's
    /// memory holds one string of tens of megabytes).
    fn words(&mut self, words: &mlua::LuaString, most: usize) -> Result<String, CodeProblem> {
        let bytes = words.as_bytes();
        let copy = String::from_utf8_lossy(&bytes[..bytes.len().min(most)]).into_owned();
        self.bytes = self.bytes.checked_sub(copy.len()).ok_or_else(|| {
            plain(format!(
                "the screen's view holds too much text: at most {} MB, in its pieces' words and \
                 their styles together",
                MAX_VIEW_BYTES / (1024 * 1024)
            ))
        })?;
        Ok(copy)
    }

    /// One more thing in a list of styles, a style or not.
    fn style(&mut self) -> Result<(), CodeProblem> {
        self.styles = self.styles.checked_sub(1).ok_or_else(|| {
            plain(format!(
                "the screen's view has too many styles: at most {} thousand, on all its pieces \
                 together",
                MAX_STYLES / 1000
            ))
        })?;
        Ok(())
    }
}

/// A style's value from the tree: a word, a number, a bool, or styles
/// inside (`hover:[…]`), at most two deep.
fn read_style(
    value: LuaValue,
    depth: usize,
    reading: &mut Reading,
) -> Result<Option<Style>, CodeProblem> {
    Ok(Some(match value {
        LuaValue::Boolean(b) => Style::Flag(b),
        LuaValue::Integer(i) => Style::Number(i as f64),
        LuaValue::Number(n) => Style::Number(n),
        LuaValue::String(s) => Style::Text(reading.words(&s, MAX_WORDS)?),
        LuaValue::Table(t) if depth < 2 => Style::Nested(read_styles(&t, depth + 1, reading)?),
        _ => return Ok(None),
    }))
}

/// A list of `{name, value}` pairs: the first 256 things in it, each
/// counted whether it's a pair or not, so a list of anything else is read
/// no further than a list of styles.
fn read_styles(
    styles: &Table,
    depth: usize,
    reading: &mut Reading,
) -> Result<Vec<(String, Style)>, CodeProblem> {
    let mut out = Vec::new();
    for pair in styles.sequence_values::<LuaValue>().take(256) {
        reading.style()?;
        let Ok(LuaValue::Table(pair)) = pair else {
            continue;
        };
        let Ok(LuaValue::String(name)) = pair.raw_get::<LuaValue>(1) else {
            continue;
        };
        // 64 letters: four bytes hold any letter.
        let name: String = reading.words(&name, 64 * 4)?.chars().take(64).collect();
        if let Some(value) = read_style(pair.raw_get(2).unwrap_or(LuaValue::Nil), depth, reading)? {
            out.push((name, value));
        }
    }
    Ok(out)
}

/// The words `t` has at `key`, cut to [`MAX_WORDS`]; none if it's no text.
fn read_words(t: &Table, key: &str, reading: &mut Reading) -> Result<String, CodeProblem> {
    match t.raw_get(key) {
        Ok(LuaValue::String(s)) => reading.words(&s, MAX_WORDS),
        _ => Ok(String::new()),
    }
}

/// A piece of the tree Lust's view gave: `{k, t, s, c, v}`.
fn read_node(t: &Table, depth: usize, reading: &mut Reading) -> Result<Node, CodeProblem> {
    reading.pieces = reading.pieces.saturating_sub(1);
    let mut node = Node {
        kind: read_words(t, "k", reading)?,
        text: read_words(t, "t", reading)?,
        value: match t.raw_get("v").unwrap_or(LuaValue::Nil) {
            LuaValue::Integer(i) => Some(i as f64),
            LuaValue::Number(n) => Some(n),
            _ => None,
        },
        ..Node::default()
    };
    if let Ok(styles) = t.raw_get::<Table>("s") {
        node.styles = read_styles(&styles, 0, reading)?;
    }
    // A button's (or a canvas shape's) { }: kept, and numbered.
    if let Ok(f) = t.raw_get::<Function>("f")
        && reading.actions.len() < MAX_ACTIONS
    {
        node.action = Some(reading.actions.len());
        reading.actions.push(f);
    }
    if depth + 1 < ui::screen::MAX_DEPTH
        && let Ok(children) = t.raw_get::<Table>("c")
    {
        for child in children.sequence_values::<LuaValue>() {
            if reading.pieces == 0 {
                break;
            }
            match child {
                Ok(LuaValue::Table(child)) => {
                    node.children.push(read_node(&child, depth + 1, reading)?);
                }
                // Anything else in the list counts as a piece too, so a
                // list of a million numbers is read no further than one
                // of pieces.
                _ => reading.pieces -= 1,
            }
        }
    }
    Ok(node)
}

#[cfg(test)]
#[path = "screens_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "screens_reading_tests.rs"]
mod reading_tests;
