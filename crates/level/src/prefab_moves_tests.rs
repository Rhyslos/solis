//! Parts of placed prefabs moved and deleted, then saved and opened
//! again: whatever is done to a copy, the level that opens is the level
//! that was saved (the review of Phase 9, BUG-56).

use super::*;
use crate::builtin::Transform;
use crate::edit::Edit;
use crate::text;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

fn asset(n: u64) -> AssetId {
    AssetId(reflect::Id::from_raw(n).unwrap())
}

const STALL: u64 = 100;
const MARKET: u64 = 200;

fn part(name: &str, parent: Option<u64>, x: f32) -> EntityData {
    let mut data = EntityData::new(name).with(&Transform {
        position: glam::Vec3::new(x, 0.0, 0.0),
        ..Transform::default()
    });
    data.parent = parent.map(id);
    data
}

fn copy_of(prefab: u64, name: &str, parent: Option<u64>, root: u64) -> EntityData {
    let mut data = part(name, parent, root as f32);
    data.components.clear();
    data.origin = Some(Origin {
        prefab: asset(prefab),
        instance: id(root),
        path: Vec::new(),
    });
    data
}

/// A stall (1): a counter (2) with a till (3) on it, a drawer (4) in the
/// till, and an awning (5).
fn stall() -> LevelFile {
    let mut level = Level::new();
    level.insert(id(1), part("Stall", None, 0.0));
    level.insert(id(2), part("Counter", Some(1), 1.0));
    level.insert(id(3), part("Till", Some(2), 2.0));
    level.insert(id(4), part("Drawer", Some(3), 3.0));
    level.insert(id(5), part("Awning", Some(1), 4.0));
    LevelFile {
        kind: FileKind::Prefab { root: id(1) },
        level,
        overrides: Default::default(),
    }
}

/// A market (10): two stalls (11 and 12) and a well (13).
fn market() -> LevelFile {
    let mut level = Level::new();
    level.insert(id(10), part("Market", None, 0.0));
    level.insert(id(11), copy_of(STALL, "North stall", Some(10), 11));
    level.insert(id(12), copy_of(STALL, "South stall", Some(10), 12));
    level.insert(id(13), part("Well", Some(10), 5.0));
    LevelFile {
        kind: FileKind::Prefab { root: id(10) },
        level,
        overrides: Default::default(),
    }
}

fn library() -> impl Fn(AssetId) -> Option<Arc<LevelFile>> {
    library_with(stall())
}

/// The library, with `stall` as the stall's file (a prefab changed
/// since the town was saved).
fn library_with(stall: LevelFile) -> impl Fn(AssetId) -> Option<Arc<LevelFile>> {
    let stall = Arc::new(stall);
    let market = Arc::new(market());
    move |wanted| match wanted {
        a if a == asset(STALL) => Some(Arc::clone(&stall)),
        a if a == asset(MARKET) => Some(Arc::clone(&market)),
        _ => None,
    }
}

/// A town: a stall (40) and a market (50) placed in it, and a shed (60)
/// with a crate (61) in it. The stall's copy still has a change to a
/// bell (9) the stall no longer has, kept as an entity under it.
fn town() -> Level {
    let mut level = Level::new();
    level.insert(id(40), copy_of(STALL, "Stall", None, 40));
    level.insert(id(50), copy_of(MARKET, "Market", None, 50));
    level.insert(id(60), part("Shed", None, 6.0));
    level.insert(id(61), part("Crate", Some(60), 7.0));
    let mut file = LevelFile::plain(level);
    let bell = Override {
        name: Some("Bell".into()),
        ..Override::default()
    };
    file.overrides.insert((id(40), vec![id(9)]), bell);
    let lib = library();
    Expander::new(&lib).expand(&file).unwrap()
}

/// The stall's part `n` in the copy at 40.
fn of_stall(n: u64) -> EntityId {
    derive(id(40), &[id(n)])
}

/// Part `n` of the market's stall `stall` (11 or 12), in the copy at 50.
fn of_market(stall: u64, n: u64) -> EntityId {
    derive(id(50), &[id(stall), id(n)])
}

/// `level` saved and opened again: what its file's text expands to.
fn saved_and_opened(level: &Level) -> Level {
    let lib = library();
    opened_with(&saved(level, &lib), &lib)
}

/// `level` as its file's text, with the prefabs of `lib`.
fn saved(level: &Level, lib: Lookup<'_>) -> String {
    let registry = crate::builtin::registry().unwrap();
    let file = Expander::new(lib).collapse(level, FileKind::Level).unwrap();
    let written = text::write_file(&file, &registry, &|_| None);
    assert_eq!(text::read_file(&written, &registry), Ok(file), "{written}");
    written
}

/// What a file's text opens as, with the prefabs of `lib`.
fn opened_with(written: &str, lib: Lookup<'_>) -> Level {
    let registry = crate::builtin::registry().unwrap();
    let read = text::read_file(written, &registry).unwrap();
    let mut expander = Expander::new(lib);
    let opened = expander.expand(&read).unwrap();
    assert_eq!(expander.mended(), [], "{written}");
    opened
}

/// Deletes `entity` and everything under it, as the editor does.
fn delete(level: &mut Level, entity: EntityId) -> Vec<Edit> {
    level.apply_all(despawn(level, &[entity])).unwrap()
}

/// Moves `entity` under `parent` (or to the top level), last among its
/// children, as dragging it in the Level panel does. Whether it could be.
fn move_to(level: &mut Level, entity: EntityId, parent: Option<EntityId>) -> bool {
    let order = level.order_at_end(parent);
    let edit = Edit::Reparent {
        entity,
        parent,
        order,
    };
    level.apply(edit).is_ok()
}

#[test]
fn a_part_moved_out_stays_when_the_part_it_was_under_is_deleted() {
    // BUG-56: the till dragged out of the counter, then the counter
    // deleted. Opened again, the till was gone: a removed part took
    // everything the prefab has under it, wherever this copy had put it.
    for new_parent in [None, Some(id(60)), Some(of_stall(5)), Some(id(40))] {
        let mut level = town();
        let (counter, till, drawer) = (of_stall(2), of_stall(3), of_stall(4));
        assert!(move_to(&mut level, till, new_parent));
        delete(&mut level, counter);
        assert!(level.contains(till) && level.contains(drawer));
        let opened = saved_and_opened(&level);
        assert!(opened.contains(till), "the till, moved to {new_parent:?}");
        assert!(opened.contains(drawer), "and the drawer still in it");
        assert!(!opened.contains(counter));
        assert_eq!(opened, level, "moved to {new_parent:?}");
    }
}

#[test]
fn a_part_moved_out_stays_where_it_was_put_when_its_prefab_loses_the_part_it_was_under() {
    // The prefab changed after the town was saved: the counter is
    // gone from the stall's own file, and the till is on the stall.
    let mut level = town();
    let (till, drawer) = (of_stall(3), of_stall(4));
    assert!(move_to(&mut level, till, Some(id(60))));
    let written = saved(&level, &library());
    let mut changed = stall();
    changed.level.remove(id(2));
    changed.level.get_mut(id(3)).unwrap().parent = Some(id(1));
    let lib = library_with(changed);
    let opened = opened_with(&written, &lib);
    assert_eq!(opened.get(till).unwrap().parent, Some(id(60)));
    assert_eq!(opened.get(drawer).unwrap().parent, Some(till));
    assert!(!opened.contains(of_stall(2)));
    assert_eq!(opened_with(&saved(&opened, &lib), &lib), opened);
    // The counter gone with everything on it: the till's changes are
    // kept as an entity, where the till was put; the drawer, which this
    // copy never changed, is gone with the prefab's.
    let mut changed = stall();
    for gone in [2, 3, 4] {
        changed.level.remove(id(gone));
    }
    let lib = library_with(changed);
    let opened = opened_with(&written, &lib);
    let kept = opened.get(till).unwrap();
    assert_eq!(
        (kept.name.as_str(), kept.parent),
        ("Missing part", Some(id(60)))
    );
    assert!(!opened.contains(drawer));
    assert_eq!(opened_with(&saved(&opened, &lib), &lib), opened);
}

#[test]
fn a_part_goes_with_what_it_was_moved_under() {
    // As any child does: the till moved under the awning (another part),
    // the shed (outside the copy) or the market's well (a part of
    // another copy), and that one deleted.
    for new_parent in [of_stall(5), id(60), derive(id(50), &[id(13)])] {
        let mut level = town();
        let (counter, till, drawer) = (of_stall(2), of_stall(3), of_stall(4));
        assert!(move_to(&mut level, till, Some(new_parent)));
        delete(&mut level, new_parent);
        assert!(!level.contains(till) && !level.contains(drawer));
        assert!(level.contains(counter));
        let opened = saved_and_opened(&level);
        assert!(!opened.contains(till) && !opened.contains(drawer));
        assert_eq!(opened, level, "moved under {new_parent}");
    }
    // The drawer taken out of the till first stays when the till goes
    // with the awning.
    let mut level = town();
    assert!(move_to(&mut level, of_stall(4), None));
    assert!(move_to(&mut level, of_stall(3), Some(of_stall(5))));
    delete(&mut level, of_stall(5));
    assert!(level.contains(of_stall(4)) && !level.contains(of_stall(3)));
    assert_eq!(saved_and_opened(&level), level);
}

#[test]
fn parts_of_a_copy_inside_a_copy_are_moved_and_deleted_the_same() {
    // The market's north stall (11) is a copy inside the market's copy.
    let (stall, counter) = (derive(id(50), &[id(11)]), of_market(11, 2));
    let (till, drawer) = (of_market(11, 3), of_market(11, 4));
    for new_parent in [
        None,
        Some(id(60)),
        Some(of_market(12, 5)),
        Some(of_stall(5)),
    ] {
        let mut level = town();
        assert!(move_to(&mut level, till, new_parent));
        assert_eq!(saved_and_opened(&level), level);
        // The counter it was on deleted: it stays, the drawer in it.
        delete(&mut level, counter);
        let opened = saved_and_opened(&level);
        assert!(opened.contains(till) && opened.contains(drawer));
        assert_eq!(opened, level, "moved to {new_parent:?}");
        // Its whole stall deleted: still there, part of the market.
        delete(&mut level, stall);
        let opened = saved_and_opened(&level);
        let kept = opened.get(till).unwrap();
        assert_eq!(kept.origin.as_ref().unwrap().instance, id(50));
        assert_eq!(kept.parent, new_parent);
        assert_eq!(opened, level, "moved to {new_parent:?}");
        // What it was moved under deleted: gone with it.
        if let Some(parent) = new_parent {
            delete(&mut level, parent);
            assert!(!level.contains(till));
            assert_eq!(saved_and_opened(&level), level, "moved to {new_parent:?}");
        }
    }
}

#[test]
fn deleting_a_copy_leaves_a_part_moved_out_of_it_as_an_ordinary_entity() {
    let mut level = town();
    let before = level.clone();
    let (till, drawer, bell) = (of_stall(3), of_stall(4), of_stall(9));
    assert!(move_to(&mut level, till, None));
    assert!(move_to(&mut level, bell, Some(id(60))));
    let moved = level.clone();
    let undo = delete(&mut level, id(40));
    // The copy is gone, but for what was no longer under it: the till,
    // the drawer in it, and the bell. They're part of nothing now.
    for left in [till, drawer, bell] {
        assert_eq!(level.get(left).unwrap().origin, None);
    }
    assert!(!level.contains(id(40)) && !level.contains(of_stall(2)));
    assert_eq!(level.len(), moved.len() - 3);
    assert_eq!(saved_and_opened(&level), level);
    // Undo brings the copy back, and they're its parts again.
    level.apply_all(undo).unwrap();
    assert_eq!(level, moved);
    // The same when what goes is something the copy is under.
    assert!(move_to(&mut level, id(40), Some(id(61))));
    delete(&mut level, id(60));
    assert_eq!(level.get(till).unwrap().origin, None);
    assert!(!level.contains(bell), "it was in the shed");
    assert_eq!(saved_and_opened(&level), level);
    // Nothing of a copy is touched by a delete that takes no copy: one
    // edit, as ever.
    let only = despawn(&before, &[id(61)]);
    assert_eq!(only, [Edit::Despawn { root: id(61) }]);
    let part = despawn(&before, &[of_stall(2)]);
    assert_eq!(part, [Edit::Despawn { root: of_stall(2) }]);
    // Several at once: the till in the shed goes with the shed, so it
    // isn't made ordinary first (it wouldn't be there to be); the bell,
    // out at the top level, is.
    let mut level = before.clone();
    assert!(move_to(&mut level, till, Some(id(60))));
    assert!(move_to(&mut level, bell, None));
    let both = despawn(&level, &[id(60), id(40)]);
    let ordinary = Edit::SetOrigin {
        entity: bell,
        origin: None,
    };
    let (shed, stall) = (
        Edit::Despawn { root: id(60) },
        Edit::Despawn { root: id(40) },
    );
    assert_eq!(both, [ordinary, shed, stall]);
    level.apply_all(both).unwrap();
    assert!(!level.contains(till) && level.contains(bell));
    assert_eq!(saved_and_opened(&level), level);
}

#[test]
fn whatever_is_moved_and_deleted_the_level_saved_is_the_level_opened() {
    // Every seed is a run of moves and deletes on the town: any entity
    // under any other or out to the top level, any entity deleted (parts,
    // copies' roots, the copy inside a copy, the part the prefab lost,
    // ordinary entities). After each, the level is saved and opened again.
    let mut steps = 0;
    for seed in 1..=150u64 {
        let mut level = town();
        let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let mut next = |below: usize| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 16) as usize % below
        };
        let mut done = Vec::new();
        for _ in 0..12 {
            let ids: Vec<EntityId> = level.iter().map(|(id, _)| id).collect();
            if ids.is_empty() {
                break;
            }
            let entity = ids[next(ids.len())];
            let name = level.get(entity).unwrap().name.clone();
            match next(10) {
                0..=4 => {
                    let parent = ids[next(ids.len())];
                    let under = level.get(parent).unwrap().name.clone();
                    let moved = move_to(&mut level, entity, Some(parent));
                    done.push(format!("{name} under {under}: {moved}"));
                }
                5..=6 => {
                    move_to(&mut level, entity, None);
                    done.push(format!("{name} to the top level"));
                }
                _ => {
                    delete(&mut level, entity);
                    done.push(format!("{name} deleted"));
                }
            }
            assert_eq!(saved_and_opened(&level), level, "seed {seed}: {done:#?}");
            steps += 1;
        }
    }
    assert!(steps > 1_200, "{steps} steps");
}
