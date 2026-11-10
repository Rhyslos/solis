//! A tile map's Lust layout read into blocks: each chunk's values gathered
//! in the file's order, however many chunks and values a file names, and
//! the most things a file has.

use super::*;
use crate::tileset::tests::in_time;

/// An empty map file's values, with `things` put in each list named.
/// (Lust takes a second over a hundred thousand things, and what's
/// checked here starts after it.)
fn with(lists: Vec<(&str, Vec<Fixed>)>) -> Values {
    let fields = [
        ("tileset", Value::Text(String::new())),
        ("size", Value::List(vec![Value::Int(64), Value::Int(64)])),
    ];
    let text = write("A tile map.", 3, &fields, &[]);
    let mut values = data::read(&text, KIND, CLASSES, 1 << 20).expect("an empty map file");
    for (list, things) in lists {
        let named = values.values.iter_mut().find(|v| v.name == list);
        named.expect("one of a map file's lists").value = Fixed::List(things);
    }
    values
}

fn ints(numbers: &[i64]) -> Fixed {
    Fixed::List(numbers.iter().map(|n| Fixed::Int(*n)).collect())
}

/// One thing of a chunk's values: `value`'s `cells` in `chunk`, and a pick
/// for each if it's a choice.
fn cells_of(value: &str, chunk: [i64; 2], cells: &[&[i64]], picks: Option<&[&str]>) -> Fixed {
    let mut fields = vec![
        ("value".to_string(), Fixed::Text(value.to_string())),
        ("chunk".to_string(), ints(&chunk)),
        (
            "cells".to_string(),
            Fixed::List(cells.iter().map(|cell| ints(cell)).collect()),
        ),
    ];
    if let Some(picks) = picks {
        let picks = picks.iter().map(|p| Fixed::Text((*p).to_string()));
        fields.push(("picks".to_string(), Fixed::List(picks.collect())));
    }
    Fixed::Thing {
        class: String::new(),
        fields,
    }
}

/// A block's fields as `name = value` lines.
fn shown(block: &Block) -> Vec<String> {
    let fields = block.fields.iter();
    fields
        .map(|f| format!("{} = {}", f.key, data::value(&f.value)))
        .collect()
}

#[test]
fn a_chunk_s_values_are_gathered_in_the_order_the_file_names_them() {
    // A chunk named by several things, and a value named twice in one
    // chunk (as a file changed by hand may): one block a chunk, the
    // chunks and their values in the order first named, cells joined.
    let values = with(vec![
        (
            "numbers",
            vec![
                cells_of("growth", [1, 0], &[&[1, 2, 3]], None),
                cells_of("growth", [0, 0], &[&[4, 5, 6]], None),
                cells_of("growth", [1, 0], &[&[7, 8, 9], &[1, 2, 0]], None),
            ],
        ),
        (
            "switches",
            vec![
                cells_of("watered", [0, 0], &[&[1, 1]], None),
                cells_of("sunny", [2, -2], &[&[0, 31]], None),
            ],
        ),
        (
            "choices",
            vec![cells_of("crop", [1, 0], &[&[3, 3]], Some(&["wheat"]))],
        ),
    ]);
    let read = blocks(&values, 3).unwrap();
    assert!(read.blocks.iter().all(|b| b.word == "values"));
    let names: Vec<&str> = read.blocks.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["1.0", "0.0", "2.-2"]);
    assert_eq!(
        shown(&read.blocks[0]),
        [
            "growth = [[1, 2, 3], [7, 8, 9], [1, 2, 0]]",
            "crop = [[3, 3, \"wheat\"]]"
        ]
    );
    assert_eq!(
        shown(&read.blocks[1]),
        ["growth = [[4, 5, 6]]", "watered = [[1, 1]]"]
    );
    assert_eq!(shown(&read.blocks[2]), ["sunny = [[0, 31]]"]);
    // Each block and value is on the line of the list that first named it.
    let line_of = |list: &str| {
        let named = values.values.iter().find(|v| v.name == list);
        named.unwrap().line as usize
    };
    let lines: Vec<usize> = read.blocks.iter().map(|b| b.line).collect();
    let (numbers, switches, choices) =
        (line_of("numbers"), line_of("switches"), line_of("choices"));
    assert_eq!(lines, [numbers, numbers, switches]);
    assert!(numbers < switches && switches < choices);
    let lines: Vec<usize> = read.blocks[0].fields.iter().map(|f| f.line).collect();
    assert_eq!(lines, [numbers, choices]);
    let lines: Vec<usize> = read.blocks[1].fields.iter().map(|f| f.line).collect();
    assert_eq!(lines, [numbers, switches]);
}

#[test]
fn as_many_chunks_and_values_as_things_are_gathered_at_once() {
    // The code review: each thing's chunk was searched for among every
    // chunk before it, and its value among that chunk's, so a file of
    // things each in a chunk of its own (80,000 of them, 5 MB: 17 seconds)
    // or each a value of its own took time that grew with their square.
    // Here are 200,000 of each, which took minutes; looked up, a moment.
    const MANY: i64 = 200_000;
    let apart = (0..MANY).map(|n| cells_of("growth", [n, 0], &[&[1, 2, n]], None));
    let together = (0..MANY).map(|n| cells_of(&format!("v{n}"), [0, 0], &[&[3, 4]], None));
    let values = with(vec![
        ("numbers", apart.collect()),
        ("switches", together.collect()),
    ]);
    let read = in_time(move || blocks(&values, 3)).unwrap();
    // A block for each chunk, in order, with its one cell.
    assert_eq!(read.blocks.len(), MANY as usize);
    for (n, block) in read.blocks.iter().enumerate().skip(1) {
        assert_eq!(block.name, format!("{n}.0"));
        assert_eq!(shown(block), [format!("growth = [[1, 2, {n}]]")]);
    }
    // And the chunk they share holds every value, in order.
    let shared = &read.blocks[0];
    assert_eq!(shared.name, "0.0");
    assert_eq!(shared.fields.len(), MANY as usize + 1);
    assert_eq!(shared.fields[0].key, "growth");
    for (n, field) in shared.fields.iter().skip(1).enumerate() {
        assert_eq!(field.key, format!("v{n}"));
        assert_eq!(data::value(&field.value), "[[3, 4]]");
    }
}

#[test]
fn a_file_has_no_more_things_than_the_biggest_map() {
    // The older layout stops at its most blocks; this one read whatever a
    // file listed. The most a map has: its 16 layers and 32 values, and
    // in each of 128 × 128 chunks every layer's cells and exact tiles and
    // every value's cells.
    assert_eq!(MAX_THINGS, 16 + 32 + 128 * 128 * (16 + 16 + 32));
    let thing = || Fixed::Thing {
        class: String::new(),
        fields: Vec::new(),
    };
    // One thing over, counted across every list: refused.
    let each = MAX_THINGS / LISTS.len() + 1;
    let over = with(
        LISTS
            .iter()
            .map(|list| (*list, vec![thing(); each]))
            .collect(),
    );
    let why = blocks(&over, 3).unwrap_err();
    assert_eq!(why.line, 0);
    assert!(
        why.message
            .contains("(1048628) than the biggest tile map has (1048624)"),
        "{why}"
    );
    // As many as that are read: these get as far as having no chunk.
    let most = with(vec![("numbers", vec![thing(); MAX_THINGS])]);
    let why = blocks(&most, 3).unwrap_err();
    assert!(why.message.contains("a chunk should be [x, y]"), "{why}");
}
