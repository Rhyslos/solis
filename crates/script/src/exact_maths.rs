//! Game code's maths, the same to the last bit on every PC (stage 1, the
//! same game on two PCs). Luau's `math.sin`, `math.exp` and the rest call
//! the C library of the system the game runs on, and Windows' rounds the
//! last bit of some answers differently from Linux's: the two-PC check
//! caught a game's sine coming out differently on its first run on
//! Windows. Here each of them is worked out by `libm`, maths written in
//! Rust, which gives exactly the same bits on every processor and system.
//!
//! - [`install`] puts them in the `math` library every Luau has, before
//!   anything else runs, so Lua's `math.sin` and Lust's `Maths:sin` (which
//!   Lust's own runtime takes from `math`) are these and no other. The
//!   library is made read-only after, so game code can't put the C ones
//!   back (it never had them).
//! - [`exact_lust`] makes Lust's own runtime raise numbers to powers with
//!   `math.pow` where it used Lua's `^` (`Maths:power`, and the easing
//!   curves of tweens that use powers of two), since `^` is the C
//!   library's too.
//!
//! What's left out is exact everywhere already: `sqrt`, `floor`, `ceil`,
//! `abs`, `min`, `max`, `fmod`, `modf`, `frexp`, `ldexp`, `deg`, `rad`,
//! `noise`, `clamp`, `sign`, `round`, `lerp` and `map` are plain arithmetic
//! or correctly rounded. Lua's own `^` in a game's Lua file still calls the
//! C library: `math.pow(a, b)` is the exact one (the manual says so).

use mlua::{Lua, Table};

/// One number in, one out.
type One = fn(f64) -> f64;

/// The `math` library's functions of one number, as `libm` works them out.
const ONE: [(&str, One); 11] = [
    ("sin", libm::sin),
    ("cos", libm::cos),
    ("tan", libm::tan),
    ("asin", libm::asin),
    ("acos", libm::acos),
    ("atan", libm::atan),
    ("exp", libm::exp),
    ("log10", libm::log10),
    ("sinh", libm::sinh),
    ("cosh", libm::cosh),
    ("tanh", libm::tanh),
];

/// Puts `libm`'s functions in place of the C library's in `lua`'s `math`
/// library. Before the library is made read-only ([`Lua::sandbox`]).
pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    let math: Table = lua.globals().raw_get("math")?;
    for (name, f) in ONE {
        math.raw_set(name, lua.create_function(move |_, x: f64| Ok(f(x)))?)?;
    }
    math.raw_set(
        "atan2",
        lua.create_function(|_, (y, x): (f64, f64)| Ok(libm::atan2(y, x)))?,
    )?;
    math.raw_set(
        "pow",
        lua.create_function(|_, (x, y): (f64, f64)| Ok(libm::pow(x, y)))?,
    )?;
    math.raw_set(
        "log",
        lua.create_function(|_, (x, base): (f64, Option<f64>)| Ok(log(x, base)))?,
    )?;
    Ok(())
}

/// Luau's `math.log`: the natural log, or with a base (2 and 10 by their
/// own functions, as Luau does, so `math.log(8, 2)` is exactly 3).
fn log(x: f64, base: Option<f64>) -> f64 {
    match base {
        None => libm::log(x),
        Some(2.0) => libm::log2(x),
        Some(10.0) => libm::log10(x),
        Some(base) => libm::log(x) / libm::log(base),
    }
}

/// Where Lust's runtime raises to a power with Lua's `^`, and what it does
/// instead: `Maths:power`, and the easing curves' `2 ^ (…)`. Whole powers
/// of two it works out with `^` elsewhere (`2 ^ n` for a bit's place) are
/// exact, and left as they are.
const LUST_POWERS: [(&str, &str); 2] = [
    (
        "function L.maths_power(a, b) return a ^ b end",
        "function L.maths_power(a, b) return math.pow(a, b) end",
    ),
    ("2 ^ (", "math.pow(2, "),
];

/// Where Lust's chunk closes its runtime (`local _L = (function() …
/// end)()`): the game's own code comes after, and is never changed.
const RUNTIME_END: &str = "\nend)()\n";

/// Lust's chunk with its runtime's powers worked out by `math.pow`
/// ([`LUST_POWERS`]). Only the runtime is changed, never the game's own
/// code after it (a text in it could hold the same characters), and no
/// line moves, so Lust still says which line of a game's file failed.
pub(crate) fn exact_lust(mut chunk: lust::LuaChunk) -> lust::LuaChunk {
    let end = chunk
        .code
        .find(RUNTIME_END)
        .map_or(0, |at| at + RUNTIME_END.len());
    let (runtime, game) = chunk.code.split_at(end);
    let mut runtime = runtime.to_string();
    for (from, to) in LUST_POWERS {
        runtime = runtime.replace(from, to);
    }
    runtime.push_str(game);
    chunk.code = runtime;
    chunk
}

#[cfg(test)]
#[path = "exact_maths_tests.rs"]
mod tests;
