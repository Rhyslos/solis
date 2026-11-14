//! The polygon mesh: points in space joined into faces with any number of
//! corners (n-gons), with named values on its elements ([attribute
//! layers](crate::attributes)), hard edges among them.
//!
//! Every point and face has an id that stays the same for its whole life
//! and is never given to another element, even after it is deleted. Ids
//! are handed out in order, so building the same mesh the same way always
//! gives the same ids.
//!
//! Adjacency (walking from an edge to its faces, loops and rings) lives in
//! [`Topology`], worked out once per set of faces ([`Mesh::topology`]).

mod layers;
mod points;

use crate::attributes::{Attributes, Domain, HARD, Layer, Value};
use crate::{Bounds, Topology};
use glam::Vec3;
use points::Points;
use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

/// A point's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexId(u32);

/// A face's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaceId(u32);

impl VertexId {
    /// The id as a number.
    pub fn get(self) -> u32 {
        self.0
    }

    /// The id with this number, as recorded earlier (by an operation, or
    /// in a saved file). It may not be in a given mesh.
    pub const fn from_raw(number: u32) -> Self {
        Self(number)
    }
}

impl FaceId {
    /// The id as a number.
    pub fn get(self) -> u32 {
        self.0
    }

    /// The id with this number, as recorded earlier. It may not be in a
    /// given mesh.
    pub const fn from_raw(number: u32) -> Self {
        Self(number)
    }
}

/// An edge between two points, whichever way round it is named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeKey(VertexId, VertexId);

impl EdgeKey {
    /// The edge between `a` and `b`.
    pub fn new(a: VertexId, b: VertexId) -> Self {
        if a <= b { Self(a, b) } else { Self(b, a) }
    }

    /// Its two ends, lower id first.
    pub fn ends(self) -> (VertexId, VertexId) {
        (self.0, self.1)
    }
}

/// Why a face couldn't be added, in plain words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshError {
    /// A face needs at least three corners.
    TooFewCorners,
    /// A face used a point that isn't in the mesh.
    UnknownVertex(VertexId),
    /// A face went through the same point twice.
    RepeatedVertex(VertexId),
    /// The mesh can't hold more elements.
    Full,
    /// A point's position isn't a real number (infinite or not a number).
    NotFinite(VertexId),
    /// A hard edge names a point that isn't in the mesh.
    UnknownEdge(EdgeKey),
    /// A face that isn't in the mesh.
    UnknownFace(FaceId),
    /// A face's corners were to be replaced by a different number of
    /// corners.
    CornerCountChanged(FaceId),
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewCorners => f.write_str("a face needs at least three corners"),
            Self::UnknownVertex(v) => write!(f, "point {} is not in the mesh", v.0),
            Self::RepeatedVertex(v) => write!(f, "a face goes through point {} twice", v.0),
            Self::Full => f.write_str("the mesh is too large"),
            Self::NotFinite(v) => write!(f, "point {} has no real position", v.0),
            Self::UnknownEdge(e) => write!(
                f,
                "a hard edge joins points {} and {}, which aren't both in the mesh",
                e.0.0, e.1.0
            ),
            Self::UnknownFace(id) => write!(f, "polygon {} is not in the mesh", id.0),
            Self::CornerCountChanged(id) => write!(
                f,
                "polygon {} would get a different number of corners",
                id.0
            ),
        }
    }
}

impl std::error::Error for MeshError {}

/// A polygon mesh.
///
/// Its parts (positions, faces, hard edges) are each shared between copies
/// until one copy changes them, so copying a mesh is cheap, and an edit
/// that only moves points copies only the positions. This matters for
/// heavy meshes, and for undo, which keeps the shape before each edit.
#[derive(Clone, Default)]
pub struct Mesh {
    /// Point positions by id; `None` once deleted.
    positions: Points,
    /// How many points exist.
    live_points: usize,
    /// Faces by id, as their corners in order (counter-clockwise seen from
    /// outside).
    faces: Arc<Faces>,
    /// Named values per point, edge, polygon or corner, hard edges among
    /// them ([`crate::attributes`]).
    attributes: Attributes,
}

/// Every face's corners, one after another in a single list, with where
/// each face starts: far less memory than a list per face, and quick to
/// walk. A deleted face has no corners. The faces' topology is worked out
/// when first asked for and kept with them, so copies of a mesh that only
/// differ in where points are share it.
#[derive(Default)]
pub(crate) struct Faces {
    /// Where each face's corners start in `corners`; one more entry than
    /// there are face slots, the last being the end.
    starts: Vec<u32>,
    corners: Vec<VertexId>,
    /// How many faces exist.
    live: usize,
    topology: OnceLock<Topology>,
    /// A fingerprint of the corner lists, worked out when first asked for.
    fingerprint: OnceLock<u64>,
}

impl Clone for Faces {
    fn clone(&self) -> Self {
        Self {
            starts: self.starts.clone(),
            corners: self.corners.clone(),
            live: self.live,
            // Whoever copies the faces is about to change them.
            topology: OnceLock::new(),
            fingerprint: OnceLock::new(),
        }
    }
}

impl Faces {
    fn slot_count(&self) -> usize {
        self.starts.len().saturating_sub(1)
    }

    fn get(&self, index: usize) -> Option<&[VertexId]> {
        let (start, end) = (*self.starts.get(index)?, *self.starts.get(index + 1)?);
        (end > start).then(|| &self.corners[start as usize..end as usize])
    }

    fn push(&mut self, corners: Option<&[VertexId]>) {
        if self.starts.is_empty() {
            self.starts.push(0);
        }
        if let Some(corners) = corners {
            self.corners.extend_from_slice(corners);
            self.live += 1;
        }
        self.starts.push(self.corners.len() as u32);
        // The faces changed: any topology worked out before is stale.
        self.topology = OnceLock::new();
        self.fingerprint = OnceLock::new();
    }
}

impl PartialEq for Faces {
    fn eq(&self, other: &Self) -> bool {
        self.starts == other.starts && self.corners == other.corners
    }
}

impl PartialEq for Mesh {
    fn eq(&self, other: &Self) -> bool {
        self.positions == other.positions
            && (Arc::ptr_eq(&self.faces, &other.faces) || self.faces == other.faces)
            && self.attributes == other.attributes
    }
}

impl std::fmt::Debug for Mesh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mesh")
            .field("points", &self.live_points)
            .field("faces", &self.faces.live)
            .field("hard_edges", &self.hard_edge_count())
            .field("attributes", &self.attributes.iter().count())
            .finish()
    }
}

impl Mesh {
    /// An empty mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a point; returns its id.
    pub fn add_vertex(&mut self, position: Vec3) -> Result<VertexId, MeshError> {
        let id = u32::try_from(self.positions.len()).map_err(|_| MeshError::Full)?;
        self.positions.push(Some(position));
        self.live_points += 1;
        Ok(VertexId(id))
    }

    /// Adds a face through `corners`, in order (counter-clockwise seen from
    /// the side it faces). Refused if it has fewer than three corners, uses
    /// an unknown point, or visits a point twice.
    pub fn add_face(&mut self, corners: &[VertexId]) -> Result<FaceId, MeshError> {
        self.check_face(corners)?;
        let id = u32::try_from(self.faces.slot_count()).map_err(|_| MeshError::Full)?;
        if u32::try_from(self.faces.corners.len() + corners.len()).is_err() {
            return Err(MeshError::Full);
        }
        Arc::make_mut(&mut self.faces).push(Some(corners));
        Ok(FaceId(id))
    }

    /// Every point slot by id number, `None` where a point was deleted
    /// (its id is never used again). For saving: [`Self::from_slots`]
    /// rebuilds the same mesh, ids included.
    pub fn point_slots(&self) -> impl ExactSizeIterator<Item = Option<Vec3>> + '_ {
        self.positions.iter()
    }

    /// How many point slots there are (points, and ids of deleted ones):
    /// one more than the highest point id.
    pub fn point_slot_count(&self) -> usize {
        self.positions.len()
    }

    /// Every face slot by id number, `None` where a face was deleted.
    pub fn face_slots(
        &self,
    ) -> impl ExactSizeIterator<Item = Option<&[VertexId]>> + DoubleEndedIterator + '_ {
        (0..self.faces.slot_count()).map(|i| self.faces.get(i))
    }

    /// The edges marked hard, in order.
    pub fn hard_edges(&self) -> impl Iterator<Item = EdgeKey> + '_ {
        self.attributes
            .get(HARD)
            .into_iter()
            .flat_map(Layer::edges)
            .filter(|(_, v)| *v == Value::Bool(true))
            .map(|(e, _)| e)
    }

    /// Rebuilds a mesh from its slots (as [`Self::point_slots`] and
    /// [`Self::face_slots`] give them) and its hard edges, keeping every
    /// id. Everything is checked, as it may come from a damaged file:
    /// positions must be real numbers, faces need three or more distinct
    /// corners that exist, and hard edges must join existing points.
    pub fn from_slots(
        points: Vec<Option<Vec3>>,
        faces: Vec<Option<Vec<VertexId>>>,
        hard: impl IntoIterator<Item = EdgeKey>,
    ) -> Result<Self, MeshError> {
        if u32::try_from(points.len()).is_err() || u32::try_from(faces.len()).is_err() {
            return Err(MeshError::Full);
        }
        if let Some(i) = points
            .iter()
            .position(|p| p.is_some_and(|p| !p.is_finite()))
        {
            return Err(MeshError::NotFinite(VertexId(i as u32)));
        }
        let live_points = points.iter().flatten().count();
        let mut mesh = Self {
            positions: Points::new(points),
            live_points,
            faces: Arc::default(),
            attributes: Attributes::default(),
        };
        let mut table = Faces::default();
        for face in &faces {
            if let Some(corners) = face {
                mesh.check_face(corners)?;
                if u32::try_from(table.corners.len() + corners.len()).is_err() {
                    return Err(MeshError::Full);
                }
            }
            table.push(face.as_deref());
        }
        mesh.faces = Arc::new(table);
        // Hard edges only change shading: one that names a missing point,
        // or a point twice, is left out rather than refusing the mesh.
        for edge in hard {
            let (a, b) = edge.ends();
            mesh.set_hard(a, b, true);
        }
        Ok(mesh)
    }

    /// Checks a face's corners: three or more, all existing, none twice.
    fn check_face(&self, corners: &[VertexId]) -> Result<(), MeshError> {
        if corners.len() < 3 {
            return Err(MeshError::TooFewCorners);
        }
        for (i, v) in corners.iter().enumerate() {
            if self.position(*v).is_none() {
                return Err(MeshError::UnknownVertex(*v));
            }
            // Short faces: compare directly. Long ones: see below.
            if corners.len() <= 32 && corners[..i].contains(v) {
                return Err(MeshError::RepeatedVertex(*v));
            }
        }
        if corners.len() > 32 {
            // A sorted copy finds repeats without comparing every pair, so
            // a huge polygon (from a hostile file) can't stall opening.
            let mut sorted = corners.to_vec();
            sorted.sort_unstable();
            if let Some(w) = sorted.windows(2).find(|w| w[0] == w[1]) {
                return Err(MeshError::RepeatedVertex(w[0]));
            }
        }
        Ok(())
    }

    /// Adds a point, for builders that know the mesh stays small (the
    /// primitives, whose sizes are capped): the id is never reused even if
    /// the mesh were ever full.
    pub(crate) fn push_vertex(&mut self, position: Vec3) -> VertexId {
        let id = VertexId(u32::try_from(self.positions.len()).unwrap_or(u32::MAX));
        self.positions.push(Some(position));
        self.live_points += 1;
        id
    }

    /// Adds a face for builders that make only valid faces; an invalid one
    /// is skipped rather than causing a crash.
    pub(crate) fn push_face(&mut self, corners: &[VertexId]) {
        let _ = self.add_face(corners);
    }

    /// Moves a point to `position`. Returns `false` (changing nothing) if
    /// the point doesn't exist.
    pub fn set_position(&mut self, id: VertexId, position: Vec3) -> bool {
        if self.position(id).is_none() {
            return false;
        }
        self.positions.set(id.0 as usize, Some(position));
        true
    }

    /// Gives face `id` new corners, as many as it has now: the face keeps
    /// its id, so a selection of it still holds (an extruded polygon is
    /// still the selected one). Refused, changing nothing, for a face that
    /// doesn't exist, a different number of corners, or corners
    /// [`Self::add_face`] would refuse.
    pub fn replace_corners(&mut self, id: FaceId, corners: &[VertexId]) -> Result<(), MeshError> {
        let index = id.0 as usize;
        let Some(old) = self.faces.get(index) else {
            return Err(MeshError::UnknownFace(id));
        };
        if old.len() != corners.len() {
            return Err(MeshError::CornerCountChanged(id));
        }
        self.check_face(corners)?;
        let faces = Arc::make_mut(&mut self.faces);
        let start = faces.starts[index] as usize;
        faces.corners[start..start + corners.len()].copy_from_slice(corners);
        faces.topology = OnceLock::new();
        faces.fingerprint = OnceLock::new();
        Ok(())
    }

    /// Gives several faces new corners, any number each (three or more),
    /// keeping their ids: the face list is rebuilt once. Refused, changing
    /// nothing, if a face doesn't exist or its new corners are ones
    /// [`Self::add_face`] would refuse.
    pub fn rewrite_faces(
        &mut self,
        changes: &std::collections::BTreeMap<FaceId, Vec<VertexId>>,
    ) -> Result<(), MeshError> {
        for (id, corners) in changes {
            if self.face(*id).is_none() {
                return Err(MeshError::UnknownFace(*id));
            }
            self.check_face(corners)?;
        }
        let carry = self.attributes.has_domain(Domain::Corner);
        let mut from = Vec::new();
        let mut table = Faces::default();
        for (index, face) in self.face_slots().enumerate() {
            let replaced = changes.get(&FaceId(index as u32)).map(Vec::as_slice);
            let corners = replaced.or(face);
            if let Some(c) = corners
                && u32::try_from(table.corners.len() + c.len()).is_err()
            {
                return Err(MeshError::Full);
            }
            if carry && let Some(c) = corners {
                // Unchanged polygons keep their corner values; rewritten
                // ones start from the default (the builder fills them in).
                let start = self.faces.starts[index] as usize;
                if replaced.is_some() {
                    from.extend(std::iter::repeat_n(None, c.len()));
                } else {
                    from.extend((start..start + c.len()).map(Some));
                }
            }
            table.push(corners);
        }
        self.faces = Arc::new(table);
        if carry {
            for layer in self.attributes.of_domain_mut(Domain::Corner) {
                layer.regather(&from);
            }
        }
        Ok(())
    }

    /// Deletes a point that no face uses any more, with any hard edges
    /// naming it; its id is never used again. Returns whether it was
    /// deleted (a point some face still uses is kept).
    pub fn remove_unused_vertex(&mut self, id: VertexId) -> bool {
        if self.position(id).is_none() || !self.topology().vertex_faces(id).is_empty() {
            return false;
        }
        self.positions.set(id.0 as usize, None);
        self.live_points -= 1;
        self.forget_edges(|e| e.0 == id || e.1 == id);
        true
    }

    /// Deletes those of `ids` that no face uses any more, with any hard
    /// edges naming them, in one pass over the faces: quicker than
    /// [`Mesh::remove_unused_vertex`] one at a time when there are many.
    /// Returns how many were deleted.
    pub fn remove_unused_vertices(&mut self, ids: &[VertexId]) -> usize {
        let mut unused: std::collections::HashSet<VertexId> = ids
            .iter()
            .copied()
            .filter(|v| self.position(*v).is_some())
            .collect();
        if unused.is_empty() {
            return 0;
        }
        for (_, corners) in self.faces() {
            for c in corners {
                unused.remove(c);
            }
        }
        for v in &unused {
            self.positions.set(v.0 as usize, None);
        }
        self.live_points -= unused.len();
        self.forget_edges(|e| unused.contains(&e.0) || unused.contains(&e.1));
        unused.len()
    }

    /// Where a point is, if it exists.
    pub fn position(&self, id: VertexId) -> Option<Vec3> {
        self.positions.get(id.0 as usize)
    }

    /// A face's corners, in order, if it exists.
    pub fn face(&self, id: FaceId) -> Option<&[VertexId]> {
        self.faces.get(id.0 as usize)
    }

    /// Every point, with its id, in id order.
    pub fn vertices(&self) -> impl Iterator<Item = (VertexId, Vec3)> + '_ {
        self.positions
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.map(|p| (VertexId(i as u32), p)))
    }

    /// Every face, with its id, in id order.
    pub fn faces(&self) -> impl Iterator<Item = (FaceId, &[VertexId])> + '_ {
        (0..self.faces.slot_count())
            .filter_map(|i| self.faces.get(i).map(|f| (FaceId(i as u32), f)))
    }

    /// How many points there are.
    pub fn vertex_count(&self) -> usize {
        self.live_points
    }

    /// How many faces there are.
    pub fn face_count(&self) -> usize {
        self.faces.live
    }

    /// Every edge of every face, once each, in order (lower point id
    /// first). Worked out with the topology, once for these faces.
    pub fn edges(&self) -> &[EdgeKey] {
        self.topology().edge_list()
    }

    /// How the mesh's points, edges and faces connect. Worked out when
    /// first asked for, then kept with the faces: copies of the mesh that
    /// only move points share it.
    pub fn topology(&self) -> &Topology {
        self.faces.topology.get_or_init(|| Topology::new(self))
    }

    /// A number standing for the mesh's structure: its faces' corners (by
    /// point id) and how many point slots it has, but not where the points
    /// are. Two meshes with the same structure name the same elements with
    /// the same ids, so an edit made on one can trust its ids on the
    /// other, even if points have moved.
    /// Worked out once per set of faces; the same in every version of the
    /// program and on every computer, so it can be saved.
    pub fn structure_fingerprint(&self) -> u64 {
        let faces = *self.faces.fingerprint.get_or_init(|| {
            let mut hash = Fingerprint::new();
            for start in &self.faces.starts {
                hash.add(*start);
            }
            for corner in &self.faces.corners {
                hash.add(corner.0);
            }
            hash.finish()
        });
        let mut hash = Fingerprint::new();
        hash.add((faces >> 32) as u32);
        hash.add(faces as u32);
        hash.add(self.positions.len() as u32);
        hash.finish()
    }

    /// Whether `other` has the very same faces and hard edges as this
    /// mesh (shared, not just equal), so the two can differ only in where
    /// points are: what a recorded move leaves behind. Caches use it to
    /// update only what moved.
    pub fn same_structure(&self, other: &Mesh) -> bool {
        Arc::ptr_eq(&self.faces, &other.faces)
            && self.attributes.shared_with(&other.attributes)
            && self.positions.len() == other.positions.len()
    }

    /// The points that are somewhere else in this mesh than in `before`,
    /// if the two have the same structure ([`Self::same_structure`]);
    /// `None` if they don't, so everything may have changed.
    pub fn moved_since(&self, before: &Mesh) -> Option<Vec<VertexId>> {
        if !self.same_structure(before) {
            return None;
        }
        Some(self.positions.differing(&before.positions))
    }

    /// The faces as one list of corners and where each face starts in it
    /// (one more start than there are face slots); a deleted face starts
    /// where it ends.
    pub fn face_layout(&self) -> (&[u32], &[VertexId]) {
        (&self.faces.starts, &self.faces.corners)
    }

    /// Marks the edge between `a` and `b` hard (shading breaks across it)
    /// or soft (shading flows over it). Edges are soft unless marked. Hard
    /// edges are the built-in edge attribute [`HARD`].
    pub fn set_hard(&mut self, a: VertexId, b: VertexId, hard: bool) {
        // Only an edge between two different, existing points can be hard
        // (anything else couldn't be saved and opened again).
        if hard && (a == b || self.position(a).is_none() || self.position(b).is_none()) {
            return;
        }
        if hard == self.is_hard(a, b) {
            return;
        }
        if hard {
            let _ = self.attributes.add(HARD, Domain::Edge, Value::Bool(false));
        }
        if let Some(layer) = self.attributes.get_mut(HARD) {
            layer.set_edge(EdgeKey::new(a, b), Value::Bool(hard));
        }
        self.drop_empty_hard();
    }

    /// Whether the edge between `a` and `b` is hard.
    pub fn is_hard(&self, a: VertexId, b: VertexId) -> bool {
        self.attributes
            .get(HARD)
            .is_some_and(|l| l.edge(EdgeKey::new(a, b)) == Value::Bool(true))
    }

    /// How many edges are marked hard.
    pub fn hard_edge_count(&self) -> usize {
        self.attributes.get(HARD).map_or(0, Layer::edge_count)
    }

    /// Without hard edges, the mesh has no hard-edge layer at all, so a
    /// mesh whose last hard edge went soft equals one that never had any.
    pub(crate) fn drop_empty_hard(&mut self) {
        if self
            .attributes
            .get(HARD)
            .is_some_and(|l| l.edge_count() == 0)
        {
            self.attributes.remove(HARD);
        }
    }

    /// Forgets every edge value on edges `gone` says yes to.
    fn forget_edges(&mut self, gone: impl Fn(EdgeKey) -> bool) {
        if !self.attributes.has_edge_values() {
            return;
        }
        for layer in self.attributes.of_domain_mut(Domain::Edge) {
            layer.forget_edges(&gone);
        }
        self.drop_empty_hard();
    }

    /// Forgets the values (hard, seam) on edges no polygon uses any more,
    /// as left where polygons were merged into one: otherwise a gone
    /// edge would still be drawn as hard, and would come back hard if the
    /// same two points were ever joined again. Returns how many edges
    /// were forgotten.
    pub fn forget_unused_edge_values(&mut self) -> usize {
        if !self.attributes.has_edge_values() {
            return 0;
        }
        let used: std::collections::HashSet<EdgeKey> = self
            .faces()
            .flat_map(|(_, c)| sides(c))
            .map(|(a, b)| EdgeKey::new(a, b))
            .collect();
        let before: usize = self
            .attributes
            .of_domain_mut(Domain::Edge)
            .map(|l| l.edge_count())
            .sum();
        self.forget_edges(|e| !used.contains(&e));
        let after: usize = self
            .attributes
            .of_domain_mut(Domain::Edge)
            .map(|l| l.edge_count())
            .sum();
        before - after
    }

    /// Deletes polygons; their ids are never used again. Points they used
    /// stay (other polygons may use them; [`Self::remove_unused_vertices`]
    /// deletes the ones left over), and so do values on edges other
    /// polygons still use. Returns how many polygons were deleted (ids not
    /// in the mesh are ignored).
    pub fn delete_faces(&mut self, ids: &[FaceId]) -> usize {
        let gone: BTreeSet<FaceId> = ids
            .iter()
            .copied()
            .filter(|f| self.face(*f).is_some())
            .collect();
        if gone.is_empty() {
            return 0;
        }
        // Their edges: values on any that no polygon uses any more go.
        let candidates: std::collections::HashSet<EdgeKey> = if self.attributes.has_edge_values() {
            gone.iter()
                .flat_map(|f| sides(self.face(*f).unwrap_or_default()))
                .map(|(a, b)| EdgeKey::new(a, b))
                .collect()
        } else {
            Default::default()
        };
        let carry = self.attributes.has_domain(Domain::Corner);
        let mut from = Vec::new();
        let mut table = Faces::default();
        for (index, face) in self.face_slots().enumerate() {
            let kept = face.filter(|_| !gone.contains(&FaceId(index as u32)));
            if carry && let Some(c) = kept {
                let start = self.faces.starts[index] as usize;
                from.extend((start..start + c.len()).map(Some));
            }
            table.push(kept);
        }
        self.faces = Arc::new(table);
        if carry {
            for layer in self.attributes.of_domain_mut(Domain::Corner) {
                layer.regather(&from);
            }
        }
        if !candidates.is_empty() {
            let still: std::collections::HashSet<EdgeKey> = self
                .faces()
                .flat_map(|(_, c)| sides(c))
                .map(|(a, b)| EdgeKey::new(a, b))
                .filter(|e| candidates.contains(e))
                .collect();
            self.forget_edges(|e| candidates.contains(&e) && !still.contains(&e));
        }
        gone.len()
    }

    /// Roughly how many bytes the mesh takes up in memory, counting what
    /// its lists hold room for. An estimate (allocator overheads aren't
    /// counted), good enough to keep the undo history within a budget.
    /// Parts shared with other copies are counted in full, and so is the
    /// topology (often the larger part): as it is once worked out, and as
    /// an estimate before, since nearly every mesh gets one.
    pub fn memory_size(&self) -> usize {
        use std::mem::size_of;
        let positions = self.positions.memory_size();
        let faces = self.faces.starts.capacity() * size_of::<u32>()
            + self.faces.corners.capacity() * size_of::<VertexId>();
        let attributes = self.attributes.memory_size();
        let topology = self.faces.topology.get().map_or_else(
            || {
                crate::topology::Topology::estimate(
                    self.positions.len(),
                    self.faces.starts.len().saturating_sub(1),
                    self.faces.corners.len(),
                )
            },
            crate::topology::Topology::memory_size,
        );
        size_of::<Self>() + positions + faces + attributes + topology
    }

    /// The box around every point, or `None` for an empty mesh. Worked
    /// out once and kept until a point moves.
    pub fn bounds(&self) -> Option<Bounds> {
        self.positions.bounds()
    }

    /// The direction a face looks: unit length, worked out from all its
    /// corners so it is sensible for n-gons that aren't quite flat. Straight
    /// up for a face with no area.
    pub fn face_normal(&self, id: FaceId) -> Vec3 {
        let Some(corners) = self.face(id) else {
            return Vec3::Z;
        };
        // Newell's method: robust for any polygon.
        let mut normal = Vec3::ZERO;
        for (a, b) in sides(corners) {
            let (Some(p), Some(q)) = (self.position(a), self.position(b)) else {
                continue;
            };
            normal += Vec3::new(
                (p.y - q.y) * (p.z + q.z),
                (p.z - q.z) * (p.x + q.x),
                (p.x - q.x) * (p.y + q.y),
            );
        }
        normal.try_normalize().unwrap_or(Vec3::Z)
    }
}

/// A fixed 64-bit hash of a list of numbers (FNV-1a, a word at a time),
/// written out here so it never changes with a library or platform.
struct Fingerprint(u64);

impl Fingerprint {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn add(&mut self, word: u32) {
        self.0 = (self.0 ^ u64::from(word)).wrapping_mul(0x0000_0100_0000_01b3);
    }

    fn finish(&self) -> u64 {
        // Mix the last words through every bit.
        let mut x = self.0;
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
        x ^= x >> 33;
        x
    }
}

/// A face's sides as pairs of corners, the last joining back to the first.
pub(crate) fn sides(corners: &[VertexId]) -> impl Iterator<Item = (VertexId, VertexId)> + '_ {
    corners
        .iter()
        .zip(corners.iter().cycle().skip(1))
        .map(|(a, b)| (*a, *b))
}

#[cfg(test)]
#[path = "polygon_tests.rs"]
mod tests;
