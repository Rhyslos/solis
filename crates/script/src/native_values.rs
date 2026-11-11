//! A native method's values, read one at a time, so a value of the wrong
//! kind is a mistake said in plain words by the call's own name:
//! `solis.World:say(words) wants text for words, and was given nothing
//! (nil)`. Only Lua can make this mistake (Lust and Rust are checked
//! before they run), and its author may be new to game engines: the words
//! are theirs, never the Lua library's.
//!
//! Left to the library, the values are read all at once and its error
//! names neither the call nor the value. So a native made with [`native`]
//! takes every value as it comes and reads each as its kind itself.

use crate::describe::{NATIVES, Native};
use mlua::{FromLua, FromLuaMulti, Function, IntoLuaMulti, Lua, Value as LuaValue};

/// What's said when game code calls a native it kept from an earlier
/// moment: the natives are lent only while game code runs (see
/// [`crate::host::with_natives`]), so one kept in a variable stops working
/// when the code that asked for it ends.
pub(crate) const KEPT: &str = "this calls something of Solis's that was kept from earlier, as \
    local say = solis.World.say at the top of a file keeps it. What solis gives works only while \
    the code that asked for it runs. Write the call out where it's used, solis.World:say(\"hello\"), \
    or keep the object and call through it: local World = solis.World, then World:say(\"hello\")";

/// A native's values (a tuple of them), each read from what Lua gave by
/// itself.
pub(crate) trait Values: Sized {
    /// What Lua gave, not read yet: one value for each.
    type Given: FromLuaMulti;

    /// The values read as their kinds; or which one (from 1) wasn't its
    /// kind, and the library's reason.
    fn read(given: Self::Given, lua: &Lua) -> Result<Self, (usize, mlua::Error)>;
}

macro_rules! values {
    (@given $kind:ident) => { LuaValue };
    ($($kind:ident $value:ident),+) => {
        impl<$($kind: FromLua),+> Values for ($($kind,)+) {
            type Given = ($(values!(@given $kind),)+);

            fn read(($($value,)+): Self::Given, lua: &Lua) -> Result<Self, (usize, mlua::Error)> {
                let mut at = 0;
                Ok(($(
                    {
                        at += 1;
                        $kind::from_lua($value, lua).map_err(|why| (at, why))?
                    },
                )+))
            }
        }
    };
}

values!(A a, B b);
values!(A a, B b, C c);
values!(A a, B b, C c, D d);
values!(A a, B b, C c, D d, E e);
values!(A a, B b, C c, D d, E e, F f);

/// The native `key` (its name in `LUST_NATIVES`: `World_say`) as a Lua
/// function for as long as `scope` lasts. A value that isn't the kind
/// `body` takes, or text `body` can't read, is a mistake in plain words
/// that names the call ([`wrong_value`]).
pub(crate) fn native<'scope, A, R>(
    scope: &'scope mlua::Scope<'scope, '_>,
    key: &'static str,
    body: impl Fn(&Lua, A) -> mlua::Result<R> + 'scope,
) -> mlua::Result<Function>
where
    A: Values,
    R: IntoLuaMulti,
{
    scope.create_function(move |lua, given: A::Given| {
        let values = A::read(given, lua).map_err(|(at, why)| wrong_value(key, Some(at), why))?;
        body(lua, values).map_err(|why| wrong_value(key, None, why))
    })
}

/// `why` in plain words, if it's a value that couldn't be read as its
/// kind: the call to the native `key` as Lua writes it, what it wants for
/// its value number `at` (from 1, as the native takes them: the object or
/// entity first) and what it was given. `at` is `None` for text the
/// native's body couldn't read. Any other failure is given back as it is.
fn wrong_value(key: &str, at: Option<usize>, why: mlua::Error) -> mlua::Error {
    let mlua::Error::FromLuaConversionError { from, to, message } = &why else {
        return why;
    };
    let native = NATIVES.iter().find(|native| native.key() == key);
    let call = native.map_or_else(|| format!("solis.{key}"), written);
    let Some(at) = at else {
        return mlua::Error::runtime(format!(
            "{call} was given text it can't read: text for Solis must be UTF-8"
        ));
    };
    // The object (`solis.World`) or the entity comes before the values the
    // native's Lust lists.
    let before = usize::from(native.is_some_and(|native| !native.owner.is_empty()));
    let value = native
        .zip(at.checked_sub(before + 1))
        .and_then(|(native, i)| values_of(native.lust).nth(i));
    let kind = value.map_or("", |(kind, _)| kind);
    let name = value.map_or_else(|| format!("value {at}"), |(_, name)| name.to_string());
    let whole = message.as_deref() == Some("out of range");
    let wanted = match (to.as_str(), kind) {
        ("string", _) => "text",
        ("f64", _) => "a number",
        ("i64", _) => "a whole number",
        (_, "rotation") => "a rotation (what solis.entity.rotation gives)",
        ("table", _) => "a table",
        _ => "another kind of value",
    };
    let given = match (*from, to.as_str()) {
        ("nil", _) => "nothing (nil)".to_string(),
        ("boolean", _) => "true or false".to_string(),
        ("string", "i64") if whole => "text that isn't a whole number".to_string(),
        ("string", "f64" | "i64") => "text that isn't a number".to_string(),
        ("string", _) => "text".to_string(),
        ("number" | "integer", "i64") => "a number that isn't whole, or is too big".to_string(),
        ("number" | "integer", _) => "a number".to_string(),
        ("vector", _) => "a vec3".to_string(),
        (other, _) => format!("a {other}"),
    };
    // A misspelt name is nil in Lua, and so is every value when the object
    // isn't given first: solis.World.say("hello") for solis.World:say("hello").
    let hint = match (*from, native.map(|native| native.owner)) {
        ("nil", Some("" | "entity") | None) => " Is a value missing, or its name misspelt?",
        ("nil", Some(_)) => {
            " Is a value missing, its name misspelt, or the colon before the call's name written \
             as a dot?"
        }
        _ => "",
    };
    mlua::Error::runtime(format!(
        "{call} wants {wanted} for {name}, and was given {given}.{hint}"
    ))
}

/// How Lua writes a call to `native`, with its values' names:
/// `solis.World:say(words)`, `solis.entity.moveTo(target, place)`,
/// `solis.facingAlong(direction)`.
fn written(native: &Native) -> String {
    let mut values: Vec<&str> = values_of(native.lust).map(|(_, name)| name).collect();
    match native.owner {
        "" => format!("solis.{}({})", native.name, values.join(", ")),
        "entity" => {
            values.insert(0, "target");
            format!("solis.entity.{}({})", native.name, values.join(", "))
        }
        owner => format!("solis.{owner}:{}({})", native.name, values.join(", ")),
    }
}

/// The values a native's Lust says it takes (`method say(string words)
/// native;`), each as its kind and name.
fn values_of(lust: &str) -> impl Iterator<Item = (&str, &str)> {
    let inside = lust
        .split_once('(')
        .and_then(|(_, rest)| rest.rsplit_once(')'))
        .map_or("", |(inside, _)| inside);
    inside
        .split(',')
        .filter_map(|value| value.trim().rsplit_once(' '))
}

#[cfg(test)]
#[path = "native_values_tests.rs"]
mod tests;
