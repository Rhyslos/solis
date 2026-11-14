//! A brush (Tools 1a, decision 0111): a shape made in the editor, a
//! polygon mesh of n-sided faces held in the area itself, drawn flat and,
//! when Solid, something the game's characters stand on and bump into.
//!
//! In an area file its mesh is three lists of numbers, written a few to a
//! line ([`crate::text`]):
//!
//! - `points`: x, y, z for each point, in metres, in the entity's space;
//! - `faces`: for each face, its number of corners and then its points'
//!   places in `points` (from 0), counter-clockwise seen from outside;
//! - `materials`: each face's material slot (0, the plain default, until
//!   brush materials arrive).
//!
//! A file is untrusted (CONTRIBUTING.md, rule 9): every count is checked
//! against the limits below before anything is made, and a mesh that
//! breaks one, or a face that names a point twice or one that isn't there,
//! reads as an empty brush ([`Brush::damaged`]), never a crash; why is
//! [`damage_of`], which the editor's Problems list shows. The file's own
//! values stay in the area untouched until the brush is changed.

use mesh::material::set_material;
use mesh::{FaceId, Mesh, Shape, VertexId};
use reflect::{
    Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Value,
};
use std::collections::HashMap;
use std::sync::Arc;

/// The most points one brush has.
pub const MAX_POINTS: usize = 65_536;
/// The most faces one brush has.
pub const MAX_FACES: usize = 65_536;
/// The most corners one face has.
pub const MAX_CORNERS_PER_FACE: usize = 256;
/// The most corners of all its faces together.
pub const MAX_CORNERS: usize = 262_144;
/// The highest material slot a face can name.
pub const MAX_SLOT: i64 = 255;

/// A shape made in the editor, at the entity.
#[derive(Debug, Clone, PartialEq)]
pub struct Brush {
    /// What it was made as.
    pub shape: Shape,
    /// Whether the game's characters and bodies collide with it.
    pub solid: bool,
    /// Its points and faces, in the entity's space. Shared, so copying a
    /// brush (the game's world does each tick it's asked for) is cheap.
    pub mesh: Arc<Mesh>,
    /// Whether the mesh read was damaged (and so is empty).
    pub damaged: bool,
}

impl Default for Brush {
    /// A solid box of the standard size.
    fn default() -> Self {
        Self::of_shape(Shape::Box)
    }
}

impl Brush {
    /// A solid brush made as `shape`, at its standard settings.
    pub fn of_shape(shape: Shape) -> Self {
        Self {
            shape,
            solid: true,
            mesh: Arc::new(shape.build(&shape.standard())),
            damaged: false,
        }
    }
}

fn shape_names() -> Vec<String> {
    Shape::ALL.iter().map(|s| s.name().to_string()).collect()
}

impl Component for Brush {
    const NAME: &'static str = "Brush";

    fn schema() -> ComponentSchema {
        let standard = Self::default().to_data();
        let numbers = |name: &str, label: &str, help: &'static str| {
            FieldInfo::new(
                name,
                label,
                FieldKind::Numbers,
                standard
                    .get(name)
                    .cloned()
                    .unwrap_or(Value::List(Vec::new())),
            )
            .hint(Hint {
                help,
                ..Hint::default()
            })
        };
        ComponentSchema {
            name: Self::NAME.into(),
            help: "A shape made in the editor: points joined into flat faces, drawn at the \
                   entity and, when Solid, walked on and bumped into in the game."
                .into(),
            fields: vec![
                FieldInfo::new(
                    "shape",
                    "Made as",
                    FieldKind::Choice(shape_names()),
                    Value::Text(Shape::Box.name().into()),
                )
                .hint(Hint {
                    help: "The shape the brush was made as.",
                    ..Hint::default()
                }),
                FieldInfo::new("solid", "Solid", FieldKind::Bool, Value::Bool(true)).hint(Hint {
                    help: "Whether characters and bodies in the game collide with it.",
                    ..Hint::default()
                }),
                numbers(
                    "points",
                    "Points",
                    "Where its points are: three numbers (x, y, z) for each, in metres.",
                ),
                numbers(
                    "faces",
                    "Faces",
                    "Its faces: each one's number of corners, then its points.",
                ),
                numbers(
                    "materials",
                    "Materials",
                    "Each face's material: 0 is the plain one.",
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let (points, faces, materials) = lists_of(&self.mesh);
        let mut data = ComponentData::default();
        data.set("shape", Value::Text(self.shape.name().into()));
        data.set("solid", Value::Bool(self.solid));
        data.set("points", Value::List(points));
        data.set("faces", Value::List(faces));
        data.set("materials", Value::List(materials));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let fields = Fields(data);
        let shape = fields
            .text("shape")
            .and_then(Shape::from_name)
            .unwrap_or(Shape::Box);
        let solid = fields.bool("solid", true);
        let (mesh, damaged) = match (data.get("points"), data.get("faces")) {
            // Neither: a brush written by hand as only its shape.
            (None, None) => (shape.build(&shape.standard()), false),
            (points, faces) => match mesh_of(points, faces, data.get("materials")) {
                Ok(mesh) => (mesh, false),
                Err(_) => (Mesh::new(), true),
            },
        };
        Self {
            shape,
            solid,
            mesh: Arc::new(mesh),
            damaged,
        }
    }
}

/// The mesh's points as the area file's list of them: x, y, z for each,
/// in id order (gaps left by deleted ones closed up, as
/// [`Brush::to_data`] writes them). When only points moved (editing a
/// brush's insides, Tools 1a B3), this is the one list that changes.
pub fn points_of(mesh: &Mesh) -> Vec<Value> {
    mesh.vertices()
        .flat_map(|(_, at)| at.to_array().map(|v| Value::Float(f64::from(v))))
        .collect()
}

/// The mesh as the three lists an area file holds: its points (in id
/// order, gaps left by deleted ones closed up), its faces by those places,
/// and each face's material slot.
fn lists_of(mesh: &Mesh) -> (Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut place: HashMap<VertexId, i64> = HashMap::with_capacity(mesh.vertex_count());
    for (id, _) in mesh.vertices() {
        place.insert(id, place.len() as i64);
    }
    let points = points_of(mesh);
    let mut faces = Vec::new();
    let mut materials = Vec::with_capacity(mesh.face_count());
    for (face, corners) in mesh.faces() {
        faces.push(Value::Int(corners.len() as i64));
        faces.extend(
            corners
                .iter()
                .map(|v| Value::Int(place.get(v).copied().unwrap_or(0))),
        );
        materials.push(Value::Int(i64::from(mesh::material::slot_of(mesh, face))));
    }
    (points, faces, materials)
}

/// Why a brush's lists read as no mesh (each count is checked against
/// the limits before anything is made), in plain words, or `None` when
/// they read: for the Problems list, which names a damaged brush and why
/// (Tools 1a, B2). A brush with neither list (written by hand as only its
/// shape) isn't damaged.
pub fn damage_of(data: &ComponentData) -> Option<&'static str> {
    match (data.get("points"), data.get("faces")) {
        (None, None) => None,
        (points, faces) => mesh_of(points, faces, data.get("materials")).err(),
    }
}

/// The mesh the lists read from a file describe, or why not if any count
/// is past its limit or anything doesn't fit (checked before the mesh is
/// made, so a hostile file can't make Solis set aside more than the
/// limits allow).
fn mesh_of(
    points: Option<&Value>,
    faces: Option<&Value>,
    materials: Option<&Value>,
) -> Result<Mesh, &'static str> {
    let (Some(Value::List(points)), Some(Value::List(faces))) = (points, faces) else {
        return Err("its list of points or of faces is missing, or isn't a list of numbers");
    };
    if !points.len().is_multiple_of(3) {
        return Err("its points aren't in threes (x, y, z)");
    }
    if points.len() / 3 > MAX_POINTS {
        return Err("it has more points than a brush may hold (65,536)");
    }
    // A face takes at least four numbers (a count and three corners).
    if faces.len() > MAX_CORNERS + MAX_FACES {
        return Err("its list of faces is longer than a brush may hold");
    }
    let mut slots = Vec::with_capacity(points.len() / 3);
    for xyz in points.chunks_exact(3) {
        let [x, y, z] = [&xyz[0], &xyz[1], &xyz[2]].map(Value::as_f32);
        let (Some(x), Some(y), Some(z)) = (x, y, z) else {
            return Err("one of its points isn't a number");
        };
        slots.push(Some(glam::Vec3::new(x, y, z)));
    }
    let count = slots.len();
    let mut corner_lists = Vec::new();
    let mut total = 0usize;
    let mut numbers = faces.iter();
    while let Some(n) = numbers.next() {
        let n = match n {
            Value::Int(n) => usize::try_from(*n).ok(),
            _ => None,
        };
        let Some(n) = n else {
            return Err("a face's count of corners isn't a whole number");
        };
        total += n;
        if !(3..=MAX_CORNERS_PER_FACE).contains(&n) {
            return Err("a face has fewer than 3 corners, or more than 256");
        }
        if corner_lists.len() >= MAX_FACES || total > MAX_CORNERS {
            return Err("it has more faces or corners than a brush may hold (65,536 faces)");
        }
        let mut corners = Vec::with_capacity(n);
        for _ in 0..n {
            let Some(Value::Int(i)) = numbers.next() else {
                return Err("a face's list of corners ends early");
            };
            let Some(i) = usize::try_from(*i).ok().filter(|i| *i < count) else {
                return Err("a face names a point that isn't there");
            };
            corners.push(VertexId::from_raw(i as u32));
        }
        corner_lists.push(Some(corners));
    }
    // `from_slots` refuses a face naming a point twice.
    let mut mesh = Mesh::from_slots(slots, corner_lists, [])
        .map_err(|_| "a face names one of its points twice")?;
    if let Some(Value::List(materials)) = materials {
        give_materials(&mut mesh, materials);
    }
    Ok(mesh)
}

/// Gives each face the material slot listed for it; a slot that isn't a
/// usable number, or a face the list doesn't reach, keeps the plain one.
fn give_materials(mesh: &mut Mesh, materials: &[Value]) {
    let mut by_slot: HashMap<u32, Vec<FaceId>> = HashMap::new();
    for ((face, _), slot) in mesh.faces().zip(materials) {
        if let Value::Int(slot) = slot
            && (1..=MAX_SLOT).contains(slot)
        {
            by_slot.entry(*slot as u32).or_default().push(face);
        }
    }
    for (slot, faces) in by_slot {
        set_material(mesh, &faces, slot);
    }
}

#[cfg(test)]
#[path = "brush_tests.rs"]
mod tests;
