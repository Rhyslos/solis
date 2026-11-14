//! Editing a brush's insides (Tools 1a, B3; decision 0111; the approved
//! mock-up `docs/design/tools-1/03-brush-edit-faces.png`): **Tab** with
//! one brush picked starts it, and Tab again (or Done) ends it. While
//! editing, **1**, **2** and **3** pick points, edges or faces, a click
//! picks one, Ctrl + click adds or takes away, a drag on empty space
//! picks with a box, and the gizmo moves, turns or stretches what's
//! picked.
//!
//! This is the state the viewport keeps while a brush is edited
//! ([`Editing`]): which brush, the mode, what's picked and under the
//! pointer, the picking geometry of the brush's mesh ([`super::parts`]),
//! and a gizmo drag under way ([`super::reshape`]). Parts are named by
//! the mesh's ids, which stay the same while points move, so a pick
//! survives every move, undo and redo that keeps the part.

use super::parts::PartCache;
use super::reshape::Drag;
use crate::brushes::BrushModel;
use mesh::{EdgeKey, FaceId, Mesh, VertexId};
use reflect::EntityId;
use std::collections::BTreeSet;
use std::sync::Arc;

/// What clicks pick while a brush is edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum Mode {
    /// Its points (1).
    Points,
    /// Its edges (2).
    Edges,
    /// Its faces (3), as the mock-up shows.
    #[default]
    Faces,
}

impl Mode {
    /// Every mode, in the ribbon's order.
    pub(crate) const ALL: [Self; 3] = [Self::Points, Self::Edges, Self::Faces];

    /// Its name on the ribbon and in the status bar.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Points => "Points",
            Self::Edges => "Edges",
            Self::Faces => "Faces",
        }
    }

    /// One of its parts, in words: "point", "edge", "face".
    pub(crate) fn one(self) -> &'static str {
        match self {
            Self::Points => "point",
            Self::Edges => "edge",
            Self::Faces => "face",
        }
    }

    /// The key that picks it while editing.
    pub(crate) fn key(self) -> egui::Key {
        match self {
            Self::Points => egui::Key::Num1,
            Self::Edges => egui::Key::Num2,
            Self::Faces => egui::Key::Num3,
        }
    }

    /// Its icon on the ribbon.
    pub(crate) fn icon(self) -> crate::icons::Icon {
        use crate::icons::Icon;
        match self {
            Self::Points => Icon::Line,
            Self::Edges => Icon::Pen,
            Self::Faces => Icon::Rectangle,
        }
    }
}

/// One part of a brush's mesh, by its ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Part {
    /// A point.
    Point(VertexId),
    /// An edge, by its two points.
    Edge(EdgeKey),
    /// A face.
    Face(FaceId),
}

impl Part {
    /// The mode it is picked in.
    pub(crate) fn mode(self) -> Mode {
        match self {
            Self::Point(_) => Mode::Points,
            Self::Edge(_) => Mode::Edges,
            Self::Face(_) => Mode::Faces,
        }
    }

    /// Whether `mesh` still has it.
    pub(crate) fn is_in(self, mesh: &Mesh) -> bool {
        match self {
            Self::Point(v) => mesh.position(v).is_some(),
            Self::Edge(e) => mesh.topology().has_edge(e),
            Self::Face(f) => mesh.face(f).is_some(),
        }
    }
}

/// How a click or a box changes the pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickHow {
    /// Only what was hit is picked (nothing, for a click on empty space).
    Replace,
    /// What was hit is added, or taken away if it was picked (Ctrl +
    /// click).
    Toggle,
    /// What was hit is added (Ctrl + a box).
    Add,
}

/// A brush whose insides are being edited, kept by the viewport.
#[derive(Debug, Clone)]
pub(crate) struct Editing {
    /// The brush's entity.
    pub(crate) entity: EntityId,
    /// Its name ("Box_03"), kept up to date each frame.
    pub(crate) name: String,
    /// What clicks pick.
    pub(crate) mode: Mode,
    /// What's picked, all of the mode's kind.
    pub(crate) picked: BTreeSet<Part>,
    /// The part under the pointer, lit before a click.
    pub(crate) hovered: Option<Part>,
    /// The undo history's state when editing began: an undo back past it
    /// ends editing.
    pub(crate) since: u64,
    /// Whether Home's own ribbon is in front (its tab clicked) instead of
    /// HOME TOOLS · Brush.
    pub(crate) home_ribbon: bool,
    /// The brush's mesh and its picking geometry, made again when the
    /// brush's model changes.
    pub(crate) parts: Option<Arc<PartCache>>,
    /// A gizmo drag under way.
    pub(crate) drag: Option<Drag>,
    /// While dragging, the brush as it would be: drawn in its place, its
    /// own mesh alone uploaded.
    pub(crate) preview: Option<Arc<BrushModel>>,
    /// An extrude or inset under way, following the mouse (B4).
    pub(crate) pull: Option<super::pull::Pull>,
}

impl Editing {
    /// Editing `entity`'s insides from history state `since`, in faces
    /// mode with nothing picked.
    pub(crate) fn new(entity: EntityId, since: u64) -> Self {
        Self {
            entity,
            name: String::new(),
            mode: Mode::Faces,
            picked: BTreeSet::new(),
            hovered: None,
            since,
            home_ribbon: false,
            parts: None,
            drag: None,
            preview: None,
            pull: None,
        }
    }

    /// The mesh drawn and picked now: the drag's while dragging, the
    /// pull's while extruding or insetting, else the brush's.
    pub(crate) fn mesh(&self) -> Option<&Mesh> {
        if let Some(drag) = &self.drag {
            return Some(drag.now());
        }
        if let Some(pull) = &self.pull {
            return Some(pull.now());
        }
        self.parts.as_ref().map(|p| p.mesh())
    }

    /// Switches to `mode`, keeping what was picked as the new mode's
    /// parts: the points of the edges or faces picked, or the edges and
    /// faces wholly made of the points picked.
    pub(crate) fn switch_to(&mut self, mode: Mode) {
        if mode == self.mode {
            return;
        }
        self.mode = mode;
        self.hovered = None;
        let Some(mesh) = self.parts.as_ref().map(|p| p.mesh()) else {
            self.picked.clear();
            return;
        };
        let points: BTreeSet<VertexId> = points_of(mesh, &self.picked).into_iter().collect();
        self.picked = match mode {
            Mode::Points => points.into_iter().map(Part::Point).collect(),
            Mode::Edges => mesh
                .edges()
                .iter()
                .filter(|e| {
                    let (a, b) = e.ends();
                    points.contains(&a) && points.contains(&b)
                })
                .map(|e| Part::Edge(*e))
                .collect(),
            Mode::Faces => mesh
                .faces()
                .filter(|(_, corners)| corners.iter().all(|v| points.contains(v)))
                .map(|(f, _)| Part::Face(f))
                .collect(),
        };
    }

    /// Changes the pick by `hits`, as `how` says.
    pub(crate) fn pick(&mut self, hits: &[Part], how: PickHow) {
        let hits = hits.iter().filter(|p| p.mode() == self.mode);
        match how {
            PickHow::Replace => self.picked = hits.copied().collect(),
            PickHow::Add => self.picked.extend(hits),
            PickHow::Toggle => {
                for hit in hits {
                    if !self.picked.remove(hit) {
                        self.picked.insert(*hit);
                    }
                }
            }
        }
    }

    /// Picks every part of the mode.
    pub(crate) fn pick_all(&mut self) {
        let Some(mesh) = self.parts.as_ref().map(|p| p.mesh()) else {
            return;
        };
        self.picked = match self.mode {
            Mode::Points => mesh.vertices().map(|(v, _)| Part::Point(v)).collect(),
            Mode::Edges => mesh.edges().iter().map(|e| Part::Edge(*e)).collect(),
            Mode::Faces => mesh.faces().map(|(f, _)| Part::Face(f)).collect(),
        };
    }

    /// Lets go of picked parts the brush no longer has (after an undo, or
    /// Made as).
    pub(crate) fn forget_missing(&mut self) {
        if let Some(parts) = &self.parts {
            let mesh = parts.mesh();
            self.picked.retain(|p| p.is_in(mesh));
            self.hovered = self.hovered.filter(|p| p.is_in(mesh));
        }
    }

    /// What's picked, in words: "1 face", "3 points", "nothing".
    pub(crate) fn picked_words(&self) -> String {
        match self.picked.len() {
            0 => "nothing".to_owned(),
            1 => format!("1 {}", self.mode.one()),
            n => format!("{} {}s", crate::models::grouped(n), self.mode.one()),
        }
    }

    /// What the status bar says: "Editing Box_03 · Faces · 1 face
    /// picked", or while extruding "Extruding 1 face: 2.00 m · …".
    pub(crate) fn status_words(&self) -> String {
        if let Some(pull) = &self.pull {
            return pull.status_words();
        }
        let name = &self.name;
        let picked = match self.picked.len() {
            0 => "nothing picked".to_owned(),
            _ => format!("{} picked", self.picked_words()),
        };
        format!("Editing {name} \u{b7} {} \u{b7} {picked}", self.mode.name())
    }

    /// What the text at the top of the view says: "Editing Box_03 ·
    /// Faces · 1 picked", or while extruding "Extruding 1 face · 2.00 m".
    pub(crate) fn top_words(&self) -> String {
        if let Some(pull) = &self.pull {
            return pull.top_words();
        }
        let name = &self.name;
        format!(
            "Editing {name} \u{b7} {} \u{b7} {} picked",
            self.mode.name(),
            crate::models::grouped(self.picked.len())
        )
    }
}

/// The points `picked` parts move: every point of each, once, in id
/// order. Parts `mesh` doesn't have are left out.
pub(crate) fn points_of(mesh: &Mesh, picked: &BTreeSet<Part>) -> Vec<VertexId> {
    let mut points = BTreeSet::new();
    for part in picked {
        match *part {
            Part::Point(v) => {
                if mesh.position(v).is_some() {
                    points.insert(v);
                }
            }
            Part::Edge(e) => {
                let (a, b) = e.ends();
                if mesh.position(a).is_some() && mesh.position(b).is_some() {
                    points.extend([a, b]);
                }
            }
            Part::Face(f) => points.extend(mesh.face(f).unwrap_or(&[]).iter().copied()),
        }
    }
    points.into_iter().collect()
}

/// How many points, edges and faces `mesh` has, for the Brush card.
pub(crate) fn counts(mesh: &Mesh) -> [usize; 3] {
    [mesh.vertex_count(), mesh.edges().len(), mesh.face_count()]
}
