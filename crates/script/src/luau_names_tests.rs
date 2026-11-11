//! Every name game code can see, held to a list: an update to Luau or
//! mlua that adds a global (as `loadstring`, `getfenv` and `setfenv` came)
//! fails here, where someone decides whether game code may have it,
//! instead of widening what game code can reach without anyone noticing.

use super::*;

/// The shared globals (under no name), then each library's own names.
/// Luau's `getfenv`, `setfenv`, `loadstring` and `require` aren't here:
/// [`sandboxed_lua`] takes them out. `debug` has no way to a function's
/// globals or locals, only `info` and `traceback`.
const NAMES: &[(&str, &str)] = &[
    (
        "",
        "_G _VERSION assert bit32 buffer collectgarbage component coroutine debug error gcinfo \
         getmetatable integer ipairs math newproxy next os pairs pcall print rawequal rawget \
         rawlen rawset select setmetatable solis string table tonumber tostring type typeof \
         unpack utf8 vec3 vector xpcall",
    ),
    (
        "bit32",
        "arshift band bnot bor btest bxor byteswap countlz countrz extract lrotate lshift \
         replace rrotate rshift",
    ),
    (
        "buffer",
        "copy create fill fromstring len readbits readf32 readf64 readi16 readi32 readi8 \
         readstring readu16 readu32 readu8 tostring writebits writef32 writef64 writei16 \
         writei32 writei8 writestring writeu16 writeu32 writeu8",
    ),
    (
        "coroutine",
        "close create isyieldable resume running status wrap yield",
    ),
    ("debug", "info traceback"),
    (
        "integer",
        "add arshift band bnot bor bswap btest bxor clamp countlz countrz create div extract \
         fromstring ge gt idiv le lrotate lshift lt max maxsigned min minsigned mod mul neg rem \
         replace rrotate rshift sub tonumber udiv uge ugt ule ult urem",
    ),
    (
        "math",
        "abs acos asin atan atan2 ceil clamp cos cosh deg e exp floor fmod frexp huge isfinite \
         isinf isnan ldexp lerp log log10 map max min modf nan noise phi pi pow rad random \
         randomseed round sign sin sinh sqrt sqrt2 tan tanh tau",
    ),
    ("os", "clock date difftime time"),
    ("solis", "send"),
    (
        "string",
        "byte char find format gmatch gsub len lower match pack packsize rep reverse split sub \
         unpack upper",
    ),
    (
        "table",
        "clear clone concat create find foreach foreachi freeze getn insert isfrozen maxn move \
         pack remove sort unpack",
    ),
    ("utf8", "char charpattern codepoint codes len offset"),
    (
        "vector",
        "abs angle ceil clamp create cross dot floor lerp magnitude max min normalize one sign \
         zero",
    ),
];

/// The names `table` holds itself, in order, with spaces between.
fn names(table: &Table) -> String {
    let mut names: Vec<String> = table
        .pairs::<String, LuaValue>()
        .map(|pair| pair.unwrap().0)
        .collect();
    names.sort();
    names.join(" ")
}

#[test]
fn game_code_sees_only_the_names_on_the_list() {
    let luau = Luau::new().unwrap();
    let mut seen = vec![(String::new(), names(&luau.globals))];
    for pair in luau.globals.pairs::<String, LuaValue>() {
        let (name, value) = pair.unwrap();
        match value {
            // The shared globals again, under their own name.
            LuaValue::Table(library) if library == luau.globals => assert_eq!(name, "_G"),
            LuaValue::Table(library) => seen.push((name, names(&library))),
            _ => {}
        }
    }
    seen.sort();
    let listed: Vec<(String, String)> = NAMES
        .iter()
        .map(|(library, names)| ((*library).to_string(), (*names).to_string()))
        .collect();
    assert_eq!(
        seen, listed,
        "what game code can see changed: decide whether it may have each new name (can it reach \
         files, other code's globals, or run without the time limit?), then update NAMES"
    );
    // A file's own globals start with only these two.
    assert_eq!(names(&luau.env().unwrap()), "_G vector");
}

#[test]
fn every_way_to_a_function_s_globals_is_gone_from_every_library() {
    let luau = Luau::new().unwrap();
    let env = luau.env().unwrap();
    let found: String = luau
        .lua
        .load(
            "local found = {}\n\
             for _, name in ipairs({ 'getfenv', 'setfenv', 'loadstring', 'require' }) do\n\
               if _G[name] ~= nil then found[#found + 1] = name end\n\
               for _, library in ipairs({ debug, coroutine, table, string, os, math }) do\n\
                 if rawget(library, name) ~= nil then found[#found + 1] = name end\n\
               end\n\
             end\n\
             return table.concat(found, ' ')",
        )
        .set_environment(env)
        .eval()
        .unwrap();
    assert_eq!(found, "");
}
