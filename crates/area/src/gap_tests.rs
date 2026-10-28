//! What mutation testing (the Phase 6 hardening milestone) found the area
//! crate's own tests didn't check: how much undo keeps, the camera's
//! projection, a character's capsule, and reading every body and shape.

use crate::builtin::{BodyKind, Camera, CharacterBody, Collider, ColliderShape, Light, RigidBody};
use crate::{Area, Edit, EntityData, History, MergeKey, Origin};
use reflect::{ComponentData, EntityId, Id, Value};

fn id(n: u64) -> EntityId {
    EntityId(Id::from_raw(n).unwrap())
}

fn set(entity: EntityId, value: Value) -> Edit {
    Edit::SetField {
        entity,
        component: "Light".into(),
        field: "strength".into(),
        value: Some(value),
    }
}

fn lamp() -> (Area, EntityId) {
    let mut area = Area::new();
    let lamp = area.spawn(EntityData::new("Lamp").with(&Light::point()));
    (area, lamp)
}

// --- How much an edit keeps ---

#[test]
fn an_edit_weighs_what_it_keeps() {
    let e = id(1);
    let text = |n: usize| set(e, Value::Text("x".repeat(n))).bytes();
    assert_eq!(text(1000) - text(0), 1000);
    let list = |n: usize| set(e, Value::List(vec![Value::Float(1.0); n])).bytes();
    assert_eq!(list(3) - list(1), 2 * std::mem::size_of::<Value>());
    assert!(set(e, Value::Float(1.0)).bytes() > std::mem::size_of::<Edit>());
    let rename = |n: usize| {
        Edit::Rename {
            entity: e,
            name: "n".repeat(n),
        }
        .bytes()
    };
    assert_eq!(rename(40) - rename(0), 40);
    let add = |field: &str| {
        let mut data = ComponentData::default();
        data.fields.insert(field.to_string(), Value::Float(1.0));
        Edit::AddComponent {
            entity: e,
            component: "Light".into(),
            data,
        }
        .bytes()
    };
    assert_eq!(add("abcdefghij") - add("a"), 9);
    let origin = |path: usize| {
        Edit::SetOrigin {
            entity: e,
            origin: Some(Origin {
                assembly: reflect::AssetId(Id::from_raw(9).unwrap()),
                instance: e,
                path: vec![e; path],
            }),
        }
        .bytes()
    };
    assert_eq!(origin(3) - origin(1), 16);
}

#[test]
fn spawning_weighs_each_entity_s_names_and_components() {
    let spawn = |name: &str, field: &str| {
        let mut data = ComponentData::default();
        data.fields
            .insert(field.to_string(), Value::Text("abc".into()));
        let mut entity = EntityData::new(name);
        entity.components.insert("Note".into(), data);
        Edit::Spawn {
            entities: vec![(id(1), entity)],
        }
        .bytes()
    };
    assert_eq!(spawn("a".repeat(101).as_str(), "f") - spawn("a", "f"), 100);
    assert_eq!(spawn("a", "field") - spawn("a", "f"), 4);
    let two = Edit::Spawn {
        entities: vec![(id(1), EntityData::new("A")), (id(2), EntityData::new("B"))],
    };
    let one = Edit::Spawn {
        entities: vec![(id(1), EntityData::new("A"))],
    };
    assert!(two.bytes() > one.bytes());
}

// --- The undo history's memory ---

#[test]
fn the_history_counts_only_what_it_keeps() {
    let (mut area, lamp) = lamp();
    let mut h = History::<()>::default();
    h.apply(
        &mut area,
        "One",
        vec![set(lamp, Value::Float(5.0))],
        None,
        (),
        (),
    )
    .unwrap();
    let one = h.bytes();
    assert!(one > 0);
    h.apply(
        &mut area,
        "Two",
        vec![set(lamp, Value::Float(6.0))],
        None,
        (),
        (),
    )
    .unwrap();
    assert_eq!(h.bytes(), 2 * one, "same-sized steps");
    h.undo(&mut area).unwrap().unwrap();
    assert_eq!(h.bytes(), one, "an undone step no longer counts");
    h.undo(&mut area).unwrap().unwrap();
    assert_eq!(h.bytes(), 0);
    h.redo(&mut area).unwrap().unwrap();
    assert_eq!(h.bytes(), one, "a redone step counts again, once");
}

#[test]
fn a_drag_merged_into_one_step_counts_once() {
    let (mut area, lamp) = lamp();
    let drag = Some(MergeKey {
        target: 1,
        gesture: 7,
    });
    let mut h = History::<()>::default();
    for strength in [2.0, 3.0, 4.0, 5.0] {
        h.apply(
            &mut area,
            "Drag",
            vec![set(lamp, Value::Float(strength))],
            drag,
            (),
            (),
        )
        .unwrap();
    }
    assert_eq!(h.undo_len(), 1);
    let merged = h.bytes();
    h.undo(&mut area).unwrap().unwrap();
    assert_eq!(h.bytes(), 0, "everything it counted goes with it: {merged}");
}

#[test]
fn selections_count_in_a_step_s_weight() {
    let (mut area, lamp) = lamp();
    let mut plain = History::<()>::default();
    plain
        .apply(
            &mut area,
            "One",
            vec![set(lamp, Value::Float(5.0))],
            None,
            (),
            (),
        )
        .unwrap();
    let (mut area, lamp) = self::lamp();
    let mut selected = History::<u32>::default();
    selected
        .apply(
            &mut area,
            "One",
            vec![set(lamp, Value::Float(5.0))],
            None,
            1,
            2,
        )
        .unwrap();
    assert_eq!(
        selected.bytes(),
        plain.bytes() + 8,
        "before and after, 4 each"
    );
}

#[test]
fn the_oldest_steps_go_past_the_limit_and_none_at_it() {
    let step_bytes = {
        let (mut area, lamp) = lamp();
        let mut h = History::<()>::default();
        h.apply(
            &mut area,
            "Step",
            vec![set(lamp, Value::Float(1.0))],
            None,
            (),
            (),
        )
        .unwrap();
        h.bytes()
    };
    let (mut area, lamp) = lamp();
    // Exactly two steps' worth: two fit, a third pushes the oldest out.
    let mut h = History::<()>::with_limit(2 * step_bytes);
    for strength in [1.0, 2.0] {
        h.apply(
            &mut area,
            "Step",
            vec![set(lamp, Value::Float(strength))],
            None,
            (),
            (),
        )
        .unwrap();
    }
    assert_eq!(h.undo_len(), 2, "at the limit, both kept");
    h.apply(
        &mut area,
        "Step",
        vec![set(lamp, Value::Float(3.0))],
        None,
        (),
        (),
    )
    .unwrap();
    assert_eq!(h.undo_len(), 2, "past it, the oldest goes");
    assert!(h.bytes() <= 2 * step_bytes);
    // Too small for even one: the newest is always kept.
    let mut tiny = History::<()>::with_limit(1);
    tiny.apply(
        &mut area,
        "Step",
        vec![set(lamp, Value::Float(4.0))],
        None,
        (),
        (),
    )
    .unwrap();
    assert_eq!(tiny.undo_len(), 1);
}

// --- The camera ---

#[test]
fn a_damaged_camera_is_made_sensible() {
    let d = Camera::default();
    let fix = |fov: f32, near: f32, far: f32| {
        Camera {
            fov,
            near,
            far,
            ..Camera::default()
        }
        .sanitized()
    };
    assert_eq!(
        fix(60.0, 0.5, 300.0),
        Camera {
            fov: 60.0,
            near: 0.5,
            far: 300.0,
            ..Camera::default()
        }
    );
    assert_eq!(fix(f32::NAN, 0.1, 100.0).fov, d.fov);
    assert_eq!(fix(60.0, 0.0, 100.0).near, d.near, "near must be past 0");
    assert_eq!(fix(60.0, -1.0, 100.0).near, d.near);
    assert_eq!(fix(60.0, 5000.0, 9000.0).near, 1000.0, "near at most 1 km");
    // Far must be past near: otherwise 10,000 times near, at least the
    // default.
    assert_eq!(fix(60.0, 0.1, 0.1).far, d.far);
    assert_eq!(fix(60.0, 0.5, 0.2).far, 5000.0);
    assert_eq!(fix(60.0, 0.1, f32::INFINITY).far, d.far);
    assert_eq!(fix(60.0, 0.1, 1.0e9).far, 1.0e7, "at most 10,000 km");
}

#[test]
fn the_projection_is_reversed_depth_for_the_aspect() {
    let camera = Camera {
        fov: 90.0,
        near: 1.0,
        far: 101.0,
        ..Camera::default()
    };
    let p = camera.projection(2.0);
    // A 90° view: 1 across the height, halved across a twice-wide image.
    assert!((p.y_axis.y - 1.0).abs() < 1e-6);
    assert!((p.x_axis.x - 0.5).abs() < 1e-6);
    // Depth 1 at near, 0 at far.
    let depth = |distance: f32| {
        let clip = p * glam::Vec4::new(0.0, 0.0, -distance, 1.0);
        clip.z / clip.w
    };
    assert!((depth(1.0) - 1.0).abs() < 1e-6);
    assert!(depth(101.0).abs() < 1e-6);
    assert!(depth(10.0) > depth(50.0));
    // An image with no width (or a damaged aspect) is drawn square.
    for aspect in [0.0, -1.0, f32::NAN] {
        let square = camera.projection(aspect);
        assert!((square.x_axis.x - square.y_axis.y).abs() < 1e-6);
    }
}

// --- Physics components ---

#[test]
fn a_character_is_a_capsule_standing_on_its_origin() {
    let c = CharacterBody {
        height: 2.0,
        radius: 0.4,
        ..CharacterBody::default()
    }
    .collider();
    assert_eq!(c.shape, ColliderShape::Capsule);
    assert_eq!((c.height, c.radius), (2.0, 0.4));
    assert_eq!(
        c.center,
        glam::Vec3::new(0.0, 0.0, 1.0),
        "feet on the origin"
    );
    assert_eq!(c.friction, 0.0, "slides along walls");
    // Wider than it's tall: as wide as it can be.
    let fat = CharacterBody {
        height: 1.0,
        radius: 3.0,
        ..CharacterBody::default()
    }
    .collider();
    assert_eq!(fat.radius, 0.5);
    // Damaged numbers: the smallest that works.
    let odd = CharacterBody {
        height: f32::NAN,
        radius: f32::INFINITY,
        ..CharacterBody::default()
    }
    .collider();
    assert_eq!(odd.height, 0.001);
    assert!(odd.radius <= odd.height * 0.5 && odd.radius > 0.0);
    let huge = CharacterBody {
        height: 5000.0,
        ..CharacterBody::default()
    }
    .collider();
    assert_eq!(huge.height, 1000.0);
}

#[test]
fn every_body_kind_and_shape_reads_back() {
    for kind in [BodyKind::Dynamic, BodyKind::Kinematic, BodyKind::Fixed] {
        let body = RigidBody {
            kind,
            ..RigidBody::default()
        };
        assert_eq!(
            EntityData::new("B").with(&body).get::<RigidBody>(),
            Some(body)
        );
    }
    for shape in [
        ColliderShape::Box,
        ColliderShape::Sphere,
        ColliderShape::Capsule,
    ] {
        let collider = Collider {
            shape,
            ..Collider::default()
        };
        assert_eq!(
            EntityData::new("C").with(&collider).get::<Collider>(),
            Some(collider)
        );
    }
}
