use super::*;
use mesh::Shape;

/// A 2 m box with its top pulled up `metres`, and the points that moved.
fn extruded(metres: f32) -> (Arc<Mesh>, Vec<VertexId>) {
    let mut mesh = Shape::Box.build(&Shape::Box.standard());
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).z > 0.99)
        .unwrap();
    mesh::edit::extrude_shaped(
        &mut mesh,
        &[top],
        metres,
        mesh::edit::Grouping::Together,
        mesh::edit::ExtrudeShape::default(),
    );
    let moved = mesh.face(top).unwrap().to_vec();
    (Arc::new(mesh), moved)
}

#[test]
fn a_whole_brush_is_checked_and_written_side_by_side() {
    let (mesh, around) = extruded(1.0);
    let made = Arc::new(BrushModel::passing(&mesh));
    let checked = both(Arc::clone(&mesh), &around, Some(Arc::clone(&made)));
    assert!(checked.whole.is_ok());
    let kept = checked.kept.unwrap();
    assert!(Arc::ptr_eq(&kept.read, &mesh), "no gaps: the same mesh");
    assert_eq!(kept.model.key, made.key, "the pull's last frame kept");
    assert!(!kept.model.unwhole && !kept.model.passing);
    assert_eq!(kept.parts.key, kept.model.key);
    let back = Brush::from_data(&kept.data);
    assert_eq!(*back.mesh, *mesh, "its values read back as the mesh");
}

#[test]
fn a_broken_brush_is_refused_with_why() {
    // Pushed in further than it is tall: it passes through itself.
    let (mesh, around) = extruded(-3.0);
    let checked = both(mesh, &around, None);
    assert!(checked.whole.is_err());
}

#[test]
fn a_brush_with_gaps_is_written_as_it_will_read_back() {
    let mut mesh = mesh::Primitive::Cube(mesh::CubeParams {
        size: glam::Vec3::new(4.0, 2.0, 2.0),
        segments: [2, 1, 1],
    })
    .mesh();
    let tops: Vec<mesh::FaceId> = mesh
        .faces()
        .map(|(f, _)| f)
        .filter(|f| mesh.face_normal(*f).z > 0.99)
        .collect();
    let merged = mesh::edit::merge_faces(&mut mesh, &tops).unwrap();
    assert!(has_gaps(&mesh));
    let around: Vec<VertexId> = mesh.face(merged[0]).unwrap().to_vec();
    let mesh = Arc::new(mesh);
    let checked = both(Arc::clone(&mesh), &around, None);
    assert!(checked.whole.is_ok());
    let kept = checked.kept.unwrap();
    assert!(!Arc::ptr_eq(&kept.read, &mesh));
    assert!(!has_gaps(&kept.read));
    assert_eq!(*kept.read, *Brush::from_data(&kept.data).mesh);
}

#[test]
fn work_ahead_gives_what_doing_it_now_does_or_is_called_off() {
    let (mesh, around) = extruded(1.5);
    let made = Arc::new(BrushModel::passing(&mesh));
    let now = both(Arc::clone(&mesh), &around, Some(Arc::clone(&made)));
    let newest = Newest::default();
    let ahead = Ahead::start(&newest, (Arc::clone(&mesh), around.clone()), made, 1.5);
    assert_eq!(ahead.metres, 1.5);
    // Given its time, a worker does it.
    let answered = loop {
        if ahead.state.load(Ordering::Acquire) != WAITING {
            break ahead.answer().expect("done by the worker");
        }
        std::thread::yield_now();
    };
    assert_eq!(answered.whole, now.whole);
    let (a, b) = (answered.kept.unwrap(), now.kept.unwrap());
    assert_eq!(a.data, b.data);
    assert_eq!(*a.read, *b.read);
    // Many asked for at once: any may be skipped or called off (the
    // caller then does the work), never wrong.
    let many: Vec<Ahead> = (0..20)
        .map(|i| {
            let (mesh, around) = extruded(0.5 + i as f32 * 0.1);
            let made = Arc::new(BrushModel::passing(&mesh));
            Ahead::start(&newest, (mesh, around), made, 0.5 + i as f32 * 0.1)
        })
        .collect();
    for ahead in &many {
        if let Some(checked) = ahead.answer() {
            assert!(checked.whole.is_ok());
        }
    }
    // One called off before a worker starts it isn't done.
    let (mesh, around) = extruded(1.0);
    let made = Arc::new(BrushModel::passing(&mesh));
    let ahead = Ahead::start(&Newest::default(), (mesh, around), made, 1.0);
    if ahead
        .state
        .compare_exchange(WAITING, CALLED_OFF, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        assert!(ahead.answer().is_none());
    }
}
