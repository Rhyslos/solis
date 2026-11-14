//! What the shaping actions do with what's picked while a brush is edited
//! (Tools 1a, B4): Extrude and Inset start a [`Pull`]; Split and Delete
//! work out the new brush at once. Each says why, in plain words, when
//! the mode or the pick doesn't allow it: the ribbon greys its button
//! with that as the hover text, and the status bar says it when a key
//! asks.
//!
//! - **Split:** points mode, two points on one face cut it between them;
//!   edges mode, each edge gets a point in its middle; faces mode, each
//!   face is cut across its middle along its longer side.
//! - **Delete:** points and edges go and the faces around them merge;
//!   in faces mode, picked faces that touch merge into one.
//!
//! Nothing here keeps anything: the result is checked
//! ([`mesh::check_whole`]) and becomes one undo step on the workbench
//! (`workbench_brush_shaping.rs`).

use super::edit::{Editing, Mode, Part};
use super::pull::{Pull, PullKind};
use glam::Mat4;
use mesh::{EdgeKey, FaceId, Mesh, VertexId};

/// Why nothing can be shaped while a pull goes on.
pub(crate) const PULLING: &str = "Finish the one under way first: click to keep it, Esc to give up";
/// Why nothing can be shaped with nothing picked.
const NOTHING_PICKED: &str = "Pick part of the brush first";

/// The faces picked.
pub(crate) fn picked_faces(edit: &Editing) -> Vec<FaceId> {
    edit.picked
        .iter()
        .filter_map(|p| match p {
            Part::Face(f) => Some(*f),
            _ => None,
        })
        .collect()
}

/// The edges picked.
pub(crate) fn picked_edges(edit: &Editing) -> Vec<EdgeKey> {
    edit.picked
        .iter()
        .filter_map(|p| match p {
            Part::Edge(e) => Some(*e),
            _ => None,
        })
        .collect()
}

/// The points picked.
pub(crate) fn picked_points(edit: &Editing) -> Vec<VertexId> {
    edit.picked
        .iter()
        .filter_map(|p| match p {
            Part::Point(v) => Some(*v),
            _ => None,
        })
        .collect()
}

/// Why `edit`'s pick can't be shaped by anything now, or `None`.
fn why_not_any(edit: &Editing) -> Option<&'static str> {
    if edit.pull.is_some() || edit.drag.is_some() {
        return Some(PULLING);
    }
    edit.picked.is_empty().then_some(NOTHING_PICKED)
}

/// Whether a pull of `kind` (or its edge twin) is under way: its own
/// button stays lit, not greyed.
fn pulling(edit: &Editing, kinds: &[PullKind]) -> bool {
    edit.pull.as_ref().is_some_and(|p| kinds.contains(&p.kind))
}

/// Why Extrude can't start now, or `None` when it can (or is under way).
pub(crate) fn why_not_extrude(edit: &Editing) -> Option<&'static str> {
    if pulling(edit, &[PullKind::Extrude, PullKind::ExtrudeEdges]) {
        return None;
    }
    if let Some(why) = why_not_any(edit) {
        return Some(match why {
            NOTHING_PICKED => "Pick faces to extrude first",
            other => other,
        });
    }
    match edit.mode {
        Mode::Points => Some("Points can't be extruded: press 3 and pick faces"),
        Mode::Faces => None,
        Mode::Edges => {
            let mesh = edit.parts.as_ref()?.mesh();
            let topology = mesh.topology();
            let open = picked_edges(edit).iter().any(|e| topology.is_border(*e));
            (!open).then_some(
                "Only an edge on an open side can be extruded, and this brush is closed: press 3 \
                 and pick faces instead",
            )
        }
    }
}

/// Why Inset can't start now, or `None` when it can (or is under way).
pub(crate) fn why_not_inset(edit: &Editing) -> Option<&'static str> {
    if pulling(edit, &[PullKind::Inset]) {
        return None;
    }
    if let Some(why) = why_not_any(edit) {
        return Some(match why {
            NOTHING_PICKED => "Pick faces to inset first",
            other => other,
        });
    }
    (edit.mode != Mode::Faces).then_some("Only faces can be inset: press 3 and pick faces")
}

/// Why Split can't be done now, or `None` when it can.
pub(crate) fn why_not_split(edit: &Editing) -> Option<&'static str> {
    if let Some(why) = why_not_any(edit) {
        return Some(why);
    }
    if edit.mode != Mode::Points {
        return None;
    }
    let mesh = edit.parts.as_ref()?.mesh();
    match picked_points(edit)[..] {
        [a, b] => {
            let topology = mesh.topology();
            let shared: Vec<FaceId> = topology
                .vertex_faces(a)
                .iter()
                .copied()
                .filter(|f| mesh.face(*f).is_some_and(|c| c.contains(&b)))
                .collect();
            let joined = |f: &FaceId| {
                let corners = mesh.face(*f).unwrap_or(&[]);
                let n = corners.len();
                (0..n).any(|i| {
                    let (p, q) = (corners[i], corners[(i + 1) % n]);
                    (p, q) == (a, b) || (p, q) == (b, a)
                })
            };
            if shared.is_empty() {
                Some(mesh::split::NOT_ON_ONE_FACE)
            } else if shared.iter().all(joined) {
                Some(mesh::split::ALREADY_JOINED)
            } else {
                None
            }
        }
        _ => Some(mesh::split::NOT_ON_ONE_FACE),
    }
}

/// Why Delete can't be done now, or `None` when it can. In faces mode, a
/// picked face touching no other picked face would leave a hole.
pub(crate) fn why_not_delete(edit: &Editing) -> Option<&'static str> {
    if let Some(why) = why_not_any(edit) {
        return Some(why);
    }
    if edit.mode != Mode::Faces {
        return None;
    }
    let mesh = edit.parts.as_ref()?.mesh();
    let topology = mesh.topology();
    let lone = edit.picked.iter().any(|part| {
        let Part::Face(face) = *part else {
            return false;
        };
        !topology.face_edges(face).iter().any(|e| {
            topology
                .edge_faces(*e)
                .any(|f| f != face && edit.picked.contains(&Part::Face(f)))
        })
    });
    lone.then_some(mesh::merge::LONE_FACE)
}

/// The pull Extrude starts with `edit`'s pick, the brush placed by
/// `world`.
pub(crate) fn extrude(edit: &Editing, world: Mat4) -> Result<Pull, &'static str> {
    if let Some(why) = why_not_extrude(edit) {
        return Err(why);
    }
    let mesh = edit.parts.as_ref().ok_or(NOTHING_PICKED)?.mesh();
    let what = edit.picked_words();
    let pull = match edit.mode {
        Mode::Edges => {
            let topology = mesh.topology();
            let open: Vec<EdgeKey> = picked_edges(edit)
                .into_iter()
                .filter(|e| topology.is_border(*e))
                .collect();
            Pull::extrude_edges(mesh, &open, world, what)
        }
        _ => Pull::extrude(mesh, &picked_faces(edit), world, what),
    };
    pull.ok_or(NOTHING_PICKED)
}

/// The pull Inset starts with `edit`'s pick.
pub(crate) fn inset(edit: &Editing, world: Mat4) -> Result<Pull, &'static str> {
    if let Some(why) = why_not_inset(edit) {
        return Err(why);
    }
    let mesh = edit.parts.as_ref().ok_or(NOTHING_PICKED)?.mesh();
    Pull::inset(mesh, &picked_faces(edit), world, edit.picked_words()).ok_or(NOTHING_PICKED)
}

/// A shaped brush to keep: the mesh, the points around what changed (for
/// the whole check), its undo step's name, and what's picked after.
#[derive(Debug, Clone)]
pub(crate) struct Shaped {
    pub(crate) mesh: Mesh,
    pub(crate) around: Vec<VertexId>,
    pub(crate) label: String,
    pub(crate) picked: Vec<Part>,
}

/// `edit`'s brush split as its mode says.
pub(crate) fn split(edit: &Editing) -> Result<Shaped, &'static str> {
    if let Some(why) = why_not_split(edit) {
        return Err(why);
    }
    let before = edit.parts.as_ref().ok_or(NOTHING_PICKED)?.mesh();
    let mut mesh = before.clone();
    let label = format!("Split {}", edit.picked_words());
    let (around, picked) = match edit.mode {
        Mode::Points => {
            let points = picked_points(edit);
            let (a, b) = (points[0], points[1]);
            mesh::edit::split_face_between(&mut mesh, a, b)?;
            (vec![a, b], vec![Part::Point(a), Part::Point(b)])
        }
        Mode::Edges => {
            let edges = picked_edges(edit);
            let made = mesh::edit::split_edges(&mut mesh, &edges);
            let mut halves = Vec::new();
            for (edge, middle) in edges.iter().zip(&made) {
                let (a, b) = edge.ends();
                halves.push(Part::Edge(EdgeKey::new(a, *middle)));
                halves.push(Part::Edge(EdgeKey::new(*middle, b)));
            }
            (made, halves)
        }
        Mode::Faces => {
            let faces = picked_faces(edit);
            let made = mesh::edit::split_faces_across(&mut mesh, &faces)?;
            let around = faces
                .iter()
                .chain(&made)
                .flat_map(|f| mesh.face(*f).unwrap_or(&[]).to_vec())
                .collect();
            let picked = faces.iter().chain(&made).map(|f| Part::Face(*f)).collect();
            (around, picked)
        }
    };
    Ok(Shaped {
        mesh,
        around,
        label,
        picked,
    })
}

/// `edit`'s pick deleted and the gap closed, as its mode says.
pub(crate) fn delete(edit: &Editing) -> Result<Shaped, &'static str> {
    if let Some(why) = why_not_delete(edit) {
        return Err(why);
    }
    let before = edit.parts.as_ref().ok_or(NOTHING_PICKED)?.mesh();
    let mut mesh = before.clone();
    let merged = match edit.mode {
        Mode::Points => mesh::edit::dissolve_points(&mut mesh, &picked_points(edit))?,
        Mode::Edges => mesh::edit::dissolve_edges(&mut mesh, &picked_edges(edit))?,
        Mode::Faces => mesh::edit::merge_faces(&mut mesh, &picked_faces(edit))?,
    };
    let mut around: Vec<VertexId> = merged
        .iter()
        .flat_map(|f| mesh.face(*f).unwrap_or(&[]).to_vec())
        .collect();
    // The points of what was there before, still in use, for a hole.
    around.extend(
        super::edit::points_of(before, &edit.picked)
            .into_iter()
            .filter(|v| mesh.position(*v).is_some()),
    );
    let picked = match edit.mode {
        Mode::Faces => merged.iter().map(|f| Part::Face(*f)).collect(),
        Mode::Points | Mode::Edges => Vec::new(),
    };
    Ok(Shaped {
        mesh,
        around,
        label: format!("Delete {}", edit.picked_words()),
        picked,
    })
}
