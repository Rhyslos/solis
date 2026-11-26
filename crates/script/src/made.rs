//! What game code makes with `new` (the uniformity pass, U6;
//! [decision 0137](../../../docs/decisions/0137-the-uniformity-pass.md)):
//! `entity ship = new Fighter { position = gate, speed = 9 };` makes a
//! copy of `Module.Fighter`, its root's components given the fields
//! written; `new Horn { at = gate };` plays `Sound.Horn` there, `new DoorOpens {
//! after = 2.0 };` starts the timer `Timer.DoorOpens`, and `new Shot {
//! from = muzzle, direction = ahead };` fires a shot.
//!
//! Each is a class Solis writes into `solis/made.lst` each time the code
//! is checked, `like Made`, with a native `made()` (Lust alpha 80's
//! things the engine makes): `new` hands the engine the thing, and
//! Solis's `new` keeps only the fields written on it
//! (`readings::values_new`), so the engine knows which were. A
//! class is named by its option (`Sound.Horn` is `Horn`); a sound whose
//! name the game already uses has `Sound` added (`HornSound`), and a
//! timer, named where its code makes it, is renamed by its writer.
//!
//! A timer, and a sound made without `at` or `on`, belong to the entity
//! whose code is running (a sound with `everywhere = true` plays the
//! same everywhere): `states::Running` notes it for every
//! Lust component of a game that makes things.

use crate::convert::{entity_of, vec3_of};
use crate::host::{Lent, Shot, SoundAt};
use crate::timers::Timing;
use mlua::{Lua, Table, Value as LuaValue};
use reflect::EntityId;
use std::collections::{BTreeMap, BTreeSet};

/// The file's name inside the engine's folder: `solis/made.lst`.
pub const FILE: &str = "made";

/// The class every made class is like.
pub const BASE: &str = "Made";

/// The class of a shot, written once.
pub const SHOT: &str = "Shot";

/// What a made class makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A copy of a module, made in the world (`Module.Fighter`); `new`
    /// gives back its entity.
    Module,
    /// A sound event played (`Sound.Horn`).
    Sound,
    /// A timer started on the running entity (`Timer.DoorOpens`).
    Timer,
    /// A shot fired.
    Shot,
}

impl Kind {
    /// Its natives' key in `LUST_NATIVES`, which every class of its kind
    /// reaches.
    fn key(self) -> &'static str {
        match self {
            Self::Module => "solis_made_module",
            Self::Sound => "solis_made_sound",
            Self::Timer => "solis_made_timer",
            Self::Shot => "solis_made_shot",
        }
    }
}

/// The made classes a program has: each class's name, with what it
/// makes and the option of the project's names it stands for (`Horn`
/// for `Sound.Horn`; empty for a shot).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MadeList {
    /// By class name: what it makes.
    pub classes: BTreeMap<String, MadeClass>,
}

/// A made class: what it makes, the option of the project's names it
/// stands for (`Horn` for `Sound.Horn`; empty for a shot), and, for a
/// module's, each of its fields that gives a root component a first value
/// (the field, the component and the component's field).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadeClass {
    /// What it makes.
    pub kind: Kind,
    /// The option it stands for.
    pub option: String,
    /// A module's root components' fields: (its field, component, field).
    pub fields: Vec<(String, String, String)>,
}

impl MadeClass {
    fn of(kind: Kind, option: &str) -> Self {
        Self {
            kind,
            option: option.to_string(),
            fields: Vec::new(),
        }
    }
}

/// What [`write()`] gives back.
#[derive(Debug, Default)]
pub struct Written {
    /// The files with `solis/made.lst` in.
    pub files: Vec<(String, String)>,
    /// What can't be made, said in the Console.
    pub notes: Vec<String>,
    /// The classes written.
    pub made: MadeList,
}

/// The fields a timer's class takes, one of which its `new` writes.
pub const TIMINGS: [&str; 3] = ["after", "every", "atHour"];

/// `files` with `solis/made.lst` in (written in `folder`): a class for
/// each sound event in `solis/project.lst`, each timer the game's Lust
/// makes ([`crate::timer_names::names_started`]), and the shot.
pub fn write(folder: &str, files: &[(String, String)]) -> Written {
    write_with(folder, files, &BTreeMap::new(), &BTreeMap::new())
}

/// [`write`], with each Lust component's fields (by component) and where
/// their types come from (by type), from [`crate::component_code`]: a
/// module's class takes its root components' fields.
pub(crate) fn write_with(
    folder: &str,
    files: &[(String, String)],
    fields: &BTreeMap<String, Vec<crate::component_code::Field>>,
    sources: &BTreeMap<String, String>,
) -> Written {
    let path = format!("{folder}/{FILE}.lst");
    let mut taken: BTreeSet<String> = BTreeSet::new();
    for (file, text) in files {
        if *file == path {
            continue;
        }
        for (_, found) in level::lust_guard::scan(text) {
            if let level::lust_guard::Found::Declared(_, name) = found {
                taken.insert(name.to_owned());
            }
        }
    }
    let mut written = Written::default();
    let mut classes: Vec<String> = vec![format!(
        "-- The class every one of these is like.\nclass {BASE} {{\n}}"
    )];
    let mut passed = vec![BASE.to_string()];
    if taken.contains(SHOT) {
        written.notes.push(format!(
            "The game has its own {SHOT}, so new {SHOT} {{ … }} is the game's: rename it to fire \
             the engine's shots with new."
        ));
    } else {
        classes.push(format!(
            "-- A shot: new {SHOT} {{ from = muzzle, direction = ahead }}; fires one, which flies\n\
             -- that way at speed metres a second for life seconds, in colour.\n\
             class {SHOT} like {BASE} {{\n    has {{\n        vec3 from = [0, 0, 0];\n        \
             vec3 direction = [1, 0, 0];\n        float speed = 20;\n        float life = 2;\n        \
             vec3 colour = [1, 1, 1];\n    }}\n\n    method made() native;\n}}"
        ));
        passed.push(SHOT.to_string());
        written
            .made
            .classes
            .insert(SHOT.to_string(), MadeClass::of(Kind::Shot, ""));
    }
    let states = crate::project_names::states_in(files);
    let roots = crate::project_names::roots_in(files);
    // The types the modules' fields need, by the file they come from.
    let mut fetches: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let modules = states.get("Module").cloned().unwrap_or_default();
    for module in modules
        .iter()
        .filter(|m| **m != crate::project_names::NO_OPTION)
    {
        let name = [module.clone(), format!("{module}Module")]
            .into_iter()
            .find(|n| !taken.contains(n) && !written.made.classes.contains_key(n));
        let Some(name) = name else {
            written.notes.push(format!(
                "The module Module.{module} can't be made with new: {module} and {module}Module \
                 are both names the game already uses. Rename one of them."
            ));
            continue;
        };
        let mut made = MadeClass::of(Kind::Module, module);
        let mut lines = vec!["        vec3 position = [0, 0, 0];".to_string()];
        let mut keys: BTreeSet<String> = ["position".to_string()].into_iter().collect();
        let root: &[String] = roots.get(module).map_or(&[], Vec::as_slice);
        // A field two of the root's components share is named with its
        // component in front (healthHp).
        let shared: BTreeSet<&str> = {
            let mut seen = BTreeSet::new();
            let mut twice = BTreeSet::new();
            for component in root {
                for field in fields.get(component).map_or(&[][..], Vec::as_slice) {
                    if !seen.insert(field.name.as_str()) {
                        twice.insert(field.name.as_str());
                    }
                }
            }
            twice
        };
        for component in root {
            for field in fields.get(component).map_or(&[][..], Vec::as_slice) {
                let mut key = field.name.clone();
                if keys.contains(&key) || shared.contains(key.as_str()) {
                    let mut chars = field.name.chars();
                    let first = chars.next().map(|c| c.to_ascii_uppercase());
                    key = format!(
                        "{}{}{}",
                        component[..1].to_ascii_lowercase(),
                        &component[1..],
                        first.map(String::from).unwrap_or_default() + chars.as_str()
                    );
                }
                if keys.contains(&key) {
                    written.notes.push(format!(
                        "new {name} can't give {component}'s {} a first value: two of its fields \
                         would be called {key}.",
                        field.name
                    ));
                    continue;
                }
                if let Some(source) = sources.get(&field.kind) {
                    fetches
                        .entry(source.clone())
                        .or_default()
                        .insert(field.kind.clone());
                } else if !crate::component_code::is_built_in(&field.kind) {
                    continue;
                }
                lines.push(format!(
                    "        {} {key} = {};",
                    field.kind,
                    field.own.as_deref().unwrap_or(&field.start)
                ));
                keys.insert(key.clone());
                made.fields
                    .push((key, component.clone(), field.name.clone()));
            }
        }
        let root_said = if root.is_empty() {
            String::new()
        } else {
            format!(" and its root's {}", root.join(", "))
        };
        classes.push(format!(
            "-- Module.{module}: new {name} {{ position = place }}; makes a copy there, giving back\n\
             -- its entity: entity made = new {name} {{ … }};. Its fields are where it's made{root_said}'s\n\
             -- first values: only those written are given.\n\
             class {name} like {BASE} {{\n    has {{\n{}\n    }}\n\n    method entity made() native;\n}}",
            lines.join("\n")
        ));
        passed.push(name.clone());
        written.made.classes.insert(name, made);
    }
    let sounds = states.get("Sound").cloned().unwrap_or_default();
    for sound in sounds
        .iter()
        .filter(|s| **s != crate::project_names::NO_OPTION)
    {
        let name = [sound.clone(), format!("{sound}Sound")]
            .into_iter()
            .find(|n| !taken.contains(n) && !written.made.classes.contains_key(n));
        let Some(name) = name else {
            written.notes.push(format!(
                "The sound event Sound.{sound} can't be made with new: {sound} and {sound}Sound \
                 are both names the game already uses. Rename one of them."
            ));
            continue;
        };
        classes.push(format!(
            "-- Sound.{sound}: new {name} {{ at = place }}; plays it there, new {name} {{ on = target }};\n\
             -- on an entity, following it, new {name} {{}}; on the entity whose code makes it, and\n\
             -- new {name} {{ everywhere = true }}; the same everywhere (music, the interface).\n\
             class {name} like {BASE} {{\n    has {{\n        vec3 at = [0, 0, 0];\n        \
             entity on = nothing;\n        bool everywhere = false;\n    }}\n\n    method made() native;\n}}"
        ));
        passed.push(name.clone());
        written
            .made
            .classes
            .insert(name, MadeClass::of(Kind::Sound, sound));
    }
    for timer in crate::timer_names::names_in(files) {
        if taken.contains(&timer) || written.made.classes.contains_key(&timer) {
            written.notes.push(format!(
                "The timer {timer} can't be made with new: the game already uses {timer} for \
                 something else. Give the timer another name."
            ));
            continue;
        }
        classes.push(format!(
            "-- Timer.{timer}: new {timer} {{ after = 2 }}; finishes once, that many seconds from now,\n\
             -- new {timer} {{ every = 5 }}; again and again, new {timer} {{ atHour = 6 }}; each day.\n\
             class {timer} like {BASE} {{\n    has {{\n        float after = 0;\n        \
             float every = 0;\n        float atHour = 0;\n    }}\n\n    method made() native;\n}}"
        ));
        passed.push(timer.clone());
        written
            .made
            .classes
            .insert(timer.clone(), MadeClass::of(Kind::Timer, &timer));
    }
    let fetched: String = fetches
        .iter()
        .map(|(from, names)| {
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            format!("fetch \"{from}\":[{}];\n", names.join(", "))
        })
        .collect();
    let text = format!(
        "-- Made by Solis: what game code makes with new. It's written again each time the\n\
         -- code is checked, so changes made here don't last.\n\n{fetched}{}{}\n\npass {};\nend\n",
        if fetched.is_empty() { "" } else { "\n" },
        classes.join("\n\n"),
        passed.join(", ")
    );
    written.files = files
        .iter()
        .filter(|(file, _)| *file != path)
        .cloned()
        .chain(std::iter::once((path.clone(), text)))
        .collect();
    written
}

/// What `solis/made.lst` holds until the code is checked: nothing, since
/// its classes are named against the game's own names, which only the
/// check reads.
pub fn placeholder(made_by: &str) -> String {
    format!(
        "-- Made by {made_by}: what game code makes with new. It's written each time the code is\n\
         -- checked, from the project's sound events and the timers the code makes.\nend\n"
    )
}

/// The entity whose Lust component's code is running, if any: an update,
/// an event, or one of its tweens' done blocks.
fn running_entity(lua: &Lua) -> Option<EntityId> {
    let running = lua.app_data_ref::<crate::states::Running>()?;
    let by = match running.table.raw_get::<LuaValue>(1).ok()? {
        LuaValue::Table(by) => by,
        _ => running.table.raw_get::<Table>("doneBy").ok()?,
    };
    entity_of(&by.raw_get::<LuaValue>("me").ok()?)
}

/// The made class a thing is of, and what it makes.
fn class_of(lua: &Lua, thing: &Table) -> mlua::Result<(String, MadeClass)> {
    let class = thing
        .metatable()
        .and_then(|m| m.raw_get::<Table>("__parent").ok())
        .and_then(|c| c.raw_get::<String>("__name").ok())
        .unwrap_or_default();
    let list = lua.app_data_ref::<std::sync::Arc<MadeList>>();
    match list.as_ref().and_then(|l| l.classes.get(&class)) {
        Some(made) => Ok((class, made.clone())),
        None => Err(mlua::Error::runtime(format!(
            "new {class} isn't one Solis makes any more: check the code again."
        ))),
    }
}

/// A field written in a thing's `new` (Solis's `new` keeps only those on
/// the thing itself).
fn written(thing: &Table, field: &str) -> Option<LuaValue> {
    thing
        .raw_get::<LuaValue>(field)
        .ok()
        .filter(|v| !matches!(v, LuaValue::Nil))
}

/// Lends what the made classes call into `natives`: a native for each
/// kind, and `natives`' `__index` giving `<Class>_made` its kind's.
pub(crate) fn lend<'s>(
    scope: &'s mlua::Scope<'s, '_>,
    lua: &Lua,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    let Some(index) = lua.app_data_ref::<MadeIndex>().map(|i| i.0.clone()) else {
        return Ok(());
    };
    natives.raw_set(
        Kind::Module.key(),
        scope.create_function(move |lua, thing: Table| {
            let (class, made) = class_of(lua, &thing)?;
            let Some(module) = crate::project_names::path_for(lua, "Module", &made.option) else {
                return Ok(LuaValue::Nil);
            };
            let place = written(&thing, "position")
                .as_ref()
                .and_then(vec3_of)
                .filter(|p| p.is_finite())
                .unwrap_or(glam::Vec3::ZERO);
            let layouts = lua
                .app_data_ref::<crate::readings::Kept>()
                .map(|kept| std::rc::Rc::clone(&kept.0))
                .unwrap_or_default();
            let mut values: crate::game::SpawnValues = Vec::new();
            for (key, component, field) in &made.fields {
                let Some(value) = written(&thing, key) else {
                    continue;
                };
                let Some(kind) = layouts
                    .get(component)
                    .and_then(|l| l.iter().find(|f| f.name == *field))
                    .map(|f| f.kind.clone())
                else {
                    continue;
                };
                let Some(value) = crate::convert::from_lua(&kind, &value) else {
                    return Err(mlua::Error::runtime(format!(
                        "new {class}: {component}'s {field} can't hold that value."
                    )));
                };
                match values.iter_mut().find(|(c, _)| c == component) {
                    Some((_, data)) => data.set(field, value),
                    None => {
                        let mut data = reflect::ComponentData::default();
                        data.set(field, value);
                        values.push((component.clone(), data));
                    }
                }
            }
            let made = h.borrow_mut().spawn_with(&module, place, values);
            Ok(made.map_or(LuaValue::Nil, |id| {
                lua.create_string(id.to_string())
                    .map_or(LuaValue::Nil, LuaValue::String)
            }))
        })?,
    )?;
    natives.raw_set(
        Kind::Sound.key(),
        scope.create_function(move |lua, thing: Table| {
            let (_, made) = class_of(lua, &thing)?;
            let Some(event) = crate::project_names::path_for(lua, "Sound", &made.option) else {
                return Ok(());
            };
            let at = if let Some(at) = written(&thing, "at") {
                vec3_of(&at).filter(|p| p.is_finite()).map(SoundAt::Place)
            } else if let Some(on) = written(&thing, "on") {
                entity_of(&on).map(SoundAt::On)
            } else if matches!(written(&thing, "everywhere"), Some(LuaValue::Boolean(true))) {
                Some(SoundAt::Nowhere)
            } else {
                Some(running_entity(lua).map_or(SoundAt::Nowhere, SoundAt::On))
            };
            if let Some(at) = at {
                let mut host = h.borrow_mut();
                host.set_started_in(crate::states::started_in(lua));
                host.play_sound(&event, at);
            }
            Ok(())
        })?,
    )?;
    natives.raw_set(
        Kind::Timer.key(),
        scope.create_function(move |lua, thing: Table| {
            let (class, made) = class_of(lua, &thing)?;
            let timer = made.option;
            let Some(entity) = running_entity(lua) else {
                return Err(mlua::Error::runtime(format!(
                    "new {class}: a timer belongs to an entity, so make it in a component's code."
                )));
            };
            let set: Vec<(&str, LuaValue)> = TIMINGS
                .iter()
                .filter_map(|t| written(&thing, t).map(|v| (*t, v)))
                .collect();
            let [(field, value)] = set.as_slice() else {
                return Err(mlua::Error::runtime(format!(
                    "new {class} {{ … }} takes one of after, every and atHour: after = 2 to \
                     finish once, every = 5 again and again, atHour = 6 each day."
                )));
            };
            let time = match value {
                LuaValue::Number(n) => *n,
                LuaValue::Integer(n) => *n as f64,
                _ => f64::NAN,
            };
            let timing = match *field {
                "after" => Timing::After(time),
                "every" => Timing::Every(time),
                _ => Timing::AtHour(time),
            };
            let mut host = h.borrow_mut();
            host.set_started_in(crate::states::started_in(lua));
            host.start_timer(entity, &timer, timing)
                .map_err(|why| mlua::Error::runtime(format!("new {class}: {why}.")))
        })?,
    )?;
    natives.raw_set(
        Kind::Shot.key(),
        scope.create_function(move |_, thing: Table| {
            let vec = |field: &str| thing.get::<LuaValue>(field).ok().as_ref().and_then(vec3_of);
            let num = |field: &str| thing.get::<f64>(field).ok();
            if let (Some(from), Some(direction), Some(colour), Some(speed), Some(life)) = (
                vec("from"),
                vec("direction"),
                vec("colour"),
                num("speed"),
                num("life"),
            ) {
                h.borrow_mut().fire(Shot {
                    from,
                    direction,
                    speed: speed as f32,
                    life: life as f32,
                    colour: colour.to_array(),
                });
            }
            Ok(())
        })?,
    )?;
    natives.set_metatable(Some(index))?;
    Ok(())
}

/// The metatable the lent natives get: its `__index` finds
/// `<Class>_made` as its kind's native. Kept as the Luau's app data while
/// a program that makes things is loaded.
pub(crate) struct MadeIndex(pub(crate) Table);

/// Keeps `made` in `lua` for the natives to read, with the metatable
/// that leads each class's `made` to its kind's native; or takes both
/// away when there's nothing to make.
pub(crate) fn keep(lua: &Lua, made: &std::sync::Arc<MadeList>) -> mlua::Result<()> {
    if made.classes.is_empty() {
        lua.remove_app_data::<std::sync::Arc<MadeList>>();
        lua.remove_app_data::<MadeIndex>();
        return Ok(());
    }
    let kinds = lua.create_table()?;
    for (class, made) in &made.classes {
        kinds.raw_set(format!("{class}_made"), made.kind.key())?;
    }
    let find = lua.create_function(move |_, (natives, key): (Table, LuaValue)| match kinds
        .raw_get::<LuaValue>(
        key,
    )? {
        LuaValue::String(kind) => natives.raw_get::<LuaValue>(kind),
        _ => Ok(LuaValue::Nil),
    })?;
    let meta = lua.create_table()?;
    meta.raw_set("__index", find)?;
    lua.set_app_data(std::sync::Arc::clone(made));
    lua.set_app_data(MadeIndex(meta));
    Ok(())
}
