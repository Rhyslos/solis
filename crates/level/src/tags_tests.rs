use super::*;

fn list(names: &[&str]) -> TagList {
    let mut tags = TagList::default();
    for name in names {
        tags.add(name).unwrap();
    }
    tags
}

#[test]
fn a_list_round_trips_through_its_file() {
    let tags = list(&["Enemy", "Collectible", "Flying"]);
    let text = write(&tags);
    assert!(text.starts_with("-- The project's tags"), "{text}");
    assert!(text.contains("fetch \"solis/data/tags\":[Tag];"), "{text}");
    assert!(text.contains("lock int version = 1;"));
    let read = read(&text).unwrap();
    assert_eq!(read.tags, tags);
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    // An empty list too.
    let empty = read(&write(&TagList::default())).unwrap();
    assert!(empty.tags.is_empty());
}

#[test]
fn the_example_file_reads() {
    let text = include_str!("../../../tests/fixtures/tags/v1/tags.lst");
    let read = read(text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.tags.names(), ["Enemy", "Collectible", "Flying", "Door"]);
    assert_eq!(
        read.tags.id_of("Enemy").map(|id| id.to_string()).as_deref(),
        Some("1BWHNR3RV8PYY")
    );
    assert!(FORMAT.is_complete());
}

#[test]
fn names_are_refused_with_a_reason() {
    let tags = list(&["Enemy"]);
    let reason = |name: &str| tags.problem(name, None).unwrap_or_default();
    assert!(reason("").contains("needs a name"));
    assert!(reason("enemy").contains("capital"), "{}", reason("enemy"));
    assert!(reason("enemy").contains("Try Enemy"), "{}", reason("enemy"));
    assert!(reason("Big bat").contains("no spaces"));
    assert!(reason("Big bat").contains("BigBat"), "{}", reason("Big bat"));
    assert!(reason("UFO").contains("all capitals"));
    assert!(reason("UFO").contains("Ufo"), "{}", reason("UFO"));
    assert!(reason("Time").contains("Lust's own"));
    assert!(reason("None").contains("no tags"));
    assert!(reason(&"A".repeat(41)).contains("at most 40"));
    assert_eq!(reason("Enemy"), "There's already a tag called Enemy.");
    assert!(reason("ENEMy").contains("capitals alone"), "{}", reason("ENEMy"));
    for fine in ["Flying", "Door2", "Big_bat", "X", "A1"] {
        assert_eq!(tags.problem(fine, None), None, "{fine}");
    }
    // A tag keeps its own name when renamed, even with other capitals.
    let enemy = tags.id_of("Enemy").unwrap();
    assert_eq!(tags.problem("Enemy", Some(enemy)), None);
}

#[test]
fn tags_are_made_renamed_and_removed() {
    let mut tags = list(&["Enemy", "Flying"]);
    let enemy = tags.id_of("Enemy").unwrap();
    tags.rename(enemy, "Monster").unwrap();
    assert_eq!(tags.name_of(enemy), Some("Monster"));
    assert_eq!(tags.id_of("Enemy"), None);
    assert!(tags.rename(enemy, "Flying").is_err());
    assert!(tags.rename(TagId::random(), "Other").is_err());
    assert!(tags.remove(enemy));
    assert!(!tags.remove(enemy));
    assert_eq!(tags.names(), ["Flying"]);
    assert!(tags.add("flying").is_err());
}

#[test]
fn a_project_has_at_most_the_most_tags() {
    let mut tags = TagList::default();
    for n in 0..MAX_TAGS {
        tags.add(&format!("Tag{n}")).unwrap();
    }
    let why = tags.add("OneMore").unwrap_err();
    assert!(why.contains("at most"), "{why}");
}

#[test]
fn tags_that_cant_be_used_are_left_out_with_a_note() {
    let id = TagId::random();
    let text = format!(
        "fetch \"solis/data/tags\":[Tag];\nlock int version = 1;\nlock Tag array tags = [\n\
         new Tag {{ id = \"{id}\", name = \"Enemy\" }},\n\
         new Tag {{ id = \"nonsense\", name = \"Lost\" }},\n\
         new Tag {{ id = \"{}\", name = \"enemy\" }},\n\
         new Tag {{ id = \"{id}\", name = \"Twin\" }},\n\
         new Tag {{ id = \"{}\", name = \"Two words\" }},\n\
         ];\nend\n",
        TagId::random(),
        TagId::random()
    );
    let read = read(&text).unwrap();
    assert_eq!(read.tags.names(), ["Enemy"]);
    assert_eq!(read.notes.len(), 4, "{:?}", read.notes);
    assert!(read.notes[0].contains("no ID"));
    assert!(read.notes[2].contains("Another tag has that ID"));
}

#[test]
fn damaged_newer_and_huge_files_are_refused() {
    assert!(read("").is_err());
    assert!(read("lock int version = 1;\nend\n").is_ok_and(|r| r.tags.is_empty()));
    let newer = read("fetch \"solis/data/tags\":[Tag];\nlock int version = 2;\nend\n").unwrap_err();
    assert!(newer.message.contains("newer"), "{newer}");
    assert!(read("class Sneaky { }\nend\n").is_err());
    assert!(read(&"x".repeat(MAX_BYTES + 1)).is_err());
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(64))]
    /// Any names make a file that reads back as the same list.
    #[test]
    fn any_list_round_trips(names in proptest::collection::vec("[A-Z][a-z0-9_]{0,12}", 0..20)) {
        let mut tags = TagList::default();
        for name in &names {
            let _ = tags.add(name);
        }
        let read = read(&write(&tags)).unwrap();
        proptest::prop_assert_eq!(read.tags, tags);
    }

    /// A damaged file is refused or read, never a crash, and what's read
    /// writes back as itself.
    #[test]
    fn a_damaged_file_never_crashes(cut in 0usize..400, junk in "\\PC{0,24}") {
        let text = write(&list(&["Enemy", "Collectible", "Flying"]));
        let cut = text.char_indices().map(|(i, _)| i).nth(cut % text.chars().count()).unwrap_or(0);
        let damaged = format!("{}{junk}{}", &text[..cut], &text[cut..]);
        if let Ok(read) = read(&damaged) {
            let again = read_back(&read.tags);
            proptest::prop_assert_eq!(again, read.tags);
        }
    }
}

fn read_back(tags: &TagList) -> TagList {
    read(&write(tags)).unwrap().tags
}
