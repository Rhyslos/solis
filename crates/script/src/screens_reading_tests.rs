//! Reading a view's tree within its budget: the tree is game code's, and
//! it's read after the view's time limit has ended, so however it's made,
//! what's copied out of it stays within what one view may hold.

use super::*;
use crate::code::{Code, compile};
use proptest::prelude::*;

/// The screens of a game whose only screen is `screen`, in `ui/wall.lst`.
fn wall(screen: &str) -> Screens {
    let code = Code {
        files: vec![
            ("main.lst".into(), "end\n".into()),
            ("ui/wall.lst".into(), screen.into()),
        ],
        ..Code::default()
    };
    let compiled = compile(&code).unwrap().unwrap();
    Screens::start(compiled.screens.as_ref().unwrap()).unwrap()
}

/// A screen of `pieces` texts, each with a tip of 65,536 letters.
fn tips(pieces: usize) -> String {
    format!(
        "assembly Wall {{\n    view {{\n        string words = \"0123456789abcdef\";\n        \
         loop 12 {{\n            words = words + words;\n        }}\n        column {{\n            \
         loop {pieces} {{\n                text \"x\" [tip words];\n            }}\n        }}\n    \
         }}\n}}\n\npass Wall;\nend\n"
    )
}

/// The view's problem; a view read whole isn't printed (it can be huge).
fn refused(screens: &Screens) -> CodeProblem {
    match screens.view("Wall", None, &mut NoReader) {
        Err(problem) => problem,
        Ok(_) => panic!("the view was read whole"),
    }
}

#[test]
fn a_style_s_words_are_cut_as_a_piece_s_are() {
    let view = wall(&tips(1)).view("Wall", None, &mut NoReader).unwrap();
    let text = &view.children[0].children[0];
    assert_eq!(text.text, "x");
    let tip = text.styles.iter().find(|(name, _)| name == "tip");
    let Some((_, Style::Text(tip))) = tip else {
        panic!("the text has no tip");
    };
    assert_eq!(tip.len(), MAX_WORDS, "cut to what a piece's words are");
}

#[test]
fn a_view_with_more_text_than_its_budget_is_refused_and_says_so() {
    // 300 pieces with 16 KB of tip each (after the cut): more than 4 MB.
    let screens = wall(&tips(300));
    let problem = refused(&screens);
    assert!(problem.message.contains("too much text"), "{problem}");
    assert!(problem.message.contains("4 MB"), "{problem}");
    assert_eq!(problem.file, "ui/wall.lst");
    // Fewer fit, and are read whole.
    let view = wall(&tips(100)).view("Wall", None, &mut NoReader).unwrap();
    assert_eq!(view.children[0].children.len(), 100);
}

#[test]
fn a_refused_view_keeps_the_buttons_of_the_last_one_read() {
    // The second time round the view holds too much: its last picture
    // stays up, so the buttons it showed must still be there to press.
    let screen = tips(300).replace(
        "column {",
        "is turns > 0? then {\n            words = \"\";\n        }\n        turns = turns - 1;\n        \
         button \"Go\" {\n            turns = 5;\n        }\n        column {",
    );
    let screen = screen.replace(
        "    view {",
        "    has {\n        int turns = 1;\n    }\n\n    view {",
    );
    let screens = wall(&screen);
    let view = screens.view("Wall", None, &mut NoReader).unwrap();
    assert_eq!(view.children[0].action, Some(0));
    refused(&screens);
    assert_eq!(screens.kept_buttons(), 1);
    assert!(screens.press("Wall", None, 0, &mut NoReader).is_ok());
    // The button gave it turns again: the next view is read, as before.
    assert!(screens.view("Wall", None, &mut NoReader).is_ok());
}

/// The tree a piece of Luau gives back, read with `reading`.
fn read_with(luau: &str, mut reading: Reading) -> (Result<Node, CodeProblem>, Reading) {
    let lua = mlua::Lua::new();
    let tree: Table = lua.load(luau).eval().unwrap();
    let node = read_node(&tree, 0, &mut reading);
    (node, reading)
}

/// The tree a piece of Luau gives back, read as a view's is.
fn read(luau: &str) -> Result<Node, CodeProblem> {
    read_with(luau, Reading::whole()).0
}

/// What a tree read holds: its pieces, its styles (those inside others
/// too) and the bytes of its text.
#[derive(Debug, Default, PartialEq)]
struct Held {
    pieces: usize,
    styles: usize,
    bytes: usize,
}

fn held_by_styles(styles: &[(String, Style)], held: &mut Held) {
    for (name, style) in styles {
        held.styles += 1;
        held.bytes += name.len();
        match style {
            Style::Text(words) => held.bytes += words.len(),
            Style::Nested(inner) => held_by_styles(inner, held),
            _ => {}
        }
    }
}

fn held_by(node: &Node, held: &mut Held) {
    held.pieces += 1;
    held.bytes += node.kind.len() + node.text.len();
    held_by_styles(&node.styles, held);
    for child in &node.children {
        held_by(child, held);
    }
}

fn held(node: &Node) -> Held {
    let mut held = Held::default();
    held_by(node, &mut held);
    held
}

#[test]
fn style_lists_shared_between_styles_can_t_multiply_what_s_read() {
    // One list of 256 styles, used by each of 256 styles, each of those
    // used by 256 more: nearly 17 million styles on one piece, from three
    // small tables.
    let problem = read(
        "local leaf, mid, top = {}, {}, {}\n\
         for i = 1, 256 do leaf[i] = {'gap', i} end\n\
         for i = 1, 256 do mid[i] = {'hover', leaf} end\n\
         for i = 1, 256 do top[i] = {'hover', mid} end\n\
         return {k = 'view', t = '', s = top, c = {}}",
    )
    .map(|_| "read whole")
    .unwrap_err();
    assert!(problem.message.contains("too many styles"), "{problem}");
    assert!(problem.message.contains("200 thousand"), "{problem}");
}

#[test]
fn one_long_string_on_many_pieces_is_refused_not_copied_for_each() {
    for key in ["t", "k"] {
        let problem = read(&format!(
            "local long = string.rep('x', 8 * 1024 * 1024)\n\
             local pieces = {{}}\n\
             for i = 1, 9000 do pieces[i] = {{k = 'text', {key} = long, c = {{}}}} end\n\
             return {{k = 'view', c = pieces}}"
        ))
        .map(|_| "read whole")
        .unwrap_err();
        assert!(
            problem.message.contains("too much text"),
            "{key}: {problem}"
        );
    }
    // As a style's words and as its name, on one list every piece shares.
    for pair in ["{'tip', long}", "{long, true}"] {
        let made = read(&format!(
            "local long = string.rep('x', 8 * 1024 * 1024)\n\
             local styles = {{}}\n\
             for i = 1, 200 do styles[i] = {pair} end\n\
             local pieces = {{}}\n\
             for i = 1, 9000 do pieces[i] = {{k = 'text', s = styles}} end\n\
             return {{k = 'view', c = pieces}}"
        ));
        let problem = made.map(|_| "read whole").unwrap_err();
        assert!(
            problem.message.contains("too much text"),
            "{pair}: {problem}"
        );
    }
}

#[test]
fn a_style_s_name_is_cut_to_64_letters_before_it_s_copied() {
    let node = read(
        "return {k = 'text', s = {\n\
         {string.rep('n', 1024 * 1024), 1},\n\
         {string.rep('é', 100), 2},\n\
         {string.rep('\\255', 100), 3},\n\
         {'gap', 4}}}",
    )
    .unwrap();
    let names: Vec<&str> = node.styles.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "n".repeat(64).as_str(),
            "é".repeat(64).as_str(),
            "\u{fffd}".repeat(64).as_str(),
            "gap"
        ]
    );
}

#[test]
fn lists_of_anything_but_pieces_and_styles_are_read_no_further() {
    // A million numbers where the styles go, on every piece: only the
    // first 256 of a list are looked at, and each look is counted.
    let problem = read(
        "local junk = table.create(1000000, 7)\n\
         local pieces = {}\n\
         for i = 1, 9000 do pieces[i] = {k = 'text', s = junk} end\n\
         return {k = 'view', c = pieces}",
    )
    .map(|_| "read whole")
    .unwrap_err();
    assert!(problem.message.contains("too many styles"), "{problem}");
    // One such list is no trouble: nothing in it is a style.
    let (node, left) = read_with(
        "return {k = 'text', s = table.create(1000000, 7)}",
        Reading::whole(),
    );
    assert!(node.unwrap().styles.is_empty());
    assert_eq!(left.styles, MAX_STYLES - 256);
    // A million numbers where the pieces go count as pieces: the rest of
    // the view is left out, as pieces past the most are.
    let (node, left) = read_with(
        "local junk = table.create(1000000, 7)\n\
         junk[#junk + 1] = {k = 'text', t = 'never read'}\n\
         return {k = 'view', c = {{k = 'row', c = junk}, {k = 'text', t = 'left out'}}}",
        Reading::whole(),
    );
    assert_eq!(held(&node.unwrap()).pieces, 2, "the view and its row");
    assert_eq!(left.pieces, 0);
}

#[test]
fn tables_that_hold_themselves_are_read_within_the_budget() {
    // Its own styles, its own pair and its own only child: read as deep
    // as a screen goes, and no further.
    let node =
        read("local t = {k = 'row', t = 'me'}\nt.s = t\nt.c = t\nt[1] = t\nreturn t").unwrap();
    let held = held(&node);
    assert_eq!(held.pieces, ui::screen::MAX_DEPTH);
    assert_eq!(held.styles, 0);
    // With 256 of itself: refused at once, however many pieces that is.
    let problem = read(
        "local t = {k = 'row', t = 'me'}\nt.s = t\nt.c = t\n\
         for i = 1, 256 do t[i] = t end\nreturn t",
    )
    .map(|_| "read whole")
    .unwrap_err();
    assert!(problem.message.contains("too many styles"), "{problem}");
}

#[test]
fn a_view_at_its_budget_is_read_and_one_past_it_is_refused() {
    let tree = "return {k = 'row', t = 'four', s = {{'gap', 8}, {'hover', {{'tip', 'six666'}}}}}";
    // 3 + 4 bytes of words, 3 + 5 + 3 + 6 of styles; three styles.
    let exact = |styles, bytes| Reading {
        pieces: 1,
        styles,
        bytes,
        actions: Vec::new(),
    };
    let (node, left) = read_with(tree, exact(3, 24));
    assert_eq!(
        held(&node.unwrap()),
        Held {
            pieces: 1,
            styles: 3,
            bytes: 24
        }
    );
    assert_eq!((left.pieces, left.styles, left.bytes), (0, 0, 0));
    let (node, _) = read_with(tree, exact(2, 24));
    assert!(node.unwrap_err().message.contains("too many styles"));
    let (node, _) = read_with(tree, exact(3, 23));
    assert!(node.unwrap_err().message.contains("too much text"));
}

#[test]
fn pieces_past_the_most_are_left_out_as_before() {
    let node = read(
        "local pieces = {}\n\
         for i = 1, 12000 do pieces[i] = {k = 'text', t = 'x', s = {{'size', i}}} end\n\
         return {k = 'view', c = pieces}",
    )
    .unwrap();
    assert_eq!(held(&node).pieces, MAX_PIECES);
    assert_eq!(node.children.len(), MAX_PIECES - 1);
}

/// A made-up value for a view's tree.
#[derive(Debug, Clone)]
enum Made {
    Nothing,
    Flag(bool),
    Number(f64),
    Words(String),
    /// A table used in many places, which holds itself too.
    Shared,
    /// A table: its list, and what's at some of `k`, `t`, `s`, `c`, `v`.
    Table(Vec<Made>, Vec<(usize, Made)>),
}

fn made() -> impl Strategy<Value = Made> {
    let leaf = prop_oneof![
        Just(Made::Nothing),
        any::<bool>().prop_map(Made::Flag),
        any::<f64>().prop_map(Made::Number),
        "[a-zé]{0,40}".prop_map(Made::Words),
        Just(Made::Shared),
    ];
    leaf.prop_recursive(5, 96, 6, |inner| {
        (
            proptest::collection::vec(inner.clone(), 0..6),
            proptest::collection::vec((0..5usize, inner), 0..5),
        )
            .prop_map(|(list, keys)| Made::Table(list, keys))
    })
}

fn build(lua: &mlua::Lua, made: &Made, shared: &Table) -> LuaValue {
    match made {
        Made::Nothing => LuaValue::Nil,
        Made::Flag(flag) => LuaValue::Boolean(*flag),
        Made::Number(number) => LuaValue::Number(*number),
        Made::Words(words) => LuaValue::String(lua.create_string(words).unwrap()),
        Made::Shared => LuaValue::Table(shared.clone()),
        Made::Table(list, keys) => {
            let table = lua.create_table().unwrap();
            for item in list {
                table.raw_push(build(lua, item, shared)).unwrap();
            }
            for (key, value) in keys {
                let key = ["k", "t", "s", "c", "v"][*key];
                table.raw_set(key, build(lua, value, shared)).unwrap();
            }
            LuaValue::Table(table)
        }
    }
}

proptest! {
    #[test]
    fn made_up_trees_are_read_within_the_budget_or_refused(
        styles in made(),
        pieces in made(),
        budget in (0..40usize, 0..60usize, 0..600usize),
    ) {
        let lua = mlua::Lua::new();
        let shared: Table = lua
            .load("local t = {k = 'row', t = 'shared'}\nt.s = t\nt.c = t\nt[1] = t\nt[2] = {'gap', 'wide'}\nreturn t")
            .eval()
            .unwrap();
        let tree = lua.create_table().unwrap();
        tree.raw_set("k", "view").unwrap();
        tree.raw_set("s", build(&lua, &styles, &shared)).unwrap();
        let children = lua.create_table().unwrap();
        children.raw_push(build(&lua, &pieces, &shared)).unwrap();
        children.raw_push(build(&lua, &styles, &shared)).unwrap();
        tree.raw_set("c", children).unwrap();
        let mut reading = Reading {
            pieces: budget.0,
            styles: budget.1,
            bytes: budget.2,
            actions: Vec::new(),
        };
        match read_node(&tree, 0, &mut reading) {
            Ok(node) => {
                let held = held(&node);
                // The view itself is always read.
                prop_assert!(held.pieces <= budget.0.max(1), "{held:?}");
                prop_assert!(held.styles <= budget.1, "{held:?}");
                prop_assert!(held.bytes <= budget.2, "{held:?}");
            }
            Err(problem) => prop_assert!(
                problem.message.contains("too many styles")
                    || problem.message.contains("too much text"),
                "{problem}"
            ),
        }
    }
}
