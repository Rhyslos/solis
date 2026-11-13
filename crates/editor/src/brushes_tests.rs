//! Brushes are cut into triangles once, again only when they change,
//! shared when they're the same, and drawn where their entity is.

use super::*;
use crate::document::Document;
use area::builtin::Transform;
use area::{Edit, EntityData};
use mesh::Shape;

fn with_boxes(count: usize) -> (Document, Vec<EntityId>) {
    let mut area = Area::new();
    let mut ids = Vec::new();
    for n in 0..count {
        ids.push(
            area.spawn(
                EntityData::new(&format!("Box_{n}"))
                    .with(&Transform {
                        position: Vec3::new(n as f32 * 3.0, 0.0, 0.0),
                        ..Transform::default()
                    })
                    .with(&Brush::default()),
            ),
        );
    }
    (Document::new(area), ids)
}

fn refresh(models: &mut BrushModels, document: &Document) -> bool {
    models.refresh(document.area(), document.brushes())
}

#[test]
fn every_brush_is_cut_into_triangles_and_the_same_ones_share_a_model() {
    let (document, ids) = with_boxes(50);
    let mut models = BrushModels::default();
    assert!(refresh(&mut models, &document));
    assert_eq!(models.models(), 1, "fifty boxes, one model");
    assert_eq!(models.made(), 1);
    for id in &ids {
        let model = models.get(*id).unwrap();
        assert_eq!(model.triangles, 12);
        assert_eq!(model.model.meshes.len(), 1);
        assert_eq!(model.model.meshes[0].indices.len(), 36);
    }
    assert_eq!(models.iter().count(), 50);
}

#[test]
fn nothing_is_made_again_until_a_brush_changes() {
    let (mut document, ids) = with_boxes(3);
    let mut models = BrushModels::default();
    refresh(&mut models, &document);
    assert!(!refresh(&mut models, &document), "the same revision");
    // Moving a brush is no change to its triangles.
    document
        .edit(
            "Move",
            vec![Edit::SetField {
                entity: ids[0],
                component: Transform::NAME.into(),
                field: "position".into(),
                value: Some(Value::floats(&[9.0, 9.0, 9.0])),
            }],
            None,
            None,
        )
        .unwrap();
    assert!(
        !refresh(&mut models, &document),
        "a move leaves brushes alone"
    );
    // Changing one brush makes one model.
    let ramp = Brush::of_shape(Shape::Ramp).to_data();
    document
        .edit(
            "Ramp",
            vec![
                Edit::SetField {
                    entity: ids[1],
                    component: Brush::NAME.into(),
                    field: "points".into(),
                    value: Some(ramp.get("points").unwrap().clone()),
                },
                Edit::SetField {
                    entity: ids[1],
                    component: Brush::NAME.into(),
                    field: "faces".into(),
                    value: Some(ramp.get("faces").unwrap().clone()),
                },
            ],
            None,
            None,
        )
        .unwrap();
    let made = models.made();
    assert!(refresh(&mut models, &document));
    assert_eq!(models.made(), made + 1, "only the changed brush");
    assert_eq!(models.models(), 2);
    assert_eq!(models.get(ids[1]).unwrap().triangles, 8);
    // Undo brings the box back without making it again: it was kept.
    document.undo().unwrap().unwrap();
    assert!(refresh(&mut models, &document));
    assert_eq!(models.models(), 1);
    assert!(models.made() <= made + 2);
}

#[test]
fn a_brush_deleted_or_added_is_seen() {
    let (mut document, ids) = with_boxes(2);
    let mut models = BrushModels::default();
    refresh(&mut models, &document);
    document
        .edit("Delete", vec![Edit::Despawn { root: ids[0] }], None, None)
        .unwrap();
    refresh(&mut models, &document);
    assert!(models.get(ids[0]).is_none());
    assert!(models.get(ids[1]).is_some());
}

#[test]
fn a_damaged_brush_draws_nothing_and_crashes_nothing() {
    let mut area = Area::new();
    let mut data = Brush::default().to_data();
    data.set("faces", Value::List(vec![Value::Int(3), Value::Int(99)]));
    let mut entity = EntityData::new("Broken");
    entity.components.insert(Brush::NAME.into(), data);
    let id = area.spawn(entity);
    let document = Document::new(area);
    let mut models = BrushModels::default();
    refresh(&mut models, &document);
    let model = models.get(id).unwrap();
    assert_eq!(model.triangles, 0);
    assert!(model.model.meshes.is_empty());
}

#[test]
fn the_view_draws_brushes_where_their_entities_are_and_picks_them() {
    let (document, ids) = with_boxes(3);
    let mut models = BrushModels::default();
    refresh(&mut models, &document);
    let hidden: HashSet<EntityId> = [ids[1]].into();
    let locked: HashSet<EntityId> = [ids[2]].into();
    let mut view = crate::area_view::AreaView::of(document.area(), &Default::default());
    view.add_brushes(
        document.area(),
        &models,
        &document.world_matrices(),
        (&hidden, &locked),
    );
    assert_eq!(view.brushes.len(), 2, "the hidden one is left out");
    let first = view.brushes.iter().find(|b| b.entity == ids[0]).unwrap();
    assert_eq!(first.transform.w_axis.x, 0.0);
    assert_eq!(view.pickable[first.pick as usize - 1], ids[0]);
    let locked_one = view.brushes.iter().find(|b| b.entity == ids[2]).unwrap();
    assert_eq!(locked_one.pick, 0);
    assert_eq!(locked_one.transform.w_axis.x, 6.0);
    assert_eq!(view.triangles, 24);
}

use std::collections::HashSet;

#[test]
fn the_box_brush_command_adds_a_solid_box_as_one_undo_step() {
    let mut w = crate::Workbench::new(crate::test_support::info());
    let ctx = egui::Context::default();
    let before = w.document().area().len();
    crate::test_support::do_command(&mut w, &ctx, "Add box brush");
    crate::test_support::do_command(&mut w, &ctx, "Add box brush");
    let area = w.document().area();
    assert_eq!(area.len(), before + 2);
    let mut names: Vec<String> = area
        .with::<Brush>()
        .map(|(id, brush)| {
            assert_eq!(brush, Brush::default());
            area.get(id).unwrap().name.clone()
        })
        .collect();
    names.sort();
    assert_eq!(names, ["Box_01", "Box_02"]);
    let picked = w.document().selection.primary().unwrap();
    assert_eq!(
        area.get(picked).unwrap().name,
        "Box_02",
        "the new one is selected"
    );
    w.undo();
    assert_eq!(w.document().area().len(), before + 1, "one undo step each");
    // It's drawn, and can be picked in the viewport.
    let document = &w.open.document;
    w.open.brushes.refresh(document.area(), document.brushes());
    assert_eq!(w.open.brushes.iter().count(), 1);
}
