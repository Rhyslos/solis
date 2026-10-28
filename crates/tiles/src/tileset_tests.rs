//! Tilesets: read as written, round trips, and what can't be used skipped.

use super::*;

fn id(n: u64) -> Id {
    Id::from_raw(n).unwrap()
}

pub(crate) fn farm() -> Tileset {
    let t = |n, name: &str, code, colour| Terrain {
        id: id(n),
        name: name.into(),
        code,
        colour,
    };
    Tileset {
        terrains: vec![
            t(1, "grass", '.', [0.31, 0.48, 0.23]),
            t(2, "tilled", 't', [0.42, 0.29, 0.18]),
            t(3, "water", '~', [0.18, 0.37, 0.56]),
            t(4, "fence", '#', [0.54, 0.48, 0.35]),
        ],
    }
}

#[test]
fn a_tileset_round_trips_and_finds_its_terrains() {
    let set = farm();
    let text = write(&set);
    assert!(text.contains("tileset version 1;"));
    assert!(text.contains("code = \"~\";"));
    let back = read(&text).unwrap();
    assert!(back.notes.is_empty(), "{:?}", back.notes);
    assert_eq!(back.tileset, set);
    assert_eq!(set.by_code('t').unwrap().name, "tilled");
    assert_eq!(set.by_name("water").unwrap().code, '~');
    assert_eq!(set.free_code(), Some('w'), "the next easy one");
}

#[test]
fn the_fixture_reads_without_notes() {
    let text = include_str!("../../../tests/fixtures/tileset/v1/farm.soltileset");
    let read = read(text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.tileset.terrains.len(), 4);
    assert_eq!(write(&read.tileset), text, "written as Solis writes it");
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
         terrain {} {{ name = \"snow\"; code = \"*\"; spin = 1; }}\n\
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
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!$%&()*+,-/:;<=>?@[]^_{|}~"
                .chars()
                .nth(n as usize - 1)
                .unwrap()
        ));
    }
    assert!(super::read(&many).unwrap().tileset.terrains.len() <= MAX_TERRAINS);
    assert!(super::read("tilemap version 1;").is_err());
    assert!(super::read(&"-".repeat(MAX_BYTES + 1)).is_err());
}
