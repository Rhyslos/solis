use super::*;

#[test]
fn settings_round_trip() {
    let mut s = Settings {
        size: [2048, 512],
        height: 96,
        ..Settings::default()
    };
    s.kinds[1].name = "Quote \" and slash \\".into();
    let text = write(&s);
    let read = read(&text).unwrap();
    assert_eq!(read.settings, s);
    assert!(read.notes.is_empty());
    assert_eq!(write(&read.settings), text);
    assert_eq!(s.extent(), UVec3::new(64, 16, 3));
    assert_eq!(s.regions(), [8, 2]);
}

#[test]
fn a_size_or_height_a_ground_cannot_have_is_refused_with_why() {
    for (size, height) in [
        ([1000, 1024], 128),
        ([0, 1024], 128),
        ([4096, 1024], 128),
        ([1024, 1024], 100),
        ([1024, 1024], 1024),
        ([1024, 1024], 0),
    ] {
        let s = Settings {
            size,
            height,
            ..Settings::default()
        };
        let err = read(&write(&s)).unwrap_err();
        assert!(err.message.contains("a ground's"), "{err}");
    }
}

#[test]
fn bad_kinds_are_left_out_with_a_note() {
    let text = write(&Settings::default())
        .replace("[92, 138, 64]", "[92, 999, 64]")
        .replace("\"Rock\"", "\"\"");
    let read = read(&text).unwrap();
    assert_eq!(read.settings.kinds.len(), 3);
    assert_eq!(read.settings.kinds[0].colour, [128, 128, 128]);
    assert_eq!(read.notes.len(), 2);
}

#[test]
fn no_kinds_gives_the_standard_ones() {
    let mut s = Settings::default();
    s.kinds.clear();
    let read = read(&write(&s)).unwrap();
    assert_eq!(read.settings.kinds, standard_kinds());
    assert_eq!(read.notes.len(), 1);
}

#[test]
fn a_newer_version_and_damaged_text_are_mistakes_never_crashes() {
    let newer = write(&Settings::default()).replace("version = 1", "version = 9");
    assert!(read(&newer).is_err());
    assert!(read("").is_err());
    assert!(read("lock int version = 1;\nend\n").is_err());
    let text = write(&Settings::default());
    for cut in (0..text.len()).step_by(7) {
        if text.is_char_boundary(cut) {
            let _ = read(&text[..cut]);
        }
    }
}

#[test]
fn the_region_folder_sits_beside_the_settings_file() {
    assert_eq!(folder_for("maps/farm.ground.lst"), "maps/farm.ground");
    assert!(is_settings_file("maps/Farm.Ground.lst"));
    assert!(!is_settings_file("maps/farm.tilemap.lst"));
}
