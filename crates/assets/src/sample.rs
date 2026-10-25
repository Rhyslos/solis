//! A small ready-made model, written in code: a textured cube on a red
//! base. Tests use it, the window test imports it, and it shows what a
//! working model looks like. `cargo run -p assets --example sample -- out.glb`
//! writes it to a file.

use std::fmt::Write as _;
use std::io::Cursor;

/// The sample as a `.glb` file: a 2 m cube with a checker texture, sitting
/// on a 6 m wide, flat red base.
pub fn sample_glb() -> Vec<u8> {
    let mut b = Builder::default();
    let cube = b.add_cube_mesh(1.0, Some(0));
    let base = b.add_box_mesh([3.0, 0.1, 3.0], Some(1));
    let texture = checker_png(64, 8);
    let image = b.add_image(&texture);
    b.json_materials = vec![
        format!(
            r#"{{"name":"Checker","pbrMetallicRoughness":{{"baseColorTexture":{{"index":0}}}}}}"#
        ),
        r#"{"name":"Red","pbrMetallicRoughness":{"baseColorFactor":[0.8,0.1,0.08,1.0]}}"#.into(),
    ];
    b.json_textures = vec![format!(r#"{{"source":{image},"sampler":0}}"#)];
    b.json_nodes = vec![
        // glTF is Y-up: the cube sits 1.1 m up, on the base.
        format!(r#"{{"name":"Cube","mesh":{cube},"translation":[0,1.1,0]}}"#),
        format!(r#"{{"name":"Base","mesh":{base}}}"#),
    ];
    b.glb()
}

/// A bigger scene for looking at materials: the sample's cube on its base;
/// in front, a row of plastic spheres from glossy to rough and a row of
/// metal ones; behind, a wall with grooves in its normal texture and a
/// glowing stripe. Help > Add the sample scene brings it in.
pub fn showcase_glb() -> Vec<u8> {
    let mut b = Builder::default();
    let checker = b.add_image(&checker_png(64, 8));
    let grooves = b.add_image(&grooves_normal_png(128, 8));
    b.json_textures = vec![
        format!(r#"{{"source":{checker},"sampler":0}}"#),
        format!(r#"{{"source":{grooves},"sampler":0}}"#),
    ];
    let mut materials = vec![
        r#"{"name":"Checker","pbrMetallicRoughness":{"baseColorTexture":{"index":0},"metallicFactor":0,"roughnessFactor":0.6}}"#.to_string(),
        r#"{"name":"Red","pbrMetallicRoughness":{"baseColorFactor":[0.8,0.1,0.08,1.0],"metallicFactor":0,"roughnessFactor":0.7}}"#.to_string(),
        r#"{"name":"Grooved wall","pbrMetallicRoughness":{"baseColorFactor":[0.55,0.57,0.6,1.0],"metallicFactor":0,"roughnessFactor":0.8},"normalTexture":{"index":1}}"#.to_string(),
        r#"{"name":"Glow","pbrMetallicRoughness":{"baseColorFactor":[0.1,0.1,0.1,1.0]},"emissiveFactor":[1.0,0.55,0.1]}"#.to_string(),
    ];
    let cube = b.add_cube_mesh(1.0, Some(0));
    let base = b.add_box_mesh([3.0, 0.1, 3.0], Some(1));
    let wall = b.add_box_mesh([3.0, 1.5, 0.1], Some(2));
    let stripe = b.add_box_mesh([2.5, 0.06, 0.04], Some(3));
    let mut nodes = vec![
        format!(r#"{{"name":"Cube","mesh":{cube},"translation":[0,1.1,0]}}"#),
        format!(r#"{{"name":"Base","mesh":{base}}}"#),
        format!(r#"{{"name":"Wall","mesh":{wall},"translation":[0,1.5,-3.6]}}"#),
        format!(r#"{{"name":"Glowing stripe","mesh":{stripe},"translation":[0,2.6,-3.46]}}"#),
    ];
    for (row, metallic) in [(0, 0.0), (1, 1.0)] {
        for i in 0..5 {
            let roughness = 0.1 + 0.2 * i as f32;
            let color = if metallic > 0.5 {
                "[0.95,0.75,0.4,1.0]"
            } else {
                "[0.15,0.35,0.8,1.0]"
            };
            materials.push(format!(
                r#"{{"name":"{} {roughness:.1}","pbrMetallicRoughness":{{"baseColorFactor":{color},"metallicFactor":{metallic},"roughnessFactor":{roughness}}}}}"#,
                if metallic > 0.5 { "Metal" } else { "Plastic" }
            ));
            let sphere = b.add_sphere_mesh(0.45, Some(materials.len() - 1));
            nodes.push(format!(
                r#"{{"name":"Sphere","mesh":{sphere},"translation":[{},0.45,{}]}}"#,
                -2.4 + 1.2 * i as f32,
                4.0 + 1.2 * f32::from(row as u8)
            ));
        }
    }
    b.json_materials = materials;
    b.json_nodes = nodes;
    b.glb()
}

/// The benchmark's ship as a `.glb` file: a fighter about 4.5 m long,
/// nose along glTF's -Z (Solis's +Y): a smooth grey hull (detailed
/// enough, about 1,000 triangles, for levels of detail to matter), flat
/// wings, and a glowing engine at the back.
pub fn ship_glb() -> Vec<u8> {
    let mut b = Builder {
        json_materials: vec![
        r#"{"name":"Hull","pbrMetallicRoughness":{"baseColorFactor":[0.62,0.65,0.7,1.0],"metallicFactor":0.6,"roughnessFactor":0.35}}"#.into(),
        r#"{"name":"Wings","pbrMetallicRoughness":{"baseColorFactor":[0.2,0.22,0.26,1.0],"metallicFactor":0.3,"roughnessFactor":0.6}}"#.into(),
        r#"{"name":"Engine","pbrMetallicRoughness":{"baseColorFactor":[0.1,0.1,0.1,1.0]},"emissiveFactor":[0.4,0.8,1.0]}"#.into(),
        ],
        ..Builder::default()
    };
    let hull = b.add_sphere_mesh(1.0, Some(0));
    let wings = b.add_box_mesh([2.2, 0.06, 0.6], Some(1));
    let engine = b.add_mesh_of(sphere_data(0.35, 12, 6), Some(2));
    b.json_nodes = vec![
        format!(r#"{{"name":"Hull","mesh":{hull},"scale":[0.6,0.4,2.2]}}"#),
        format!(r#"{{"name":"Wings","mesh":{wings},"translation":[0,0,0.6]}}"#),
        format!(r#"{{"name":"Engine","mesh":{engine},"translation":[0,0,2.15]}}"#),
    ];
    b.glb()
}

/// The playground's models (Help > Make the playground…): each a 1 m
/// shape for the playground's entities to scale, by name: `block` (a
/// grey checker box, for floors, steps and ramps), `crate` (a wooden
/// orange box), `platform` (a blue box) and `character` (a figure 1.8 m
/// tall, feet at its origin, with a visor showing which way it faces,
/// Solis's +Y).
pub fn playground_glb(which: &str) -> Vec<u8> {
    let mut b = Builder::default();
    match which {
        "character" => {
            b.json_materials = vec![
                r#"{"name":"Suit","pbrMetallicRoughness":{"baseColorFactor":[0.9,0.55,0.15,1.0],"metallicFactor":0,"roughnessFactor":0.6}}"#.into(),
                r#"{"name":"Visor","pbrMetallicRoughness":{"baseColorFactor":[0.05,0.05,0.07,1.0],"metallicFactor":0.2,"roughnessFactor":0.2}}"#.into(),
            ];
            let body = b.add_sphere_mesh(1.0, Some(0));
            let visor = b.add_box_mesh([0.18, 0.06, 0.06], Some(1));
            // glTF is Y-up with forward along -Z (Solis's +Y).
            b.json_nodes = vec![
                format!(
                    r#"{{"name":"Body","mesh":{body},"translation":[0,0.9,0],"scale":[0.35,0.9,0.35]}}"#
                ),
                format!(r#"{{"name":"Visor","mesh":{visor},"translation":[0,1.55,-0.3]}}"#),
            ];
        }
        _ => {
            let (name, colour, checker) = match which {
                "crate" => ("Crate", "[0.7,0.42,0.18,1.0]", false),
                "platform" => ("Platform", "[0.2,0.45,0.85,1.0]", false),
                _ => ("Block", "[0.75,0.75,0.78,1.0]", true),
            };
            let texture = if checker {
                let image = b.add_image(&checker_png(64, 4));
                b.json_textures = vec![format!(r#"{{"source":{image},"sampler":0}}"#)];
                r#","baseColorTexture":{"index":0}"#
            } else {
                ""
            };
            b.json_materials = vec![format!(
                r#"{{"name":"{name}","pbrMetallicRoughness":{{"baseColorFactor":{colour},"metallicFactor":0,"roughnessFactor":0.8{texture}}}}}"#
            )];
            let mesh = b.add_cube_mesh(0.5, Some(0));
            b.json_nodes = vec![format!(r#"{{"name":"{name}","mesh":{mesh}}}"#)];
        }
    }
    b.glb()
}

/// A normal texture of horizontal grooves: `size` pixels square, `count`
/// grooves down it. Flat parts face straight out (the light blue of normal
/// textures); each groove's sides lean up and down.
pub fn grooves_normal_png(size: u32, count: u32) -> Vec<u8> {
    let period = (size / count.max(1)).max(4);
    let image = image::RgbaImage::from_fn(size, size, |_, y| {
        let t = (y % period) as f32 / period as f32;
        // A V-shaped groove in the middle third of each band.
        let lean: f32 = if (0.33..0.5).contains(&t) {
            -0.6
        } else if (0.5..0.67).contains(&t) {
            0.6
        } else {
            0.0
        };
        let n = glam::Vec3::new(0.0, lean, 1.0).normalize();
        let byte = |v: f32| ((v * 0.5 + 0.5) * 255.0).round() as u8;
        image::Rgba([byte(n.x), byte(n.y), byte(n.z), 255])
    });
    let mut bytes = Vec::new();
    let _ = image.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png);
    bytes
}

/// A checker texture as a PNG: `size` pixels square, `squares` across.
pub fn checker_png(size: u32, squares: u32) -> Vec<u8> {
    let cell = (size / squares.max(1)).max(1);
    let image = image::RgbaImage::from_fn(size, size, |x, y| {
        if (x / cell + y / cell).is_multiple_of(2) {
            image::Rgba([235, 235, 235, 255])
        } else {
            image::Rgba([40, 110, 150, 255])
        }
    });
    let mut bytes = Vec::new();
    // Encoding an in-memory image only fails if memory runs out.
    let _ = image.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png);
    bytes
}

/// Builds small glTF files in code, for the sample and for tests.
#[derive(Debug, Default)]
pub struct Builder {
    /// The binary data so far.
    pub bin: Vec<u8>,
    /// Buffer views, as JSON objects.
    pub json_views: Vec<String>,
    /// Accessors, as JSON objects.
    pub json_accessors: Vec<String>,
    /// Meshes, as JSON objects.
    pub json_meshes: Vec<String>,
    /// Materials, as JSON objects.
    pub json_materials: Vec<String>,
    /// Textures, as JSON objects.
    pub json_textures: Vec<String>,
    /// Images, as JSON objects.
    pub json_images: Vec<String>,
    /// Nodes, as JSON objects; every one is placed in the scene.
    pub json_nodes: Vec<String>,
    /// Lights (the `KHR_lights_punctual` extension), as JSON objects.
    pub json_lights: Vec<String>,
}

impl Builder {
    /// Adds bytes to the binary data, with a buffer view over them, and
    /// returns the view's index.
    pub fn add_view(&mut self, bytes: &[u8]) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        let offset = self.bin.len();
        self.bin.extend_from_slice(bytes);
        self.json_views.push(format!(
            r#"{{"buffer":0,"byteOffset":{offset},"byteLength":{}}}"#,
            bytes.len()
        ));
        self.json_views.len() - 1
    }

    /// Adds an accessor over a new view of `bytes`, and returns its index.
    /// `kind` is "VEC3", "VEC2" or "SCALAR"; `component` is glTF's number
    /// for the type (5126 float, 5123 u16, 5125 u32).
    pub fn add_accessor(
        &mut self,
        bytes: &[u8],
        count: usize,
        kind: &str,
        component: u32,
        bounds: Option<([f32; 3], [f32; 3])>,
    ) -> usize {
        let view = self.add_view(bytes);
        let mut json = format!(
            r#"{{"bufferView":{view},"componentType":{component},"count":{count},"type":"{kind}""#
        );
        if let Some((min, max)) = bounds {
            let _ = write!(
                json,
                r#","min":[{},{},{}],"max":[{},{},{}]"#,
                min[0], min[1], min[2], max[0], max[1], max[2]
            );
        }
        json.push('}');
        self.json_accessors.push(json);
        self.json_accessors.len() - 1
    }

    /// Adds a PNG image held in the binary data, and returns its index.
    pub fn add_image(&mut self, png: &[u8]) -> usize {
        let view = self.add_view(png);
        self.json_images
            .push(format!(r#"{{"bufferView":{view},"mimeType":"image/png"}}"#));
        self.json_images.len() - 1
    }

    /// Adds a cube `half` metres from its middle to each face, and
    /// returns the mesh's index.
    pub fn add_cube_mesh(&mut self, half: f32, material: Option<usize>) -> usize {
        self.add_box_mesh([half; 3], material)
    }

    /// Adds a box reaching `half` metres from its middle along each glTF
    /// axis, with normals and texture coordinates, and returns the mesh's
    /// index.
    pub fn add_box_mesh(&mut self, half: [f32; 3], material: Option<usize>) -> usize {
        let (positions, normals, uvs, indices) = box_data(half);
        self.add_mesh(&positions, &normals, &uvs, &indices, material)
    }

    /// Adds a sphere `radius` metres across from its middle, and returns
    /// the mesh's index.
    pub fn add_sphere_mesh(&mut self, radius: f32, material: Option<usize>) -> usize {
        let (positions, normals, uvs, indices) = sphere_data(radius, 32, 16);
        self.add_mesh(&positions, &normals, &uvs, &indices, material)
    }

    /// Adds a mesh from its corners, normals, texture coordinates and
    /// triangles, together, and returns its index.
    fn add_mesh_of(&mut self, data: BoxData, material: Option<usize>) -> usize {
        let (positions, normals, uvs, indices) = data;
        self.add_mesh(&positions, &normals, &uvs, &indices, material)
    }

    /// Adds a mesh from its corners, normals, texture coordinates and
    /// triangles, and returns its index.
    pub fn add_mesh(
        &mut self,
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        uvs: &[[f32; 2]],
        indices: &[u16],
        material: Option<usize>,
    ) -> usize {
        let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
        for p in positions {
            for i in 0..3 {
                min[i] = min[i].min(p[i]);
                max[i] = max[i].max(p[i]);
            }
        }
        let p = self.add_accessor(
            bytemuck::cast_slice(positions),
            positions.len(),
            "VEC3",
            5126,
            Some((min, max)),
        );
        let n = self.add_accessor(
            bytemuck::cast_slice(normals),
            normals.len(),
            "VEC3",
            5126,
            None,
        );
        let t = self.add_accessor(bytemuck::cast_slice(uvs), uvs.len(), "VEC2", 5126, None);
        let i = self.add_accessor(
            bytemuck::cast_slice(indices),
            indices.len(),
            "SCALAR",
            5123,
            None,
        );
        let material = material.map_or_else(String::new, |m| format!(r#","material":{m}"#));
        self.json_meshes.push(format!(
            r#"{{"primitives":[{{"attributes":{{"POSITION":{p},"NORMAL":{n},"TEXCOORD_0":{t}}},"indices":{i}{material}}}]}}"#
        ));
        self.json_meshes.len() - 1
    }

    /// The glTF JSON, with the binary data at `buffer_uri` (or inside a
    /// `.glb` when `None`).
    pub fn json(&self, buffer_uri: Option<&str>) -> String {
        let list = |items: &[String]| items.join(",");
        let uri = buffer_uri.map_or_else(String::new, |u| format!(r#","uri":"{u}""#));
        let scene_nodes: Vec<String> = (0..self.json_nodes.len()).map(|i| i.to_string()).collect();
        let mut json = format!(
            r#"{{"asset":{{"version":"2.0","generator":"Solis sample"}},"scene":0,"scenes":[{{"nodes":[{}]}}],"nodes":[{}],"meshes":[{}],"accessors":[{}],"bufferViews":[{}],"buffers":[{{"byteLength":{}{uri}}}]"#,
            scene_nodes.join(","),
            list(&self.json_nodes),
            list(&self.json_meshes),
            list(&self.json_accessors),
            list(&self.json_views),
            self.bin.len(),
        );
        for (key, items) in [
            ("materials", &self.json_materials),
            ("textures", &self.json_textures),
            ("images", &self.json_images),
        ] {
            if !items.is_empty() {
                let _ = write!(json, r#","{key}":[{}]"#, list(items));
            }
        }
        if !self.json_textures.is_empty() {
            json.push_str(r#","samplers":[{"wrapS":10497,"wrapT":10497}]"#);
        }
        if !self.json_lights.is_empty() {
            let _ = write!(
                json,
                r#","extensionsUsed":["KHR_lights_punctual"],"extensions":{{"KHR_lights_punctual":{{"lights":[{}]}}}}"#,
                list(&self.json_lights)
            );
        }
        json.push('}');
        json
    }

    /// The whole model as one `.glb` file.
    pub fn glb(&self) -> Vec<u8> {
        let mut json = self.json(None).into_bytes();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        let mut bin = self.bin.clone();
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        let total = 12 + 8 + json.len() + 8 + bin.len();
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&json);
        out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(&bin);
        out
    }
}

type BoxData = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u16>);

/// A sphere's corners, normals, texture coordinates and triangles:
/// `segments` around and `rings` from top to bottom.
fn sphere_data(radius: f32, segments: u16, rings: u16) -> BoxData {
    use std::f32::consts::{PI, TAU};
    let (mut positions, mut normals, mut uvs, mut indices) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for ring in 0..=rings {
        let v = f32::from(ring) / f32::from(rings);
        let (sin_t, cos_t) = (v * PI).sin_cos();
        for seg in 0..=segments {
            let u = f32::from(seg) / f32::from(segments);
            let (sin_p, cos_p) = (u * TAU).sin_cos();
            // glTF is Y-up: rings go from +Y down to -Y.
            let n = [sin_t * cos_p, cos_t, -sin_t * sin_p];
            positions.push(n.map(|c| c * radius));
            normals.push(n);
            uvs.push([u, v]);
        }
    }
    let row = segments + 1;
    for ring in 0..rings {
        for seg in 0..segments {
            let a = ring * row + seg;
            let b = a + row;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    (positions, normals, uvs, indices)
}

/// A box's corners (four per face, so each face has its own normal),
/// normals, texture coordinates and triangles.
fn box_data(half: [f32; 3]) -> BoxData {
    let (mut positions, mut normals, mut uvs, mut indices) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    // Each face: its normal, and two directions across it.
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ];
    for (normal, u, v) in faces {
        let base = positions.len() as u16;
        for (su, sv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let corner: [f32; 3] =
                std::array::from_fn(|i| (normal[i] + u[i] * su + v[i] * sv) * half[i]);
            positions.push(corner);
            normals.push(normal);
            uvs.push([(su + 1.0) / 2.0, (1.0 - sv) / 2.0]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (positions, normals, uvs, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sample_is_a_valid_glb() {
        let glb = sample_glb();
        assert_eq!(&glb[..4], b"glTF");
        assert_eq!(glb.len() % 4, 0);
        assert!(gltf::Gltf::from_slice(&glb).is_ok());
    }

    #[test]
    fn the_playground_models_import_at_their_sizes() {
        for which in ["block", "crate", "platform", "character"] {
            let model = crate::load_bytes(
                &playground_glb(which),
                None,
                which,
                &crate::Limits::default(),
            )
            .unwrap();
            assert!(model.warnings.is_empty(), "{which}: {:?}", model.warnings);
            let size = model.bounds.max - model.bounds.min;
            if which == "character" {
                assert!((size.z - 1.8).abs() < 0.05, "{size}");
                assert!(model.bounds.min.z.abs() < 0.05, "feet at the origin");
                assert!(model.bounds.max.y > 0.3, "the visor faces +Y");
            } else {
                assert!(
                    (size - glam::Vec3::ONE).abs().max_element() < 1e-4,
                    "{which}: {size}"
                );
            }
        }
    }

    #[test]
    fn the_showcase_is_a_valid_glb() {
        assert!(gltf::Gltf::from_slice(&showcase_glb()).is_ok());
    }

    #[test]
    fn the_ship_imports_with_levels_of_detail() {
        let ship = crate::load_bytes(&ship_glb(), None, "Ship", &crate::Limits::default()).unwrap();
        assert!(ship.warnings.is_empty(), "{:?}", ship.warnings);
        let size = ship.bounds.max - ship.bounds.min;
        assert!(size.y > size.x && size.y > 4.0, "long along +Y: {size}");
        assert!(ship.triangles() > 1000);
        assert!(ship.meshes.iter().any(|m| !m.lods.is_empty()));
    }

    #[test]
    fn sphere_faces_point_outwards() {
        let (positions, _, _, indices) = sphere_data(1.0, 12, 6);
        for t in indices.chunks_exact(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| glam::Vec3::from(positions[i as usize]));
            let face = (b - a).cross(c - a);
            if face.length() < 1e-6 {
                continue; // the slivers at the poles
            }
            assert!(face.dot((a + b + c) / 3.0) > 0.0);
        }
    }

    #[test]
    fn flat_parts_of_the_grooves_face_straight_out() {
        let png = grooves_normal_png(64, 4);
        let image = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(image.get_pixel(0, 0).0, [128, 128, 255, 255]);
    }

    #[test]
    fn box_faces_point_outwards() {
        let (positions, normals, _, indices) = box_data([1.0, 2.0, 3.0]);
        for t in indices.chunks_exact(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| glam::Vec3::from(positions[i as usize]));
            let face = (b - a).cross(c - a).normalize();
            let n = glam::Vec3::from(normals[t[0] as usize]);
            assert!(face.dot(n) > 0.99, "{face} vs {n}");
        }
    }
}
