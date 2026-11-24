//! The engine's own objects' settings as fields (the uniformity pass, U5b;
//! decision 0137): `Screens.textSize = 1.2;` instead of
//! `Screens:setTextSize(1.2)`, and `Screens.textSize` to read it.
//!
//! Each field is a getter and a setter underneath, the natives the calls
//! were: a read gives the value as the tick started (or what this core's
//! code wrote this tick, so code reads back what it set), and a write
//! lands at the end of the tick in the order the calls' changes did, so
//! the game plays the same on any number of cores. The objects aren't
//! saved or ticked like a game's own (`pass Clock;`): the settings are
//! the player's, and the game's speed and hour are kept by the engine.
//!
//! How: once the Lust starts, each field is taken out of its object's
//! table and the object is given a metatable whose `__index` and
//! `__newindex` call the natives lent for the tick (`WRAP`).

use mlua::{Function, Table, Value as LuaValue};

/// One of an engine object's fields.
#[derive(Debug, Clone, Copy)]
pub struct EngineField {
    /// The object: `Screens`.
    pub object: &'static str,
    /// The field, as Lust declares it in the object's `has`.
    pub lust: &'static str,
    /// Its name: `textSize`.
    pub name: &'static str,
    /// What it is, for the host file and the reference.
    pub about: &'static str,
    /// The native giving its value: `textSize`.
    pub getter: &'static str,
    /// The native setting it: `setTextSize`.
    pub setter: &'static str,
}

impl EngineField {
    /// The getter's key in `LUST_NATIVES`: `Screens_textSize`.
    pub fn getter_key(&self) -> String {
        format!("{}_{}", self.object, self.getter)
    }

    /// The field as the host file declares it, with its starting value:
    /// the language's is the state's first option, `first_language`.
    pub fn starting(&self, first_language: &str) -> String {
        match self.lust {
            "Language language;" => format!("Language language = Language.{first_language};"),
            lust => lust.to_owned(),
        }
    }

    /// The setter's key in `LUST_NATIVES`: `Screens_setTextSize`.
    pub fn setter_key(&self) -> String {
        format!("{}_{}", self.object, self.setter)
    }
}

/// Every engine object's fields.
pub const FIELDS: &[EngineField] = &[
    EngineField {
        object: "Screens",
        lust: "float textSize = 1;",
        name: "textSize",
        about: "The player's text size: every screen's text times this (1 as designed, 1.5 half as big again; 0.5 to 3). Set at the end of the tick: every screen's text grows or shrinks, and the pieces round it make room. Kept for the next time the game starts.",
        getter: "textSize",
        setter: "setTextSize",
    },
    EngineField {
        object: "Screens",
        lust: "float uiSize = 1;",
        name: "uiSize",
        about: "The player's UI size: every screen times this (1 as designed; 0.5 to 2). Set at the end of the tick, and kept for the next time the game starts.",
        getter: "uiSize",
        setter: "setUiSize",
    },
    EngineField {
        object: "Screens",
        lust: "Language language;",
        name: "language",
        about: "The language the player reads and hears the game in. Set at the end of the tick: every screen's words change, and it's kept for the next time the game starts.",
        getter: "language",
        setter: "setLanguage",
    },
    EngineField {
        object: "Screens",
        lust: "bool subtitles = true;",
        name: "subtitles",
        about: "Whether what characters say is shown as they say it. Set at the end of the tick, and kept for the next time the game starts.",
        getter: "subtitles",
        setter: "setSubtitles",
    },
    EngineField {
        object: "Screens",
        lust: "bool captions = false;",
        name: "captions",
        about: "Whether sounds heard are shown in words (captions). Set at the end of the tick, and kept for the next time the game starts.",
        getter: "captions",
        setter: "setCaptions",
    },
    EngineField {
        object: "Screens",
        lust: "float captionSize = 1;",
        name: "captionSize",
        about: "How big subtitles and captions are, times as designed (0.5 to 2). Set at the end of the tick, and kept for the next time the game starts.",
        getter: "captionSize",
        setter: "setCaptionSize",
    },
    EngineField {
        object: "Screens",
        lust: "float captionBackground = 0.6;",
        name: "captionBackground",
        about: "How dark the box behind subtitles and captions is: 0 none, 1 solid. Set at the end of the tick, and kept for the next time the game starts.",
        getter: "captionBackground",
        setter: "setCaptionBackground",
    },
    EngineField {
        object: "Window",
        lust: "bool fullscreen = false;",
        name: "fullscreen",
        about: "Whether the game fills the screen. Set at the end of the tick, and kept for the next time the game starts.",
        getter: "fullscreen",
        setter: "setFullscreen",
    },
    EngineField {
        object: "World",
        lust: "float gameSpeed = 1;",
        name: "gameSpeed",
        about: "How fast the game runs: 1 as usual, 0.5 half speed, 0 paused (screens and pause menus still work). Set at the end of the tick. Kept in save games.",
        getter: "gameSpeed",
        setter: "setGameSpeed",
    },
    EngineField {
        object: "World",
        lust: "float timeOfDay = 0;",
        name: "timeOfDay",
        about: "The game's hour, from 0 up to 24 (the day's length is in Project settings › World). Set at the end of the tick, and the day carries on from there: World.timeOfDay = 6; after a night's sleep. Kept in save games.",
        getter: "timeOfDay",
        setter: "setTimeOfDay",
    },
];

/// Whether `object` is one of the engine's own objects with fields: not
/// a game's own, so not wrapped, ticked or saved as one.
pub fn is_engine_object(object: &str) -> bool {
    FIELDS.iter().any(|f| f.object == object)
}

/// Whether the native `key` is a field's getter or setter now: game code
/// reaches it through the field, so the host files and the reference
/// don't list it as a method.
pub fn is_field_native(key: &str) -> bool {
    FIELDS
        .iter()
        .any(|f| f.getter_key() == key || f.setter_key() == key)
}

/// The field whose getter or setter is the native `key`.
pub fn field_of_native(key: &str) -> Option<&'static EngineField> {
    FIELDS
        .iter()
        .find(|f| f.getter_key() == key || f.setter_key() == key)
}

/// Wraps an engine object: `wrap(object, fields, env)`, with `fields`
/// mapping each field's key to `{ getter key, setter key }` and `env` the
/// Lust's globals, where `LUST_NATIVES` is lent. Gives back a function
/// that forgets what was written, called as each tick starts.
pub(crate) const WRAP: &str = r#"
local next, rawget, rawset, setmetatable, getmetatable, type = next, rawget, rawset, setmetatable, getmetatable, type
return function(o, fields, env)
  local class = getmetatable(o)
  local written = {}
  for k in next, fields do rawset(o, k, nil) end
  local mt = {}
  if class then
    for k, v in next, class do
      if type(k) == "string" and k:sub(1, 2) == "__" then mt[k] = v end
    end
  end
  mt.__index = function(_, k)
    local f = fields[k]
    if f then
      local w = written[k]
      if w ~= nil then return w end
      local n = rawget(env, "LUST_NATIVES")
      local get = n and n[f[1]]
      if get then return get(o) end
      return nil
    end
    return class and class[k]
  end
  mt.__newindex = function(_, k, v)
    local f = fields[k]
    if f then
      written[k] = v
      local n = rawget(env, "LUST_NATIVES")
      local set = n and n[f[2]]
      if set then set(o, v) end
      return
    end
    rawset(o, k, v)
  end
  setmetatable(o, mt)
  return function()
    for k in next, written do written[k] = nil end
  end
end
"#;

/// Wraps each engine object the Lust's `api` passes, giving back the
/// functions that forget what this tick wrote.
pub(crate) fn wrap(lua: &mlua::Lua, api: &Table, env: &Table) -> mlua::Result<Vec<Function>> {
    let Ok(LuaValue::Table(objects)) = api.raw_get::<LuaValue>("objects") else {
        return Ok(Vec::new());
    };
    let wrap: Function = lua.load(WRAP).set_name("=engine objects").eval()?;
    let mut forgets = Vec::new();
    let mut names: Vec<&str> = FIELDS.iter().map(|f| f.object).collect();
    names.dedup();
    for name in names {
        let LuaValue::Table(object) = objects.raw_get::<LuaValue>(name)? else {
            continue;
        };
        let fields = lua.create_table()?;
        for field in FIELDS.iter().filter(|f| f.object == name) {
            let pair = lua.create_sequence_from([field.getter_key(), field.setter_key()])?;
            fields.raw_set(field.name, pair)?;
        }
        forgets.push(wrap.call((object, fields, env.clone()))?);
    }
    Ok(forgets)
}
