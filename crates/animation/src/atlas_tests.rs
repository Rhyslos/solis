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
