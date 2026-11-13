//! What game code can call, as one text (Phase 8, C3;
//! [decision 0053](../../../docs/decisions/0053-compatibility-before-2d.md)).
//!
//! [`snapshot`] lists everything a game's Lust, Lua or Rust can reach in
//! Solis: each native with its Lust, the names Lua's `solis` offers (asked
//! of a real Luau with the natives lent, so it's what Lua can reach), the
//! methods of the traits Rust components use, and the ready-made
//! components' names and fields. A test compares it with
//! `docs/game-code-interface.txt`, so a change to what games call can't
//! slip in by accident: the pull request that makes it must update the
//! file too, where a reviewer sees it, and a rename must keep the old
//! name working ([`crate::describe::RENAMED`]).

use crate::describe::{ENGINE_EVENTS, KIT, NATIVES, RENAMED};

/// The interface as text, a line per thing a game can call, in a fixed
/// order, so a change shows as a line added, removed or changed.
pub fn snapshot() -> String {
    let mut out = String::from(
        "# What game code can call\n\
         #\n\
         # Written by a test (crates/script/src/interface.rs). If it fails, the interface\n\
         # changed: if that's meant, run the test with SOLIS_UPDATE_INTERFACE=1 and commit\n\
         # this file, and keep any renamed name working (describe::RENAMED).\n",
    );
    out.push_str("\n## Lust (solis/<file>.lst)\n\n");
    for native in NATIVES {
        match native.owner {
            "" | "entity" => out.push_str(&format!("{}: {}\n", native.file, native.lust)),
            object => out.push_str(&format!(
                "{}: object {object}: {}\n",
                native.file, native.lust
            )),
        }
    }
    out.push_str("\n## Events the engine sends (fetch \"solis/<file>\":<Name>)\n\n");
    for event in ENGINE_EVENTS {
        out.push_str(&format!("{}: {}\n", event.file, event.lust));
    }
    out.push_str("\n## Lua (solis.<name>)\n\n");
    let mut lua = match lua_ways() {
        Ok(ways) => ways,
        Err(err) => vec![format!("(Luau couldn't be asked: {err})")],
    };
    lua.push("send".to_owned());
    lua.sort();
    for way in lua {
        out.push_str(&format!("solis.{way}\n"));
    }
    out.push_str("\n## Lua's own names Solis adds or changes (its prelude)\n\n");
    for line in crate::luau::PRELUDE.lines() {
        let line = line.trim_end();
        // Each global it defines or replaces, as written: a change to what
        // they take shows here.
        let defines = (line.starts_with("function ") && !line.starts_with("function("))
            || (!line.starts_with(' ')
                && !line.starts_with("local ")
                && !line.starts_with("--")
                && line.contains(" = function("));
        if defines {
            out.push_str(&format!("{line}\n"));
        }
    }
    out.push_str("\n## Rust (crates/script: Host, Context, RustComponent)\n\n");
    for (name, source, header) in [
        ("Host", include_str!("host.rs"), "pub trait Host {"),
        (
            "RustComponent",
            include_str!("rust_component.rs"),
            "pub trait RustComponent {",
        ),
        (
            "Context",
            include_str!("rust_component.rs"),
            "impl<'a> Context<'a> {",
        ),
    ] {
        for method in methods_in(source, header) {
            out.push_str(&format!("{name}: {method}\n"));
        }
    }
    out.push_str("\n## Old names still working (renamed)\n\n");
    for renamed in RENAMED {
        out.push_str(&format!(
            "{}: {} is now {}, until {}\n",
            renamed.file, renamed.old, renamed.new, renamed.until
        ));
    }
    out.push_str("\n## Ready-made components (solis/kit), with their fields\n\n");
    for (file, body) in KIT {
        for line in kit_outline(body) {
            out.push_str(&format!("kit/{file}: {line}\n"));
        }
    }
    out
}

/// Each native as Lua writes it after `solis.` while the game runs
/// (`World:say`, `entity.moveTo`, `facingAlong`). A real Luau with the
/// natives lent is asked, so a native Lua can't reach the way it's
/// described shows here (once `solis` left out four engine objects, and
/// this list, then made from the natives' names alone, never showed it).
fn lua_ways() -> mlua::Result<Vec<String>> {
    let luau = crate::luau::Luau::new()?;
    let mut host = crate::host::NoHost;
    let ways = crate::host::with_natives(&luau, None, &mut host, None, |_| luau.ways())?;
    Ok(ways
        .iter()
        .map(|way| match way.as_slice() {
            // An entity's method is handed the entity; an engine object's
            // is called on the object.
            [owner, method] if owner == "entity" => format!("entity.{method}"),
            [owner, method] => format!("{owner}:{method}"),
            names => names.join("."),
        })
        .collect())
}

/// The `fn` lines (up to their `;` or `{`, joined onto one line) of the
/// block starting with `header`, which ends at a `}` alone at the start of
/// a line. Only public functions in an `impl`; every one in a trait.
fn methods_in(source: &str, header: &str) -> Vec<String> {
    let Some(start) = source.find(header) else {
        return Vec::new();
    };
    let body = &source[start + header.len()..];
    let body = &body[..body.find("\n}").unwrap_or(body.len())];
    let in_impl = header.starts_with("impl");
    let mut methods = Vec::new();
    let mut lines = body.lines().map(str::trim).peekable();
    while let Some(line) = lines.next() {
        let public = line.starts_with("pub fn ");
        if !(line.starts_with("fn ") && !in_impl || public) {
            continue;
        }
        let mut signature = line.to_owned();
        while !signature.contains(['{', ';']) {
            match lines.next() {
                Some(more) => {
                    signature.push(' ');
                    signature.push_str(more);
                }
                None => break,
            }
        }
        let end = signature.find(['{', ';']).unwrap_or(signature.len());
        methods.push(
            signature[..end]
                .trim()
                .replace("( ", "(")
                .replace(", )", ")"),
        );
    }
    methods
}

/// A ready-made component file's `component` lines and the fields in
/// their `has` blocks.
fn kit_outline(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut component = "";
    let mut in_has = false;
    for line in body.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix("component ") {
            component = name.split_whitespace().next().unwrap_or_default();
            out.push(format!("component {component}"));
        } else if line.starts_with("has {") {
            in_has = true;
        } else if in_has && line.starts_with('}') {
            in_has = false;
        } else if in_has && !line.is_empty() && !line.starts_with("--") {
            out.push(format!("{component}.{}", line.trim_end_matches(';')));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The file the interface is kept in.
    fn kept() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/game-code-interface.txt")
    }

    #[test]
    fn what_game_code_can_call_changes_only_on_purpose() {
        let now = snapshot();
        if std::env::var_os("SOLIS_UPDATE_INTERFACE").is_some() {
            std::fs::write(kept(), &now).unwrap();
            return;
        }
        // A Windows checkout may have turned its line endings into CRLF.
        let was = std::fs::read_to_string(kept())
            .unwrap_or_default()
            .replace("\r\n", "\n");
        if was != now {
            // Lines listed a different number of times show in neither.
            let gone: Vec<&str> = was
                .lines()
                .filter(|l| !now.lines().any(|n| n == *l))
                .collect();
            let new: Vec<&str> = now
                .lines()
                .filter(|l| !was.lines().any(|w| w == *l))
                .collect();
            panic!(
                "What game code can call changed (lines below; none means a line is listed a \
                 different number of times).\n\nGone or changed:\n  {}\n\nNew:\n  {}\n\n\
                 If that's meant: keep any renamed name working (describe::RENAMED), run this \
                 test with SOLIS_UPDATE_INTERFACE=1, commit docs/game-code-interface.txt, and \
                 say so under Breaking in CHANGELOG.md when a name went away.",
                gone.join("\n  "),
                new.join("\n  ")
            );
        }
    }

    /// The keys of the natives table as Lust is lent it, asked of a real
    /// Luau, so a native lent by any route counts: each native's key,
    /// without the old names renamed natives are lent under too
    /// (`describe::RENAMED`).
    fn lent_names() -> Vec<String> {
        let luau = crate::luau::Luau::new().unwrap();
        let lust = luau.lua.create_table().unwrap();
        let mut host = crate::host::NoHost;
        let mut names: Vec<String> =
            crate::host::with_natives(&luau, Some(&lust), &mut host, None, |_| {
                let natives: mlua::Table = lust.raw_get("LUST_NATIVES")?;
                natives
                    .pairs::<String, mlua::Value>()
                    .map(|pair| pair.map(|(key, _)| key))
                    .collect()
            })
            .unwrap();
        names.retain(|name| !RENAMED.iter().any(|r| r.old == name));
        names.sort();
        names
    }

    #[test]
    fn every_native_is_lent_to_lust_and_lua() {
        let lent = lent_names();
        for native in NATIVES {
            assert!(
                lent.iter().any(|n| *n == native.key()),
                "{} isn't lent",
                native.key()
            );
        }
        for name in &lent {
            assert!(
                NATIVES.iter().any(|n| n.key() == *name),
                "{name} is lent but not described"
            );
        }
    }

    #[test]
    fn the_outline_finds_signatures_over_lines_and_fields() {
        let source = "pub trait T {\n    /// Doc.\n    fn a(&self) -> u32;\n    fn b(\n        &mut self,\n        x: f32,\n    ) {}\n}\nfn outside() {}\n";
        assert_eq!(
            methods_in(source, "pub trait T {"),
            ["fn a(&self) -> u32", "fn b(&mut self, x: f32)"]
        );
        assert!(methods_in(source, "pub trait Missing {").is_empty());
        let kit = "component A {\n    has {\n        float speed = 2;\n        -- a note\n    }\n}\ncomponent B { }\n";
        assert_eq!(
            kit_outline(kit),
            ["component A", "A.float speed = 2", "component B"]
        );
    }
}
