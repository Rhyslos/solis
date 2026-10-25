//! Game code: one Lust file and one Lua file, checked, then loaded and
//! run for a tick in a sandboxed runtime, within its time limits.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    // A byte 0x01 splits the Lust (before) from the Lua (after).
    let (lust, lua) = text.split_once('\u{1}').unwrap_or((text, ""));
    let mut files = script::describe::descriptions("Fuzz 0.0.0", "solis");
    files.push(("main.lst".into(), lust.to_string()));
    let code = script::Code {
        files,
        lua: vec![("fuzz.lua".into(), lua.to_string())],
        notes: Vec::new(),
    };
    let Ok(Some(compiled)) = script::compile(&code) else {
        return;
    };
    let Ok(mut runtime) = script::Runtime::new() else {
        return;
    };
    if runtime.load(&compiled, &mut script::NoHost).is_ok() {
        runtime.tick(1.0 / 60.0, &mut script::NoHost);
    }
});
