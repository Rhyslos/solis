use super::*;

fn fixture(path: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures")
            .join(path),
    )
    .unwrap()
    .replace("\r\n", "\n")
}

#[test]
fn an_assembly_s_file_becomes_the_module_fixture() {
    let moved = from_old(&fixture("area/v4/crate.solassembly")).unwrap();
    assert_eq!(moved, fixture("module/v1/crate.module.lst"));
    // A module's own text is given back as it is.
    assert_eq!(from_old(&moved).unwrap(), moved);
    // A level isn't a module.
    assert!(from_old(&fixture("area/v5/blockout.sollevel")).is_err());
}

#[test]
fn a_damaged_assembly_is_refused_or_moved_never_a_crash() {
    let registry = level::builtin::registry().unwrap();
    let old = fixture("area/v2/crate.solassembly");
    let mut seed = 0x0118_0005_u64;
    for _ in 0..500 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut bytes = old.clone().into_bytes();
        let at = (seed as usize) % bytes.len();
        bytes[at] = b"assembly version;{}\n\r 0123456789-\"="[(seed >> 40) as usize % 34];
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        if let Ok(moved) = from_old(&text) {
            assert_eq!(
                level::module_file::read(&moved, &registry).unwrap(),
                level::module_file::read(&text, &registry).unwrap()
            );
        }
    }
}

#[test]
fn a_new_module_takes_a_free_name() {
    let root = reflect::test_support::TempFolder::new("module-names").unwrap();
    assert_eq!(free_path(&root, "Chicken"), root.join("Chicken.module.lst"));
    std::fs::write(root.join("Chicken.module.lst"), "").unwrap();
    assert_eq!(
        free_path(&root, "Chicken"),
        root.join("Chicken 2.module.lst")
    );
    // A name no system takes is made one.
    let odd = free_path(&root, "a/b:c");
    assert!(odd.starts_with(&*root));
    assert!(odd.to_string_lossy().ends_with(".module.lst"));
}

#[test]
fn saving_an_assembly_s_older_file_keeps_it_first() {
    let root = reflect::test_support::TempFolder::new("module-save").unwrap();
    let registry = level::builtin::registry().unwrap();
    let path = root.join("crate.module.lst");
    let old = fixture("area/v4/crate.solassembly");
    std::fs::write(&path, &old).unwrap();
    let (file, note) = read(&path, &registry).unwrap();
    assert!(
        note.is_none(),
        "an assembly's file isn't this format's older version"
    );
    let copy = save(Some(&root), &path, &file, &registry).unwrap();
    let copy = copy.expect("the older file is kept");
    assert_eq!(std::fs::read_to_string(copy).unwrap(), old);
    let now = std::fs::read_to_string(&path).unwrap();
    assert_eq!(now, fixture("module/v1/crate.module.lst"));
    // Saved again: current, so nothing more is kept.
    assert!(
        save(Some(&root), &path, &file, &registry)
            .unwrap()
            .is_none()
    );
}
