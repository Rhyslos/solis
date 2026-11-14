//! Tests for the Brush component: its values and its text round trip,
//! and damaged or hostile lists read as an empty brush, never a crash.

use super::*;
use crate::document::{Area, EntityData};
use crate::text;
use mesh::{MeshReport, ShapeSettings};

fn registry() -> reflect::Registry {
    crate::builtin::registry().unwrap()
}

fn data_with(points: Value, faces: Value) -> ComponentData {
    let mut data = Brush::default().to_data();
    data.set("points", points);
    data.set("faces", faces);
    data
}

fn ints(values: &[i64]) -> Value {
    Value::List(values.iter().map(|v| Value::Int(*v)).collect())
}

fn floats(values: &[f64]) -> Value {
    Value::List(values.iter().map(|v| Value::Float(*v)).collect())
}

/// A triangle's points and its one face.
fn triangle() -> (Value, Value) {
    (
        floats(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
        ints(&[3, 0, 1, 2]),
    )
}

#[test]
fn every_shape_round_trips_through_values() {
    for shape in Shape::ALL {
        let brush = Brush {
            solid: false,
            ..Brush::of_shape(shape)
        };
        let data = brush.to_data();
        for field in &Brush::schema().fields {
            let value = data.get(&field.name).unwrap();
            assert!(field.kind.accepts(value), "{}: {value:?}", field.name);
        }
        let back = Brush::from_data(&data);
        assert_eq!(back, brush, "{}", shape.name());
        assert!(!back.damaged);
    }
}

#[test]
fn a_new_brush_is_a_solid_closed_box() {
    let brush = Brush::from_data(&ComponentData::defaults(&Brush::schema()));
    assert_eq!(brush, Brush::default());
    assert_eq!(brush.shape, Shape::Box);
    assert!(brush.solid);
    assert_eq!((brush.mesh.vertex_count(), brush.mesh.face_count()), (8, 6));
    assert!(MeshReport::new(&brush.mesh).is_closed());
}

#[test]
fn a_brush_with_only_its_shape_is_that_shape_at_its_standard_size() {
    let mut data = ComponentData::default();
    data.set("shape", Value::Text("Stairs".into()));
    let brush = Brush::from_data(&data);
    assert_eq!(brush.shape, Shape::Stairs);
    assert_eq!(*brush.mesh, Shape::Stairs.build(&Shape::Stairs.standard()));
    assert!(!brush.damaged);
    // An unknown shape is a Box.
    data.set("shape", Value::Text("Teapot".into()));
    assert_eq!(Brush::from_data(&data).shape, Shape::Box);
}

#[test]
fn materials_and_gaps_left_by_deleted_points_survive() {
    let mut mesh = Shape::Box.build(&Shape::Box.standard());
    let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
    set_material(&mut mesh, &faces[..2], 3);
    // A loose point, deleted: the file closes up the gap.
    let loose = mesh.add_vertex(glam::Vec3::splat(9.0)).unwrap();
    mesh.remove_unused_vertex(loose);
    let brush = Brush {
        mesh: Arc::new(mesh.clone()),
        ..Brush::default()
    };
    let back = Brush::from_data(&brush.to_data());
    assert_eq!(back.mesh.vertex_count(), 8);
    let slots: Vec<u32> = back
        .mesh
        .faces()
        .map(|(f, _)| mesh::material::slot_of(&back.mesh, f))
        .collect();
    assert_eq!(slots, [3, 3, 0, 0, 0, 0]);
}

#[test]
fn a_brush_round_trips_through_an_area_file_a_few_numbers_to_a_line() {
    let mut area = Area::new();
    let settings = ShapeSettings {
        sides: 64,
        ..Shape::Sphere.standard()
    };
    let sphere = Brush {
        mesh: Arc::new(Shape::Sphere.build(&settings)),
        ..Brush::of_shape(Shape::Sphere)
    };
    area.spawn(EntityData::new("Ball").with(&sphere));
    area.spawn(EntityData::new("Step").with(&Brush::of_shape(Shape::Stairs)));
    let written = text::write(&area, &registry(), &|_| None);
    assert!(
        written.contains("    Brush.points = [\n        "),
        "{written}"
    );
    let longest = written.lines().map(str::len).max().unwrap();
    assert!(longest < 400, "a line of {longest} characters");
    let read = text::read(&written, &registry()).unwrap();
    assert_eq!(read, area);
    let (_, back) = read
        .with::<Brush>()
        .find(|(_, b)| b.shape == Shape::Sphere)
        .unwrap();
    assert_eq!(back, sphere);
    assert_eq!(text::write(&read, &registry(), &|_| None), written);
}

#[test]
fn damaged_lists_read_as_an_empty_brush() {
    let (points, faces) = triangle();
    assert!(!Brush::from_data(&data_with(points.clone(), faces.clone())).damaged);
    let cases = [
        // Points not in threes.
        data_with(floats(&[0.0, 1.0]), faces.clone()),
        // A face naming a point that isn't there.
        data_with(points.clone(), ints(&[3, 0, 1, 7])),
        // A face with two corners.
        data_with(points.clone(), ints(&[2, 0, 1])),
        // A face naming one point twice.
        data_with(points.clone(), ints(&[3, 0, 1, 1])),
        // A face cut short.
        data_with(points.clone(), ints(&[3, 0, 1])),
        // A negative count, and a negative corner.
        data_with(points.clone(), ints(&[-3, 0, 1, 2])),
        data_with(points.clone(), ints(&[3, 0, 1, -2])),
        // A count past the limit.
        data_with(
            points.clone(),
            ints(&[MAX_CORNERS_PER_FACE as i64 + 1, 0, 1, 2]),
        ),
        // A corner written as a decimal, a point as text.
        data_with(points.clone(), floats(&[3.0, 0.0, 1.0, 2.0])),
        data_with(Value::List(vec![Value::Text("x".into()); 3]), faces.clone()),
        // A point that isn't a number, and lists that aren't lists.
        data_with(
            floats(&[f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
            faces.clone(),
        ),
        data_with(Value::Int(3), faces.clone()),
        data_with(points.clone(), Value::Bool(true)),
    ];
    for data in cases {
        let brush = Brush::from_data(&data);
        assert!(brush.damaged, "{data:?}");
        assert_eq!(brush.mesh.face_count(), 0);
        assert!(damage_of(&data).is_some(), "says why: {data:?}");
    }
    // Why, in plain words (Tools 1a, B2: the Problems list says it).
    let why = |points: &Value, faces: Value| damage_of(&data_with(points.clone(), faces));
    assert_eq!(
        why(&points, ints(&[3, 0, 1, 7])),
        Some("a face names a point that isn't there")
    );
    assert_eq!(
        why(&points, ints(&[3, 0, 1, 1])),
        Some("a face names one of its points twice")
    );
    assert_eq!(
        why(&floats(&[0.0, 1.0]), faces.clone()),
        Some("its points aren't in threes (x, y, z)")
    );
    assert_eq!(why(&points, faces.clone()), None, "a sound brush");
    let mut hand_written = ComponentData::default();
    hand_written.set("shape", Value::Text("Ramp".into()));
    assert_eq!(damage_of(&hand_written), None, "only its shape");
    // One list without the other is damaged too.
    let mut half = ComponentData::default();
    half.set("points", points);
    assert!(Brush::from_data(&half).damaged);
}

#[test]
fn counts_past_the_limits_are_refused_before_anything_is_made() {
    let (_, faces) = triangle();
    let many_points = Value::List(vec![Value::Float(0.0); (MAX_POINTS + 1) * 3]);
    assert!(Brush::from_data(&data_with(many_points, faces)).damaged);
    let points = floats(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    let mut many_faces = Vec::new();
    for _ in 0..=MAX_FACES {
        many_faces.extend([3, 0, 1, 2]);
    }
    assert!(Brush::from_data(&data_with(points.clone(), ints(&many_faces))).damaged);
    // Faces within the count, corners past it.
    let wide = MAX_CORNERS_PER_FACE as i64;
    let mut corners = Vec::new();
    for _ in 0..(MAX_CORNERS / MAX_CORNERS_PER_FACE + 1) {
        corners.push(wide);
        corners.extend((0..wide).map(|i| i % 3));
    }
    assert!(Brush::from_data(&data_with(points, ints(&corners))).damaged);
}

#[test]
fn material_slots_out_of_range_keep_the_plain_one() {
    let (points, faces) = triangle();
    let mut data = data_with(points, faces);
    for slot in [Value::Int(-1), Value::Int(MAX_SLOT + 1), Value::Float(2.0)] {
        data.set("materials", Value::List(vec![slot]));
        let brush = Brush::from_data(&data);
        assert!(!brush.damaged);
        let face = brush.mesh.faces().next().unwrap().0;
        assert_eq!(mesh::material::slot_of(&brush.mesh, face), 0);
    }
    // A list shorter or longer than the faces, or none at all.
    data.set("materials", ints(&[]));
    assert!(!Brush::from_data(&data).damaged);
    data.set("materials", ints(&[1, 2, 3]));
    assert!(!Brush::from_data(&data).damaged);
}

/// A small, fixed random stream (SplitMix64), so the run is the same
/// every time.
struct Stream(u64);

impl Stream {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

#[test]
fn thousands_of_damaged_brush_files_are_read_or_refused_never_a_crash() {
    let mut area = Area::new();
    area.spawn(EntityData::new("Arch").with(&Brush::of_shape(Shape::Arch)));
    let written = text::write(&area, &registry(), &|_| None);
    let bytes = written.as_bytes();
    let mut stream = Stream(7);
    let junk = *b"09-,[];\n.e\" ";
    for _ in 0..3_000 {
        let mut copy = bytes.to_vec();
        for _ in 0..1 + stream.below(6) {
            let at = stream.below(copy.len());
            match stream.below(3) {
                0 => copy[at] = junk[stream.below(junk.len())],
                1 => {
                    copy.remove(at);
                }
                _ => copy.insert(at, junk[stream.below(junk.len())]),
            }
        }
        let Ok(text) = std::str::from_utf8(&copy) else {
            continue;
        };
        if let Ok(read) = text::read(text, &registry()) {
            for (_, brush) in read.with::<Brush>() {
                assert!(brush.mesh.vertex_count() <= MAX_POINTS);
                assert!(brush.mesh.face_count() <= MAX_FACES);
            }
            // Why a brush is damaged is said for every damaged one.
            for (_, entity) in read.iter() {
                if let Some(data) = entity.components.get(Brush::NAME) {
                    assert_eq!(damage_of(data).is_some(), Brush::from_data(data).damaged);
                }
            }
        }
    }
}

#[test]
fn the_version_3_example_is_what_this_build_writes() {
    // tests/fixtures/area/v3: a box floor, a ramp and stairs.
    let blockout = include_str!("../../../../tests/fixtures/area/v3/blockout.solarea");
    assert_eq!((text::FORMAT.version_of)(blockout), Some(3));
    let area = text::read(blockout, &registry()).unwrap();
    let shapes: Vec<Shape> = area.with::<Brush>().map(|(_, b)| b.shape).collect();
    assert_eq!(shapes, [Shape::Box, Shape::Ramp, Shape::Stairs]);
    for (_, brush) in area.with::<Brush>() {
        assert!(!brush.damaged);
        assert!(MeshReport::new(&brush.mesh).is_closed());
    }
    assert_eq!(text::write(&area, &registry(), &|_| None), blockout);
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
#[allow(clippy::print_stdout)]
fn scale_check_a_5000_face_brush_written_read_and_cut_into_triangles() {
    // What a change to a big brush costs before it's drawn: its values
    // written (as an edit does), read back, and cut into flat triangles.
    // The editor's whole path is `editor`'s scale check of the same name.
    const TARGET_MS: f64 = 8.0;
    let cube = mesh::Primitive::Cube(mesh::CubeParams {
        size: glam::Vec3::splat(100.0),
        segments: [29, 29, 29],
    })
    .mesh();
    let faces = cube.face_count();
    let brush = Brush {
        mesh: Arc::new(cube),
        ..Brush::default()
    };
    let runs = 10;
    let start = std::time::Instant::now();
    for _ in 0..runs {
        let data = brush.to_data();
        let back = Brush::from_data(&data);
        assert_eq!(back.mesh.face_count(), faces);
        assert_eq!(mesh::flat_triangles(&back.mesh).len(), faces * 2);
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(runs);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: a brush of {faces} faces written, read and cut into triangles {ms:.2} ms \
         (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "a big brush takes {ms:.2} ms");
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
#[allow(clippy::print_stdout)]
fn scale_check_an_area_of_10000_box_brushes_read() {
    // Opening an area of 10,000 box brushes, each its own size: the file
    // read, and every brush's mesh made from its values.
    const TARGET_MS: f64 = 1_000.0;
    let mut area = Area::new();
    for i in 0..10_000 {
        let size = glam::Vec3::new(
            1.0 + (i % 100) as f32 * 0.01,
            1.0 + (i / 100) as f32 * 0.01,
            1.0,
        );
        let brush = Brush {
            mesh: Arc::new(Shape::Box.build(&ShapeSettings {
                size,
                ..Shape::Box.standard()
            })),
            ..Brush::default()
        };
        area.spawn(EntityData::new("Box").with(&brush));
    }
    let text = text::write(&area, &registry(), &|_| None);
    let start = std::time::Instant::now();
    let read = text::read(&text, &registry()).unwrap();
    let read_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = std::time::Instant::now();
    let faces: usize = read.with::<Brush>().map(|(_, b)| b.mesh.face_count()).sum();
    let made_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(faces, 60_000);
    let ms = read_ms + made_ms;
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: an area of 10,000 box brushes ({} KB) read {read_ms:.0} ms, their meshes \
         made {made_ms:.0} ms (target {TARGET_MS} ms) {verdict}",
        text.len() / 1024
    );
    assert!(ms <= TARGET_MS);
}

#[test]
fn points_of_is_the_points_list_the_brush_writes() {
    // Editing a brush's insides (Tools 1a, B3) writes this list alone.
    for shape in Shape::ALL {
        let brush = Brush::of_shape(shape);
        let data = brush.to_data();
        assert_eq!(
            data.get("points"),
            Some(&Value::List(points_of(&brush.mesh))),
            "{shape:?}"
        );
    }
    assert!(points_of(&Mesh::new()).is_empty());
}
