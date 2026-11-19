//! The game's text while it runs (stage 1, translations): `World:text`
//! and each line's method with values (`ShopText:welcome(name)`), lent
//! from functions each Luau makes once for the text it was given
//! ([`Lent`]): made again only when the text or the player's language
//! changes, never once a phase.
//!
//! Text shown is never game state: two players in different languages
//! play the same game, so nothing here is saved or checksummed, and the
//! manual says game code shouldn't let what it reads change the game.

use mlua::{Function, Lua, Table as LuaTable, Value as LuaValue};
use std::sync::Arc;
use translations::{TextValue, Texts};

/// Wraps the Rust behind each native so a mistake stops at the game
/// code's own line (Lua's `error` two calls up). Made once, before any
/// game code runs.
pub(crate) const MAKE: &str = r#"
local error, type = error, type
return function(text, withValues)
  local function plain(_, key)
    if type(key) ~= "string" then
      error("solis.World:text(key) wants a line's key, such as \"Shop.Welcome\" (Text.Shop.Welcome in Lust)", 2)
    end
    local t, why = text(key)
    if t == nil then error(why, 2) end
    return t
  end
  local function method(key, names)
    return function(_, ...)
      local t, why = withValues(key, names, ...)
      if t == nil then error(why, 2) end
      return t
    end
  end
  return plain, method
end
"#;

/// What a Luau lends for the text it was last given: the text, and the
/// natives by key (`World_text`, `ShopText_welcome`).
#[derive(Default)]
pub(crate) struct Lent {
    from: Option<Texts>,
    natives: Option<LuaTable>,
    /// The objects the lines with values are on (`ShopText`), for Lua's
    /// `solis`.
    pub(crate) groups: Vec<String>,
}

/// Why `key` has no line, with the nearest that has one.
fn no_line(texts: &Texts, key: &str) -> String {
    let keys = texts.all().keys();
    let near = translations::keys::nearest(key, keys.iter().map(String::as_str))
        .map_or_else(String::new, |n| format!(": did you mean {n}?"));
    format!("solis.World:text(key): there's no line of text {key}{near}")
}

/// A value game code handed a line, as Fluent takes it.
fn value_of(value: &LuaValue) -> Option<TextValue> {
    Some(match value {
        LuaValue::Integer(n) => TextValue::Number(*n as f64),
        LuaValue::Number(x) => TextValue::Number(*x),
        LuaValue::String(s) => TextValue::Text(s.to_string_lossy()),
        LuaValue::Boolean(b) => TextValue::Text(b.to_string()),
        _ => return None,
    })
}

/// The engine objects' names, which no group's object may have.
fn taken() -> Vec<String> {
    crate::describe::owners_of_natives()
        .into_iter()
        .map(str::to_owned)
        .collect()
}

impl Lent {
    /// Puts the text's natives into `natives` (the ones lent while game
    /// code runs), made again first when `texts` aren't the ones they were
    /// made for. `make` is [`MAKE`]; `methods` are the lines with values,
    /// each its key, object, method and values' names.
    pub(crate) fn lend(
        &mut self,
        lua: &Lua,
        make: &Function,
        texts: Option<Texts>,
        natives: &LuaTable,
    ) -> mlua::Result<()> {
        let texts = texts.unwrap_or_else(|| {
            Texts::new(
                Arc::new(translations::Translations::default()),
                translations::Language::english(),
            )
        });
        let same = self.from.as_ref().is_some_and(|f| f.same(&texts));
        if !same {
            let made = lua.create_table()?;
            let plain_texts = texts.clone();
            let text = lua.create_function(move |_, key: mlua::LuaString| {
                let key = key.to_string_lossy();
                if !plain_texts.all().has(&key) {
                    return Ok((None, Some(no_line(&plain_texts, &key))));
                }
                Ok((Some(plain_texts.text(&key)), None))
            })?;
            let value_texts = texts.clone();
            let with_values = lua.create_function(
                move |_, (key, names, values): (String, LuaTable, mlua::Variadic<LuaValue>)| {
                    let names: Vec<String> = names.sequence_values().collect::<Result<_, _>>()?;
                    let mut given = Vec::with_capacity(names.len());
                    for (name, value) in names.iter().zip(values.iter()) {
                        if let Some(v) = value_of(value) {
                            given.push((name.as_str(), v));
                        }
                    }
                    if given.len() < names.len() {
                        return Ok((
                            None,
                            Some(format!(
                                "{key} takes {} values ({}), and was given {}",
                                names.len(),
                                names.join(", "),
                                given.len()
                            )),
                        ));
                    }
                    Ok((Some(value_texts.text_with(&key, &given)), None))
                },
            )?;
            let (plain, method): (Function, Function) = make.call((text, with_values))?;
            made.raw_set("World_text", plain)?;
            let (names, _) = crate::text_code::TextNames::of(texts.all(), &taken());
            for m in &names.methods {
                let fluent: Vec<&str> = m.values.iter().map(|(f, _, _)| f.as_str()).collect();
                let f: Function =
                    method.call((m.key.as_str(), lua.create_sequence_from(fluent)?))?;
                made.raw_set(format!("{}_{}", m.object, m.method), f)?;
            }
            self.groups = names.objects().into_iter().map(str::to_owned).collect();
            self.natives = Some(made);
            self.from = Some(texts);
        }
        if let Some(made) = &self.natives {
            for pair in made.pairs::<LuaValue, LuaValue>() {
                let (key, native) = pair?;
                natives.raw_set(key, native)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "texts_tests.rs"]
mod tests;
