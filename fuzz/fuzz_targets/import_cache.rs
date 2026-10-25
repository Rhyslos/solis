//! An import cache entry (`.solis/cache/*.solbin`, which a project can
//! ship): refused, or read with every index pointing somewhere.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let limits = assets::Limits::default();
    if let Ok(model) = asset_db::cache::decode(data, &limits) {
        assert!(model.is_finite());
        for mesh in &model.meshes {
            assert!(mesh.indices.iter().all(|&i| (i as usize) < mesh.vertices.len()));
        }
        // Written again, it reads back.
        assert!(asset_db::cache::decode(&asset_db::cache::encode(&model), &limits).is_ok());
    }
});
