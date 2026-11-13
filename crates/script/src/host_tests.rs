//! Random whole numbers, facing along a direction, and old names lent
//! with the new.

use super::*;

#[test]
fn random_whole_numbers_cover_their_range_and_never_leave_it() {
    assert_eq!(random_int(1.0, 6.0, 0.0), 1.0);
    assert_eq!(random_int(1.0, 6.0, 0.999_999_9), 6.0);
    assert_eq!(random_int(6.0, 1.0, 0.0), 1.0, "either way round");
    assert_eq!(random_int(3.0, 3.0, 0.7), 3.0);
    assert_eq!(random_int(1.5, 2.5, 0.9), 2.0, "whole numbers between");
    assert_eq!(
        random_int(1.2, 1.8, 0.5),
        2.0,
        "none between: the low end rounded up"
    );
    assert_eq!(random_int(f64::NAN, 2.0, 0.5), 0.0);
    for i in 0..=100 {
        let x = f64::from(i) / 100.0 * 0.999_999;
        let n = random_int(-3.0, 3.0, x);
        assert!((-3.0..=3.0).contains(&n) && n.fract() == 0.0);
    }
}

#[test]
fn facing_along_turns_forward_to_the_direction() {
    for d in [
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(-2.0, 1.0, 1.0),
        Vec3::new(0.3, -0.2, -0.9),
    ] {
        let forward = facing_along(d) * Vec3::Y;
        assert!((forward - d.normalize()).length() < 1e-5, "{d}: {forward}");
        // Upright: its right side stays level.
        assert!((facing_along(d) * Vec3::X).z.abs() < 1e-5);
    }
    for nonsense in [Vec3::ZERO, Vec3::NAN, Vec3::splat(f32::INFINITY)] {
        assert_eq!(facing_along(nonsense), Quat::IDENTITY);
    }
}

#[test]
fn an_old_name_is_lent_as_the_same_function() {
    let lua = mlua::Lua::new();
    let natives = lua.create_table().unwrap();
    let f = lua.create_function(|_, ()| Ok(7)).unwrap();
    natives.raw_set("spawn", f).unwrap();
    let renamed = [crate::describe::Renamed {
        file: "world",
        old: "spawnAt",
        new: "spawn",
        until: "Phase 12",
    }];
    lend_old_names(&natives, &renamed).unwrap();
    let old: mlua::Function = natives.raw_get("spawnAt").unwrap();
    assert_eq!(old.call::<i32>(()).unwrap(), 7);
}
