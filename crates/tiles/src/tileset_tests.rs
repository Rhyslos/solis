//! Tilesets: read as written, round trips, and what can't be used skipped.

use super::*;

fn id(n: u64) -> Id {
    Id::from_raw(n).unwrap()
}

/// The D4 fixture's terrains, every one plain, no art.
pub(crate) fn farm() -> Tileset {
    let t = |n, name: &str, code, colour| Terrain {
        id: id(n),
        name: name.into(),
        code,
        colour,
        kind: TileKind::Plain,
        solid: false,
    };
    Tileset {
        terrains: vec![
            t(1, "grass", '.', [0.31, 0.48, 0.23]),
            t(2, "tilled", 't', [0.42, 0.29, 0.18]),
            t(3, "water", '~', [0.18, 0.37, 0.56]),
            t(4, "fence", '#', [0.54, 0.48, 0.35]),
        ],
        ..Tileset::default()
    }
}

/// `farm` with art: grass plain (two variations), tilled 16-tile (every
/// slot but 6 and 9), water 47-tile (only its middle), fence edges (every
/// slot, the first animated), and a rule terrain, rocks.
pub(crate) fn farm_art() -> Tileset {
    let mut set = farm();
    set.picture = Some(reflect::AssetId(id(46)));
    set.tile = [16, 16];
    set.terrains[1].kind = TileKind::Dual16;
    set.terrains[2].kind = TileKind::Blob47;
    set.terrains[3].kind = TileKind::Edges;
    // Water and fence are solid (decision 0071).
    set.terrains[2].solid = true;
    set.terrains[3].solid = true;
    set.terrains.push(Terrain {
        id: id(5),
        name: "rocks".into(),
        code: 'r',
        colour: [0.5, 0.5, 0.5],
        kind: TileKind::Rules,
        solid: true,
    });
    let mut n = 100;
    let mut piece = |terrain: u64, slot: u32, at: [u32; 2], weight: f32, animation| {
        n += 1;
        Piece {
            id: id(n),
            terrain: id(terrain),
            slot,
            at,
            weight,
            animation,
        }
    };
    let mut pieces = vec![
        piece(1, 0, [0, 4], 3.0, Animation::None),
        piece(1, 0, [1, 4], 1.0, Animation::None),
    ];
    for slot in 1..16 {
        if slot != 6 && slot != 9 {
            pieces.push(piece(2, slot, [slot % 4, slot / 4], 1.0, Animation::None));
        }
    }
    pieces.push(piece(
        3,
        crate::autotile::blob_slot(255),
        [4, 0],
        1.0,
        Animation::Frames,
    ));
    for slot in 0..16 {
        let animation = if slot == 0 {
            Animation::Columns { count: 4, ms: 150 }
        } else {
            Animation::None
        };
        pieces.push(piece(4, slot, [slot % 4, 5 + slot / 4], 1.0, animation));
    }
    pieces.push(piece(5, 0, [6, 0], 1.0, Animation::None));
    pieces.push(piece(5, 1, [7, 0], 1.0, Animation::None));
    set.pieces = pieces;
    let cond = |c| Cond::of(c).unwrap();
    set.rules = vec![
        Rule {
            id: id(200),
            terrain: id(5),
            size: 3,
            pattern: "?.?=*=???".chars().map(cond).collect(),
            slot: 1,
        },
        Rule {
            id: id(201),
            terrain: id(5),
            size: 1,
            pattern: vec![Cond::Same],
            slot: 0,
        },
    ];
    set
}

#[test]
fn a_tileset_round_trips_and_finds_its_terrains() {
    let set = farm();
    let text = write(&set);
    assert!(text.contains("lock int version = 4;"));
    assert!(!text.contains("picture ="), "no art, no picture");
    assert!(text.contains("code = \"~\""));
    let back = read(&text).unwrap();
    assert!(back.notes.is_empty(), "{:?}", back.notes);
    assert_eq!(back.tileset, set);
    assert_eq!(set.by_code('t').unwrap().name, "tilled");
    assert_eq!(set.by_name("water").unwrap().code, '~');
    assert_eq!(set.free_code(), Some('w'), "the next easy one");
}

#[test]
fn art_pieces_and_rules_round_trip() {
    let set = farm_art();
    let text = write(&set);
    assert!(text.contains("kind = \"16-tile\""), "{text}");
    assert!(text.contains("animation = \"columns\""), "{text}");
    assert!(
        text.contains("pattern = [\"?.?\", \"=*=\", \"???\"]"),
        "{text}"
    );
    assert!(text.contains("weight = 3.0"), "{text}");
    assert_eq!(text.matches("solid = true").count(), 3, "{text}");
    let back = read(&text).unwrap();
    assert!(back.notes.is_empty(), "{:?}", back.notes);
    assert_eq!(back.tileset, set);
    assert_eq!(set.pieces_of(id(1), 0).count(), 2, "two variations");
    assert!(set.has_art(id(2)));
    assert_eq!(set.rules_of(id(5)).count(), 2);
    let mut bare = set.clone();
    bare.picture = None;
    assert!(!bare.has_art(id(2)), "no picture, no art");
}

#[test]
fn version_1_reads_as_plain_terrains_with_no_art() {
    let text = include_str!("../../../tests/fixtures/tileset/v1/farm.soltileset");
    let read = read(text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.tileset, farm());
}

#[test]
fn the_fixture_reads_without_notes() {
    let text = include_str!("../../../tests/fixtures/tileset/v3/farm.soltileset");
    let read = read(text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.tileset, farm_art());
    // Written again, it's the version 4 example: the same, as Lust.
    let v4 = include_str!("../../../tests/fixtures/tileset/v4/farm.tileset.lst");
    assert_eq!(write(&read.tileset), v4, "written as Solis writes it");
}

#[test]
fn the_lust_fixture_reads_back_as_itself() {
    let text = include_str!("../../../tests/fixtures/tileset/v4/farm.tileset.lst");
    let read = read(text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.tileset, farm_art());
    assert_eq!(write(&read.tileset), text);
    assert_eq!((FORMAT.version_of)(text), Some(4));
    let wrong = text.replacen("slot = 0", "slot = \"zero\"", 1);
    assert!(read_err_line(&wrong) > 0);
}

fn read_err_line(text: &str) -> usize {
    read(text).unwrap_err().line
}

#[test]
fn version_2_reads_with_nothing_solid() {
    let text = include_str!("../../../tests/fixtures/tileset/v2/farm.soltileset");
    let read = read(text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    let mut want = farm_art();
    for t in &mut want.terrains {
        t.solid = false;
    }
    assert_eq!(read.tileset, want);
}

#[test]
fn pieces_and_rules_it_can_t_use_are_skipped_with_a_note() {
    let i = |n: u64| id(n).to_string();
    let (g, t, r) = (i(1), i(2), i(3));
    let text = format!(
        "tileset version 2;\npicture = \"nope\";\ntile = [0, 16];\n\
         terrain {g} {{ name = \"grass\"; code = \".\"; }}\n\
         terrain {t} {{ name = \"tilled\"; code = \"t\"; kind = \"16-tile\"; }}\n\
         terrain {r} {{ name = \"rocks\"; code = \"r\"; kind = \"rules\"; }}\n\
         terrain {} {{ name = \"odd\"; code = \"o\"; kind = \"hexagons\"; }}\n\
         terrain {} {{ name = \"star\"; code = \"*\"; }}\n\
         piece {} {{ terrain = \"{t}\"; slot = 16; at = [0, 0]; }}\n\
         piece {} {{ terrain = \"{}\"; slot = 0; at = [0, 0]; }}\n\
         piece {} {{ terrain = \"{t}\"; slot = 1; at = [0, -1]; }}\n\
         piece {} {{ terrain = \"{g}\"; slot = 0; at = [0, 0]; weight = -2.0; }}\n\
         piece {} {{ terrain = \"{g}\"; slot = 0; at = [0, 0]; animation = \"columns\"; frames = 1; ms = 100; }}\n\
         piece {} {{ terrain = \"{g}\"; slot = 0; at = [0, 0]; animation = \"spin\"; }}\n\
         piece {} {{ terrain = \"{g}\"; slot = 0; at = [1, 0]; colour = 1; }}\n\
         piece {g} {{ terrain = \"{g}\"; slot = 0; at = [2, 0]; }}\n\
         rule {} {{ terrain = \"{g}\"; pattern = [\"*\"]; slot = 0; }}\n\
         rule {} {{ terrain = \"{r}\"; pattern = [\"??\", \"*?\"]; slot = 0; }}\n\
         rule {} {{ terrain = \"{r}\"; pattern = [\"???\", \"?=?\", \"???\"]; slot = 0; }}\n\
         rule {} {{ terrain = \"{r}\"; pattern = [\"?\\\"?\", \"?*?\", \"???\"]; slot = 0; }}\n\
         rule {} {{ terrain = \"{r}\"; pattern = [\"*\"]; slot = 64; }}\n\
         rule {} {{ terrain = \"{r}\"; pattern = [\"*\"]; slot = 3; }}\n",
        i(4),
        i(5),
        i(10),
        i(11),
        i(99),
        i(12),
        i(13),
        i(14),
        i(15),
        i(16),
        i(20),
        i(21),
        i(22),
        i(23),
        i(24),
        i(25),
    );
    let read = read(&text).unwrap();
    let notes = read.notes.join("\n");
    for expected in [
        "\"nope\" isn't an asset ID",
        "a tile's size is two whole numbers",
        "\"hexagons\" isn't a kind",
        "\"*\" isn't a code",
        "its slot should be a whole number from 0 to 15",
        "no terrain has the ID",
        "its place in the picture should be",
        "its weight should be a number",
        "an animation's frames should be 2 to",
        "its animation should be",
        "\"colour\" isn't a piece setting",
        "a second part with this ID",
        "isn't of its own rules",
        "should be 1, 3, 5, 7 or 9 rows",
        "only its middle, is \"*\"",
        "isn't part of a pattern",
        "from 0 to 63",
    ] {
        assert!(notes.contains(expected), "{expected} in\n{notes}");
    }
    assert_eq!(read.tileset.picture, None);
    assert_eq!(read.tileset.tile, [16, 16]);
    assert_eq!(read.tileset.pieces.len(), 1, "{:?}", read.tileset.pieces);
    assert_eq!(read.tileset.rules.len(), 1);
}

#[test]
fn what_can_t_be_used_is_skipped_with_a_note() {
    let i = |n: u64| id(n).to_string();
    let text = format!(
        "tileset version 1;\nshiny = 1;\n\
         terrain {} {{ name = \"grass\"; code = \".\"; colour = [2.0, -1.0, 0.5]; }}\n\
         terrain {} {{ name = \"grass\"; code = \",\"; }}\n\
         terrain {} {{ name = \"mud\"; code = \".\"; }}\n\
         terrain {} {{ name = \"two words\"; code = \"m\"; }}\n\
         terrain {} {{ name = \"rock\"; code = \"ab\"; }}\n\
         terrain {} {{ name = \"sand\"; code = \" \"; }}\n\
         terrain {} {{ name = \"clay\"; code = \"c\"; colour = 3; }}\n\
         terrain {} {{ name = \"snow\"; code = \"+\"; spin = 1; }}\n\
         terrain x {{ }}\n\
         tile 1 {{ }}\n",
        i(1),
        i(2),
        i(3),
        i(4),
        i(5),
        i(6),
        i(7),
        i(8)
    );
    let read = read(&text).unwrap();
    let notes = read.notes.join("\n");
    let names: Vec<&str> = read
        .tileset
        .terrains
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(names, ["grass", "snow"]);
    assert_eq!(
        read.tileset.terrains[0].colour,
        [1.0, 0.0, 0.5],
        "within 0 to 1"
    );
    assert_eq!(read.tileset.terrains[1].colour, [0.5; 3], "grey with none");
    for expected in [
        "\"shiny\" isn't a tileset setting",
        "a second terrain called \"grass\"",
        "a second terrain written \".\"",
        "\"two words\" isn't a name",
        "\"ab\" isn't a code",
        "\" \" isn't a code",
        "its colour should be three numbers",
        "\"spin\" isn't a terrain setting",
        "\"x\" isn't an ID",
        "\"tile\" isn't a part of a tileset",
    ] {
        assert!(notes.contains(expected), "{expected} in\n{notes}");
    }
    let mut many = String::from("tileset version 1;\n");
    for n in 1..=(MAX_TERRAINS as u64 + 5) {
        many.push_str(&format!(
            "terrain {} {{ name = \"t{n}\"; code = \"{}\"; }}\n",
            i(n),
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789$%&()+,-/:;<>@[]^{|}~#.'"
                .chars()
                .nth(n as usize - 1)
                .unwrap()
        ));
    }
    assert!(super::read(&many).unwrap().tileset.terrains.len() <= MAX_TERRAINS);
    assert!(super::read("tilemap version 1;").is_err());
    assert!(super::read(&"-".repeat(MAX_BYTES + 1)).is_err());
}

#[test]
fn solid_is_read_only_as_true_or_false() {
    let text = "tileset version 2;\n\
         terrain 0000000000001 { name = \"water\"; code = \"~\"; solid = true; }\n\
         terrain 0000000000002 { name = \"odd\"; code = \"o\"; solid = 3; }\n\
         terrain 0000000000003 { name = \"grass\"; code = \".\"; }\n";
    let read = read(text).unwrap();
    let solid: Vec<(&str, bool)> = read
        .tileset
        .terrains
        .iter()
        .map(|t| (t.name.as_str(), t.solid))
        .collect();
    assert_eq!(solid, [("water", true), ("grass", false)], "odd is skipped");
    assert!(
        read.notes.iter().any(|n| n.contains("true or false")),
        "{:?}",
        read.notes
    );
}

/// What `read` answers. A reader that never finishes fails the test here,
/// rather than holding up the whole run: fixed, it answers in well under a
/// thousandth of this long.
pub(crate) fn in_time<T: Send + 'static>(read: impl FnOnce() -> T + Send + 'static) -> T {
    let (sender, answer) = std::sync::mpsc::channel();
    std::thread::spawn(move || sender.send(read()));
    answer
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect("the reader never finished")
}

#[test]
fn two_classes_with_one_name_are_refused_never_a_hang() {
    // The tileset fuzzer's first run stopped on a file the reader never
    // finished (over a minute, then given up on), here made small: a `new`
    // turned into `class` twice, and Lust never finishes checking two
    // classes with one name. A data file holds no class, so it's refused
    // before Lust sees it.
    let text = include_str!("../tests/data/two-classes-one-name.tileset.lst");
    let why = in_time(|| read(text).map(|_| ())).unwrap_err();
    assert_eq!(why.line, 9);
    assert!(why.message.contains("only lock values"), "{why}");
    // The fixture, damaged the same way.
    let fixture = include_str!("../../../tests/fixtures/tileset/v4/farm.tileset.lst");
    let damaged = fixture.replace("new Terrain", "class Terrain");
    let why = in_time(move || read(&damaged).map(|_| ())).unwrap_err();
    assert_eq!(why.line, 12);
    assert!(why.message.contains("only lock values"), "{why}");
}

#[test]
fn a_character_that_can_t_be_seen_is_refused_in_lust_and_skipped_in_the_older_layout() {
    // What the atlas fuzzer found (see `area::lust_data`): a control
    // character in a data file's text is read by Lust and left out when
    // written. A terrain's name with one was always skipped, and no other
    // text of a tileset is kept as it's read; now the Lust file is
    // refused, on the line its list starts.
    let fixture = include_str!("../../../tests/fixtures/tileset/v4/farm.tileset.lst");
    let why = read(&fixture.replace("\"grass\"", "\"gra\u{17}ss\"")).unwrap_err();
    assert_eq!(why.line, 11);
    assert!(why.message.contains("can't be seen (U+0017)"), "{why}");
    // The older layout writes one as `\u{17}`, and skips the terrain.
    let older = include_str!("../../../tests/fixtures/tileset/v3/farm.soltileset");
    let read = read(&older.replace("\"grass\"", "\"gra\\u{17}ss\"")).unwrap();
    assert!(read.tileset.by_name("grass").is_none());
    assert!(
        read.notes.iter().any(|n| n.contains("isn't a name")),
        "{:?}",
        read.notes
    );
}

/// `good` with a few changes of the kinds that reach past Lust's own
/// checks: a letter put into a text (some that can't be seen, some Lust
/// writes with a `\`), a number in place of another, a line twice or
/// gone, a letter anywhere at all. `next` gives a number below its limit.
pub(crate) fn damaged(good: &str, next: &mut impl FnMut(usize) -> usize) -> String {
    const LETTERS: [&str; 32] = [
        "\u{17}", "\r", "\0", "\u{7f}", "\u{85}", "\u{1b}", "\t", "\\t", "\\n", "\\\"", "\\\\",
        "\\{", "\\}", "}", "{ ", " ", "é", "\u{a0}", "\u{2028}", "\u{feff}", "..", "/", ".", "_",
        "a", "A", "1", "'", "--", "//", ";", "grass",
    ];
    const NUMBERS: [&str; 16] = [
        "0",
        "1",
        "-1",
        "31",
        "32",
        "64",
        "4096",
        "4097",
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

/// A way to choose numbers below a limit, the same every run from `seed`.
pub(crate) fn chooser(mut seed: u64) -> impl FnMut(usize) -> usize {
    move |below| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 24) as usize % below
    }
}

/// What the fuzz target (`fuzz/fuzz_targets/tileset.rs`) holds the reader
/// to, for `text`: refused, or read within the limits, each name and code
/// used once, every piece's terrain there and its slot one of its kind's,
/// written back to itself, and a small map of it autotiled inside the
/// map's reach. Says whether it was read.
fn keeps_its_promises(text: &str) -> bool {
    let Ok(read) = read(text) else {
        return false;
    };
    let set = &read.tileset;
    assert!(set.terrains.len() <= MAX_TERRAINS, "{text:?}");
    for terrain in &set.terrains {
        assert!(usable_code(terrain.code), "{text:?}");
        let letters = terrain.name.chars().count();
        assert!((1..=MAX_NAME).contains(&letters), "{text:?}");
        assert!(
            terrain.colour.iter().all(|c| (0.0..=1.0).contains(c)),
            "{text:?}"
        );
        let by_code = set.by_code(terrain.code).map(|t| t.id);
        assert_eq!(by_code, Some(terrain.id), "{text:?}");
        let by_name = set.by_name(&terrain.name).map(|t| t.id);
        assert_eq!(by_name, Some(terrain.id), "{text:?}");
    }
    let written = write(set);
    let again = super::read(&written).unwrap_or_else(|e| panic!("{e}\n{text:?}\n{written:?}"));
    assert_eq!(&again.tileset, set, "{text:?}");
    if let Some(code) = set.free_code() {
        assert!(set.by_code(code).is_none() && usable_code(code), "{text:?}");
    }
    for piece in &set.pieces {
        let kind = set.terrain(piece.terrain).map(|t| t.kind);
        assert!(
            kind.is_some_and(|k| piece.slot < crate::autotile::slot_count(k)),
            "{text:?}"
        );
    }
    let mut map = crate::TileMap::new([9, 7], None, None);
    for (x, y) in (0..7u32).flat_map(|y| (0..9u32).map(move |x| (x, y))) {
        let i = (x * 7 + y * 3) as usize % (set.terrains.len() + 1);
        if let Some(terrain) = set.terrains.get(i) {
            let _ = map.set_code(0, x, y, terrain.code as u8);
        }
    }
    for placed in crate::autotile::chunk_tiles(&map, 0, set, [0, 0]) {
        assert!(
            (-1..=18).contains(&placed.x) && (-1..=14).contains(&placed.y),
            "{text:?}"
        );
        if let crate::autotile::Art::Piece(i) = placed.art {
            assert!(i < set.pieces.len(), "{text:?}");
        }
    }
    for terrain in &set.terrains {
        let _ = crate::autotile::missing_slots(set, terrain);
    }
    true
}

#[test]
fn no_damaged_tileset_breaks_a_promise() {
    // Fuzz-style, the same every run: thousands of damaged copies of each
    // version's example, the older layouts too, each held to what the fuzz
    // target holds the reader to.
    let mut next = chooser(0x5EC2_0000_711E_5E70);
    let mut read = 0;
    for good in [
        include_str!("../../../tests/fixtures/tileset/v1/farm.soltileset"),
        include_str!("../../../tests/fixtures/tileset/v2/farm.soltileset"),
        include_str!("../../../tests/fixtures/tileset/v3/farm.soltileset"),
        include_str!("../../../tests/fixtures/tileset/v4/farm.tileset.lst"),
    ] {
        for _ in 0..1000 {
            read += usize::from(keeps_its_promises(&damaged(good, &mut next)));
        }
    }
    // Both sides are reached: files read, and files refused.
    assert!((1000..3000).contains(&read), "{read}");
}
