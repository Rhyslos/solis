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
    set.terrains.push(Terrain {
        id: id(5),
        name: "rocks".into(),
        code: 'r',
        colour: [0.5, 0.5, 0.5],
        kind: TileKind::Rules,
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
    assert!(text.contains("tileset version 2;"));
    assert!(!text.contains("picture ="), "no art, no picture line");
    assert!(text.contains("code = \"~\";"));
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
    assert!(text.contains("kind = \"16-tile\";"), "{text}");
    assert!(text.contains("animation = \"columns\";"), "{text}");
    assert!(
        text.contains("pattern = [\"?.?\", \"=*=\", \"???\"];"),
        "{text}"
    );
    assert!(text.contains("weight = 3.0;"), "{text}");
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
    let text = include_str!("../../../tests/fixtures/tileset/v2/farm.soltileset");
    let read = read(text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.tileset, farm_art());
    assert_eq!(write(&read.tileset), text, "written as Solis writes it");
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
