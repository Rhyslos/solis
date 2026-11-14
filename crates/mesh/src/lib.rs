//! Polygon meshes: the data structure, topology queries, modelling
//! operations and the shapes brushes start as.
//!
//! Layer 0 (core). Knows nothing about areas, files, the GPU or the
//! editor. See this crate's README for its purpose and limits.

pub mod attributes;
pub mod bounds;
pub mod builder;
pub mod check;
pub mod edit;
pub mod extrude;
pub mod flat;
pub mod intact;
pub mod material;
pub mod parallel;
pub mod polygon;
pub mod primitives;
#[cfg(test)]
mod samples;
pub mod shading;
pub mod shapes;
pub mod topology;
pub mod triangulate;

pub use attributes::{
    AttributeElement, AttributeError, AttributeKind, AttributeValues, Domain, Value,
};
pub use bounds::Bounds;
pub use builder::{Builder, CornerFrom, PointFrom};
pub use check::MeshReport;
pub use flat::{FlatTriangles, flat_triangles, solid_triangles};
pub use intact::{Broken, check_moved};
pub use polygon::{EdgeKey, FaceId, Mesh, MeshError, VertexId};
pub use primitives::{
    ConeParams, CubeParams, CylinderParams, DiscParams, PlaneParams, Primitive, PrimitiveKind,
    SphereParams, TubeParams,
};
pub use shading::{
    Hardness, ShadedTriangle, corner_normal_list, corner_normals, corner_shading,
    corner_shading_with, shaded_triangles, smooth_groups, smooth_groups_with,
};
pub use shapes::{Shape, ShapeSettings};
pub use topology::{EdgeUse, Topology};
pub use triangulate::{face_triangles, triangulate};
