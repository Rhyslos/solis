use super::*;
use level::builtin::{Tags, Transform};
use level::{EntityData, Level};

#[test]
fn tags_are_counted_in_each_level_copies_of_modules_among_them() {
    let root = reflect::test_support::TempFolder::new("count-tags").unwrap();
    let registry = level::builtin::registry().unwrap();
    let mut tags = TagList::default();
    let enemy = tags.add("Enemy").unwrap();
    let flying = tags.add("Flying").unwrap();
    // A module, a bat, carrying Enemy.
    let mut bat = Level::new();
    let bat_root = bat.spawn(
        EntityData::new("Bat")
            .with(&Transform::default())
            .with(&Tags::of([enemy])),
    );
    let bat_file = level::text::LevelFile {
        kind: level::text::FileKind::Module { root: bat_root },
        level: bat,
        overrides: BTreeMap::new(),
    };
    let bat_id = AssetId::random();
    let modules = BTreeMap::from([(bat_id, Arc::new(bat_file))]);
    // A cave with two bats placed and a crow carrying both tags.
    let lookup = |id: AssetId| modules.get(&id).cloned();
    let mut cave = Level::new();
    let mut expander = level::module::Expander::new(&lookup);
    for _ in 0..2 {
        let placed = EntityData::new("Bat").with(&Transform::default());
        for (id, data) in expander
            .place(bat_id, reflect::EntityId::random(), &placed)
            .unwrap()
        {
            cave.insert(id, data);
        }
    }
    cave.spawn(EntityData::new("Crow").with(&Tags::of([enemy, flying])));
    let cave_file = expander
        .collapse(&cave, level::text::FileKind::Level)
        .unwrap();
    std::fs::write(
        root.join("cave.sollevel"),
        level::text::write_file(&cave_file, &registry, &|_| None),
    )
    .unwrap();
    std::fs::write(root.join("empty.sollevel"), "level version 5;\n").unwrap();
    std::fs::write(root.join("broken.sollevel"), "nonsense").unwrap();
    let levels: Vec<PathBuf> = [
        "cave.sollevel",
        "empty.sollevel",
        "broken.sollevel",
        "gone.sollevel",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect();
    let counted = count_in(&root, &levels, &modules);
    assert_eq!(counted.levels[&enemy], [("cave.sollevel".to_string(), 3)]);
    assert_eq!(counted.levels[&flying], [("cave.sollevel".to_string(), 1)]);
    assert_eq!(counted.levels.len(), 2);
}

#[test]
fn the_tags_file_is_saved_read_back_and_a_damaged_one_kept() {
    let root = reflect::test_support::TempFolder::new("tags-file").unwrap();
    assert_eq!(
        crate::play_files::load_tags(&root),
        (TagList::default(), Vec::new())
    );
    assert_eq!(stamp(&root), None);
    let mut tags = TagList::default();
    tags.add("Enemy").unwrap();
    tags.add("Door").unwrap();
    assert_eq!(crate::play_files::save_tags(&root, &tags).unwrap(), None);
    assert!(stamp(&root).is_some());
    let (read, notes) = crate::play_files::load_tags(&root);
    assert_eq!(read, tags);
    assert!(notes.is_empty(), "{notes:?}");
    // A damaged file: no tags and a note; saving keeps it as a backup.
    std::fs::write(root.join(level::tags::FILE_NAME), "class Oops { }\nend\n").unwrap();
    let (read, notes) = crate::play_files::load_tags(&root);
    assert!(read.is_empty());
    assert_eq!(notes.len(), 1, "{notes:?}");
    let kept = crate::play_files::save_tags(&root, &tags).unwrap();
    assert!(
        matches!(kept, Some(crate::play_files::Kept::Damaged(_))),
        "{kept:?}"
    );
    assert_eq!(crate::play_files::load_tags(&root).0, tags);
}

#[test]
fn the_library_shows_the_project_s_tags_file_as_its_tags() {
    use asset_db::{FileEntry, FileKind, Listing};
    let entry = |path: &str| FileEntry {
        path: PathBuf::from(path),
        kind: FileKind::of(Path::new(path)),
        id: None,
    };
    let listing = Listing {
        folders: vec![PathBuf::from("enemies")],
        files: vec![
            entry("tags.lst"),
            entry("enemies/tags.lst"),
            entry("main.lst"),
        ],
    };
    let (items, _) = crate::project::Projects::library(&listing, |_| None);
    let kind = |path: &str| items.iter().find(|i| i.path == path).unwrap().kind;
    assert_eq!(kind("tags.lst"), editor::LibraryKind::Tags);
    assert_eq!(
        kind("enemies/tags.lst"),
        editor::LibraryKind::Code,
        "only the top one"
    );
    assert_eq!(kind("main.lst"), editor::LibraryKind::Code);
}
