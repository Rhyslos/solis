use super::*;
use crate::sample::{Builder, checker_png, sample_glb};
use glam::Vec3;

fn limits() -> Limits {
    Limits::default()
}

fn close(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-4
}

/// A small, repeatable random number source for the fuzz tests.
struct Noise(u64);

impl Noise {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as usize % n.max(1)
    }
}

#[test]
fn the_sample_reads_with_its_meshes_materials_and_texture() {
    let model = load_bytes(&sample_glb(), None, "Sample", &limits()).unwrap();
    assert_eq!(model.name, "Sample");
    assert_eq!(model.meshes.len(), 2);
    assert_eq!(model.triangles(), 24);
    assert_eq!(model.materials.len(), 2);
    assert_eq!(model.textures.len(), 1);
    assert_eq!(model.textures[0].width, 64);
    assert_eq!(model.materials[0].base_color_texture, Some(0));
    assert_eq!(model.materials[1].base_color_texture, None);
    assert!(model.warnings.is_empty(), "{:?}", model.warnings);
}

#[test]
fn y_up_models_stand_up_in_solis() {
    // The sample's cube sits 1.1 m up in glTF's Y; in Solis that is Z.
    let model = load_bytes(&sample_glb(), None, "Sample", &limits()).unwrap();
    assert!(
        close(model.bounds.min, Vec3::new(-3.0, -3.0, -0.1)),
        "{:?}",
        model.bounds
    );
    assert!(
        close(model.bounds.max, Vec3::new(3.0, 3.0, 2.1)),
        "{:?}",
        model.bounds
    );
    let cube = &model.meshes[0];
    let top = cube
        .vertices
        .iter()
        .filter(|v| v.position[2] > 2.0)
        .map(|v| Vec3::from(v.normal))
        .collect::<Vec<_>>();
    assert!(top.iter().any(|n| close(*n, Vec3::Z)), "the top faces up");
}

#[test]
fn normals_stay_length_one() {
    let model = load_bytes(&sample_glb(), None, "Sample", &limits()).unwrap();
    for v in model.meshes.iter().flat_map(|m| &m.vertices) {
        assert!((Vec3::from(v.normal).length() - 1.0).abs() < 1e-4);
    }
}

#[test]
fn a_gltf_with_separate_files_reads_from_its_folder() {
    let folder = std::env::temp_dir().join(format!("solis-assets-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut b = Builder::default();
    let mesh = b.add_cube_mesh(0.5, Some(0));
    b.json_materials = vec![r#"{"pbrMetallicRoughness":{"baseColorTexture":{"index":0}}}"#.into()];
    b.json_textures = vec![r#"{"source":0,"sampler":0}"#.into()];
    b.json_images = vec![r#"{"uri":"checker%20image.png"}"#.into()];
    b.json_nodes = vec![format!(r#"{{"mesh":{mesh}}}"#)];
    std::fs::write(folder.join("cube.bin"), &b.bin).unwrap();
    std::fs::write(folder.join("checker image.png"), checker_png(16, 4)).unwrap();
    let path = folder.join("cube.gltf");
    std::fs::write(&path, b.json(Some("cube.bin"))).unwrap();

    let model = load(&path, &limits()).unwrap();
    assert_eq!(model.name, "cube");
    assert_eq!(model.triangles(), 12);
    assert_eq!(model.textures.len(), 1, "{:?}", model.warnings);

    // Without the texture, the model still reads, with a warning.
    std::fs::remove_file(folder.join("checker image.png")).unwrap();
    let model = load(&path, &limits()).unwrap();
    assert!(model.textures.is_empty());
    assert!(
        model.warnings[0].contains("isn't there"),
        "{:?}",
        model.warnings
    );
    assert_eq!(model.materials[0].base_color_texture, None);

    // Without its buffer, it can't.
    std::fs::remove_file(folder.join("cube.bin")).unwrap();
    let err = load(&path, &limits()).unwrap_err();
    assert!(err.to_string().contains("cube.bin"), "{err}");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_gltf_holding_its_data_inline_reads() {
    use base64::Engine as _;
    let mut b = Builder::default();
    let mesh = b.add_cube_mesh(1.0, None);
    b.json_nodes = vec![format!(r#"{{"mesh":{mesh}}}"#)];
    let uri = format!(
        "data:application/octet-stream;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&b.bin)
    );
    let json = b.json(Some(&uri));
    let model = load_bytes(json.as_bytes(), None, "Inline", &limits()).unwrap();
    assert_eq!(model.triangles(), 12);
}

#[test]
fn a_gltf_needing_files_without_a_folder_is_refused() {
    let mut b = Builder::default();
    let mesh = b.add_cube_mesh(1.0, None);
    b.json_nodes = vec![format!(r#"{{"mesh":{mesh}}}"#)];
    for uri in [
        "cube.bin",
        "https://example.com/cube.bin",
        "C:/cube.bin",
        "/etc/cube.bin",
    ] {
        let err = load_bytes(b.json(Some(uri)).as_bytes(), None, "X", &limits()).unwrap_err();
        assert!(matches!(err, ImportError::Damaged(_)), "{uri}: {err}");
    }
}

#[test]
fn nodes_place_and_mirror_meshes_correctly() {
    let mut b = Builder::default();
    let mesh = b.add_cube_mesh(1.0, None);
    // Moved 5 m along glTF's X, and mirrored in X (scale -1).
    b.json_nodes = vec![format!(
        r#"{{"mesh":{mesh},"translation":[5,0,0],"scale":[-1,1,1]}}"#
    )];
    let model = load_bytes(&b.glb(), None, "X", &limits()).unwrap();
    assert!(close(model.bounds.center(), Vec3::new(5.0, 0.0, 0.0)));
    // Faces still point outwards after the mirror.
    let mesh = &model.meshes[0];
    for t in mesh.indices.chunks_exact(3) {
        let [a, b, c] = [t[0], t[1], t[2]].map(|i| Vec3::from(mesh.vertices[i as usize].position));
        let face = (b - a).cross(c - a).normalize();
        let out = ((a + b + c) / 3.0 - Vec3::new(5.0, 0.0, 0.0)).normalize();
        assert!(face.dot(out) > 0.5);
    }
}

#[test]
fn child_nodes_add_up_their_parents_placement() {
    let mut b = Builder::default();
    let mesh = b.add_cube_mesh(0.5, None);
    b.json_nodes = vec![
        r#"{"translation":[0,10,0],"children":[1]}"#.into(),
        format!(r#"{{"mesh":{mesh},"translation":[2,0,0]}}"#),
    ];
    // Only node 0 is in the scene; the builder lists both, so make a scene
    // by hand.
    let json = b.json(None).replace(r#""nodes":[0,1]"#, r#""nodes":[0]"#);
    let mut bin = b.bin.clone();
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let file = glb_from(&json, &bin);
    let model = load_bytes(&file, None, "X", &limits()).unwrap();
    // glTF (2, 10, 0) is Solis (2, 0, 10).
    assert!(
        close(model.bounds.center(), Vec3::new(2.0, 0.0, 10.0)),
        "{:?}",
        model.bounds
    );
}

fn glb_from(json: &str, bin: &[u8]) -> Vec<u8> {
    let mut json = json.as_bytes().to_vec();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = Vec::new();
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(bin);
    out
}

#[test]
fn a_model_without_normals_gets_smooth_ones() {
    let mut b = Builder::default();
    let mesh = b.add_cube_mesh(1.0, None);
    b.json_nodes = vec![format!(r#"{{"mesh":{mesh}}}"#)];
    let json = b.json(None).replace(r#""NORMAL":1,"#, "");
    let model = load_bytes(&glb_from(&json, &b.bin), None, "X", &limits()).unwrap();
    for v in &model.meshes[0].vertices {
        let n = Vec3::from(v.normal);
        assert!((n.length() - 1.0).abs() < 1e-4);
        assert!(n.dot(Vec3::from(v.position)) > 0.0, "points outwards");
    }
}

#[test]
fn files_over_the_limit_are_refused_before_reading() {
    let small = Limits {
        max_file_bytes: 100,
        ..limits()
    };
    let err = load_bytes(&sample_glb(), None, "X", &small).unwrap_err();
    assert!(matches!(err, ImportError::TooBig { .. }));
}

#[test]
fn models_over_the_triangle_limit_are_refused() {
    let small = Limits {
        max_triangles: 10,
        ..limits()
    };
    let err = load_bytes(&sample_glb(), None, "X", &small).unwrap_err();
    assert!(matches!(err, ImportError::TooManyTriangles(_)), "{err}");
}

#[test]
fn rubbish_and_empty_files_are_refused_politely() {
    for bytes in [&b""[..], b"hello", b"{}", b"glTF", &[0u8; 64][..]] {
        let err = load_bytes(bytes, None, "X", &limits()).unwrap_err();
        assert!(!err.to_string().is_empty());
    }
    let empty = r#"{"asset":{"version":"2.0"}}"#;
    assert_eq!(
        load_bytes(empty.as_bytes(), None, "X", &limits()).unwrap_err(),
        ImportError::Empty
    );
}

#[test]
fn an_accessor_running_past_its_data_is_refused() {
    let mut b = Builder::default();
    let mesh = b.add_cube_mesh(1.0, None);
    b.json_nodes = vec![format!(r#"{{"mesh":{mesh}}}"#)];
    // Claim 1000 positions where there are 24.
    let json = b.json(None).replacen(r#""count":24"#, r#""count":1000"#, 1);
    let err = load_bytes(&glb_from(&json, &b.bin), None, "X", &limits()).unwrap_err();
    assert!(matches!(err, ImportError::Damaged(_)), "{err}");
}

#[test]
fn a_triangle_pointing_at_a_missing_corner_is_refused() {
    let mut b = Builder::default();
    let mesh = b.add_cube_mesh(1.0, None);
    b.json_nodes = vec![format!(r#"{{"mesh":{mesh}}}"#)];
    let mut bin = b.bin.clone();
    // The indices are the last accessor's data: set the last one to 999.
    let len = bin.len();
    bin[len - 2..].copy_from_slice(&999u16.to_le_bytes());
    let err = load_bytes(&glb_from(&b.json(None), &bin), None, "X", &limits()).unwrap_err();
    assert!(err.to_string().contains("corner"), "{err}");
}

/// Thousands of randomly damaged copies of the sample: every one is either
/// read or refused with a reason, and none crashes or reads past the end
/// of its data.
#[test]
fn damaged_files_never_crash_the_reader() {
    let good = sample_glb();
    let mut noise = Noise(42);
    let mut read = 0;
    for round in 0..3_000 {
        let mut bytes = good.clone();
        match round % 4 {
            0 => bytes.truncate(noise.below(bytes.len())),
            1 => {
                for _ in 0..1 + noise.below(6) {
                    let at = noise.below(bytes.len());
                    bytes[at] = noise.below(256) as u8;
                }
            }
            2 => {
                // Damage inside the JSON, where numbers and names live.
                let json_end =
                    20 + u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
                for _ in 0..1 + noise.below(3) {
                    let at = 20 + noise.below(json_end - 20);
                    bytes[at] = b"0123456789,[]{}\":-e"[noise.below(19)];
                }
            }
            _ => {
                let at = noise.below(bytes.len());
                let len = noise.below(16).min(bytes.len() - at);
                bytes.drain(at..at + len);
            }
        }
        if let Ok(model) = load_bytes(&bytes, None, "X", &limits()) {
            read += 1;
            assert!(model.triangles() > 0);
            for mesh in &model.meshes {
                assert!(
                    mesh.indices
                        .iter()
                        .all(|&i| (i as usize) < mesh.vertices.len())
                );
                assert!(
                    mesh.vertices
                        .iter()
                        .all(|v| v.position.iter().all(|c| c.is_finite()))
                );
                assert!(mesh.material.is_none_or(|m| m < model.materials.len()));
            }
            for material in &model.materials {
                assert!(
                    material
                        .base_color_texture
                        .is_none_or(|t| t < model.textures.len())
                );
            }
        }
    }
    // Some damage (inside texture pixels, say) leaves a readable model;
    // make sure the test saw both kinds.
    assert!(read > 0 && read < 3_000, "{read}");
}

/// Damaged JSON in a `.gltf`: a different path through the parser.
#[test]
fn damaged_json_never_crashes_the_reader() {
    use base64::Engine as _;
    let mut b = Builder::default();
    let mesh = b.add_cube_mesh(1.0, None);
    b.json_nodes = vec![format!(r#"{{"mesh":{mesh}}}"#)];
    let uri = format!(
        "data:application/octet-stream;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&b.bin)
    );
    let good = b.json(Some(&uri)).into_bytes();
    let mut noise = Noise(7);
    for _ in 0..3_000 {
        let mut bytes = good.clone();
        for _ in 0..1 + noise.below(4) {
            let at = noise.below(bytes.len());
            bytes[at] = b"0123456789,[]{}\":-e.x"[noise.below(21)];
        }
        let _ = load_bytes(&bytes, None, "X", &limits());
    }
}

#[test]
fn lights_in_the_file_come_in_placed_and_in_blender_units() {
    use crate::lights::{LUMENS_PER_WATT, LampKind};
    use std::f32::consts::{FRAC_1_SQRT_2, PI};
    let mut b = Builder::default();
    let mesh = b.add_cube_mesh(0.5, None);
    b.json_lights = vec![
        // 1000 W point light, as Blender exports it.
        format!(
            r#"{{"name":"Bulb","type":"point","color":[1,0.5,0.25],"intensity":{},"range":20}}"#,
            1000.0 * LUMENS_PER_WATT / (4.0 * PI)
        ),
        r#"{"type":"spot","intensity":100,"spot":{"innerConeAngle":0.2,"outerConeAngle":0.6}}"#
            .into(),
        format!(
            r#"{{"type":"directional","intensity":{}}}"#,
            3.0 * LUMENS_PER_WATT
        ),
    ];
    b.json_nodes = vec![
        format!(r#"{{"mesh":{mesh}}}"#),
        // 3 m up (glTF Y).
        r#"{"translation":[0,3,0],"extensions":{"KHR_lights_punctual":{"light":0}}}"#.into(),
        // Turned to point straight down: -90 degrees about X.
        format!(
            r#"{{"translation":[2,4,0],"rotation":[{},0,0,{}],"extensions":{{"KHR_lights_punctual":{{"light":1}}}}}}"#,
            -FRAC_1_SQRT_2, FRAC_1_SQRT_2
        ),
        r#"{"extensions":{"KHR_lights_punctual":{"light":2}}}"#.into(),
    ];
    let model = load_bytes(&b.glb(), None, "Lit", &limits()).unwrap();
    assert_eq!(model.lamps.len(), 3);
    let bulb = &model.lamps[0];
    assert_eq!(bulb.name, "Bulb");
    assert_eq!(bulb.kind, LampKind::Point);
    assert!(close(bulb.position, Vec3::new(0.0, 0.0, 3.0)));
    assert!((bulb.strength - 1000.0).abs() < 0.1);
    assert_eq!(bulb.color, [1.0, 0.5, 0.25]);
    assert_eq!(bulb.range, Some(20.0));
    let spot = &model.lamps[1];
    assert!(
        matches!(spot.kind, LampKind::Spot { inner, outer } if (inner - 0.2).abs() < 1e-6 && (outer - 0.6).abs() < 1e-6)
    );
    assert!(close(spot.position, Vec3::new(2.0, 0.0, 4.0)));
    assert!(close(spot.direction, -Vec3::Z), "{}", spot.direction);
    let sun = &model.lamps[2];
    assert_eq!(sun.kind, LampKind::Sun);
    assert!((sun.strength - 3.0).abs() < 1e-3);
    // glTF lights shine along -Z, which is -Y... turned: glTF's -Z is
    // Solis's +Y.
    assert!(close(sun.direction, Vec3::Y), "{}", sun.direction);
}

#[test]
fn a_file_of_lights_only_still_reads() {
    let json = r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],
        "nodes":[{"translation":[1,2,3],"extensions":{"KHR_lights_punctual":{"light":0}}}],
        "extensionsUsed":["KHR_lights_punctual"],
        "extensions":{"KHR_lights_punctual":{"lights":[{"type":"point","intensity":10}]}}}"#;
    let model = load_bytes(json.as_bytes(), None, "Lights", &limits()).unwrap();
    assert!(model.meshes.is_empty());
    assert_eq!(model.lamps.len(), 1);
    assert!(close(model.bounds.center(), Vec3::new(1.0, -3.0, 2.0)));
}

#[test]
fn silly_light_values_are_tidied() {
    let mut b = Builder::default();
    let mesh = b.add_cube_mesh(0.5, None);
    b.json_lights = vec![
        r#"{"type":"spot","intensity":-5,"color":[9,-1,0.5],"range":-3,"spot":{"innerConeAngle":1.0,"outerConeAngle":0.5}}"#.into(),
    ];
    b.json_nodes = vec![
        format!(r#"{{"mesh":{mesh}}}"#),
        r#"{"extensions":{"KHR_lights_punctual":{"light":0}}}"#.into(),
    ];
    match load_bytes(&b.glb(), None, "X", &limits()) {
        Ok(model) => {
            let lamp = &model.lamps[0];
            assert!(lamp.strength >= 0.0);
            assert_eq!(lamp.color, [1.0, 0.0, 0.5]);
            assert_eq!(lamp.range, None);
            if let crate::lights::LampKind::Spot { inner, outer } = lamp.kind {
                assert!(inner <= outer);
            }
        }
        // The gltf crate may refuse values outside the rules; that's fine
        // too, as long as it doesn't crash.
        Err(e) => assert!(!e.to_string().is_empty()),
    }
}

/// Randomly damaged files with lights in them: read or refused, never a
/// crash, and any lights read are usable.
#[test]
fn damaged_files_with_lights_never_crash_the_reader() {
    let mut b = Builder::default();
    let mesh = b.add_cube_mesh(0.5, None);
    b.json_lights = vec![
        r#"{"type":"point","intensity":50,"range":10}"#.into(),
        r#"{"type":"spot","intensity":80,"spot":{"innerConeAngle":0.1,"outerConeAngle":0.5}}"#
            .into(),
        r#"{"type":"directional","intensity":2000}"#.into(),
    ];
    b.json_nodes = vec![
        format!(r#"{{"mesh":{mesh}}}"#),
        r#"{"translation":[0,3,0],"extensions":{"KHR_lights_punctual":{"light":0}}}"#.into(),
        r#"{"translation":[2,4,0],"extensions":{"KHR_lights_punctual":{"light":1}}}"#.into(),
        r#"{"extensions":{"KHR_lights_punctual":{"light":2}}}"#.into(),
    ];
    let good = b.glb();
    let json_end = 20 + u32::from_le_bytes([good[12], good[13], good[14], good[15]]) as usize;
    let mut noise = Noise(99);
    for _ in 0..3_000 {
        let mut bytes = good.clone();
        for _ in 0..1 + noise.below(3) {
            let at = 20 + noise.below(json_end - 20);
            bytes[at] = b"0123456789,[]{}\":-e.x"[noise.below(21)];
        }
        if let Ok(model) = load_bytes(&bytes, None, "X", &limits()) {
            for lamp in &model.lamps {
                assert!(lamp.position.is_finite() && lamp.direction.is_finite());
                assert!(lamp.strength.is_finite() && lamp.strength >= 0.0);
                assert!(lamp.color.iter().all(|c| (0.0..=1.0).contains(c)));
            }
        }
    }
}
