use super::*;
use area::EntityData;
use std::f32::consts::FRAC_PI_2;

fn at(position: Vec3) -> Transform {
    Transform {
        position,
        ..Transform::default()
    }
}

fn area_with(entities: &[(&str, Transform, Option<usize>)]) -> (Area, Vec<EntityId>) {
    let mut area = Area::new();
    let mut ids = Vec::new();
    for (name, t, parent) in entities {
        let mut data = EntityData::new(name).with(t);
        data.parent = parent.map(|p| ids[p]);
        ids.push(area.spawn(data));
    }
    (area, ids)
}

fn select(ids: &[EntityId]) -> Selection {
    let mut s = Selection::default();
    for &id in ids {
        s.toggle(id);
    }
    s
}

#[test]
fn the_gizmo_sits_in_the_middle_and_children_move_with_their_parent() {
    let (area, ids) = area_with(&[
        ("A", at(Vec3::new(0.0, 0.0, 0.0)), None),
        ("B", at(Vec3::new(4.0, 2.0, 0.0)), None),
        ("Child of A", at(Vec3::new(1.0, 0.0, 0.0)), Some(0)),
    ]);
    let t = targets(&area, &select(&ids), false).unwrap();
    let moving: Vec<EntityId> = t.entities.iter().map(|e| e.id).collect();
    assert_eq!(moving.len(), 2, "the child moves with A, not twice");
    assert!(!moving.contains(&ids[2]));
    assert!(t.pivot.abs_diff_eq(Vec3::new(2.0, 1.0, 0.0), 1e-6));
    assert_eq!(t.axes, WORLD_AXES);
}

#[test]
fn nothing_to_move_without_a_transform() {
    let mut area = Area::new();
    let id = area.spawn(EntityData::new("Settings"));
    assert!(targets(&area, &select(&[id]), false).is_none());
    assert!(targets(&area, &Selection::default(), false).is_none());
}

#[test]
fn a_move_keeps_rotation_and_scale_exactly() {
    let start = Transform {
        position: Vec3::new(1.0, 2.0, 3.0),
        rotation: Quat::from_rotation_z(0.3),
        scale: Vec3::new(1.0, 2.0, 0.5),
    };
    let (area, ids) = area_with(&[("A", start, None)]);
    let t = targets(&area, &select(&ids), false).unwrap();
    let new = moved(&t, Change::Translate(Vec3::new(5.0, 0.0, 0.0)));
    assert_eq!(new[0].1.position, Vec3::new(6.0, 2.0, 3.0));
    assert_eq!(new[0].1.rotation, start.rotation);
    assert_eq!(new[0].1.scale, start.scale);
}

#[test]
fn a_child_moves_in_the_world_however_its_parent_is_turned() {
    let parent = Transform {
        position: Vec3::new(10.0, 0.0, 0.0),
        rotation: Quat::from_rotation_z(FRAC_PI_2),
        scale: Vec3::splat(2.0),
    };
    let (area, ids) = area_with(&[("P", parent, None), ("C", at(Vec3::X), Some(0))]);
    let t = targets(&area, &select(&[ids[1]]), false).unwrap();
    let before = area.world_matrix(ids[1]).w_axis.truncate();
    let new = moved(&t, Change::Translate(Vec3::new(0.0, 0.0, 4.0)));
    let world = parent.matrix() * new[0].1.matrix();
    assert!(
        world
            .w_axis
            .truncate()
            .abs_diff_eq(before + Vec3::new(0.0, 0.0, 4.0), 1e-5)
    );
}

#[test]
fn turning_several_goes_round_the_pivot() {
    let (area, ids) = area_with(&[
        ("A", at(Vec3::new(-1.0, 0.0, 0.0)), None),
        ("B", at(Vec3::new(1.0, 0.0, 0.0)), None),
    ]);
    let t = targets(&area, &select(&ids), false).unwrap();
    let new = moved(
        &t,
        Change::Rotate {
            axis: Vec3::Z,
            angle: FRAC_PI_2,
        },
    );
    let a = new.iter().find(|(id, _)| *id == ids[0]).unwrap().1;
    assert!(a.position.abs_diff_eq(Vec3::new(0.0, -1.0, 0.0), 1e-5));
    assert!(
        (a.rotation * Vec3::X).abs_diff_eq(Vec3::Y, 1e-5),
        "turned too"
    );
    assert_eq!(a.scale, Vec3::ONE, "scale untouched");
}

#[test]
fn scaling_along_own_axes_multiplies_the_scale_exactly() {
    let start = Transform {
        position: Vec3::new(2.0, 0.0, 0.0),
        rotation: Quat::from_rotation_z(0.7),
        scale: Vec3::new(1.0, 3.0, 1.0),
    };
    let (area, ids) = area_with(&[("A", start, None)]);
    let t = targets(&area, &select(&ids), true).unwrap();
    let new = moved(&t, Change::Scale(Vec3::new(2.0, 1.0, 1.0)));
    assert_eq!(new[0].1.scale, Vec3::new(2.0, 3.0, 1.0));
    assert_eq!(new[0].1.rotation, start.rotation);
    assert!(new[0].1.position.abs_diff_eq(start.position, 1e-6));
}

#[test]
fn scaling_never_squashes_flat() {
    let (area, ids) = area_with(&[("A", at(Vec3::ZERO), None)]);
    let t = targets(&area, &select(&ids), false).unwrap();
    let new = moved(&t, Change::Scale(Vec3::new(0.0, f32::NAN, -0.0)));
    assert!(new[0].1.scale.x > 0.0);
    assert_eq!(new[0].1.scale.y, 1.0, "not a number: unchanged");
}

#[test]
fn snapping_puts_moves_on_the_grid_and_ctrl_flips_it() {
    let (area, ids) = area_with(&[("A", at(Vec3::new(0.3, 0.0, 0.0)), None)]);
    let t = targets(&area, &select(&ids), false).unwrap();
    let on = Snapping {
        grid: true,
        ..Snapping::default()
    };
    let Change::Translate(v) = snapped(Change::Translate(Vec3::new(1.4, 0.0, 0.0)), &t, &on, false)
    else {
        panic!()
    };
    // From 0.3, 1.4 further is 1.7: on the grid that's 2.
    assert!(v.abs_diff_eq(Vec3::new(1.7, 0.0, 0.0), 1e-5), "{v}");
    // Ctrl flips it off for the drag.
    let raw = Change::Translate(Vec3::new(1.4, 0.0, 0.0));
    assert_eq!(snapped(raw, &t, &on, true), raw);
    // And on, when the button is off.
    let off = Snapping::default();
    assert_ne!(snapped(raw, &t, &off, true), raw);
}

#[test]
fn turns_and_scaling_snap_to_their_steps() {
    let (area, ids) = area_with(&[("A", at(Vec3::ZERO), None)]);
    let t = targets(&area, &select(&ids), false).unwrap();
    let on = Snapping {
        angle: true,
        scale: true,
        ..Snapping::default()
    };
    let Change::Rotate { angle, .. } = snapped(
        Change::Rotate {
            axis: Vec3::Z,
            angle: 20f32.to_radians(),
        },
        &t,
        &on,
        false,
    ) else {
        panic!()
    };
    assert!((angle.to_degrees() - 15.0).abs() < 1e-4);
    let Change::Scale(f) = snapped(Change::Scale(Vec3::new(1.26, 0.02, 1.0)), &t, &on, false)
    else {
        panic!()
    };
    assert!(f.abs_diff_eq(Vec3::new(1.3, 0.1, 1.0), 1e-5), "{f}");
}

#[test]
fn saved_settings_out_of_range_come_back_to_the_defaults() {
    let s: Snapping = ron::from_str("(grid: true, grid_step: 7.0, angle_step: 15.0)").unwrap();
    let s = s.sanitized();
    assert!(s.grid);
    assert_eq!(s.grid_step, 1.0);
    assert_eq!(s.angle_step, 15.0);
    assert_eq!(Snapping::default().sanitized(), Snapping::default());
}
