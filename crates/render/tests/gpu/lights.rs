//! The sun's shadows and lamps: points, spots, ranges, and many at once.

use super::*;

/// A camera at `eye` looking straight down (-Z), +X to the right and +Y
/// up the image, 60° tall view, square image.
pub(crate) fn looking_down(eye: Vec3) -> Mat4 {
    projection() * Mat4::from_translation(-eye)
}

/// The pixel showing ground point (`x`, `y`) to [`looking_down`] from 10 m
/// above the origin, in a `size`-pixel image.
pub(crate) fn ground_pixel(pixels: &[u8], size: u32, x: f32, y: f32) -> [u8; 3] {
    let half = 10.0 * 30.0_f32.to_radians().tan();
    let px = ((x / half + 1.0) * 0.5 * size as f32) as u32;
    let py = ((1.0 - y / half) * 0.5 * size as f32) as u32;
    pixel(pixels, size, px, py)
}

/// A flat square facing up at height `z`, `half` metres each way from
/// (`cx`, 0).
pub(crate) fn square(cx: f32, z: f32, half: f32) -> assets::Mesh {
    let corner = |x: f32, y: f32| assets::Vertex {
        position: [cx + x, y, z],
        normal: [0.0, 0.0, 1.0],
        uv: [0.0, 0.0],
    };
    assets::Mesh {
        vertices: vec![
            corner(-half, -half),
            corner(half, -half),
            corner(half, half),
            corner(-half, half),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        material: None,
        lods: Vec::new(),
    }
}

/// A grey floor 12 m across (raised a little, above the grid and axes),
/// and optionally a small square floating over its middle.
pub(crate) fn floor(with_roof: bool) -> assets::Model {
    let mut meshes = vec![square(0.0, 0.05, 6.0)];
    if with_roof {
        meshes.push(square(0.0, 1.5, 0.5));
    }
    let mut model = assets::Model {
        name: "Floor".into(),
        meshes,
        nodes: Vec::new(),
        materials: Vec::new(),
        textures: Vec::new(),
        lamps: Vec::new(),
        bounds: assets::Bounds {
            min: Vec3::new(-6.0, -6.0, 0.0),
            max: Vec3::new(6.0, 6.0, 1.5),
        },
        warnings: Vec::new(),
    };
    model.show_every_mesh();
    model
}

pub(crate) fn brightness(c: [u8; 3]) -> i32 {
    c.iter().map(|&v| i32::from(v)).sum()
}

/// A sun shining down at 45° towards +X.
pub(crate) fn slanted_sun(shadows: bool) -> Sun {
    Sun {
        direction: Vec3::new(1.0, 0.0, -1.0).normalize(),
        shadows,
        ..Sun::default()
    }
}

#[test]
fn the_sun_casts_a_shadow_where_it_should() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    for look in [Look::Standard, Look::Stylized] {
        let render = |shadows| {
            render_lit(
                &floor(true),
                looking_down(eye),
                eye,
                Shading {
                    look,
                    ..Shading::default()
                },
                128,
                slanted_sun(shadows),
                &[],
            )
        };
        let Some((with, size)) = render(true) else {
            return;
        };
        let (without, _) = render(false).unwrap();
        // The square 1.5 m up throws its shadow 1.5 m towards +X.
        let shadow = ground_pixel(&with, size, 1.5, 0.2);
        let open = ground_pixel(&with, size, -2.5, 0.2);
        assert!(
            brightness(shadow) + 60 < brightness(open),
            "{look:?}: shadow {shadow:?} not darker than open ground {open:?}"
        );
        // Without shadows, the same spot is lit like the rest.
        let unshadowed = ground_pixel(&without, size, 1.5, 0.2);
        assert!(
            (brightness(unshadowed) - brightness(open)).abs() < 12,
            "{look:?}: {unshadowed:?} vs {open:?}"
        );
        // Beyond the shadow, the ground is lit again.
        let past = ground_pixel(&with, size, 3.5, 0.2);
        assert!((brightness(past) - brightness(open)).abs() < 12, "{past:?}");
    }
}

#[test]
fn the_sun_shadow_falls_on_the_ground_grid_too() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let mut floating = floor(true);
    floating.meshes.remove(0);
    floating.show_every_mesh();
    floating.bounds.min = Vec3::new(-0.5, -0.5, 1.5);
    let render = |shadows| {
        render_lit(
            &floating,
            looking_down(eye),
            eye,
            Shading::default(),
            128,
            slanted_sun(shadows),
            &[],
        )
    };
    let Some((with, size)) = render(true) else {
        return;
    };
    let (without, _) = render(false).unwrap();
    let shadow = ground_pixel(&with, size, 1.5, 0.2);
    let open = ground_pixel(&with, size, -2.5, 0.2);
    assert!(
        brightness(shadow) + 15 < brightness(open),
        "{shadow:?} vs {open:?}"
    );
    let plain = ground_pixel(&without, size, 1.5, 0.2);
    assert!(
        (brightness(plain) - brightness(open)).abs() < 6,
        "{plain:?}"
    );
}

fn no_sun() -> Sun {
    Sun {
        strength: 0.0,
        ..Sun::default()
    }
}

fn lamp(kind: LightKind, position: Vec3) -> Light {
    Light {
        kind,
        position,
        direction: -Vec3::Z,
        color: [1.0, 0.6, 0.3],
        strength: 60.0,
        range: None,
    }
}

#[test]
fn a_point_light_lights_the_ground_near_it_most() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    for look in [Look::Standard, Look::Stylized] {
        let shading = Shading {
            look,
            ..Shading::default()
        };
        let render = |lights: &[Light]| {
            render_lit(
                &floor(false),
                looking_down(eye),
                eye,
                shading,
                128,
                no_sun(),
                lights,
            )
        };
        let Some((dark, size)) = render(&[]) else {
            return;
        };
        let (lit, _) = render(&[lamp(LightKind::Point, Vec3::new(2.0, 0.0, 1.0))]).unwrap();
        let under = ground_pixel(&lit, size, 2.0, 0.3);
        let far = ground_pixel(&lit, size, -3.5, 0.3);
        let before = ground_pixel(&dark, size, 2.0, 0.3);
        assert!(
            brightness(under) > brightness(before) + 50,
            "{look:?}: {under:?} vs unlit {before:?}"
        );
        assert!(
            brightness(under) > brightness(far) + 60,
            "{look:?}: {under:?} vs far {far:?}"
        );
        // Warm light: it adds more red than blue.
        let gain = |c: usize| i32::from(under[c]) - i32::from(before[c]);
        assert!(gain(0) > gain(2) + 20, "{look:?}: {under:?} vs {before:?}");
    }
}

#[test]
fn a_spotlight_lights_only_inside_its_cone() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let spot = Light {
        strength: 1000.0,
        ..lamp(
            LightKind::Spot {
                inner: 0.25,
                outer: 0.35,
            },
            Vec3::new(0.0, 0.0, 3.0),
        )
    };
    let render = |lights: &[Light]| {
        render_lit(
            &floor(false),
            looking_down(eye),
            eye,
            Shading::default(),
            128,
            no_sun(),
            lights,
        )
    };
    let Some((lit, size)) = render(&[spot]) else {
        return;
    };
    let (dark, _) = render(&[]).unwrap();
    // The cone reaches about 1.1 m out on the ground.
    let inside = ground_pixel(&lit, size, 0.4, 0.3);
    let outside = ground_pixel(&lit, size, 2.5, 0.3);
    let outside_dark = ground_pixel(&dark, size, 2.5, 0.3);
    assert!(
        brightness(inside) > brightness(outside) + 100,
        "{inside:?} {outside:?}"
    );
    assert!(
        (brightness(outside) - brightness(outside_dark)).abs() < 6,
        "outside the cone stays as it was: {outside:?} vs {outside_dark:?}"
    );
}

#[test]
fn a_lamp_with_a_range_stops_there() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let mut short = lamp(LightKind::Point, Vec3::new(0.0, 0.0, 0.5));
    short.range = Some(2.0);
    let render = |lights: &[Light]| {
        render_lit(
            &floor(false),
            looking_down(eye),
            eye,
            Shading::default(),
            128,
            no_sun(),
            lights,
        )
    };
    let Some((lit, size)) = render(&[short]) else {
        return;
    };
    let (dark, _) = render(&[]).unwrap();
    let near = ground_pixel(&lit, size, 0.5, 0.3);
    let beyond = ground_pixel(&lit, size, 3.0, 0.3);
    let beyond_dark = ground_pixel(&dark, size, 3.0, 0.3);
    assert!(
        brightness(near) > brightness(beyond) + 60,
        "{near:?} {beyond:?}"
    );
    assert!((brightness(beyond) - brightness(beyond_dark)).abs() < 6);
}

#[test]
fn many_lamps_and_odd_values_draw_without_trouble() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let mut lights: Vec<Light> = (0..40)
        .map(|i| lamp(LightKind::Point, Vec3::new(i as f32 * 0.2 - 4.0, 0.0, 1.0)))
        .collect();
    lights.push(Light {
        strength: f32::NAN,
        ..lamp(LightKind::Point, Vec3::ZERO)
    });
    lights.push(lamp(
        LightKind::Spot {
            inner: f32::INFINITY,
            outer: -1.0,
        },
        Vec3::new(0.0, 0.0, f32::NAN),
    ));
    let weird_sun = Sun {
        direction: Vec3::ZERO,
        strength: f32::INFINITY,
        ..Sun::default()
    };
    let Some((pixels, size)) = render_lit(
        &floor(true),
        looking_down(eye),
        eye,
        Shading::default(),
        64,
        weird_sun,
        &lights,
    ) else {
        return;
    };
    assert_eq!(pixels.len(), (size * size * 4) as usize);
}
