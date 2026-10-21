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
        let p = self.add_accessor(
            bytemuck::cast_slice(&positions),
            positions.len(),
            "VEC3",
            5126,
            Some(([-half[0], -half[1], -half[2]], half)),
        );
        let n = self.add_accessor(
            bytemuck::cast_slice(&normals),
            normals.len(),
            "VEC3",
            5126,
            None,
        );
        let t = self.add_accessor(bytemuck::cast_slice(&uvs), uvs.len(), "VEC2", 5126, None);
        let i = self.add_accessor(
            bytemuck::cast_slice(&indices),
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
