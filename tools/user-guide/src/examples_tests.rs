use super::*;
use crate::blocks::parse;

const THREE: &str = "```lust\ncomponent A {}\n```\n\n```rust\nstruct A;\n```\n\n```lua\nlocal A = component(\"A\", {})\n```\n";

#[test]
fn three_blocks_in_order_are_one_example() {
    let blocks = parse(THREE);
    assert_eq!(
        three_at(&blocks, 0),
        Some([
            "component A {}",
            "struct A;",
            "local A = component(\"A\", {})"
        ])
    );
    assert_eq!(three_at(&blocks, 1), None);
    let found = on_page("p", &blocks);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].page, "p");
}

#[test]
fn another_order_or_a_missing_language_is_not_an_example() {
    let swapped = "```lust\na\n```\n```lua\nb\n```\n```rust\nc\n```\n";
    assert_eq!(three_at(&parse(swapped), 0), None);
    let two = "```lust\na\n```\n```rust\nb\n```\n";
    assert!(on_page("p", &parse(two)).is_empty());
}

#[test]
fn examples_inside_steps_and_boxes_are_found() {
    let indented: String = THREE.lines().map(|l| format!("   {l}\n")).collect();
    let page = format!(
        "1. Write it:\n\n{indented}\n2. Save.\n\n> **Good to know:**\n>\n{}",
        THREE
            .lines()
            .map(|l| format!("> {l}\n"))
            .collect::<String>()
    );
    assert_eq!(on_page("p", &parse(&page)).len(), 2);
}

#[test]
fn untagged_lust_is_noticed_and_other_code_is_not() {
    assert!(looks_like_lust("fetch \"solis/world\":World;\nend"));
    assert!(looks_like_lust("component Mover {\n}"));
    assert!(!looks_like_lust("solis --check-project \"C:\\\\Games\""));
    assert!(!looks_like_lust("area version 3;\nentity 2B8 {\n}"));
}

#[test]
fn rust_brackets_are_counted_outside_strings_characters_and_comments() {
    assert_eq!(
        brackets_balance("fn f(cx: &mut Context<'_>) { let s = \"}\"; let c = '{'; }"),
        Ok(())
    );
    assert_eq!(
        brackets_balance("// } in a comment\nfn f() {}\n/* { */"),
        Ok(())
    );
    assert_eq!(
        brackets_balance("let n = '\\n'; let v = vec![(1, 2)];"),
        Ok(())
    );
    let open = brackets_balance("fn f() {\n    g(\n}").unwrap_err();
    assert!(
        open.contains("line 3") && open.contains("`}` closes the `(`"),
        "{open}"
    );
    let left = brackets_balance("fn f() {\n").unwrap_err();
    assert!(
        left.contains("line 1") && left.contains("isn't closed"),
        "{left}"
    );
    assert!(
        brackets_balance("}")
            .unwrap_err()
            .contains("closes nothing")
    );
}

const INTERFACE_TEXT: &str = "solis.World:say\nsolis.entity.position\nsolis.send\n\
    function solis.send(to, name, ...)\n\
    Host: fn say(&mut self, words: &str)\nContext: pub fn me(&self) -> EntityId\n\
    Context: pub fn send(&mut self, to: Option<EntityId>, name: &str, values: Vec<Value>) -> bool\n";

fn example(rust: &str, lua: &str) -> Example {
    Example {
        page: "p".into(),
        code: [String::new(), rust.into(), lua.into()],
    }
}

#[test]
fn calls_game_code_can_make_pass() {
    let good = example(
        "let me = cx.me();\ncx.host().say(\"hi\");\ncx.send(Some(me), \"Hit\", vec![]);",
        "solis.World:say(solis.entity.position(self.me).x)\nsolis.send(self.me, \"Hit\")\nlocal World = solis.World",
    );
    assert_eq!(unknown_calls(&good, INTERFACE_TEXT), Vec::<String>::new());
}

#[test]
fn calls_game_code_cant_make_are_named() {
    let bad = example(
        "cx.host().teleport(me);\ncx.shout();",
        "solis.World:shout(\"x\")\nsolis.entity.fly(self.me)",
    );
    assert_eq!(
        unknown_calls(&bad, INTERFACE_TEXT),
        [
            "Lua's solis.World:shout",
            "Lua's solis.entity.fly",
            "Rust's Context::shout",
            "Rust's Host::teleport"
        ]
    );
}
