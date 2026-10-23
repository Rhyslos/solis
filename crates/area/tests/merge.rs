//! Area files merge in Git (decisions 0006 and 0020): each case makes two
//! changes to one area on two "branches" and merges them with Git's own
//! three-way merge, which must succeed and keep both changes. Skipped
//! where Git isn't installed.

// Tests stop at the first surprise, as the other test files do.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use area::builtin::{self, Light, Transform};
use area::{Area, EntityData, text};
use reflect::{EntityId, Registry};
use std::path::Path;
use std::process::Command;

fn registry() -> Registry {
    builtin::registry().unwrap()
}

fn id(text: &str) -> EntityId {
    text.parse().unwrap()
}

/// A small level: twenty lamps with fixed IDs, spread through the ID range,
/// some under others.
fn base() -> Area {
    let mut area = Area::new();
    let mut parent = None;
    for i in 0..20u64 {
        let raw = (i + 1) * (u64::MAX / 21);
        let entity_id = EntityId(reflect::Id::from_raw(raw).unwrap());
        let mut data = EntityData::new(&format!("Lamp {i}"))
            .with(&Transform {
                position: glam::Vec3::new(i as f32, 0.0, 2.0),
                ..Transform::default()
            })
            .with(&Light::point());
        if i % 5 == 0 {
            parent = Some(entity_id);
        } else {
            data.parent = parent;
            data.order = area::order_after(Some(&format!("a{}", i % 5)));
        }
        area.insert(entity_id, data);
    }
    area
}

fn nth(area: &Area, n: usize) -> EntityId {
    area.iter().nth(n).unwrap().0
}

/// Merges `ours` and `theirs` (both changed from `base`) with
/// `git merge-file`, or `None` without Git.
fn merge(base: &Area, ours: &Area, theirs: &Area) -> Option<Result<Area, String>> {
    let git = Command::new("git").arg("--version").output().ok()?;
    if !git.status.success() {
        return None;
    }
    let folder = std::env::temp_dir().join(format!(
        "solis-merge-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&folder).unwrap();
    let write = |name: &str, area: &Area| {
        let path = folder.join(name);
        std::fs::write(&path, text::write(area, &registry(), &|_| None)).unwrap();
        path
    };
    let (o, b, t) = (
        write("ours", ours),
        write("base", base),
        write("theirs", theirs),
    );
    let out = Command::new("git")
        .args(["merge-file", "-p"])
        .args(
            [&o, &b, &t]
                .map(|p: &std::path::PathBuf| p.as_path())
                .map(Path::as_os_str),
        )
        .output()
        .ok()?;
    let _ = std::fs::remove_dir_all(&folder);
    let merged = String::from_utf8(out.stdout).unwrap();
    if !out.status.success() {
        return Some(Err(merged));
    }
    Some(text::read(&merged, &registry()).map_err(|e| e.to_string()))
}

fn strength(area: &Area, entity: EntityId) -> f32 {
    area.get(entity).unwrap().get::<Light>().unwrap().strength
}

#[test]
fn changes_to_entities_side_by_side_merge() {
    let base = base();
    let (a, b) = (nth(&base, 6), nth(&base, 7));
    let mut ours = base.clone();
    ours.get_mut(a).unwrap().set(&Light {
        strength: 5.0,
        ..Light::point()
    });
    let mut theirs = base.clone();
    theirs.get_mut(b).unwrap().set(&Light {
        strength: 7.0,
        ..Light::point()
    });
    let Some(merged) = merge(&base, &ours, &theirs) else {
        return;
    };
    let merged = merged.unwrap();
    assert_eq!((strength(&merged, a), strength(&merged, b)), (5.0, 7.0));
}

#[test]
fn changes_to_different_components_of_one_entity_merge() {
    let base = base();
    let e = nth(&base, 3);
    let mut ours = base.clone();
    ours.get_mut(e).unwrap().set(&Light {
        strength: 9.0,
        ..Light::point()
    });
    let mut theirs = base.clone();
    theirs.get_mut(e).unwrap().set(&Transform {
        position: glam::Vec3::new(0.0, 5.0, 0.0),
        ..Transform::default()
    });
    let Some(merged) = merge(&base, &ours, &theirs) else {
        return;
    };
    let merged = merged.unwrap();
    let entity = merged.get(e).unwrap();
    assert_eq!(entity.get::<Light>().unwrap().strength, 9.0);
    assert_eq!(entity.get::<Transform>().unwrap().position.y, 5.0);
}

#[test]
fn two_new_entities_in_different_places_merge() {
    let base = base();
    let mut ours = base.clone();
    ours.insert(id("1000000000001"), EntityData::new("Crate"));
    let mut theirs = base.clone();
    theirs.insert(id("D000000000001"), EntityData::new("Barrel"));
    let Some(merged) = merge(&base, &ours, &theirs) else {
        return;
    };
    let merged = merged.unwrap();
    assert_eq!(merged.len(), base.len() + 2);
}

#[test]
fn reordered_children_merge() {
    let base = base();
    // Two people each move a different child to the end of its parent's
    // list: only their own `order` lines change.
    let (a, b) = (nth(&base, 1), nth(&base, 12));
    let mut ours = base.clone();
    ours.get_mut(a).unwrap().order = "z".into();
    let mut theirs = base.clone();
    theirs.get_mut(b).unwrap().order = "z".into();
    let Some(merged) = merge(&base, &ours, &theirs) else {
        return;
    };
    let merged = merged.unwrap();
    assert_eq!(merged.get(a).unwrap().order, "z");
    assert_eq!(merged.get(b).unwrap().order, "z");
}

#[test]
fn a_delete_next_to_an_edit_merges() {
    let base = base();
    let (gone, edited) = (nth(&base, 8), nth(&base, 9));
    let mut ours = base.clone();
    ours.remove(gone);
    let mut theirs = base.clone();
    theirs.get_mut(edited).unwrap().name = "Renamed".into();
    let Some(merged) = merge(&base, &ours, &theirs) else {
        return;
    };
    let merged = merged.unwrap();
    assert!(!merged.contains(gone));
    assert_eq!(merged.get(edited).unwrap().name, "Renamed");
}

/// Merges two changed area files as text with `git merge-file`.
fn merge_files(
    base: &text::AreaFile,
    ours: &text::AreaFile,
    theirs: &text::AreaFile,
) -> Option<text::AreaFile> {
    let as_area = |file: &text::AreaFile| text::write_file(file, &registry(), &|_| None);
    let folder = std::env::temp_dir().join(format!("solis-merge-files-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let paths: Vec<std::path::PathBuf> = [("ours", ours), ("base", base), ("theirs", theirs)]
        .iter()
        .map(|(name, file)| {
            let path = folder.join(name);
            std::fs::write(&path, as_area(file)).unwrap();
            path
        })
        .collect();
    let out = Command::new("git")
        .args(["merge-file", "-p"])
        .args(paths.iter().map(|p| p.as_os_str()))
        .output()
        .ok()?;
    let _ = std::fs::remove_dir_all(&folder);
    assert!(
        out.status.success(),
        "conflict:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    Some(text::read_file(&String::from_utf8(out.stdout).unwrap(), &registry()).unwrap())
}

#[test]
fn changes_to_two_copies_of_an_assembly_merge() {
    use area::Origin;
    use text::{AreaFile, Override};
    let assembly: reflect::AssetId = "4WQ0B7M2KD9TS".parse().unwrap();
    let lamp = id("0000000000002");
    let mut base = AreaFile::plain(Area::new());
    for root in ["1000000000001", "C000000000001"] {
        let mut placed = EntityData::new("Lamp post");
        placed.origin = Some(Origin {
            assembly,
            instance: id(root),
            path: Vec::new(),
        });
        base.area.insert(id(root), placed);
    }
    let change = |field: &str, value: reflect::Value| {
        let mut light = reflect::ComponentData::default();
        light.fields.insert(field.into(), value);
        Override {
            components: [("Light".to_string(), light)].into(),
            ..Override::default()
        }
    };
    let mut ours = base.clone();
    ours.overrides.insert(
        (id("1000000000001"), vec![lamp]),
        change("strength", reflect::Value::Float(5.0)),
    );
    let mut theirs = base.clone();
    theirs.overrides.insert(
        (id("C000000000001"), vec![lamp]),
        change("color", reflect::Value::floats(&[1.0, 0.5, 0.0])),
    );
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }
    let merged = merge_files(&base, &ours, &theirs).unwrap();
    assert_eq!(merged.overrides.len(), 2);
}
