//! The camera's tests: its picture, flying with the mouse, keys and
//! wheel, saving it, framing, and flat views.

use super::*;

fn close(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-3
}

fn frame(input: FlyInput) -> FlyInput {
    FlyInput {
        dt: 0.1,
        view_height: 600.0,
        ..input
    }
}

#[test]
fn the_new_camera_looks_at_the_origin() {
    let camera = Camera::default();
    let to_target = (Vec3::new(0.0, 0.0, 1.0) - camera.position).normalize();
    assert!(close(camera.forward(), to_target));
    assert_eq!(camera.speed, DEFAULT_SPEED);
}

#[test]
fn the_target_lands_in_the_middle_of_the_picture() {
    let camera = Camera::default();
    let clip = camera.view_projection(16.0 / 9.0) * Vec3::new(0.0, 0.0, 1.0).extend(1.0);
    let ndc = clip.truncate() / clip.w;
    assert!(ndc.x.abs() < 1e-4 && ndc.y.abs() < 1e-4, "{ndc}");
    assert!(ndc.z > 0.0 && ndc.z < 1.0);
}

#[test]
fn depth_is_reversed_one_near_and_zero_far() {
    let camera = Camera {
        position: Vec3::ZERO,
        heading: 0.0,
        tilt: 0.0,
        ..Camera::default()
    };
    let vp = camera.view_projection(1.0);
    let depth = |distance: f32| {
        let c = vp * Vec3::new(0.0, distance, 0.0).extend(1.0);
        c.z / c.w
    };
    assert!((depth(Camera::NEAR) - 1.0).abs() < 1e-5);
    assert!(depth(Camera::FAR).abs() < 1e-5);
    assert!(depth(10.0) > depth(100.0), "nearer is deeper");
    // A kilometre away, a few centimetres still tell surfaces apart.
    let gap = depth(1000.0) - depth(1000.05);
    assert!(gap > 4.0 * f32::EPSILON * depth(1000.0), "{gap}");
}

#[test]
fn a_box_too_big_to_measure_leaves_the_camera_as_it_was() {
    let mut camera = Camera::default();
    let before = camera;
    camera.frame(Vec3::splat(-3e38), Vec3::splat(3e38));
    assert_eq!(camera, before);
    assert!(camera.view_projection(1.0).is_finite());
}

#[test]
fn up_in_the_world_is_up_on_screen() {
    let camera = Camera::default();
    let vp = camera.view_projection(1.0);
    let screen = |p: Vec3| {
        let c = vp * p.extend(1.0);
        c.truncate() / c.w
    };
    assert!(screen(Vec3::new(0.0, 0.0, 3.0)).y > screen(Vec3::new(0.0, 0.0, 1.0)).y);
}

#[test]
fn right_forward_and_up_are_square_to_each_other() {
    let mut camera = Camera::default();
    for (heading, tilt) in [(0.0, 0.0), (1.0, 0.5), (-2.5, -1.2), (3.0, MAX_TILT)] {
        camera.heading = heading;
        camera.tilt = tilt;
        let (f, r, u) = (camera.forward(), camera.right(), camera.up());
        assert!(f.dot(r).abs() < 1e-5 && f.dot(u).abs() < 1e-5 && r.dot(u).abs() < 1e-5);
        assert!((f.length() - 1.0).abs() < 1e-5 && (u.length() - 1.0).abs() < 1e-5);
        assert!(u.z >= 0.0, "up never points into the ground");
    }
}

#[test]
fn moving_the_mouse_right_turns_right_and_down_looks_down() {
    let mut camera = Camera::default();
    let (heading, tilt) = (camera.heading, camera.tilt);
    camera.fly(&frame(FlyInput {
        look: Vec2::new(50.0, 30.0),
        ..Default::default()
    }));
    assert!(camera.heading > heading);
    assert!(camera.tilt < tilt);
}

#[test]
fn looking_never_flips_over_the_top() {
    let mut camera = Camera::default();
    camera.fly(&frame(FlyInput {
        look: Vec2::new(0.0, -1.0e6),
        ..Default::default()
    }));
    assert_eq!(camera.tilt, MAX_TILT);
    camera.fly(&frame(FlyInput {
        look: Vec2::new(0.0, 1.0e6),
        ..Default::default()
    }));
    assert_eq!(camera.tilt, -MAX_TILT);
}

#[test]
fn heading_stays_in_range_after_many_turns() {
    let mut camera = Camera::default();
    for _ in 0..1000 {
        camera.fly(&frame(FlyInput {
            look: Vec2::new(400.0, 0.0),
            ..Default::default()
        }));
    }
    assert!(camera.heading.abs() <= std::f32::consts::PI + 1e-5);
}

#[test]
fn w_moves_forward_at_the_chosen_speed() {
    let mut camera = Camera::default();
    let start = camera.position;
    camera.fly(&frame(FlyInput {
        movement: Vec3::Y,
        ..Default::default()
    }));
    let expected = start + camera.forward() * camera.speed_mps() * 0.1;
    assert!(close(camera.position, expected));
}

#[test]
fn sideways_stays_level_and_e_goes_straight_up() {
    let mut camera = Camera::default();
    let start = camera.position;
    camera.fly(&frame(FlyInput {
        movement: Vec3::X,
        ..Default::default()
    }));
    assert!((camera.position.z - start.z).abs() < 1e-5);
    let side = camera.position;
    camera.fly(&frame(FlyInput {
        movement: Vec3::Z,
        ..Default::default()
    }));
    assert!(close(
        camera.position - side,
        Vec3::Z * camera.speed_mps() * 0.1
    ));
}

#[test]
fn diagonal_movement_is_not_faster() {
    let mut straight = Camera::default();
    let mut diagonal = Camera::default();
    let start = straight.position;
    straight.fly(&frame(FlyInput {
        movement: Vec3::X,
        ..Default::default()
    }));
    diagonal.fly(&frame(FlyInput {
        movement: Vec3::new(1.0, 1.0, 1.0),
        ..Default::default()
    }));
    let a = (straight.position - start).length();
    let b = (diagonal.position - start).length();
    assert!((a - b).abs() < 1e-4);
}

#[test]
fn shift_moves_faster() {
    let mut normal = Camera::default();
    let mut boosted = Camera::default();
    let start = normal.position;
    let input = FlyInput {
        movement: Vec3::Y,
        ..Default::default()
    };
    normal.fly(&frame(input));
    boosted.fly(&frame(FlyInput {
        boost: true,
        ..input
    }));
    let ratio = (boosted.position - start).length() / (normal.position - start).length();
    assert!((ratio - BOOST).abs() < 1e-3);
}

#[test]
fn the_wheel_while_flying_changes_speed_within_its_steps() {
    let mut camera = Camera::default();
    camera.fly(&frame(FlyInput {
        speed_notches: 2,
        ..Default::default()
    }));
    assert_eq!(camera.speed, DEFAULT_SPEED + 2);
    camera.fly(&frame(FlyInput {
        speed_notches: 100,
        ..Default::default()
    }));
    assert_eq!(camera.speed, 8);
    camera.fly(&frame(FlyInput {
        speed_notches: -100,
        ..Default::default()
    }));
    assert_eq!(camera.speed, 1);
}

#[test]
fn a_saved_speed_out_of_range_is_still_usable() {
    let camera = Camera {
        speed: 0,
        ..Camera::default()
    };
    assert_eq!(camera.speed_mps(), SPEEDS[0]);
    let camera = Camera {
        speed: 200,
        ..Camera::default()
    };
    assert_eq!(camera.speed_mps(), SPEEDS[7]);
}

#[test]
fn the_wheel_moves_forward_and_back() {
    let mut camera = Camera::default();
    let start = camera.position;
    camera.fly(&frame(FlyInput {
        dolly_notches: 1.0,
        ..Default::default()
    }));
    assert!((camera.position - start).dot(camera.forward()) > 0.0);
    camera.fly(&frame(FlyInput {
        dolly_notches: -1.0,
        ..Default::default()
    }));
    assert!(close(camera.position, start));
}

#[test]
fn dragging_right_slides_the_camera_left() {
    let mut camera = Camera::default();
    let start = camera.position;
    camera.fly(&frame(FlyInput {
        pan: Vec2::new(100.0, 0.0),
        ..Default::default()
    }));
    assert!((camera.position - start).dot(camera.right()) < 0.0);
    // The view direction doesn't change.
    assert!(close(camera.forward(), Camera::default().forward()));
}

#[test]
fn a_long_hitch_does_not_throw_the_camera_away() {
    let mut camera = Camera::default();
    let start = camera.position;
    camera.fly(&FlyInput {
        movement: Vec3::Y,
        dt: 30.0,
        ..Default::default()
    });
    assert!((camera.position - start).length() <= camera.speed_mps() * MAX_STEP + 1e-4);
}

#[test]
fn broken_input_leaves_the_camera_alone() {
    let mut camera = Camera::default();
    let before = camera;
    camera.fly(&FlyInput {
        look: Vec2::NAN,
        movement: Vec3::new(f32::NAN, 0.0, 0.0),
        dolly_notches: f32::INFINITY,
        pan: Vec2::new(f32::NAN, 1.0),
        dt: f32::NAN,
        view_height: 0.0,
        ..Default::default()
    });
    assert_eq!(camera, before);
}

#[test]
fn the_grid_centres_where_the_view_meets_the_ground() {
    let mut camera = Camera {
        position: Vec3::new(0.0, 0.0, 10.0),
        ..Camera::default()
    };
    camera.look_at(Vec3::new(0.0, 10.0, 0.0));
    assert!(close(camera.focus(), Vec3::new(0.0, 10.0, 0.0)));
    // Looking up, the grid stays under the camera.
    camera.look_at(Vec3::new(0.0, 10.0, 20.0));
    assert!(close(camera.focus(), Vec3::new(0.0, 0.0, 0.0)));
    // Below the ground too.
    camera.position.z = -5.0;
    assert!(close(camera.focus(), Vec3::ZERO));
}

#[test]
fn a_bad_aspect_does_not_break_the_picture() {
    let camera = Camera::default();
    for aspect in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(camera.view_projection(aspect).is_finite());
    }
}

#[test]
fn the_camera_saves_and_loads() {
    let camera = Camera {
        speed: 6,
        position: Vec3::new(1.0, 2.0, 3.0),
        ..Camera::default()
    };
    let text = ron::to_string(&camera).unwrap();
    let back: Camera = ron::from_str(&text).unwrap();
    assert_eq!(back, camera);
    // Missing fields (an older save) fall back to the defaults.
    let partial: Camera = ron::from_str("(speed: 2)").unwrap();
    assert_eq!(partial.speed, 2);
    assert_eq!(partial.position, Camera::default().position);
}

fn usable(camera: &Camera) -> bool {
    camera.position.is_finite()
        && camera.heading.is_finite()
        && camera.tilt.abs() <= MAX_TILT
        && FOV_RANGE.contains(&camera.fov_y)
        && (1..=8).contains(&camera.speed)
        && camera.view_projection(1.5).is_finite()
}

#[test]
fn a_damaged_save_is_made_usable() {
    let broken = Camera {
        position: Vec3::new(f32::NAN, 1.0, f32::INFINITY),
        heading: f32::NAN,
        tilt: 50.0,
        fov_y: 0.0,
        speed: 0,
        kind: ViewKind::Top,
        flat_height: f32::NAN,
        zoom: f32::NAN,
    };
    let fixed = broken.sanitized();
    assert_eq!(fixed.flat_height, Camera::default().flat_height);
    assert_eq!(fixed.zoom, FIRST_ZOOM);
    assert_eq!(fixed.kind, ViewKind::Top);
    assert!(usable(&fixed));
    assert_eq!(fixed.position, Camera::default().position);
    assert_eq!(fixed.tilt, MAX_TILT);
    assert_eq!(fixed.speed, 1);
    // A good camera is left as it is.
    let good = Camera::default();
    assert_eq!(good.sanitized(), good);
}

/// Random damage to a saved camera, thousands of times: reading it
/// never crashes, and whatever reads back is made usable.
#[test]
fn random_damage_to_a_saved_camera_never_breaks_it() {
    let saved = ron::to_string(&Camera::default()).unwrap();
    let bytes = saved.as_bytes();
    let replacements: &[&[u8]] = &[
        b"NaN", b"inf", b"-inf", b"1e39", b"-0", b"999", b"(", b")", b",", b"\"", b"",
    ];
    let mut seed: u64 = 7;
    let mut next = |n: usize| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) as usize % n.max(1)
    };
    for _ in 0..3_000 {
        let mut damaged = bytes.to_vec();
        for _ in 0..1 + next(3) {
            let at = next(damaged.len());
            let cut = next(4).min(damaged.len() - at);
            let with = replacements[next(replacements.len())];
            damaged.splice(at..at + cut, with.iter().copied());
        }
        let text = String::from_utf8_lossy(&damaged);
        if let Ok(camera) = ron::from_str::<Camera>(&text) {
            assert!(usable(&camera.sanitized()), "{text}");
        }
    }
}

#[test]
fn framing_a_box_brings_all_of_it_into_view() {
    for (min, max) in [
        (Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 2.0)),
        (Vec3::new(100.0, 50.0, 0.0), Vec3::new(400.0, 90.0, 30.0)),
        (Vec3::splat(-0.01), Vec3::splat(0.01)),
    ] {
        let mut camera = Camera::default();
        camera.frame(min, max);
        let vp = camera.view_projection(1.0);
        for corner in 0..8 {
            let p = Vec3::new(
                if corner & 1 == 0 { min.x } else { max.x },
                if corner & 2 == 0 { min.y } else { max.y },
                if corner & 4 == 0 { min.z } else { max.z },
            );
            let clip = vp * p.extend(1.0);
            let ndc = clip.truncate() / clip.w;
            assert!(
                clip.w > 0.0 && ndc.x.abs() <= 1.0 && ndc.y.abs() <= 1.0,
                "{p} -> {ndc}"
            );
        }
    }
}

#[test]
fn framing_picks_a_speed_to_suit_the_size() {
    let mut small = Camera::default();
    small.frame(Vec3::splat(-0.5), Vec3::splat(0.5));
    let mut big = Camera::default();
    big.frame(Vec3::splat(-200.0), Vec3::splat(200.0));
    assert!(small.speed < big.speed);
    assert_eq!(big.speed, 8);
}

#[test]
fn framing_looks_down_when_the_camera_looked_up() {
    let mut camera = Camera {
        tilt: 0.5,
        ..Camera::default()
    };
    camera.frame(Vec3::ZERO, Vec3::ONE);
    assert!(camera.tilt < 0.0);
    let before = camera;
    camera.frame(Vec3::NAN, Vec3::ONE);
    assert_eq!(camera, before, "a broken box changes nothing");
}

fn flat(kind: ViewKind) -> Camera {
    Camera {
        position: Vec3::new(2.0, 3.0, 1.0),
        kind,
        flat_height: 10.0,
        ..Camera::default()
    }
}

#[test]
fn flat_views_look_along_an_axis_and_keep_sizes() {
    for (kind, forward) in [
        (ViewKind::Top, Vec3::NEG_Z),
        (ViewKind::Front, Vec3::Y),
        (ViewKind::Right, Vec3::NEG_X),
    ] {
        let camera = flat(kind);
        assert_eq!(camera.forward(), forward);
        assert!(
            close(camera.right().cross(camera.up()), -forward),
            "{kind:?}"
        );
        // The middle of the view is in the middle of the picture,
        // near or far.
        let size = Vec2::new(800.0, 600.0);
        for depth in [-100.0, 0.0, 400.0] {
            let p = camera.position + forward * depth;
            let at = camera.project(p, size).unwrap();
            assert!((at - size * 0.5).length() < 1e-2, "{kind:?} {depth}");
        }
        // One metre is the same size on screen at any depth.
        let near = camera
            .project(camera.position + camera.right(), size)
            .unwrap();
        let far = camera
            .project(camera.position + camera.right() + forward * 300.0, size)
            .unwrap();
        assert!((near - far).length() < 1e-2);
        assert!(
            (near.x - size.x * 0.5 - 60.0).abs() < 1e-2,
            "10 m in 600 points"
        );
        assert!((camera.world_per_pixel(Vec3::ZERO, size) - 10.0 / 600.0).abs() < 1e-6);
        // Every ray points the same way, starting behind what it sees.
        let ray = camera.ray_through(Vec2::new(100.0, 50.0), size);
        assert_eq!(ray.direction, forward);
        assert!((ray.origin - camera.position).dot(forward) < -1000.0);
        assert!(camera.view_projection(1.3).is_finite());
        assert_eq!(camera.focus(), kind.onto_plane(camera.position));
    }
}

#[test]
fn a_flat_view_slides_with_the_mouse_and_zooms_towards_it() {
    let mut camera = flat(ViewKind::Top);
    let size = Vec2::new(800.0, 600.0);
    // Dragging right by 60 points (one metre here): the world follows.
    let under = camera.ray_through(Vec2::new(400.0, 300.0), size).origin;
    camera.fly(&FlyInput {
        pan: Vec2::new(60.0, 0.0),
        view_height: 600.0,
        view_width: 800.0,
        dt: 0.016,
        ..FlyInput::default()
    });
    let moved = camera
        .project(Vec3::new(under.x, under.y, 0.0), size)
        .unwrap();
    assert!((moved.x - 460.0).abs() < 1e-2, "{moved:?}");
    // Zooming in at a corner keeps the point under the mouse there.
    let mouse = Vec2::new(700.0, 100.0);
    let ray = camera.ray_through(mouse, size);
    let point = Vec3::new(ray.origin.x, ray.origin.y, 0.0);
    camera.fly(&FlyInput {
        dolly_notches: 3.0,
        mouse: Some(mouse),
        view_height: 600.0,
        view_width: 800.0,
        dt: 0.016,
        ..FlyInput::default()
    });
    assert!(camera.flat_height < 10.0);
    let still = camera.project(point, size).unwrap();
    assert!((still - mouse).length() < 1e-2, "{still:?}");
    // Zooming never goes past its limits, and bad input is ignored.
    for notches in [500.0, -500.0, f32::NAN] {
        camera.fly(&FlyInput {
            dolly_notches: notches,
            view_height: 600.0,
            ..FlyInput::default()
        });
        assert!(FLAT_HEIGHT_RANGE.contains(&camera.flat_height));
    }
    let before = camera;
    camera.fly(&FlyInput {
        pan: Vec2::new(5.0, 5.0),
        view_height: 0.0,
        ..FlyInput::default()
    });
    assert_eq!(camera, before);
}

#[test]
fn switching_kinds_keeps_looking_at_the_same_place() {
    let camera = Camera::default();
    let focus = camera.focus();
    let top = camera.as_kind(ViewKind::Top);
    assert_eq!(top.position, focus);
    assert!(top.flat_height > 1.0);
    let back = top.as_kind(ViewKind::Perspective);
    assert!(!back.is_flat());
    assert!((back.focus() - focus).length() < 0.5, "{:?}", back.focus());
    assert_eq!(top.as_kind(ViewKind::Front).position, top.position);
    // Framing a box in a flat view centres it and fits it.
    let mut front = flat(ViewKind::Front);
    front.frame(Vec3::new(10.0, 10.0, 0.0), Vec3::new(12.0, 12.0, 2.0));
    assert_eq!(front.position, Vec3::new(11.0, 11.0, 1.0));
    assert!(front.flat_height > 2.0 && front.flat_height < 10.0);
    // look_at centres a flat view rather than turning it.
    front.look_at(Vec3::ONE);
    assert_eq!(front.position, Vec3::ONE);
    assert_eq!(front.forward(), Vec3::Y);
}
