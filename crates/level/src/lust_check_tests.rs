use super::*;
use crate::lust_data::{Fixed, field_float, field_ints, field_option, field_text, read, things};

const CLASSES: &str = "-- What a test file holds.\n\
    state Kind = [Number, Switch];\n\
    class Part {\n    has {\n        string id = \"\";\n        int array at = [];\n    }\n}\n\
    class Item {\n    has {\n        string name = \"\";\n        float weight = 0;\n        \
    int count = 0;\n        bool on = false;\n        Kind kind = Kind.Number;\n        \
    int array array cells = [];\n        string array rows = [];\n        \
    float array colour = [];\n        Part part = new Part {};\n        \
    Part array parts = [];\n    }\n}\n\
    pass Kind, Part, Item;\nend\n";

/// A file's first four lines: what follows starts on line 5.
const HEAD: &str =
    "-- A test.\nfetch \"solis/data/test\":[Kind, Part, Item];\n\nlock int version = 2;\n";

fn check(text: &str) -> Result<(), DataError> {
    Classes::read("test", CLASSES)
        .expect("the test's classes")
        .check(text)
}

/// The mistake in a file of `body` after [`HEAD`].
fn refused(body: &str) -> DataError {
    check(&format!("{HEAD}{body}\nend\n")).expect_err(body)
}

fn read_test(text: &str) -> Result<crate::lust_data::Values, DataError> {
    read(text, "test", CLASSES, 64 << 20)
}

/// How many mistakes Lust itself finds in `text`.
fn found_by_lust(text: &str) -> usize {
    let files = vec![
        ("data.lst".to_string(), text.to_string()),
        ("solis/data/test.lst".to_string(), CLASSES.to_string()),
    ];
    let engine = lust::Engine {
        folders: vec!["solis".into()],
    };
    lust::read_values_with(&engine, "data.lst", &files)
        .map_or_else(|problems| problems.len(), |_| 0)
}

#[test]
fn what_a_data_file_holds_is_let_through() {
    // Every shape Solis writes, and the ways a hand may lay them out:
    // comments of each kind, values over several lines, a `,` after the
    // last item, fields in any order or left out.
    let text = "-- A test.\n// Another comment.\n--[[ A block\ncomment. ]]\n\
        fetch \"solis/data/test\":[Kind, Part, Item,];\nfetch \"solis/data/test\":Kind;\n\n\
        lock int version = 2;\n\
        lock Item item = new Item {\n    \
            name = \"a \\\"b\\\" \\{c\\} \\\\ \\n\\t { d } [0-9]{4} }\", -- plain braces\n    \
            weight = -2.5e-7, count = -3, on = true, kind = Kind . Switch,\n    \
            cells = [[4, -1], [], [0, 0, 0,],], rows = [\"....\", \"\"],\n    \
            colour = [0, 1, 0.5, 1e3, 2E+2,],\n    \
            part = new Part { id = \"p\", at = [1] },\n    \
            parts = [new Part {}, new Part { at = [], id = \"q\", },],\n\
        };\n\
        lock Item array items = [];\n\
        lock Kind array kinds = [Kind.Number, Kind.Switch];\n\
        lock string title = \"é ü\";\n\
        lock float array array grid = [[1, 2.5], []];\n\
        lock bool on = false;\n\
        lock Part part = new Part {};\n\
        lock float big = 9223372036854775807;\n\
        end\n";
    for text in [text.to_string(), text.replace('\n', "\r\n")] {
        check(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        let values = read_test(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        let item = values.get("item").expect("the item");
        assert_eq!(
            field_text(item, "name"),
            "a \"b\" {c} \\ \n\t { d } [0-9]{4} }"
        );
        assert_eq!(field_float(item, "weight"), Some(-2.5e-7));
        assert_eq!(field_option(item, "kind"), Some("Switch"));
        let cells = item.field("cells").and_then(Fixed::as_list).expect("cells");
        assert_eq!(cells.len(), 3);
        let parts = item.field("parts").and_then(Fixed::as_list).expect("parts");
        assert_eq!(field_text(&parts[1], "id"), "q");
        assert_eq!(field_ints(&parts[1], "at"), Some(vec![]));
        assert_eq!(things(&values, "kinds").len(), 2);
        assert_eq!(values.values.len(), 9);
    }
    // A file with only its version, and one without its fetch line.
    assert!(read_test("lock int version = 1;\nend\n").is_ok());
    assert!(read_test("-- Nothing at all.\nend\n").is_ok());
    // One without its end is Lust's to refuse, in its own words.
    let open = format!("{HEAD}lock bool on = true;\n");
    assert_eq!(check(&open), Ok(()));
    let why = read_test(&open).unwrap_err();
    assert!(why.message.contains("ends with the word end"), "{why}");
}

#[test]
fn anything_else_is_refused_with_its_line() {
    for (body, line, why) in [
        // Not a value at all.
        ("x = 5;", 5, "only lock values"),
        ("pass version;", 5, "only lock values"),
        ("state S = [A];", 5, "only lock values"),
        ("method m() {}", 5, "only lock values"),
        ("@", 5, "only lock values"),
        ("\n;", 6, "only lock values"),
        ("fetch \"solis/data/test\":Kind;", 5, "only lock values"),
        (
            "lock int a = 1\nlock int b = 2;",
            6,
            "`;` goes here, not the word lock",
        ),
        ("lock int a = 1", 6, "`;` goes here, not the word end"),
        (
            "lock int array = [1];",
            5,
            "the value's name goes here, not `=`",
        ),
        ("lock int a 1;", 5, "`=` goes here, not 1"),
        (
            "lock a = 1;",
            5,
            "a isn't a state or class this file fetches",
        ),
        ("lock Nope a = 1;", 5, "Nope isn't a state or class"),
        (
            "lock int Kind = 1;",
            5,
            "Kind is one of this file's states and classes",
        ),
        (
            "lock int a = 1;\nlock bool a = true;",
            6,
            "a second value called a",
        ),
        ("lock int version = 3;", 5, "a second value called version"),
        // A value that isn't of its type.
        ("lock int a = 1.5;", 5, "a whole number goes here, not 1.5"),
        ("lock int a = 1e3;", 5, "a whole number goes here, not 1e3"),
        (
            "lock int a = \"1\";",
            5,
            "a whole number goes here, not text",
        ),
        (
            "lock int a = x;",
            5,
            "a whole number goes here, not the word x",
        ),
        ("lock int a = nothing;", 5, "not the word nothing"),
        ("lock int a = [1];", 5, "a whole number goes here, not `[`"),
        (
            "lock float a = true;",
            5,
            "a number goes here, not the word true",
        ),
        ("lock string a = 5;", 5, "text in quotes goes here, not 5"),
        (
            "lock string a = -\"x\";",
            5,
            "text in quotes goes here, not `-`",
        ),
        ("lock bool a = 1;", 5, "true or false goes here, not 1"),
        (
            "lock bool a = True;",
            5,
            "true or false goes here, not the word True",
        ),
        ("lock int array a = 5;", 5, "a list in [ ] goes here, not 5"),
        (
            "lock int array a = [1, [2]];",
            5,
            "a whole number goes here, not `[`",
        ),
        (
            "lock int array array a = [1];",
            5,
            "a list in [ ] goes here, not 1",
        ),
        (
            "lock int array a = [1 2];",
            5,
            "`,` or `]` goes here, not 2",
        ),
        (
            "lock int array a = [,];",
            5,
            "a whole number goes here, not `,`",
        ),
        (
            "lock int array a = [1,, 2];",
            5,
            "a whole number goes here, not `,`",
        ),
        (
            "lock int array a = [\n1,\n2",
            8,
            "`,` or `]` goes here, not the word end",
        ),
        // Anything that would run.
        ("lock int a = 1 + 2;", 5, "`;` goes here, not `+`"),
        ("lock int a = (1);", 5, "a whole number goes here, not `(`"),
        ("lock int a = - -1;", 5, "a whole number goes here, not `-`"),
        (
            "lock int a = #\"x\";",
            5,
            "a whole number goes here, not `#`",
        ),
        ("lock int a = Maths:sqrt(2);", 5, "not the word Maths"),
        ("lock int array a = [1][0];", 5, "`;` goes here, not `[`"),
        (
            "lock string a = \"x\".upper();",
            5,
            "`;` goes here, not `.`",
        ),
        (
            "lock bool a = not true;",
            5,
            "true or false goes here, not the word not",
        ),
        ("lock bool a = 1 == 1;", 5, "true or false goes here, not 1"),
        (
            "lock string a = \"hp: {hp}\";",
            5,
            "not text with a value in { }",
        ),
        // Numbers Lust can't hold.
        (
            "lock int a = 9223372036854775808;",
            5,
            "9223372036854775808 is too big for a whole",
        ),
        (
            "lock float a = 99999999999999999999;",
            5,
            "is too big for a whole number",
        ),
        ("lock float a = 1e999;", 5, "1e999 is too big for a number"),
        // Text Lust would find a mistake in.
        (
            "lock string a = \"a\\qb\";",
            5,
            "a \\ in text starts one of",
        ),
        (
            "lock string a = \"a\\\nb\";",
            5,
            "a \\ in text starts one of",
        ),
        ("lock string a = \"open;", 5, "not text that's never closed"),
        (
            "lock string a = 'x';",
            5,
            "text in quotes goes here, not `'`",
        ),
        // States and things.
        (
            "lock Kind a = Kind.Nope;",
            5,
            "(Kind.Number, Kind.Switch) goes here, not the word Nope",
        ),
        ("lock Kind a = Switch;", 5, "one of Kind's options"),
        ("lock Kind a = Kind;", 5, "`.` goes here, not `;`"),
        ("lock Kind a = \"Switch\";", 5, "one of Kind's options"),
        (
            "lock Item a = new Part {};",
            5,
            "new Item { … } goes here, not the word Part",
        ),
        (
            "lock Item a = Item {};",
            5,
            "new Item { … } goes here, not the word Item",
        ),
        ("lock Item a = new Item;", 5, "`{` goes here, not `;`"),
        ("lock Item a = new Item();", 5, "`{` goes here, not `(`"),
        (
            "lock Item a = nothing;",
            5,
            "new Item { … } goes here, not the word nothing",
        ),
        (
            "lock Item a = new Item { nope = 1 };",
            5,
            "Item has no field called nope: its fields are name, weight",
        ),
        (
            "lock Item a = new Item {\nname = \"a\",\nname = \"b\" };",
            7,
            "name is already filled in here",
        ),
        (
            "lock Item a = new Item { name = \"a\"; count = 1 };",
            5,
            "`,` or `}` goes here, not `;`",
        ),
        (
            "lock Item a = new Item { name \"a\" };",
            5,
            "`=` goes here, not text",
        ),
        (
            "lock Item a = new Item { count = 1.5 };",
            5,
            "a whole number goes here, not 1.5",
        ),
        (
            "lock Item a = new Item { part = new Item {} };",
            5,
            "new Part { … } goes here",
        ),
        (
            "lock Item a = new Item { parts = [new Part { at = [true] }] };",
            5,
            "not the word true",
        ),
        (
            "lock Item array a = [new Item {},\nnew Item { on = 1 }];",
            6,
            "true or false goes here",
        ),
        // After the end.
        ("end\nlock int a = 1;", 6, "nothing can come after end"),
        ("end;", 5, "nothing can come after end"),
    ] {
        let found = refused(body);
        assert!(found.message.contains(why), "{body:?}: {found}");
        assert_eq!(found.line, line, "{body:?}: {found}");
    }
    // A file fetches only its own classes, and uses only what it fetches.
    for (fetch, why) in [
        (
            "fetch \"solis/project\":Picture;",
            "fetches only its own classes",
        ),
        (
            "fetch \"solis/data/test.lst\":Kind;",
            "fetches only its own classes",
        ),
        (
            "fetch \"solis/data/{test}\":Kind;",
            "fetches only its own classes",
        ),
        ("fetch lua;", "fetches only its own classes"),
        ("get \"solis/data/test\":Kind;", "only lock values"),
        (
            "fetch \"solis/data/test\":[Kind, Nope];",
            "solis/data/test has no Nope",
        ),
        (
            "fetch \"solis/data/test\":Kind, Item;",
            "`;` goes here, not `,`",
        ),
        (
            "fetch \"solis/data/test\" [Kind];",
            "`:` goes here, not `[`",
        ),
    ] {
        let found = check(&format!("{fetch}\nlock int version = 2;\nend\n")).unwrap_err();
        assert!(found.message.contains(why), "{fetch:?}: {found}");
        assert_eq!(found.line, 1, "{fetch:?}");
    }
    for (body, name) in [
        ("lock Kind a = Kind.Switch;", "Kind"),
        ("lock Item a = new Item { kind = Kind.Switch };", "Kind"),
        ("lock Item a = new Item { part = new Part {} };", "Part"),
        ("lock Item a = new Item { parts = [new Part {}] };", "Part"),
    ] {
        let found = check(&format!("fetch \"solis/data/test\":Item;\n{body}\nend\n")).unwrap_err();
        let why = format!("{name} isn't a state or class this file fetches from solis/data/test");
        assert_eq!(
            (found.line, found.message.as_str()),
            (2, why.as_str()),
            "{body:?}"
        );
    }
    // However long the word, the mistake stays short.
    let long = "x".repeat(100_000);
    for body in [
        format!("lock int a = {long};"),
        format!("lock int a = 1{long};"),
        format!("lock int a = {};", "9".repeat(100_000)),
        format!("lock float a = {}.5;", "9".repeat(100_000)),
        format!("lock {long} a = 1;"),
        format!("lock int a = 1;\nlock int {long} = 1;\nlock int {long} = 1;"),
        format!("lock int _{long} = 1;"),
        format!("lock Item a = new Item {{ {long} = 1 }};"),
        format!("lock Kind a = Kind.{long};"),
        long.clone(),
    ] {
        let found = refused(&body);
        assert!(found.message.len() < 300, "{found}");
    }
}

#[test]
fn a_name_lust_keeps_for_itself_cant_name_a_value() {
    // Lust stops reading a value at such a name and picks up inside it,
    // finding a mistake in every thing of its list: so the name is asked
    // about on its own first.
    let with = |name: &str| {
        let things = "    new Item { name = \"x\" },\n".repeat(50);
        check(&format!(
            "{HEAD}\nlock Item array {name} = [\n{things}];\nend\n"
        ))
    };
    for name in [
        "end", "lock", "loop", "state", "new", "int", "map", "is", "check", "when", "pass",
        "fetch", "class", "true", "nothing", "lua", "native", "print", "wait", "ask", "Random",
        "Items", "ITEMS", "_items",
    ] {
        let found = with(name).expect_err(name);
        assert!(
            found.message.starts_with("a value can't have this name: "),
            "{name}: {found}"
        );
        assert_eq!(found.line, 6, "{name}: {found}");
    }
    for name in [
        "items", "tileMap", "a1", "crops_2", "é", "ends", "locked", "text", "value",
    ] {
        assert_eq!(with(name), Ok(()), "{name}");
    }
}

#[test]
fn a_file_has_so_many_values_and_goes_so_deep() {
    let values = |count: usize| {
        let locks: String = (0..count)
            .map(|i| format!("lock int a{i} = {i};\n"))
            .collect();
        format!("fetch \"solis/data/test\":Kind;\n{locks}end\n")
    };
    assert_eq!(
        read_test(&values(MAX_VALUES - 1)).map(|v| v.values.len()),
        Ok(MAX_VALUES - 1)
    );
    let found = check(&values(MAX_VALUES)).unwrap_err();
    assert!(
        found
            .message
            .contains("more than 64 fetch lines and values"),
        "{found}"
    );
    assert_eq!(found.line, MAX_VALUES + 1);
    // Lists as deep as they go are read by Lust too.
    let lists = |depth: usize| {
        format!(
            "lock int{} deep = {}7{};\nend\n",
            " array".repeat(depth),
            "[".repeat(depth),
            "]".repeat(depth)
        )
    };
    let mut deepest = &read_test(&lists(MAX_DEPTH))
        .expect("the deepest list")
        .values[0]
        .value;
    for _ in 0..MAX_DEPTH {
        deepest = &deepest.as_list().expect("a list")[0];
    }
    assert_eq!(deepest.as_int(), Some(7));
    let found = check(&lists(MAX_DEPTH + 1)).unwrap_err();
    assert!(found.message.contains("more than 32 deep"), "{found}");
    // An empty list of a type that deep is as much a mistake.
    let empty = format!(
        "lock int{} deep = [];\nend\n",
        " array".repeat(MAX_DEPTH + 1)
    );
    assert!(
        check(&empty)
            .unwrap_err()
            .message
            .contains("more than 32 deep")
    );
    // Things inside things can't go deeper than their classes do.
    let found = refused("lock Item a = new Item { part = new Part { at = [[1]] } };");
    assert!(
        found.message.contains("a whole number goes here, not `[`"),
        "{found}"
    );
}

#[test]
fn a_file_has_so_many_pieces() {
    // The code review: Lust keeps about 100 bytes for each piece of a file
    // it reads, whatever the piece, so a file of short numbers as long as
    // a tile map's may be (64 MB) asked it for gigabytes. A file with more
    // pieces than Lust can be handed is refused before it is, as the whole
    // file, by the reader every data file goes through: were this one
    // handed to Lust, the test would take a gigabyte and several seconds.
    let numbers = "1,".repeat(MAX_PIECES / 2);
    let read = read_test(&format!("lock int array a = [{numbers}];\nend\n"));
    let found = read.map(drop).unwrap_err();
    assert_eq!(found.line, 0);
    assert_eq!(
        found.message,
        "it's too big to read: it holds more than 12 million numbers, texts, names and symbols"
    );
}

#[test]
fn what_a_writer_is_told_fits_is_never_too_big_to_read() {
    // The review of Phase 9, BUG-55: a tile map could be saved as a file
    // with more pieces than are read, and never opened again. A writer
    // now asks `fits` first, which keeps the reader's own two limits.
    let count = |text: &str| {
        let mut pieces = Pieces::new(text);
        let mut count = 0usize;
        while pieces.next().is_some() {
            count += 1;
        }
        count
    };
    let around = count(&format!("{HEAD}lock int array a = [];\nend\n"));
    // A file of exactly `pieces` pieces: numbers, a `,` between them.
    let file = |pieces: usize| {
        let inside = pieces - around;
        let mut numbers = "1,".repeat(inside / 2);
        if inside % 2 == 1 {
            numbers.push('1');
        }
        format!("{HEAD}lock int array a = [{numbers}];\nend\n")
    };
    // At the limit to the piece: it fits, and the check lets it through.
    let most = file(MAX_PIECES);
    assert_eq!(count(&most), MAX_PIECES);
    assert_eq!(crate::lust_data::fits(&most, 64 << 20), Ok(()));
    assert_eq!(check(&most), Ok(()));
    // One piece more doesn't fit, in the reader's own words.
    let past = file(MAX_PIECES + 1);
    let refused = crate::lust_data::fits(&past, 64 << 20).unwrap_err();
    assert_eq!(refused, error(0, too_big()));
    // Text no longer than the limit can't hold more pieces than it.
    assert_eq!(within_pieces(&"1,".repeat(MAX_PIECES / 2)), Ok(()));
    // The length, too: just long enough fits; a byte less is refused as
    // the reader refuses it.
    let short = format!("{HEAD}end\n");
    assert_eq!(crate::lust_data::fits(&short, short.len()), Ok(()));
    assert!(read(&short, "test", CLASSES, short.len()).is_ok());
    let refused = crate::lust_data::fits(&short, short.len() - 1).unwrap_err();
    let by_reader = read(&short, "test", CLASSES, short.len() - 1).map(drop);
    assert_eq!(by_reader, Err(refused));
}

#[test]
fn what_lust_takes_long_over_never_reaches_it() {
    // Each of these took Lust (rev 930afae) from seconds to more than a
    // minute to refuse, its time growing with the square of the file's
    // length; none is a data file, so none reaches it. Refused here, each
    // is answered in thousandths of a second, on its first wrong line.
    let lines = |count: usize, line: &str| line.repeat(count);
    let item = |fields: &str| format!("lock Item item = new Item {{ {fields} }};\n");
    for (name, body, line, why) in [
        ("200 KB of @", "@".repeat(200_000), 5, "only lock values"),
        ("100 KB of '", "'".repeat(100_000), 5, "only lock values"),
        (
            "! before a name",
            lines(50_000, "!a "),
            5,
            "only lock values",
        ),
        (
            "25,000 values of something unknown",
            (0..25_000)
                .map(|i| format!("lock int a{i} = x;\n"))
                .collect(),
            5,
            "a whole number goes here, not the word x",
        ),
        (
            "a state of 120,000 options",
            format!("state S = [{}];\n", lines(120_000, "Option, ")),
            5,
            "only lock values",
        ),
        (
            "100,000 unknown escapes in a text",
            item(&format!("name = \"{}\"", lines(100_000, "\\q"))),
            5,
            "a \\ in text starts one of",
        ),
        (
            "300,000 things with a field of the wrong type",
            format!(
                "lock Item array items = [\n{}];\n",
                lines(300_000, "    new Item { name = 5 },\n")
            ),
            6,
            "text in quotes goes here, not 5",
        ),
        (
            "300,000 things of an unknown class",
            format!(
                "lock Item array items = [\n{}];\n",
                lines(300_000, "    new Nope {},\n")
            ),
            6,
            "new Item { … } goes here, not the word Nope",
        ),
        (
            "200,000 texts in a list of numbers",
            item(&format!(
                "rows = [\"a\"],\n cells = [[{}]]",
                lines(200_000, "\"a\", ")
            )),
            6,
            "a whole number goes here, not text",
        ),
        (
            "100,000 numbers too big for Lust",
            item(&format!(
                "cells = [[\n{}]]",
                lines(100_000, "99999999999999999999, ")
            )),
            6,
            "99999999999999999999 is too big for a whole number",
        ),
        (
            "one field filled in 20,000 times",
            item(&format!(
                "name = \"a\",\n{}",
                lines(20_000, "name = \"a\", ")
            )),
            6,
            "name is already filled in here",
        ),
        (
            "a list under a name of Lust's own",
            format!(
                "lock Item array end = [\n{}];\n",
                lines(200_000, "    new Item {},\n")
            ),
            5,
            "a value can't have this name",
        ),
        (
            "a state's name taken by a value",
            format!(
                "lock int Kind = 1;\nlock Kind array kinds = [{}];\n",
                lines(200_000, "Kind.Switch, ")
            ),
            5,
            "Kind is one of this file's states and classes",
        ),
        (
            "things of a class the file didn't fetch",
            format!(
                "lock Piece array pieces = [\n{}];\n",
                lines(200_000, "    new Piece {},\n")
            ),
            5,
            "Piece isn't a state or class this file fetches",
        ),
    ] {
        let found = read_test(&format!("{HEAD}{body}end\n")).expect_err(name);
        assert!(found.message.contains(why), "{name}: {found}");
        assert_eq!(found.line, line, "{name}: {found}");
    }
}

/// Changes up to three places in `pieces`: a piece swapped for another,
/// one added, one taken out, or four in a row said twice.
fn muddle<'a>(pieces: &mut Vec<&'a str>, from: &[&'a str], next: &mut impl FnMut(usize) -> usize) {
    for _ in 0..next(4) {
        let at = next(pieces.len());
        match next(4) {
            0 => pieces[at] = from[next(from.len())],
            1 => pieces.insert(at, from[next(from.len())]),
            2 if pieces.len() > 1 => drop(pieces.remove(at)),
            _ => {
                let run: Vec<&str> = pieces[at..].iter().take(4).copied().collect();
                pieces.splice(at..at, run);
            }
        }
    }
}

#[test]
fn whatever_is_let_through_lust_finds_nothing_wrong_with() {
    // Fuzz-style: thousands of files, each a list of forty things, with a
    // few pieces of the file swapped, added, taken out or said twice, some
    // under a name Lust keeps for itself or without all they use fetched.
    // A mistake Lust would find in each thing (or the run of them a bad
    // name sets off) shows forty times over; none may be let through.
    // Without its end, or with a block comment left open, a file gives
    // Lust one mistake or two, and nothing else does.
    const PIECES: [&str; 64] = [
        "new",
        "Item",
        "Part",
        "{",
        "}",
        "[",
        "]",
        ",",
        ";",
        "=",
        ".",
        ":",
        "-",
        "name",
        "weight",
        "count",
        "on",
        "kind",
        "cells",
        "rows",
        "colour",
        "part",
        "parts",
        "id",
        "at",
        "Kind",
        "Switch",
        "Nope",
        "true",
        "false",
        "nothing",
        "\"x\"",
        "5",
        "1.5",
        "2e3",
        "x",
        "end",
        "is",
        "state",
        "(",
        "fetch",
        "lock",
        "int",
        "array",
        "version",
        "items",
        "@",
        "'",
        "!",
        "\"a{b}\"",
        "\"\\q\"",
        "\"open",
        "99999999999999999999",
        "1e999",
        "--[[",
        "]]",
        "-- ",
        "\n",
        "string",
        "float",
        "bool",
        "\"solis/data/test\"",
        "print",
        "Items",
    ];
    const THINGS: [&str; 9] = [
        "Item : new Item { name = \"x\" , weight = 1.5 , count = 5 , on = true , kind = Kind . Switch }",
        "Item : new Item { cells = [ [ 4 , - 1 ] , [ ] ] , rows = [ \"x\" , ] , colour = [ 0 , 1.5 ] }",
        "Item : new Item { part = new Part { id = \"x\" , at = [ 5 ] } , parts = [ new Part { } ] }",
        "Part : new Part { id = \"x\" , at = [ 5 , 5 ] , }",
        "int : - 5",
        "float : 2e3",
        "Kind : Kind . Switch",
        "string : \"x\"",
        "int array : [ 5 , - 5 ]",
    ];
    const NAMES: [&str; 16] = [
        "end", "lock", "loop", "state", "new", "map", "print", "wait", "Items", "ITEMS", "_items",
        "version", "Kind", "Item", "Part", "lua",
    ];
    const FETCHES: [&str; 4] = ["Item", "Kind , Item", "Part , Item , Part", "Kind"];
    let mut seed = 0x5EC2_1000_D00D_F00Du64;
    let mut next = move |below: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 24) as usize % below
    };
    let (mut through, mut clean) = (0, 0);
    for case in 0..16_000 {
        let (ty, thing) = THINGS[case % THINGS.len()]
            .split_once(" : ")
            .expect("a type");
        let (mut fetch, mut name) = ("Kind , Part , Item", "items");
        match next(8) {
            0 => fetch = FETCHES[next(FETCHES.len())],
            1 => name = NAMES[next(NAMES.len())],
            _ => {}
        }
        let head = format!(
            "fetch \"solis/data/test\" : [ {fetch} ] ; \n lock int version = 2 ; \n \
             lock {ty} array {name} = [ \n"
        );
        let mut head: Vec<&str> = head.split(' ').collect();
        let mut thing: Vec<&str> = thing.split(' ').collect();
        let mut tail = vec!["]", ";", "\n", "end", "\n"];
        match next(4) {
            0 => muddle(&mut head, &PIECES, &mut next),
            1 | 2 => muddle(&mut thing, &PIECES, &mut next),
            _ => muddle(&mut tail, &PIECES, &mut next),
        }
        let things = format!("    {} ,\n", thing.join(" ")).repeat(40);
        let text = format!("{}{things}{}", head.join(" "), tail.join(" "));
        if check(&text).is_err() {
            continue;
        }
        through += 1;
        let found = found_by_lust(&text);
        assert!(
            found <= 2,
            "case {case}: Lust found {found} mistakes in {text}"
        );
        clean += usize::from(found == 0);
    }
    // Both sides of the check are reached, and what's let through is
    // nearly always simply right.
    assert!((1000..15_000).contains(&through), "{through}");
    assert!(clean * 10 > through * 9, "{clean} of {through}");
}

#[test]
fn classes_a_data_file_cant_hold_are_a_mistake_in_solis() {
    for classes in [
        "",
        "class A { has { int x = 0; } }",
        "method m() {}\nend\n",
        "class A { create { } }\nend\n",
        "class A { has { Nope x = 1; } }\nend\n",
        "class A { has { int x; } }\nend\n",
        "state S = A;\nend\n",
    ] {
        let found = read("lock int version = 1;\nend\n", "test", classes, 1 << 20).unwrap_err();
        assert!(
            found
                .message
                .starts_with("its classes (solis/data/test.lst) can't be read: "),
            "{classes:?}: {found}"
        );
        assert_eq!(found.line, 0);
    }
    let many: String = (0..=MAX_FIELDS)
        .map(|i| format!("int f{i} = 0; "))
        .collect();
    let classes = format!("class A {{ has {{ {many} }} }}\nend\n");
    let found = Classes::read("test", &classes);
    assert!(found.is_err_and(|e| e.message.contains("more than 64 fields")));
    // A class with as many as there can be keeps each to once.
    let most: String = (0..MAX_FIELDS).map(|i| format!("int f{i} = 0; ")).collect();
    let classes = format!("class A {{ has {{ {most} }} }}\npass A;\nend\n");
    let classes = Classes::read("test", &classes).expect("64 fields");
    let file = |fields: &str| {
        format!("fetch \"solis/data/test\":A;\nlock A a = new A {{ {fields} }};\nend\n")
    };
    let all: Vec<String> = (0..MAX_FIELDS).map(|i| format!("f{i} = {i}")).collect();
    assert_eq!(classes.check(&file(&all.join(", "))), Ok(()));
    let twice = classes
        .check(&file("f63 = 1, f0 = 2, f63 = 3"))
        .unwrap_err();
    assert_eq!(twice.message, "f63 is already filled in here");
}
