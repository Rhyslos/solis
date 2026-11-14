//! Making a chunk's triangles from its samples, each look's competing
//! ways.

mod blocky;
mod cases;
mod dual;
mod marching;
mod normals;
mod qef;

use crate::ground::Samples;
use glam::Vec3;

/// A chunk's triangles, ready to draw and to make solid.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ChunkMesh {
    /// Each point's place in the world, in voxels.
    pub positions: Vec<Vec3>,
    /// Each point's facing (towards the air).
    pub normals: Vec<Vec3>,
    /// Each point's material.
    pub materials: Vec<u8>,
    /// Three points per triangle, counter-clockwise from outside.
    pub indices: Vec<u32>,
}

impl ChunkMesh {
    /// How many triangles it has.
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    /// The bytes it takes on the graphics card: a point as the engine's
    /// model vertex is (place, facing and a slot the material would take:
    /// 32 bytes) and four bytes an index.
    pub fn upload_bytes(&self) -> usize {
        self.positions.len() * 32 + self.indices.len() * 4
    }

    fn push_point(&mut self, position: Vec3, normal: Vec3, material: u8) -> u32 {
        let index = self.positions.len() as u32;
        self.positions.push(position);
        self.normals.push(normal);
        self.materials.push(material);
        index
    }
}

/// A way of meshing, by look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// Smooth: a point per edge the surface crosses, triangles from a
    /// table of the 256 ways a cube's corners can be in or out.
    MarchingCubes,
    /// Smooth: a point per cube the surface passes through, at the middle
    /// of where it crosses the cube's edges; a square per crossed edge.
    SurfaceNets,
    /// Sharp: surface nets' squares, each point put where the surface's
    /// planes meet (solving a small least-squares problem), hard edges
    /// where faces meet at a steep angle.
    DualContouring,
    /// Sharp: surface nets, each point pulled a few steps towards the
    /// planes the surface crosses, hard edges where faces meet steeply.
    SharpNets,
    /// Blocky: a square per cube face between ground and air.
    CubeFaces,
    /// Blocky: cube faces, merged into the largest rectangles of one
    /// material.
    GreedyCubes,
}

impl Method {
    /// Every way, in the report's order.
    pub const ALL: [Self; 6] = [
        Self::MarchingCubes,
        Self::SurfaceNets,
        Self::DualContouring,
        Self::SharpNets,
        Self::CubeFaces,
        Self::GreedyCubes,
    ];

    /// Its name in the report.
    pub fn name(self) -> &'static str {
        match self {
            Self::MarchingCubes => "Marching cubes",
            Self::SurfaceNets => "Surface nets",
            Self::DualContouring => "Dual contouring",
            Self::SharpNets => "Surface nets, sharpened",
            Self::CubeFaces => "Cube faces",
            Self::GreedyCubes => "Cube faces, merged",
        }
    }

    /// Which look it draws.
    pub fn look(self) -> &'static str {
        match self {
            Self::MarchingCubes | Self::SurfaceNets => "Smooth",
            Self::DualContouring | Self::SharpNets => "Sharp",
            Self::CubeFaces | Self::GreedyCubes => "Blocky",
        }
    }

    /// The chunk's mesh, made from `samples`.
    pub fn mesh(self, samples: &Samples) -> ChunkMesh {
        match self {
            Self::MarchingCubes => marching::mesh(samples),
            Self::SurfaceNets => dual::mesh(samples, dual::Placement::Middle),
            Self::DualContouring => dual::mesh(samples, dual::Placement::Planes),
            Self::SharpNets => dual::mesh(samples, dual::Placement::Pulled),
            Self::CubeFaces => blocky::mesh(samples, false),
            Self::GreedyCubes => blocky::mesh(samples, true),
        }
    }
}

#[cfg(test)]
#[path = "mesh_tests.rs"]
mod tests;
