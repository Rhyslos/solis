//! An import cache entry (`.solis/cache/*.solbin`, which a project can
//! ship): refused, or read with every index pointing somewhere.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
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

/// Panics Solis catches on purpose (a library's, such as the glTF
/// parser's, which Solis turns into a refused file) aren't failures: only
/// one that escapes Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
