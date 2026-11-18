use super::*;

/// Coloured code read back: its text, and each character's class.
fn painted(html: &str) -> (String, Vec<Option<String>>) {
    let mut text = String::new();
    let mut classes = Vec::new();
    let mut open: Option<String> = None;
    let mut rest = html;
    while let Some(c) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("<span class=\"") {
            let end = after.find("\">").unwrap();
            open = Some(after[..end].to_string());
            rest = &after[end + 2..];
        } else if let Some(after) = rest.strip_prefix("</span>") {
            open = None;
            rest = after;
        } else {
            let (plain, used) = [
                ("&amp;", '&'),
                ("&lt;", '<'),
                ("&gt;", '>'),
                ("&quot;", '"'),
            ]
            .iter()
            .find(|(entity, _)| rest.starts_with(entity))
            .map_or((c, c.len_utf8()), |(entity, plain)| (*plain, entity.len()));
            text.push(plain);
            classes.push(open.clone());
            rest = &rest[used..];
        }
    }
    (text, classes)
}

/// The class of the first `token` in `source`, coloured as `language`:
/// `None` when it's plain, `"mixed"` when its letters differ.
fn class_of(language: &str, source: &str, token: &str) -> Option<String> {
    let (text, classes) = painted(&code(language, source));
    assert_eq!(text, source);
    // The first one standing alone: `is` isn't the one in `solis`.
    let alone = |at: usize| {
        let name = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        let first = token.chars().next();
        let last = token.chars().next_back();
        !(name(first) && name(text[..at].chars().next_back()))
            && !(name(last) && name(text[at + token.len()..].chars().next()))
    };
    let at = text
        .match_indices(token)
        .map(|(at, _)| at)
        .find(|at| alone(*at))
        .unwrap_or_else(|| panic!("{token} isn't in {source}"));
    let from = text[..at].chars().count();
    let letters = &classes[from..from + token.chars().count()];
    if letters.iter().all(|c| *c == letters[0]) {
        letters[0].clone()
    } else {
        Some("mixed".to_string())
    }
}

fn check(language: &str, source: &str, expected: &[(&str, Option<&str>)]) {
    for (token, class) in expected {
        assert_eq!(
            class_of(language, source, token).as_deref(),
            *class,
            "{token} in {source:?}"
        );
    }
}

/// Well-formed: only the guide's spans, never nested, never empty, each
/// closed; `&` only in the four entities; no stray `<` or `>`.
fn well_formed(html: &str) -> Result<(), String> {
    let mut open = false;
    let mut rest = html;
    while let Some(c) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("<span class=\"tk-") {
            let ok = after.len() > 3
                && "kftvcnops".contains(&after[..1])
                && after[1..].starts_with("\">");
            if open || !ok {
                return Err(format!("a bad or nested span at {rest:.40?}"));
            }
            if after[3..].starts_with("</span>") {
                return Err("an empty span".to_string());
            }
            open = true;
            rest = &after[3..];
        } else if let Some(after) = rest.strip_prefix("</span>") {
            if !open {
                return Err("a span closed that isn't open".to_string());
            }
            open = false;
            rest = after;
        } else if c == '&' {
            let entity = ["&amp;", "&lt;", "&gt;", "&quot;"]
                .iter()
                .find(|e| rest.starts_with(**e))
                .ok_or_else(|| format!("a bare & at {rest:.20?}"))?;
            rest = &rest[entity.len()..];
        } else if c == '<' || c == '>' {
            return Err(format!("a stray {c} at {rest:.20?}"));
        } else {
            rest = &rest[c.len_utf8()..];
        }
    }
    if open {
        return Err("a span is never closed".to_string());
    }
    Ok(())
}

const K: Option<&str> = Some("tk-k");
const F: Option<&str> = Some("tk-f");
const T: Option<&str> = Some("tk-t");
const V: Option<&str> = Some("tk-v");
const C: Option<&str> = Some("tk-c");
const N: Option<&str> = Some("tk-n");
const O: Option<&str> = Some("tk-o");
const P: Option<&str> = Some("tk-p");
const S: Option<&str> = Some("tk-s");

// ---------- Lust ----------

#[test]
fn lust_keywords_are_structure_and_flow() {
    let source = "fetch \"solis/world\":World;\n\ncomponent Turret {\n    has {\n        float reload = 0;\n    }\n\n    method update(float dt) {\n        is reload <= 0? then {\n            send Hit(this, 2) to target;\n        } or {\n            return;\n        }\n    }\n}\n\npass Turret;\nend";
    check(
        "lust",
        source,
        &[
            ("fetch", K),
            ("component", K),
            ("has", K),
            ("method", K),
            ("pass", K),
            ("end", K),
            ("is", F),
            ("?", F),
            ("then", F),
            ("send", F),
            ("to", F),
            ("or", F),
            ("return", F),
            ("float", T),
            ("reload", None),
            ("update", None),
            ("this", V),
            ("Turret", N),
            ("World", N),
            ("{", P),
            (";", P),
        ],
    );
}

#[test]
fn lust_states_are_marine() {
    check(
        "lust",
        "state Mood = [Calm, Busy [Rushed]];",
        &[("state", S), ("Mood", S), ("Calm", None), ("Rushed", None)],
    );
    // A thing's own state set with = (Lust alpha 74): the option it's set
    // to stays plain, like the options where the state is declared.
    check(
        "lust",
        "state = Mood.Calm;",
        &[("state", S), ("Mood", None), ("Calm", None)],
    );
    check(
        "lust",
        "has {\n    state = Mood.Calm;\n}\nmethod rush() {\n    state=Mood.Busy;\n}",
        &[("state", S), ("Mood", None), ("Calm", None), ("Busy", None)],
    );
    // `is` only asks questions: comparing a state is a question, and its
    // option is the state's.
    check(
        "lust",
        "is state == Mood.Calm? then {",
        &[
            ("is", F),
            ("state", S),
            ("Mood", S),
            ("Calm", S),
            ("?", F),
            ("then", F),
        ],
    );
    // A variable of a state set with =: the state's name is a state, its
    // value an object's state, as anywhere a state is used.
    check(
        "lust",
        "Mood now = Mood.Calm;\nnow = Mood.Busy;",
        &[("now", None), ("Mood", N), ("Busy", S)],
    );
    // A state's value used: the object it's on, then the state.
    check(
        "lust",
        "is Input:pressed(Action.Jump)? then {",
        &[("Input", O), ("pressed", None), ("Action", O), ("Jump", S)],
    );
    check(
        "lust",
        "animate(Clip.Sprites.Farmer.Idle);",
        &[("Clip", O), ("Sprites", S), ("Farmer", S), ("Idle", S)],
    );
}

#[test]
fn lust_classes_and_objects() {
    check(
        "lust",
        "RayHit below = Physics:castRay(position(), [0, 0, -1], 2);\nobject Clock {\n}\nlit = Clock:isNight();\nfloat r = Maths:sqrt(2);\nprint(\"hi\");\nPlayer ada = new Player;",
        &[
            ("RayHit", N),
            ("below", None),
            ("Physics", O),
            ("castRay", None),
            ("object", K),
            ("Clock", O),
            ("isNight", None),
            ("Maths", O),
            ("print", O),
            ("new", K),
            ("Player", N),
        ],
    );
    // After a file's name, the name fetched is itself.
    check(
        "lust",
        "fetch \"solis/input\":Input;\nfetch \"x\":[A, B];",
        &[("Input", N), ("A", N)],
    );
    // A value's member stays plain.
    check(
        "lust",
        "float speed = velocity:length() + gap.x;",
        &[("length", None), ("x", None)],
    );
}

#[test]
fn lust_text_numbers_and_values() {
    check(
        "lust",
        "World:say(\"say \\\"hi\\\" now\" + 2.5 + 1e-3 + 42); x = nothing; y = true;",
        &[
            ("\"say \\\"hi\\\" now\"", V),
            ("2.5", V),
            ("1e-3", V),
            ("42", V),
            ("nothing", V),
            ("true", V),
            ("+", None),
        ],
    );
    // Text that isn't closed stops at the line's end.
    check("lust", "x = \"open\ny = 1;", &[("y", None), ("1", V)]);
}

#[test]
fn lust_comments_line_and_block() {
    check(
        "lust",
        "x = 1; -- a note\n--[[ over\ntwo lines ]] y = 2;\n// also a note",
        &[
            ("-- a note", C),
            ("--[[ over\ntwo lines ]]", C),
            ("y", None),
            ("2", V),
            ("// also a note", C),
        ],
    );
    check("lust", "--[[ never closed\nis", &[("is", C)]);
}

#[test]
fn lust_view_pieces_and_style_words() {
    let source = "assembly PauseMenu {\n    view {\n        stack [width fill, height fill, center] {\n            text \"Paused\" [size 40, bold];\n            button \"Leave\" [width 240, hover:[background red7]] { send Quit() to Ui:entity(); }\n        }\n    }\n}";
    check(
        "lust",
        source,
        &[
            ("assembly", K),
            ("PauseMenu", O),
            ("view", K),
            ("stack", N),
            ("width", T),
            ("fill", None),
            ("height", T),
            ("center", T),
            ("text", N),
            ("size", T),
            ("40", V),
            ("bold", T),
            ("button", N),
            ("hover", T),
            ("background", T),
            ("red7", None),
            ("send", F),
            ("Quit", N),
            ("Ui", O),
        ],
    );
    // Words that name pieces, used otherwise, aren't pieces.
    check(
        "lust",
        "text = name;\nlist.add(1);\nx = [width 2];",
        &[("text", None), ("list", None), ("width", None)],
    );
}

#[test]
fn lust_blocks_after_a_piece_are_flow_but_fields_stay_plain() {
    check(
        "lust",
        "has {\n    float open = 0;\n}\ntween open to 1 over 0.5 [ease OutCubic] {\n}\nopen {\n}",
        &[
            ("open", None),
            ("tween", F),
            ("over", F),
            ("ease", None),
            ("OutCubic", N),
        ],
    );
    let (text, classes) = painted(&lust("x\nopen {\n}"));
    let at = text.find("open").unwrap();
    assert_eq!(classes[at].as_deref(), F);
}

// ---------- Rust ----------

#[test]
fn rust_keywords_types_and_classes() {
    let source = "use script::{Context, RustComponent};\n\npub struct Mover {\n    speed: f64,\n}\n\nimpl RustComponent for Mover {\n    fn update(&mut self, cx: &mut Context<'_>, dt: f64) {\n        if let Some(s) = data.get(\"speed\") {\n            return;\n        }\n        match x { _ => false }\n    }\n}\nconst MAX_SPEED: u32 = 4;";
    check(
        "rust",
        source,
        &[
            ("use", K),
            ("pub", K),
            ("struct", K),
            ("impl", K),
            ("fn", K),
            ("mut", K),
            ("let", K),
            ("const", K),
            ("for", F),
            ("if", F),
            ("return", F),
            ("match", F),
            ("self", F),
            ("f64", T),
            ("u32", T),
            ("Mover", N),
            ("Context", N),
            ("Some", N),
            ("MAX_SPEED", S),
            ("false", V),
            ("update", None),
            ("::", P),
            ("=>", P),
            ("{", P),
        ],
    );
}

#[test]
fn rust_lifetimes_characters_and_macros() {
    check(
        "rust",
        "fn f<'a>(cx: &mut Context<'_>) -> &'static str {\n    let c = 'x';\n    let n = '\\n';\n    let v = vec![1];\n    say(&format!(\"rolled {roll}\"));\n    a != b;\n    x?;\n}",
        &[
            ("'a", V),
            ("'_", V),
            ("'static", V),
            ("'x'", V),
            ("'\\n'", V),
            ("vec!", F),
            ("format!", F),
            ("a != b", None),
            ("->", P),
            ("?", F),
        ],
    );
    check("rust", "let q = '\\", &[("'\\", V)]);
}

#[test]
fn rust_text_numbers_comments_and_attributes() {
    check(
        "rust",
        "#[derive(Debug, Clone)]\n// a note\nlet s = \"a \\\"b\\\"\nc\"; /* x\ny */ let n = 2.0f32 + 0x1F + 1_000 + 2.5;",
        &[
            ("#[derive(Debug, Clone)]", T),
            ("// a note", C),
            ("\"a \\\"b\\\"\nc\"", V),
            ("/* x\ny */", C),
            ("2.0f32", V),
            ("0x1F", V),
            ("1_000", V),
            ("2.5", V),
        ],
    );
}

// ---------- Lua ----------

#[test]
fn lua_keywords_values_and_classes() {
    let source = "local Mover = component(\"Mover\", {\n    { \"speed\", \"float\", 2 },\n})\n\nfunction Mover:update(dt)\n    if self.hp > 0 and not done then\n        return nil\n    elseif x or y then\n    else\n    end\nend";
    check(
        "lua",
        source,
        &[
            ("local", K),
            ("function", K),
            ("end", K),
            ("if", F),
            ("self", F),
            ("and", F),
            ("not", F),
            ("then", F),
            ("return", F),
            ("elseif", F),
            ("or", F),
            ("nil", V),
            ("Mover", N),
            ("update", None),
            ("\"speed\"", V),
            ("2", V),
            (":", P),
            ("(", P),
        ],
    );
}

#[test]
fn lua_text_numbers_and_comments() {
    check(
        "lua",
        "-- a note\nlocal s = 'it\\'s' .. \"a \\\"b\\\"\" .. [[long\ntext]]\n--[[ block\nnote ]] x = 0x1F + 2.5",
        &[
            ("-- a note", C),
            ("'it\\'s'", V),
            ("..", None),
            ("\"a \\\"b\\\"\"", V),
            ("[[long\ntext]]", V),
            ("--[[ block\nnote ]]", C),
            ("x", None),
            ("0x1F", V),
            ("2.5", V),
        ],
    );
}

// ---------- every language ----------

#[test]
fn other_code_stays_plain_and_safe() {
    assert_eq!(code("", "<b> & \"x\""), "&lt;b&gt; &amp; &quot;x&quot;");
    assert_eq!(code("text", "-- not a comment"), "-- not a comment");
}

#[test]
fn tokens_of_one_colour_share_a_span() {
    assert_eq!(lust("x();"), "x<span class=\"tk-p\">();</span>");
    assert_eq!(
        lust("is a? then"),
        "<span class=\"tk-f\">is</span> a<span class=\"tk-f\">? then</span>"
    );
}

#[test]
fn text_of_takes_the_colours_away() {
    let source = "World:say(\"a < b & c\");";
    assert_eq!(text_of(&lust(source)), source);
}

#[test]
fn edge_cases_neither_crash_nor_change_the_text() {
    for source in [
        "", "\n", " ", "\"", "'", "--", "--[[", "]]", "/*", "#[", "#![", "'\\", "é", "é'", "'é'",
        "\"\\", "\"\\\n", "1e", "1e+", "1.", "0x", "a!", "=>", "::", "..", "...", "[[", "\t\t",
        "style", "state", "state is", "state =", "state ==", "view [", "[[]]]]", "}}}]]]", "\r\n",
    ] {
        for language in ["lust", "rust", "lua", "text"] {
            let html = code(language, source);
            assert_eq!(text_of(&html), source, "{language}: {source:?}");
            assert_eq!(well_formed(&html), Ok(()), "{language}: {source:?}");
        }
    }
}

/// A tiny random number source, the same every run.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Thousands of random pieces of code: each comes out well-formed, with
/// the same text (rule 9).
#[test]
fn any_code_comes_out_well_formed_with_its_text() {
    let pieces = [
        "component",
        "has",
        "state",
        "is",
        "view",
        "column",
        "text",
        "[",
        "]",
        "{",
        "}",
        "(",
        ")",
        "\"",
        "'",
        "\\",
        "--",
        "--[[",
        "]]",
        "//",
        "/*",
        "*/",
        "#[",
        "!",
        "?",
        ":",
        ".",
        "..",
        ";",
        ",",
        "=",
        "=>",
        "::",
        "->",
        "<",
        ">",
        "&",
        "&amp;",
        "<span>",
        "1",
        "2.5",
        "1e-3",
        "0x1F",
        "'_",
        "'a'",
        "Action",
        "Jump",
        "World",
        "self",
        "fn",
        "local",
        "end",
        "é",
        "日本",
        "\n",
        " ",
        "\t",
        "\r",
        "hover",
        "open",
        "style",
        "[[",
        "MAX",
    ];
    let mut dice = Dice(0x00c0_10a5_2026_1114);
    for _ in 0..4_000 {
        let length = 1 + dice.next() % 60;
        let source: String = (0..length)
            .map(|_| pieces[(dice.next() as usize) % pieces.len()])
            .collect();
        for language in ["lust", "rust", "lua", ""] {
            let html = code(language, &source);
            assert_eq!(text_of(&html), source, "{language}: {source:?}");
            if let Err(why) = well_formed(&html) {
                panic!("{language}: {source:?}: {why}");
            }
        }
    }
}
