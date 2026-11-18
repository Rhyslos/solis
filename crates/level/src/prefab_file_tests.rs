use super::*;
use crate::builtin::{Light, Transform, registry};
use crate::document::{EntityData, Origin};
use proptest::prelude::*;
use reflect::{AssetId, ComponentData, Value};

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

fn asset(n: u64) -> AssetId {
    AssetId(reflect::Id::from_raw(n).unwrap())
}

/// The fixtures' folder.
fn fixture(path: &str) -> String {
    let at = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(path);
    std::fs::read_to_string(&at)
        .unwrap_or_else(|e| panic!("{}: {e}", at.display()))
        .replace("\r\n", "\n")
}

/// A lamp post: a post, a lamp under it, and a crate (a placed prefab)
/// with every kind of change a copy can make to its parts.
fn lamp_post() -> LevelFile {
    let mut level = Level::new();
    level.insert(
        id(1),
        EntityData::new("Lamp \"post\"").with(&Transform::default()),
    );
    let mut lamp = EntityData::new("Lamp")
        .with(&Light::point())
        .with(&Transform {
            position: glam::Vec3::Z * 3.0,
            ..Transform::default()
        });
    lamp.parent = Some(id(1));
    lamp.order = "a1".into();
    level.insert(id(2), lamp);
    let mut placed = EntityData::new("Crate");
    placed.parent = Some(id(1));
    placed.order = "a2".into();
    placed.origin = Some(Origin {
        prefab: asset(99),
        instance: id(3),
        path: Vec::new(),
    });
    level.insert(id(3), placed);
    let mut overrides = BTreeMap::new();
    let mut root_changes = Override {
        remove: vec!["Light".into()],
        ..Override::default()
    };
    let mut fields = ComponentData::default();
    fields
        .fields
        .insert("position".into(), Value::floats(&[1.0, 2.0, 3.0]));
    root_changes.components.insert("Transform".into(), fields);
    overrides.insert((id(3), Vec::new()), root_changes);
    overrides.insert(
        (id(3), vec![id(7)]),
        Override {
            removed: true,
            ..Override::default()
        },
    );
    overrides.insert(
        (id(3), vec![id(8)]),
        Override {
            name: Some("Lid; {open}".into()),
            parent: ParentChange::TopLevel,
            order: Some("b0".into()),
            ..Override::default()
        },
    );
    let mut tagged = ComponentData::default();
    tagged
        .fields
        .insert("words".into(), Value::Text("a \"b\"; } {c}\n".into()));
    overrides.insert(
        (id(3), vec![id(9), id(10)]),
        Override {
            parent: ParentChange::Under(id(2)),
            components: [
                ("Note".to_string(), tagged),
                ("Empty".to_string(), ComponentData::default()),
            ]
            .into_iter()
            .collect(),
            ..Override::default()
        },
    );
    LevelFile {
        kind: FileKind::Prefab { root: id(1) },
        level,
        overrides,
    }
}

#[test]
fn a_prefab_reads_back_exactly_and_writes_the_same_again() {
    let registry = registry().unwrap();
    let file = lamp_post();
    let text = write(&file, &registry).unwrap();
    assert!(text.starts_with("-- A prefab."), "{text}");
    assert!(text.contains("fetch \"solis/data/prefab\":[Part];"));
    assert!(text.contains("lock int version = 1;"));
    let read_back = read(&text, &registry).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert_eq!(read_back, file);
    assert_eq!(write(&read_back, &registry).unwrap(), text);
}

#[test]
fn each_setting_is_a_line_of_its_own() {
    let registry = registry().unwrap();
    let text = write(&lamp_post(), &registry).unwrap();
    assert!(
        text.lines()
            .any(|l| l.trim() == "\"Transform.position = [0.0, 0.0, 3.0]\","),
        "{text}"
    );
    // A change with nothing but fields of its own stays on one line.
    assert!(
        text.lines()
            .any(|l| l.contains("removed = true") && l.trim_start().starts_with("new Part {")),
        "{text}"
    );
}

#[test]
fn the_fixture_of_version_1_reads() {
    let registry = registry().unwrap();
    let text = fixture("prefab/v1/crate.prefab.lst");
    let file = read(&text, &registry).unwrap();
    assert!(matches!(file.kind, FileKind::Prefab { .. }));
    assert_eq!(file.level.len(), 2);
    assert_eq!((FORMAT.version_of)(&text), Some(1));
}

#[test]
fn an_assembly_s_older_file_reads_as_the_same_prefab() {
    let registry = registry().unwrap();
    for version in ["v1", "v2", "v4"] {
        let old = fixture(&format!("area/{version}/crate.solassembly"));
        assert!(is_old_layout(&old));
        let file = read(&old, &registry).unwrap();
        let new = write(&file, &registry).unwrap();
        assert!(!is_old_layout(&new));
        assert_eq!(read(&new, &registry).unwrap(), file, "{version}");
    }
    // The new fixture is the version 4 assembly, moved.
    let moved = write(
        &read(&fixture("area/v4/crate.solassembly"), &registry).unwrap(),
        &registry,
    )
    .unwrap();
    assert_eq!(moved, fixture("prefab/v1/crate.prefab.lst"));
}

#[test]
fn an_empty_prefab_is_its_root_alone() {
    let registry = registry().unwrap();
    let file = empty("Chicken");
    let text = write(&file, &registry).unwrap();
    let back = read(&text, &registry).unwrap();
    assert_eq!(back, file);
    let FileKind::Prefab { root } = back.kind else {
        panic!("a level")
    };
    assert_eq!(back.level.get(root).unwrap().name, "Chicken");
}

#[test]
fn a_brush_s_long_lists_are_broken_over_lines_as_they_re_read() {
    let registry = registry().unwrap();
    let mut file = empty("Wall");
    let FileKind::Prefab { root } = file.kind else {
        unreachable!()
    };
    let numbers: Vec<f32> = (0..5000u16).map(|n| f32::from(n) * 0.5).collect();
    let mut data = ComponentData::default();
    data.fields.insert("long".into(), Value::floats(&numbers));
    let entity = file.level.get_mut(root).unwrap();
    entity.components.insert("Numbers".into(), data);
    let text = write(&file, &registry).unwrap();
    assert!(text.lines().any(|l| l.len() > text::MAX_LINE / 2));
    assert_eq!(read(&text, &registry).unwrap(), file);
}

#[test]
fn a_level_isn_t_a_prefab() {
    let registry = registry().unwrap();
    let level = LevelFile::plain(Level::new());
    assert!(write(&level, &registry).is_err());
    let level_text = text::write(&Level::new(), &registry, &|_| None);
    let why = read(&level_text, &registry).unwrap_err();
    assert!(why.message.contains("a level, not a prefab"), "{why}");
}

#[test]
fn a_newer_version_is_refused_and_never_half_read() {
    let registry = registry().unwrap();
    let text = write(&lamp_post(), &registry)
        .unwrap()
        .replace("lock int version = 1;", "lock int version = 2;");
    let why = read(&text, &registry).unwrap_err();
    assert!(why.message.contains("newer version"), "{why}");
}

/// A prefab file with one part whose fields are `fields`.
fn one_part(fields: &str) -> String {
    format!(
        "fetch \"solis/data/prefab\":[Part];\n\nlock int version = 1;\n\
         lock string root = \"0000000000001\";\nlock Part array parts = [\n    \
         new Part {{ {fields} }},\n];\nend\n"
    )
}

#[test]
fn a_part_can_t_say_more_than_its_own_block() {
    let registry = registry().unwrap();
    let ok =
        one_part("id = \"0000000000001\", name = \"A\", settings = [\"Light.strength = 2.0\"]");
    assert!(read(&ok, &registry).is_ok(), "{:?}", read(&ok, &registry));
    for (fields, says) in [
        (
            "id = \"0000000000001\", settings = [\"Light.strength = 2.0;\\n}\\nentity 0000000000002 \\{\"]",
            "isn't one field's line",
        ),
        (
            "id = \"0000000000001\", settings = [\"Light.strength = 2.0; name = \\\"B\\\"\"]",
            "isn't one field's line",
        ),
        (
            "id = \"0000000000001\", settings = [\"= 2.0\"]",
            "isn't one field's line",
        ),
        (
            "id = \"0000000000001\", settings = [\"Light.name = \\\"open\"]",
            "isn't one field's line",
        ),
        ("id = \"00000 1\"", "isn't an entity's ID"),
        ("id = \"\"", "isn't an entity's ID"),
        ("id = \"0000000000001//2\"", "isn't an entity's ID"),
        ("id = \"0000000000001\", parent = \"x y\"", "isn't an ID"),
        ("id = \"0000000000001\", prefab = \"\"", "isn't an ID"),
        (
            "id = \"0000000000001\", settings = [\"Light.strength = nope\"]",
            "the part 0000000000001",
        ),
    ] {
        let text = one_part(fields);
        let why = read(&text, &registry).unwrap_err();
        assert!(why.message.contains(says), "{fields}: {why}");
        assert!(why.line > 0, "{fields}: {why}");
    }
}

#[test]
fn a_root_missing_or_wrong_is_said() {
    let registry = registry().unwrap();
    let text =
        one_part("id = \"0000000000001\"").replace("lock string root = \"0000000000001\";\n", "");
    assert!(read(&text, &registry).unwrap_err().message.contains("root"));
    let text = one_part("id = \"0000000000002\"");
    assert!(read(&text, &registry).unwrap_err().message.contains("root"));
}

#[test]
fn what_isn_t_data_never_reaches_lust() {
    let registry = registry().unwrap();
    let text = one_part("id = \"0000000000001\"").replace("end\n", "class Part {}\nend\n");
    assert!(read(&text, &registry).is_err());
    assert!(read("", &registry).is_err());
    assert!(read("lock int version = 1;", &registry).is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Damaged prefab files are refused or read, never a crash.
    #[test]
    fn damaged_files_never_crash(cuts in proptest::collection::vec((0usize..4000, any::<u8>()), 1..8)) {
        let registry = registry().unwrap();
        let mut bytes = write(&lamp_post(), &registry).unwrap().into_bytes();
        for (at, b) in cuts {
            if bytes.is_empty() {
                break;
            }
            let at = at % bytes.len();
            match b % 3 {
                0 => bytes[at] = b,
                1 => {
                    bytes.remove(at);
                }
                _ => bytes.insert(at, b),
            }
        }
        let text = String::from_utf8_lossy(&bytes);
        let _ = read(&text, &registry);
    }

    /// Any names, orders and settings written read back the same.
    #[test]
    fn anything_written_reads_back(
        names in proptest::collection::vec("[ -~é\\n\\t]{0,12}", 1..6),
        strengths in proptest::collection::vec(-1.0e6f32..1.0e6, 1..6),
    ) {
        let registry = registry().unwrap();
        let mut file = empty("Root");
        let FileKind::Prefab { root } = file.kind else { unreachable!() };
        for (n, (name, strength)) in names.iter().zip(&strengths).enumerate() {
            let mut light = Light::point();
            light.strength = *strength;
            let mut e = EntityData::new(name).with(&light);
            e.parent = Some(root);
            e.order = format!("a{n}");
            file.level.insert(id(100 + n as u64), e);
        }
        let text = write(&file, &registry).unwrap();
        prop_assert_eq!(read(&text, &registry).unwrap(), file);
    }
}
