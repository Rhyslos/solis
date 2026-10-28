//! Reading models and textures from files into plain data the renderer
//! can upload: glTF 2.0 (`.gltf` and `.glb`) with PNG and JPEG textures.
//!
//! Layer 1 (engine). Knows nothing about the GPU or the editor. Every
//! file is treated as hostile: sizes are checked before anything big is
//! allocated, and damaged files give a plain-language [`ImportError`],
//! never a crash. See this crate's README for its purpose and limits.

pub mod aseprite;
mod geometry;
pub mod lights;
pub mod lods;
pub mod model;
mod reader;
pub mod sample;
pub mod sprite_sheet;
pub mod texture;

pub use geometry::Y_UP_TO_Z_UP;
pub use lights::{Lamp, LampKind};
pub use model::{
    AlphaMode, Bounds, ImportError, Limits, Lod, Material, Mesh, Model, Node, Sampler, Texture,
    Vertex, Wrap,
};
pub use reader::{EXTENSIONS, load, load_bytes, referenced_files, reserved_on_windows};

/// Whether `path` names a file this crate reads (by its extension).
pub fn is_model_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn model_files_are_known_by_extension() {
        assert!(is_model_file(Path::new("ship.glb")));
        assert!(is_model_file(Path::new("C:/models/Ship.GLTF")));
        assert!(!is_model_file(Path::new("ship.fbx")));
        assert!(!is_model_file(Path::new("glb")));
    }
}
