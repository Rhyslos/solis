//! The Problems panel: mistakes found in the open area, such as a model
//! whose file is gone or a component no one knows. Each names its entity;
//! clicking one selects it. Looked for again only when something they
//! depend on changed, so a big area costs nothing frame to frame.

use crate::icons::{self, Icon};
use crate::models::ModelInfo;
use crate::theme;
use area::Area;
use area::builtin::Model;
use egui::{RichText, Ui};
use reflect::{AssetId, EntityId, Registry};
use std::collections::{BTreeMap, HashSet};

/// What kind of mistake a problem is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProblemKind {
    /// A Model component whose model isn't in the project.
    MissingModel,
    /// A component no one has registered (perhaps from a newer version,
    /// or game code not built yet): kept as it is, but not used.
    UnknownComponent,
    /// Part of an assembly whose file isn't in the project.
    MissingAssembly,
    /// An entity whose parent isn't in the area (perhaps after a merge).
    MissingParent,
    /// Entities that are each other's parents (a damaged file): they're
    /// placed differently by the editor and the game until it's fixed.
    ParentLoop,
}

/// One mistake, and the entity it's on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// What kind it is.
    pub kind: ProblemKind,
    /// The entity it's on.
    pub entity: EntityId,
    /// What's wrong, in plain words, naming the entity.
    pub text: String,
}

/// What the problems were found from, to tell when to look again.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Fingerprint {
    revision: u64,
    sources: u64,
    file: Option<std::path::PathBuf>,
}

/// The problems last found, and what they were found from.
#[derive(Debug, Clone, Default)]
pub(crate) struct Problems {
    pub(crate) list: Vec<Problem>,
    from: Option<Fingerprint>,
}

/// What problems are found in.
pub(crate) struct Sources<'a> {
    pub(crate) area: &'a Area,
    pub(crate) registry: &'a Registry,
    /// Models loaded, by asset.
    pub(crate) models: &'a BTreeMap<AssetId, ModelInfo>,
    /// Every asset in the project (from the Library).
    pub(crate) known: &'a HashSet<AssetId>,
    /// Assemblies read, by asset.
    pub(crate) assemblies: &'a HashSet<AssetId>,
}

impl Problems {
    /// Whether the problems were found from `fingerprint` (nothing they
    /// depend on changed since).
    pub(crate) fn is_from(&self, fingerprint: &Fingerprint) -> bool {
        self.from.as_ref() == Some(fingerprint)
    }

    /// Looks for problems again if anything they depend on changed.
    /// Returns how many more there are than before.
    pub(crate) fn refresh(&mut self, fingerprint: Fingerprint, sources: &Sources) -> usize {
        if self.is_from(&fingerprint) {
            return 0;
        }
        let before = self.list.len();
        self.list = find(sources);
        self.from = Some(fingerprint);
        self.list.len().saturating_sub(before)
    }
}

impl Fingerprint {
    /// What problems in `document` depend on, with `sources` counting
    /// the changes to what it's checked against.
    pub(crate) fn of(document: &crate::document::Document, sources: u64) -> Self {
        Self {
            // Not every revision: moving things changes nothing checked.
            revision: document.shape(),
            sources,
            file: document.file().map(std::path::Path::to_path_buf),
        }
    }
}

/// Every problem in `sources.area`, entity by entity, in a steady order.
pub(crate) fn find(sources: &Sources) -> Vec<Problem> {
    let area = sources.area;
    let mut found = Vec::new();
    // Entities a walk down from the top never reaches are caught in a loop
    // of parents (the tree order lists them last).
    let mut reached = std::collections::HashSet::new();
    let mut todo: Vec<EntityId> = area
        .iter()
        .filter(|(_, e)| e.parent.is_none_or(|p| !area.contains(p)))
        .map(|(id, _)| id)
        .collect();
    let mut children: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
    for (id, e) in area.iter() {
        if let Some(p) = e.parent {
            children.entry(p).or_default().push(id);
        }
    }
    while let Some(id) = todo.pop() {
        if reached.insert(id)
            && let Some(kids) = children.get(&id)
        {
            todo.extend(kids);
        }
    }
    for (id, entity) in area.iter() {
        if !reached.contains(&id) {
            found.push(Problem {
                kind: ProblemKind::ParentLoop,
                entity: id,
                text: format!(
                    "{} is inside itself: its parents lead back to it. Drag it onto empty \
                     space in the Area panel to fix it",
                    entity.name
                ),
            });
        }
    }
    for (id, entity) in area.iter() {
        let name = &entity.name;
        if let Some(parent) = entity.parent
            && !area.contains(parent)
        {
            found.push(Problem {
                kind: ProblemKind::MissingParent,
                entity: id,
                text: format!("{name}'s parent ({parent}) isn't in the area"),
            });
        }
        for component in entity.components.keys() {
            if sources.registry.get(component).is_none() {
                found.push(Problem {
                    kind: ProblemKind::UnknownComponent,
                    entity: id,
                    text: format!(
                        "{name} has a {component} component that isn't known; it's kept, \
                         but does nothing"
                    ),
                });
            }
        }
        if let Some(model) = entity.get::<Model>() {
            match model.asset.map(|r| r.asset) {
                None => found.push(Problem {
                    kind: ProblemKind::MissingModel,
                    entity: id,
                    text: format!("{name}'s Model has no model chosen"),
                }),
                Some(asset)
                    if !sources.models.contains_key(&asset) && !sources.known.contains(&asset) =>
                {
                    found.push(Problem {
                        kind: ProblemKind::MissingModel,
                        entity: id,
                        text: format!(
                            "{name}'s model ({asset}) isn't in the project: its file was \
                             deleted, or never committed"
                        ),
                    });
                }
                Some(_) => {}
            }
        }
        // Only the copy's root names its assembly.
        if let Some(origin) = &entity.origin
            && origin.path.is_empty()
            && !sources.assemblies.contains(&origin.assembly)
            && !sources.known.contains(&origin.assembly)
        {
            found.push(Problem {
                kind: ProblemKind::MissingAssembly,
                entity: id,
                text: format!(
                    "{name} is a copy of an assembly ({}) that isn't in the project",
                    origin.assembly
                ),
            });
        }
    }
    found
}

/// The Problems panel: each problem with its icon; clicking one returns
/// its entity.
pub(crate) fn problems_panel(ui: &mut Ui, problems: &[Problem]) -> Option<EntityId> {
    if problems.is_empty() {
        crate::panels::placeholder(ui, "No problems found in the area.");
        return None;
    }
    let mut clicked = None;
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            let s = if problems.len() == 1 { "" } else { "s" };
            ui.label(
                RichText::new(format!(
                    "{} problem{s} in the area. Click one to select its entity.",
                    problems.len()
                ))
                .color(theme::TEXT_WEAK),
            );
            ui.add_space(4.0);
            egui::ScrollArea::vertical().auto_shrink(false).show_rows(
                ui,
                ROW,
                problems.len(),
                |ui, rows| {
                    for problem in &problems[rows] {
                        if problem_row(ui, problem).clicked() {
                            clicked = Some(problem.entity);
                        }
                    }
                },
            );
        });
    clicked
}

/// The height of one problem's row.
const ROW: f32 = 20.0;

fn problem_row(ui: &mut Ui, problem: &Problem) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW), egui::Sense::click());
    let response = response.on_hover_text(&problem.text);
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &problem.text));
    if response.hovered() {
        ui.painter().rect_filled(rect, 3.0, theme::HOVER);
    }
    let size = 11.0;
    icons::icon(
        ui.painter(),
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 4.0, rect.center().y - size / 2.0),
            egui::vec2(size, size),
        ),
        Icon::Warning,
        theme::WARNING,
    );
    // Painted rather than a label, so the row is one thing to a screen
    // reader: its button.
    let left = rect.left() + 4.0 + size + 6.0;
    let mut job = egui::text::LayoutJob::simple_singleline(
        problem.text.clone(),
        egui::TextStyle::Body.resolve(ui.style()),
        theme::TEXT,
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width((rect.right() - left).max(0.0));
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(
        egui::pos2(left, rect.center().y - galley.size().y / 2.0),
        galley,
        theme::TEXT,
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use area::EntityData;
    use area::builtin::Transform;
    use reflect::{ComponentData, Value};

    struct World {
        area: Area,
        registry: Registry,
        models: BTreeMap<AssetId, ModelInfo>,
        known: HashSet<AssetId>,
        assemblies: HashSet<AssetId>,
    }

    impl World {
        fn new() -> Self {
            Self {
                area: Area::default(),
                registry: area::builtin::registry().unwrap(),
                models: BTreeMap::new(),
                known: HashSet::new(),
                assemblies: HashSet::new(),
            }
        }

        fn find(&self) -> Vec<Problem> {
            find(&Sources {
                area: &self.area,
                registry: &self.registry,
                models: &self.models,
                known: &self.known,
                assemblies: &self.assemblies,
            })
        }

        fn kinds(&self) -> Vec<ProblemKind> {
            self.find().into_iter().map(|p| p.kind).collect()
        }
    }

    fn model(asset: Option<AssetId>) -> Model {
        Model {
            asset: asset.map(|asset| reflect::AssetRef { asset, part: None }),
        }
    }

    #[test]
    fn a_sound_area_has_no_problems() {
        let mut w = World::new();
        let asset = AssetId::random();
        w.known.insert(asset);
        w.area.insert(
            EntityId::random(),
            EntityData::new("Crate")
                .with(&Transform::default())
                .with(&model(Some(asset))),
        );
        assert!(w.find().is_empty());
        assert!(
            World {
                area: area::builtin::starter_area(),
                ..World::new()
            }
            .find()
            .is_empty(),
            "nor does a new area"
        );
    }

    #[test]
    fn missing_models_are_found_unless_loaded_or_in_the_project() {
        let mut w = World::new();
        let (gone, loaded) = (AssetId::random(), AssetId::random());
        let a = EntityId::random();
        w.area
            .insert(a, EntityData::new("Crate").with(&model(Some(gone))));
        w.area.insert(
            EntityId::random(),
            EntityData::new("Barrel").with(&model(Some(loaded))),
        );
        w.area.insert(
            EntityId::random(),
            EntityData::new("Empty").with(&model(None)),
        );
        let mut info = crate::test_support::crate_model();
        info.asset = loaded;
        w.models.insert(loaded, info);
        let found = w.find();
        assert_eq!(found.len(), 2, "{found:?}");
        let crate_problem = found.iter().find(|p| p.entity == a).unwrap();
        assert!(crate_problem.text.starts_with("Crate's model"));
        assert!(
            found
                .iter()
                .any(|p| p.text == "Empty's Model has no model chosen")
        );
        w.known.insert(gone);
        assert_eq!(w.find().len(), 1, "it's in the project, still loading");
    }

    #[test]
    fn unknown_components_missing_parents_and_assemblies_are_found() {
        let mut w = World::new();
        let mut odd = EntityData::new("Odd");
        let mut data = ComponentData::default();
        data.set("speed", Value::Float(2.0));
        odd.components.insert("Wobble".into(), data);
        odd.parent = Some(EntityId::random());
        w.area.insert(EntityId::random(), odd);
        let root = EntityId::random();
        let assembly = AssetId::random();
        let mut copy = EntityData::new("Boat");
        copy.origin = Some(area::Origin {
            assembly,
            instance: root,
            path: Vec::new(),
        });
        w.area.insert(root, copy);
        let mut inner = EntityData::new("Mast");
        inner.parent = Some(root);
        inner.origin = Some(area::Origin {
            assembly,
            instance: root,
            path: vec![EntityId::random()],
        });
        w.area.insert(EntityId::random(), inner);
        let mut kinds = w.kinds();
        kinds.sort();
        assert_eq!(
            kinds,
            [
                ProblemKind::UnknownComponent,
                ProblemKind::MissingAssembly,
                ProblemKind::MissingParent
            ],
            "the copy is named once, at its root"
        );
        w.assemblies.insert(assembly);
        assert_eq!(w.kinds().len(), 2);
    }

    #[test]
    fn problems_are_looked_for_again_only_after_a_change() {
        let document = crate::document::Document::default();
        let mut problems = Problems::default();
        let mut w = World::new();
        w.area.insert(
            EntityId::random(),
            EntityData::new("Crate").with(&model(None)),
        );
        let sources = Sources {
            area: &w.area,
            registry: &w.registry,
            models: &w.models,
            known: &w.known,
            assemblies: &w.assemblies,
        };
        let print = Fingerprint::of(&document, 0);
        assert_eq!(problems.refresh(print.clone(), &sources), 1, "one new");
        problems.list.clear();
        assert_eq!(
            problems.refresh(print.clone(), &sources),
            0,
            "not looked at"
        );
        assert!(problems.list.is_empty());
        assert_eq!(problems.refresh(Fingerprint::of(&document, 1), &sources), 1);
        assert_eq!(problems.refresh(Fingerprint::of(&document, 2), &sources), 0);
        assert_eq!(problems.list.len(), 1, "the same one, not new");
    }

    #[test]
    fn entities_that_are_each_others_parents_are_reported() {
        let mut w = World::new();
        let (a, b) = (EntityId::random(), EntityId::random());
        let mut ea = EntityData::new("A");
        ea.parent = Some(b);
        let mut eb = EntityData::new("B");
        eb.parent = Some(a);
        w.area.insert(a, ea);
        w.area.insert(b, eb);
        w.area.spawn(EntityData::new("Fine"));
        assert_eq!(
            w.kinds(),
            [ProblemKind::ParentLoop, ProblemKind::ParentLoop]
        );
        assert!(w.find()[0].text.contains("inside itself"));
    }

    #[test]
    fn problems_are_looked_for_again_when_what_they_are_checked_against_changes() {
        // Found by review: counting models, assemblies and Library files
        // missed one replaced by another (a Git branch switch).
        let document = crate::document::Document::default();
        let mut problems = Problems::default();
        let mut w = World::new();
        let gone = AssetId::random();
        w.area.insert(
            EntityId::random(),
            EntityData::new("Crate").with(&model(Some(gone))),
        );
        fn sources(w: &World) -> Sources<'_> {
            Sources {
                area: &w.area,
                registry: &w.registry,
                models: &w.models,
                known: &w.known,
                assemblies: &w.assemblies,
            }
        }
        problems.refresh(Fingerprint::of(&document, 7), &sources(&w));
        assert_eq!(problems.list.len(), 1);
        w.known.insert(gone);
        // Same document, sources changed: looked at again.
        assert!(!problems.is_from(&Fingerprint::of(&document, 8)));
        problems.refresh(Fingerprint::of(&document, 8), &sources(&w));
        assert!(problems.list.is_empty());
    }
}
