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
}

/// The file only screens fetch (decision 0087): `Ui`, and a reading
/// method on entities for each component.
pub const READERS: &str = "solis/screen.lst";

/// The reading method for a component called `name` (`Health` gives
/// `health`).
pub fn reader_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// `solis/screen.lst` for `components` (each Lust component's name and
/// file): `Ui:entity()` and a reading method for each, except one whose
/// name an engine call on entities has already. Gives the file, the
/// readers kept, and notes on those left out.
pub fn readers_file(
    components: &[(String, String)],
) -> (String, Vec<(String, String)>, Vec<String>) {
    let taken: Vec<&str> = crate::describe::NATIVES
        .iter()
        .filter(|n| n.owner == "entity")
        .map(|n| n.name)
        .collect();
    let mut text = String::from(
        "-- Made by Solis for its screens (decision 0087): what a screen can read. Only\n\
         -- screens fetch it.\n\n",
    );
    let mut readers = Vec::new();
    let mut notes = Vec::new();
    for (name, file) in components {
        let reader = reader_name(name);
        if taken.contains(&reader.as_str()) {
            notes.push(format!(
                "Screens can't read the component {name}: entities already have a {reader}(). \
                 Give the component another name to read it from a screen."
            ));
            continue;
        }
        let module = file.strip_suffix(".lst").unwrap_or(file);
        text.push_str(&format!("fetch \"{module}\":{name};\n"));
        readers.push((name.clone(), reader));
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
    text.push_str("pass Ui;\nend\n");
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

/// The entry that opens every screen.
pub fn entry(screens: &[(String, String)]) -> String {
    let mut text = String::from("-- Made by Solis for its screens: never saved.\n");
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
/// Lust component's name and file). Mistakes name the screen's file and
/// line.
pub fn compile(
    files: &[(String, String)],
    engine: &lust::Engine,
    components: &[(String, String)],
) -> Result<Option<ScreenCode>, Vec<CodeProblem>> {
    let (screens, mut problems) = find(files);
    if screens.is_empty() {
        return if problems.is_empty() {
            Ok(None)
        } else {
            Err(problems)
        };
    }
    let (readers_text, readers, notes) = readers_file(components);
    let mut with_entry = files.to_vec();
    with_entry.retain(|(p, _)| p != ENTRY && p != READERS);
    with_entry.push((ENTRY.to_string(), entry(&screens)));
    with_entry.push((READERS.to_string(), readers_text));
    // The readers' file is the engine's, like solis/'s others.
    let mut engine = lust::Engine {
        folders: engine.folders.clone(),
    };
    if !engine.folders.iter().any(|f| f == "solis") {
        engine.folders.push("solis".into());
    }
    match std::panic::catch_unwind(|| lust::to_lua_with(&engine, ENTRY, &with_entry)) {
        Ok(Ok(chunk)) if problems.is_empty() => Ok(Some(ScreenCode {
            chunk,
            screens,
            readers,
            notes,
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
    /// why not: it failed (at its file and line), it took too long, or
    /// there's no such screen.
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
        let file = self.files.get(at).cloned().unwrap_or_default();
        let tree = self
            .runtime
            .run_view(&self.views[at].1, VIEW_LIMIT, entity, &self.readers, reader)
            .map_err(|mut p| {
                if p.file.is_empty() {
                    p.file = file;
                }
                p
            })?;
        let mut count = 0;
        let mut actions = Vec::new();
        let node = read_node(&tree, 0, &mut count, &mut actions);
        self.actions
            .borrow_mut()
            .insert((name.to_string(), entity), actions);
        Ok(node)
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

/// A style's value from the tree: a word, a number, a bool, or styles
/// inside (`hover:[…]`), at most two deep.
fn read_style(value: LuaValue, depth: usize) -> Option<Style> {
    Some(match value {
        LuaValue::Boolean(b) => Style::Flag(b),
        LuaValue::Integer(i) => Style::Number(i as f64),
        LuaValue::Number(n) => Style::Number(n),
        LuaValue::String(s) => Style::Text(s.to_string_lossy()),
        LuaValue::Table(t) if depth < 2 => Style::Nested(read_styles(&t, depth + 1)),
        _ => return None,
    })
}

/// A list of `{name, value}` pairs.
fn read_styles(styles: &Table, depth: usize) -> Vec<(String, Style)> {
    let mut out = Vec::new();
    for pair in styles.sequence_values::<Table>().flatten().take(256) {
        let Ok(LuaValue::String(name)) = pair.raw_get::<LuaValue>(1) else {
            continue;
        };
        let name: String = name.to_string_lossy().chars().take(64).collect();
        if let Some(value) = read_style(pair.raw_get(2).unwrap_or(LuaValue::Nil), depth) {
            out.push((name, value));
        }
    }
    out
}

/// A piece of the tree Lust's view gave: `{k, t, s, c, v}`.
fn read_node(t: &Table, depth: usize, count: &mut usize, actions: &mut Vec<Function>) -> Node {
    *count += 1;
    let text = |v: LuaValue| match v {
        LuaValue::String(s) => {
            let bytes = s.as_bytes();
            String::from_utf8_lossy(&bytes[..bytes.len().min(16 * 1024)]).into_owned()
        }
        _ => String::new(),
    };
    let mut node = Node {
        kind: text(t.raw_get("k").unwrap_or(LuaValue::Nil)),
        text: text(t.raw_get("t").unwrap_or(LuaValue::Nil)),
        value: match t.raw_get("v").unwrap_or(LuaValue::Nil) {
            LuaValue::Integer(i) => Some(i as f64),
            LuaValue::Number(n) => Some(n),
            _ => None,
        },
        ..Node::default()
    };
    if let Ok(styles) = t.raw_get::<Table>("s") {
        node.styles = read_styles(&styles, 0);
    }
    // A button's (or a canvas shape's) { }: kept, and numbered.
    if let Ok(f) = t.raw_get::<Function>("f")
        && actions.len() < MAX_ACTIONS
    {
        node.action = Some(actions.len());
        actions.push(f);
    }
    if depth + 1 < ui::screen::MAX_DEPTH
        && let Ok(children) = t.raw_get::<Table>("c")
    {
        for child in children.sequence_values::<Table>().flatten() {
            if *count >= MAX_PIECES {
                break;
            }
            node.children
                .push(read_node(&child, depth + 1, count, actions));
        }
    }
    node
}

#[cfg(test)]
#[path = "screens_tests.rs"]
mod tests;
