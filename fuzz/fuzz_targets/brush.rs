//! An area file holding brushes: refused or read; every brush read is
//! within the limits, writes back to itself, and is cut into the
//! triangles it's drawn and made solid with, never a panic.
#![no_main]
use area::builtin::{Brush, MAX_FACES, MAX_POINTS};
use reflect::Component;
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(registry) = area::builtin::registry() else {
        return;
    };
    let Ok(area) = area::text::read(text, &registry) else {
        return;
    };
    for (_, brush) in area.with::<Brush>() {
        assert!(brush.mesh.vertex_count() <= MAX_POINTS);
        assert!(brush.mesh.face_count() <= MAX_FACES);
        assert!(!brush.damaged || brush.mesh.face_count() == 0);
        let again = Brush::from_data(&brush.to_data());
        if !brush.damaged {
            assert_eq!(again.mesh.vertex_count(), brush.mesh.vertex_count());
            assert_eq!(again.mesh.face_count(), brush.mesh.face_count());
        }
        let drawn = mesh::flat_triangles(&brush.mesh);
        let (points, triangles) = mesh::solid_triangles(&brush.mesh);
        assert_eq!(drawn.len(), triangles.len());
        assert!(triangles.iter().flatten().all(|i| (*i as usize) < points.len()));
    }
});
