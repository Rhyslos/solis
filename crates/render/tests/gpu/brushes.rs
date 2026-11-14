//! Brushes (Tools 1a, B1): a brush's mesh cut into flat triangles (as the
//! editor hands it over) is drawn shaded face by face, crisp at its edges.

use super::*;

/// A brush's flat triangles as a model, as the editor makes it.
fn brush_model(mesh: &mesh::Mesh) -> assets::Model {
    let flat = mesh::flat_triangles(mesh);
    let vertices = flat
        .positions
        .iter()
        .zip(&flat.normals)
        .map(|(p, n)| assets::Vertex {
            position: p.to_array(),
            normal: n.to_array(),
            uv: [0.0, 0.0],
        })
        .collect();
    assets::Model {
        name: "Brush".into(),
        meshes: vec![assets::Mesh {
            vertices,
            indices: flat.indices,
            material: None,
            lods: Vec::new(),
        }],
        nodes: vec![assets::Node {
            name: "Brush".into(),
            parent: None,
            local: Mat4::IDENTITY,
            world: Mat4::IDENTITY,
            meshes: vec![0],
        }],
        materials: Vec::new(),
        textures: Vec::new(),
        lamps: Vec::new(),
        bounds: assets::Bounds::around(flat.positions.iter().copied()).unwrap(),
        warnings: Vec::new(),
    }
}

#[test]
fn a_brush_face_is_drawn_one_shade_all_over() {
    // A wedge seen from the front: its front face is one flat triangle,
    // lit by a sun from the right, the same shade all over (flat, not
    // smoothed towards the slopes beside it).
    let wedge = mesh::Shape::Wedge;
    let settings = mesh::ShapeSettings {
        size: Vec3::new(4.0, 2.0, 2.0),
        ..wedge.standard()
    };
    let model = brush_model(&wedge.build(&settings));
    let eye = Vec3::new(0.0, -8.0, 1.0);
    let sun = Sun {
        direction: Vec3::new(-1.0, 0.2, -0.3).normalize(),
        shadows: false,
        ..Sun::default()
    };
    let Some((pixels, size)) = render_lit(
        &model,
        looking_north(eye),
        eye,
        Shading::default(),
        96,
        sun,
        &[],
    ) else {
        return;
    };
    // Two points on the front face, a little apart either side of the
    // middle.
    let at = |dx: i32, dy: i32| {
        let (x, y) = ((size as i32 / 2 + dx) as u32, (size as i32 / 2 + dy) as u32);
        brightness(pixel(&pixels, size, x, y))
    };
    let (a, b) = (at(-3, 4), at(3, 4));
    assert!(
        (a - b).abs() <= 6,
        "the front face is one shade: {a} and {b}"
    );
    let backdrop = brightness(SKY_HORIZON);
    assert!(
        a > backdrop + 20,
        "the brush is drawn: {a} against {backdrop}"
    );
}

#[test]
fn a_box_brush_fills_the_middle_of_the_view() {
    let model = brush_model(&mesh::Shape::Box.build(&mesh::Shape::Box.standard()));
    let eye = Vec3::new(0.0, -8.0, 1.0);
    let Some((pixels, size)) = render_lit(
        &model,
        looking_north(eye),
        eye,
        Shading::default(),
        64,
        Sun::default(),
        &[],
    ) else {
        return;
    };
    // The box fills the middle and leaves the corners to the sky.
    let middle = brightness(pixel(&pixels, size, size / 2, size / 2));
    let corner = pixel(&pixels, size, 1, 1);
    assert!(between(corner, SKY_HORIZON, SKY_TOP), "corner {corner:?}");
    assert!(middle != brightness(corner));
}
