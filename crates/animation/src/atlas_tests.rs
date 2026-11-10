use super::*;

fn tools() -> Atlas {
    Atlas {
        picture: "tools.png".into(),
        grid: [16, 8],
        crops: vec![
            Crop {
                name: "Axe".into(),
                rect: [0, 0, 16, 16],
            },
            Crop {
                name: "Hoe_2".into(),
                rect: [16, 8, 32, 8],
            },
        ],
    }
}

#[test]
fn an_atlas_round_trips_through_its_file() {
    let text = write(&tools());
    assert!(text.starts_with("-- Pictures cut from tools.png (Atlas Crop)."));
    assert!(text.contains("fetch \"solis/data/atlas\":[Atlas, Crop];"));
    assert!(
        text.contains("new Crop { name = \"Axe\", box = [0, 0, 16, 16] }"),
        "{text}"
    );
    let read = read(&text).unwrap();
    assert_eq!(read.atlas, tools());
    assert!(read.notes.is_empty());
    let empty = Atlas::new("a.png");
    assert_eq!(super::read(&write(&empty)).unwrap().atlas, empty);
}

#[test]
fn bad_crops_are_left_out_with_a_note_and_bad_files_refused() {
    let text = write(&tools())
        .replace("\"Hoe_2\"", "\"2hoe\"")
        .replace("[0, 0, 16, 16]", "[0, 0, 0, 16]");
    let read = read(&text).unwrap();
    assert!(read.atlas.crops.is_empty());
    assert_eq!(read.notes.len(), 2, "{:?}", read.notes);
    let twice = write(&tools()).replace("\"Hoe_2\"", "\"axe\"");
    assert_eq!(super::read(&twice).unwrap().atlas.crops.len(), 1);
    for bad in [
        write(&tools()).replace("\"tools.png\"", "\"../secret.png\""),
        write(&tools()).replace("version = 1", "version = 9"),
        "not Lust at all".to_string(),
        String::new(),
    ] {
        assert!(super::read(&bad).is_err(), "{bad}");
    }
    let grid = write(&tools()).replace("grid = [16, 8]", "grid = [0, 99999]");
    let read = super::read(&grid).unwrap();
    assert_eq!(read.atlas.grid, [16, 16]);
}

#[test]
fn every_tile_with_something_in_it_is_cropped_once() {
    let mut atlas = Atlas::new("tiles.png");
    atlas.grid = [16, 16];
    // The cell at column 1, row 0 is empty.
    let filled = |r: [u32; 4]| r != [16, 0, 16, 16];
    assert_eq!(atlas.crop_every_tile([48, 32], &filled), 5);
    assert_eq!(atlas.crops[0].name, "Tile_0_0");
    assert_eq!(atlas.crops[1].rect, [32, 0, 16, 16]);
    assert_eq!(atlas.crop_every_tile([48, 32], &filled), 0, "not twice");
    assert_eq!(atlas.free_name("Tile_0_0"), "Tile_0_02");
}

#[test]
fn names_and_paths() {
    assert!(good_name("Axe") && good_name("a_1"));
    assert!(!good_name("") && !good_name("1a") && !good_name("a b") && !good_name(&"a".repeat(65)));
    assert_eq!(file_for("sprites/tools.png"), "sprites/tools.atlas.lst");
    assert_eq!(file_for("hero.aseprite"), "hero.atlas.lst");
    assert_eq!(
        picture_path("sprites/tools.atlas.lst", &tools()),
        "sprites/tools.png"
    );
    assert_eq!(picture_path("tools.atlas.lst", &tools()), "tools.png");
}

proptest::proptest! {
    #[test]
    fn any_text_is_read_without_a_panic(text in ".{0,400}") {
        let _ = read(&text);
        let start = write(&tools());
        let cut = text.len().min(start.len());
        let _ = read(&format!("{}{}", &start[..start.len() - cut], text));
    }
}

/// What the fuzz target (`fuzz/fuzz_targets/atlas.rs`) holds the reader
/// to, for `text`: refused, or read within the limits, each crop's name
/// good and used once, its box inside the biggest picture, and written
/// back to itself. Says whether it was read.
fn keeps_its_promises(text: &str) -> bool {
    let Ok(read) = read(text) else {
        return false;
    };
    let atlas = &read.atlas;
    assert!(atlas.crops.len() <= MAX_CROPS, "{text:?}");
    assert!(
        atlas.grid.iter().all(|&g| (1..=MAX_SIDE).contains(&g)),
        "{text:?}"
    );
    for (i, crop) in atlas.crops.iter().enumerate() {
        assert!(good_name(&crop.name), "{text:?}");
        assert!(
            !atlas.crops[..i]
                .iter()
                .any(|c| c.name.eq_ignore_ascii_case(&crop.name)),
            "{text:?}"
        );
        assert!(crop.rect[2] >= 1 && crop.rect[3] >= 1, "{text:?}");
        assert!(crop.rect.iter().all(|&v| v <= MAX_SIDE), "{text:?}");
    }
    let written = write(atlas);
    let again = super::read(&written).unwrap_or_else(|e| panic!("{e}\n{text:?}\n{written:?}"));
    assert_eq!(&again.atlas, atlas, "{text:?}");
    true
}

#[test]
fn a_character_that_can_t_be_seen_is_refused_not_read_and_lost() {
    // The atlas fuzzer's first run stopped on this file, here made small:
    // the dot of its picture's name had become a control character, which
    // was read into the name but left out when the atlas was written, so
    // what was written didn't read back the same. No text of a data file
    // holds one now: the file is refused, with where and which.
    let text = include_str!("../tests/data/unseen-character.atlas.lst");
    assert!(text.contains("\"tools\u{17}png\""));
    let why = read(text).unwrap_err();
    assert_eq!(why.line, 8);
    assert!(
        why.message
            .contains("\"tools?png\" holds a character that can't be seen (U+0017)"),
        "{why}"
    );
    // In a crop's name it's the whole file's mistake too, never a crop
    // quietly left out; and the fixture with one after another is refused.
    let named = write(&tools()).replace("\"Axe\"", "\"Axe\u{7f}\"");
    assert_eq!(read(&named).unwrap_err().line, 6);
    let fixture = include_str!("../../../tests/fixtures/atlas/v1/tools.atlas.lst");
    for unseen in [
        '\0', '\u{1}', '\u{8}', '\r', '\u{1b}', '\u{7f}', '\u{85}', '\u{9f}',
    ] {
        let damaged = fixture.replace("tools.png\"", &format!("tools.png{unseen}\""));
        assert!(read(&damaged).is_err(), "{unseen:?}");
    }
    // What can be written is still read, and written back as itself: a
    // tab, a line break, and letters that only look like nothing.
    for seen in ["\t", "\\t", "\\n", "\u{a0}", "\u{feff}", "\u{2028}", "é"] {
        let odd = fixture.replace("tools.png\"", &format!("tools{seen}.png\""));
        assert!(keeps_its_promises(&odd), "{seen:?}");
    }
    assert!(keeps_its_promises(&text.replace('\u{17}', ".")));
}

/// `good` with a few changes of the kinds that reach past Lust's own
/// checks: a letter put into a text (some that can't be seen, some Lust
/// writes with a `\`), a number in place of another, a line twice or
/// gone, a letter anywhere at all. `next` gives a number below its limit.
fn damaged(good: &str, next: &mut impl FnMut(usize) -> usize) -> String {
    const LETTERS: [&str; 32] = [
        "\u{17}", "\r", "\0", "\u{7f}", "\u{85}", "\u{1b}", "\t", "\\t", "\\n", "\\\"", "\\\\",
        "\\{", "\\}", "}", "{ ", " ", "é", "\u{a0}", "\u{2028}", "\u{feff}", "..", "/", ".", "_",
        "a", "A", "1", "'", "--", "//", ";", "axe",
    ];
    const NUMBERS: [&str; 16] = [
        "0",
        "1",
        "-1",
        "16",
        "4096",
        "16383",
        "16384",
        "16385",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "1.5",
        "1e3",
        "007",
        "-0",
    ];
    let mut text = good.to_string();
    for _ in 0..1 + next(3) {
        let places = |of: fn(char) -> bool| -> Vec<usize> {
            text.match_indices(of).map(|(at, _)| at).collect()
        };
        let lines: Vec<usize> = std::iter::once(0)
            .chain(places(|c| c == '\n').into_iter().map(|at| at + 1))
            .collect();
        let line = lines[next(lines.len())];
        let end = text[line..].find('\n').map_or(text.len(), |n| line + n + 1);
        match next(6) {
            0 | 1 => {
                let quotes = places(|c| c == '"');
                let at = quotes[next(quotes.len())] + next(2);
                text.insert_str(at, LETTERS[next(LETTERS.len())]);
            }
            2 | 3 => {
                let digits = places(|c| c.is_ascii_digit());
                let at = digits[next(digits.len())];
                let end = text[at..]
                    .find(|c: char| !c.is_ascii_digit())
                    .map_or(text.len(), |n| at + n);
                text.replace_range(at..end, NUMBERS[next(NUMBERS.len())]);
            }
            4 => {
                let copy = text[line..end].to_string();
                text.insert_str(line, &copy);
            }
            _ if next(2) == 0 => text.replace_range(line..end, ""),
            _ => {
                let mut at = line + next(end - line + 1);
                while !text.is_char_boundary(at) {
                    at -= 1;
                }
                text.insert_str(at, LETTERS[next(LETTERS.len())]);
            }
        }
    }
    text
}

#[test]
fn no_damaged_atlas_breaks_a_promise() {
    // Fuzz-style, the same every run: thousands of damaged copies of a
    // good file, each held to what the fuzz target holds the reader to.
    let good = include_str!("../../../tests/fixtures/atlas/v1/tools.atlas.lst");
    let mut seed = 0x5EC2_0000_A71A_5000u64;
    let mut next = move |below: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 24) as usize % below
    };
    let read = (0..4000)
        .filter(|_| keeps_its_promises(&damaged(good, &mut next)))
        .count();
    // Both sides are reached: files read, and files refused.
    assert!((1000..3000).contains(&read), "{read}");
}
