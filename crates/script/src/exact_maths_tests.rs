//! Game code's maths gives the same bits on every PC: each function's
//! answers for a set of numbers are pinned to exact bit patterns (a PC
//! whose answers differ fails here), Lua's `math` reaches `libm`'s, never
//! the C library's, and Lust's runtime raises to powers with them too.

use super::*;
use crate::luau::Luau;

/// The functions checked, each with the Lua that calls it on `x` (and a
/// second number, `y`, where it takes two).
const CALLS: [(&str, &str); 16] = [
    ("sin", "math.sin(x)"),
    ("cos", "math.cos(x)"),
    ("tan", "math.tan(x)"),
    ("asin", "math.asin(x / 8)"),
    ("acos", "math.acos(x / 8)"),
    ("atan", "math.atan(x)"),
    ("atan2", "math.atan2(x, y)"),
    ("exp", "math.exp(x)"),
    ("log", "math.log(x + 4)"),
    ("log base 2", "math.log(x + 4, 2)"),
    ("log base 3", "math.log(x + 4, 3)"),
    ("log10", "math.log10(x + 4)"),
    ("pow", "math.pow(x + 4, y)"),
    ("sinh", "math.sinh(x)"),
    ("cosh", "math.cosh(x)"),
    ("tanh", "math.tanh(x)"),
];

/// The numbers each is given: (x, y).
const INPUTS: [(f64, f64); 6] = [
    (0.5, 1.5),
    (1.0, -2.25),
    (std::f64::consts::E, 0.1),
    (-3.0, 7.0),
    (0.000123, 0.333),
    (3.1, 2.0),
];

/// What each call gives for each input, as Lua's game code sees it.
fn answers() -> Vec<(String, u64)> {
    let luau = Luau::new().unwrap();
    let env = luau.env().unwrap();
    let mut found = Vec::new();
    for (name, call) in CALLS {
        for (n, (x, y)) in INPUTS.iter().enumerate() {
            let code = format!("local x, y = {x:?}, {y:?}\nreturn {call}");
            let f: mlua::Function = crate::luau::as_text(luau.lua.load(&code))
                .set_environment(env.clone())
                .into_function()
                .unwrap();
            let v: f64 = f.call(()).unwrap();
            found.push((format!("{name} #{n}"), v.to_bits()));
        }
    }
    found
}

#[test]
fn each_function_gives_exactly_the_pinned_bits() {
    let found = answers();
    let text: Vec<String> = found
        .iter()
        .map(|(name, bits)| format!("(\"{name}\", 0x{bits:016x}),"))
        .collect();
    assert_eq!(found.len(), PINNED.len(), "{}", text.join("\n"));
    for ((name, bits), (pinned_name, pinned)) in found.iter().zip(PINNED) {
        assert_eq!(name, pinned_name);
        assert_eq!(
            *bits,
            pinned,
            "{name} gave {} ({bits:016x}), not {} ({pinned:016x}): this PC's maths differs",
            f64::from_bits(*bits),
            f64::from_bits(pinned)
        );
    }
}

#[test]
fn lua_s_math_is_libm_s_and_never_the_c_library_s() {
    let found = answers();
    let at = |name: &str| {
        found
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| f64::from_bits(*b))
            .unwrap()
    };
    assert_eq!(at("sin #0"), libm::sin(0.5));
    assert_eq!(at("atan2 #1"), libm::atan2(1.0, -2.25));
    assert_eq!(at("pow #3"), libm::pow(1.0, 7.0));
    assert_eq!(at("log base 2 #0"), libm::log2(4.5));
    assert_eq!(at("log base 3 #5"), libm::log(7.1) / libm::log(3.0));
    // Luau's own builtins carry their names; these are Solis's.
    let luau = Luau::new().unwrap();
    let env = luau.env().unwrap();
    for name in [
        "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "exp", "log", "log10", "pow", "sinh",
        "cosh", "tanh",
    ] {
        let code = format!("return debug.info(math.{name}, \"n\")");
        let f: mlua::Function = crate::luau::as_text(luau.lua.load(&code))
            .set_environment(env.clone())
            .into_function()
            .unwrap();
        let shown: Option<String> = f.call(()).unwrap();
        assert_ne!(
            shown.as_deref(),
            Some(name),
            "math.{name} is still Luau's own"
        );
    }
    // And game code can't put them back: the library is read-only.
    let f: mlua::Function = crate::luau::as_text(luau.lua.load("math.sin = nil"))
        .set_environment(env)
        .into_function()
        .unwrap();
    assert!(f.call::<()>(()).is_err());
}

#[test]
fn lust_s_runtime_raises_to_powers_with_math_pow_and_leaves_the_game_s_code() {
    let (mut files, _) = crate::describe::descriptions_for(
        "Test 1.0",
        "solis",
        &crate::project_names::Names::standard(),
    );
    files.push((
        "main.lst".into(),
        "component Text {\n    has {\n        string s = \"2 ^ (x)\";\n    }\n}\npass Text;\nend\n"
            .into(),
    ));
    let code = crate::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let compiled = crate::compile(&code).unwrap().unwrap();
    let text = &compiled.lust.as_ref().unwrap().code;
    let end = text.find(RUNTIME_END).unwrap();
    let (runtime, game) = text.split_at(end);
    // Both of Lust's ways of raising to a power were found and changed:
    // a Lust that changes them fails here, to be looked at again.
    assert!(runtime.contains(LUST_POWERS[0].1), "Maths:power");
    assert!(
        runtime.matches(LUST_POWERS[1].1).count() >= 8,
        "the easing curves' powers of two"
    );
    assert!(!runtime.contains(" ^ ("), "no power left to the C library");
    assert!(
        game.contains("2 ^ (x)"),
        "the game's own text is left alone"
    );
}

/// Each answer's bits, pinned: worked out once by `libm`, the same on
/// every PC. A PC giving others fails [`each_function_gives_exactly_the_pinned_bits`].
const PINNED: [(&str, u64); 96] = [
    ("sin #0", 0x3fdeaee8744b05f0),
    ("sin #1", 0x3feaed548f090cee),
    ("sin #2", 0x3fda4a3d9c2131df),
    ("sin #3", 0xbfc210386db6d55b),
    ("sin #4", 0x3f201f31f3c039be),
    ("sin #5", 0x3fa54a0f8298102e),
    ("cos #0", 0x3fec1528065b7d50),
    ("cos #1", 0x3fe14a280fb5068c),
    ("cos #2", 0xbfed2cec9a554007),
    ("cos #3", 0xbfefae04be85e5d2),
    ("cos #4", 0x3feffffffbf057d1),
    ("cos #5", 0xbfeff8ea4756a624),
    ("tan #0", 0x3fe17b4f5bf3474a),
    ("tan #1", 0x3ff8eb245cbee3a6),
    ("tan #2", 0xbfdcd5cdb683b402),
    ("tan #3", 0x3fc23ef71254b86f),
    ("tan #4", 0x3f201f31f5cc0357),
    ("tan #5", 0xbfa54ec734d41690),
    ("asin #0", 0x3fb002abde953619),
    ("asin #1", 0x3fc00abe0c129e1e),
    ("asin #2", 0x3fd63025169f3e50),
    ("asin #3", 0xbfd899f4edc962d3),
    ("asin #4", 0x3ef01f31f4718ca8),
    ("asin #5", 0x3fd9777dbd067616),
    ("acos #0", 0x3ff821d0965ad9b7),
    ("acos #1", 0x3ff720a392c1d954),
    ("acos #2", 0x3ff395f20e9c5d84),
    ("acos #3", 0x3fff48788fb685cd),
    ("acos #4", 0x3ff921eb351238a7),
    ("acos #5", 0x3ff2c41be5028f93),
    ("atan #0", 0x3fddac670561bb4f),
    ("atan #1", 0x3fe921fb54442d18),
    ("atan #2", 0x3ff37e1637253389),
    ("atan #3", 0xbff3fc176b7a8560),
    ("atan #4", 0x3f201f31f311a135),
    ("atan #5", 0x3ff423db7312cebc),
    ("atan2 #0", 0x3fd4978fa3269ee1),
    ("atan2 #1", 0x4005c97554f4b39e),
    ("atan2 #2", 0x3ff88b5dc2f82fa8),
    ("atan2 #3", 0xbfd9e9bf3d20dc70),
    ("atan2 #4", 0x3f3834fd4a189d83),
    ("atan2 #5", 0x3fefee3990dad712),
    ("exp #0", 0x3ffa61298e1e069c),
    ("exp #1", 0x4005bf0a8b14576a),
    ("exp #2", 0x402e4efb75e4527a),
    ("exp #3", 0x3fa97db0ccceb0af),
    ("exp #4", 0x3ff00080fb977d03),
    ("exp #5", 0x403632acef67fe3d),
    ("log #0", 0x3ff810b375dce91e),
    ("log #1", 0x3ff9c041f7ed8d33),
    ("log #2", 0x3ffe7a31950d6104),
    ("log #3", 0x0000000000000000),
    ("log #4", 0x3ff62e633d3da5b7),
    ("log #5", 0x3fff5c8c5928b0f9),
    ("log base 2 #0", 0x40015c01a39fbd68),
    ("log base 2 #1", 0x4002934f0979a371),
    ("log base 2 #2", 0x4005fc17d3248eac),
    ("log base 2 #3", 0x0000000000000000),
    ("log base 2 #4", 0x40000017423141cd),
    ("log base 2 #5", 0x40069f5f94ade770),
    ("log base 3 #0", 0x3ff5e7b633e56563),
    ("log base 3 #1", 0x3ff77088130fd4e5),
    ("log base 3 #2", 0x3ffbbddcdca6026f),
    ("log base 3 #3", 0x0000000000000000),
    ("log base 3 #4", 0x3ff430b0f18fe8a9),
    ("log base 3 #5", 0x3ffc8be6480ad839),
    ("log10 #0", 0x3fe4e71dee0018fe),
    ("log10 #1", 0x3fe65df657b04301),
    ("log10 #2", 0x3fea78e63a20a773),
    ("log10 #3", 0x0000000000000000),
    ("log10 #4", 0x3fe3442f522f5d47),
    ("log10 #5", 0x3feb3d82260640a6),
    ("pow #0", 0x40231785a67b5a75),
    ("pow #1", 0x3f9b6440164d7276),
    ("pow #2", 0x3ff35b7b00c48231),
    ("pow #3", 0x3ff0000000000000),
    ("pow #4", 0x3ff9630eb12986fe),
    ("pow #5", 0x4049347ae147ae14),
    ("sinh #0", 0x3fe0acd00fe63b97),
    ("sinh #1", 0x3ff2cd9fc44eb982),
    ("sinh #2", 0x401e2d3246d48e6f),
    ("sinh #3", 0xc0240926e70949ae),
    ("sinh #4", 0x3f201f31f51d6ace),
    ("sinh #5", 0x402627249735007d),
    ("cosh #0", 0x3ff20ac1862ae8d0),
    ("cosh #1", 0x3ff8b07551d9f551),
    ("cosh #2", 0x401e70c4a4f41685),
    ("cosh #3", 0x402422a497d6185e),
    ("cosh #4", 0x3ff000000207d417),
    ("cosh #5", 0x40263e35479afbfd),
    ("tanh #0", 0x3fdd9353d7568af3),
    ("tanh #1", 0x3fe85efab514f394),
    ("tanh #2", 0x3fefb8f76b1e2ab6),
    ("tanh #3", 0xbfefd77d111a0b00),
    ("tanh #4", 0x3f201f31f311a135),
    ("tanh #5", 0x3fefded130ca9124),
];

/// How long `calls` calls of `call` take in a loop of game code, in
/// nanoseconds a call, in a Luau made by `make` (a file's own globals, as
/// game code has, so no call takes Luau's builtin shortcut).
fn time_calls(lua: &mlua::Lua, env: mlua::Table, call: &str, calls: u32) -> f64 {
    let code = format!(
        "local s = 0\nfor i = 1, {calls} do\n  local x = i * 0.001\n  s = s + {call}\nend\nreturn s"
    );
    let f: mlua::Function = crate::luau::as_text(lua.load(&code))
        .set_environment(env)
        .into_function()
        .unwrap();
    let _: f64 = f.call(()).unwrap();
    let start = std::time::Instant::now();
    let _: f64 = f.call(()).unwrap();
    start.elapsed().as_nanos() as f64 / f64::from(calls)
}

/// The cost of a call to `libm`'s maths against the C library's, as game
/// code makes it (`cargo test --release -p script exact_maths_cost --
/// --ignored --nocapture`): the loop alone, then each function both ways.
#[test]
#[ignore = "a measurement; run by hand in release"]
fn exact_maths_cost() {
    let calls = 2_000_000;
    let ours = Luau::new().unwrap();
    // Today's: Luau's own library, the C one, reached the same way.
    let theirs = mlua::Lua::new_with(mlua::StdLib::ALL_SAFE, mlua::LuaOptions::default()).unwrap();
    let theirs_env = {
        let env = theirs.create_table().unwrap();
        let through = theirs.create_table().unwrap();
        through.raw_set("__index", theirs.globals()).unwrap();
        env.set_metatable(Some(through)).unwrap();
        env
    };
    for call in [
        "x",
        "math.sin(x)",
        "math.exp(x)",
        "math.atan2(x, 2)",
        "math.pow(x, 1.5)",
    ] {
        let c = time_calls(&theirs, theirs_env.clone(), call, calls);
        let r = time_calls(&ours.lua, ours.env().unwrap(), call, calls);
        eprintln!(
            "{call:>18}: C library {c:6.1} ns a call, libm {r:6.1} ns ({:+.1} ns)",
            r - c
        );
    }
}
