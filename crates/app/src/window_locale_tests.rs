use super::*;

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("solis-locale-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_languages_and_translation_files_are_read_and_stamped() {
    let root = folder("read");
    let mut languages = Languages::default();
    languages.add("nb").unwrap();
    std::fs::write(
        root.join("languages.lst"),
        level::languages::write(&languages),
    )
    .unwrap();
    std::fs::create_dir_all(root.join("locale/nb")).unwrap();
    std::fs::create_dir_all(root.join("locale/en")).unwrap();
    std::fs::write(root.join("locale/en/shop.ftl"), "Shop-Welcome = Welcome!\n").unwrap();
    std::fs::write(
        root.join("locale/nb/shop.ftl"),
        "Shop-Welcome = Velkommen!\n",
    )
    .unwrap();
    std::fs::write(root.join("locale/nb/notes.txt"), "not read").unwrap();
    std::fs::write(root.join("locale/stray.ftl"), "not in a language's folder").unwrap();
    let (read, files, notes) = load_locale(&root, &root);
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(read, languages);
    let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(paths, ["locale/en/shop.ftl", "locale/nb/shop.ftl"]);
    let before = stamp(&root, &root);
    assert_eq!(before.len(), 3);
    std::fs::write(root.join("locale/nb/shop.ftl"), "Shop-Welcome = Hei!\n").unwrap();
    assert_ne!(stamp(&root, &root), before, "a change is seen");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_project_without_them_is_written_in_english_alone() {
    let root = folder("none");
    let (read, files, notes) = load_locale(&root, &root);
    assert_eq!(read.all(), ["en"]);
    assert!(files.is_empty() && notes.is_empty());
    std::fs::write(root.join("languages.lst"), "damaged").unwrap();
    let (read, _, notes) = load_locale(&root, &root);
    assert_eq!(read.all(), ["en"]);
    assert_eq!(notes.len(), 1, "{notes:?}");
    let _ = std::fs::remove_dir_all(&root);
}
