//! Tags in approved pictures (stage 1, tags; the approved mock-up
//! `docs/design/tags/`): Project settings › Tags with four tags, one being
//! made; a bat's tags under its name in Properties with the + Tag list
//! open; and the Level panel showing tags faintly after names. Drawn for
//! real at 1440 × 900 and cropped to what they show.

use crate::file_pages::Tab;
use crate::interface_look::InterfaceLook;
use crate::launcher::ProjectKind;
use crate::picture_support::{Picture, drawn, editor_window, matches_approved};
use crate::tags::TagAsk;
use crate::workbench::Workbench;
use egui_kittest::kittest::Queryable;
use level::EntityData;
use level::builtin::{Sprite, Transform};
use reflect::EntityId;

const SIZE: [f32; 2] = [1440.0, 900.0];

/// The mock-up's project: Farm Days, 2D, with the tags Enemy,
/// Collectible and Flying, and a bat carrying Enemy and Flying beside
/// the sun, the environment and a map.
fn farm_days() -> (Workbench, EntityId) {
    let mut w = Workbench::new(crate::test_support::info());
    w.set_project(Some(crate::ProjectInfo {
        name: "Farm Days".into(),
        root: "/projects/farm-days".into(),
        assets: "/projects/farm-days".into(),
    }));
    w.set_project_kind(ProjectKind::TwoD);
    let mut tags = level::tags::TagList::default();
    let enemy = tags.add("Enemy").unwrap();
    tags.add("Collectible").unwrap();
    let flying = tags.add("Flying").unwrap();
    w.set_tags(tags);
    // The starter level's sun and environment, then the bat, with IDs and
    // places in the list that are the same every time.
    let fixed = |n| EntityId(reflect::Id::from_raw(n).unwrap());
    let mut level = level::Level::new();
    let mut starter: Vec<EntityData> = level::builtin::starter_level()
        .iter()
        .map(|(_, e)| e.clone())
        .collect();
    starter.sort_by(|a, b| b.name.cmp(&a.name));
    for (n, mut entity) in starter.into_iter().enumerate() {
        entity.order = level.order_at_end(None);
        level.insert(fixed(n as u64 + 1), entity);
    }
    let id = fixed(9);
    let mut bat = EntityData::new("Bat")
        .with(&Transform::default())
        .with(&Sprite::default());
    bat.order = level.order_at_end(None);
    level.insert(id, bat);
    w.open.document = crate::document::Document::new(level);
    w.change_tags(&[id], TagAsk::Add(enemy));
    w.change_tags(&[id], TagAsk::Add(flying));
    w.select(id);
    w.open.outbox.clear();
    (w, id)
}

/// Draws `w`, letting `before` act on the window first, and crops a part.
fn part_of(
    w: Workbench,
    before: impl FnOnce(&mut egui_kittest::Harness<'static, Workbench>),
    min: [u32; 2],
    size: [u32; 2],
) -> Option<Picture> {
    let mut h = editor_window(w, InterfaceLook::default(), SIZE)?;
    for _ in 0..4 {
        h.run();
    }
    before(&mut h);
    for _ in 0..3 {
        h.run();
    }
    Some(drawn(&mut h).cropped(min, size))
}

#[test]
fn the_tags_tab_of_project_settings() {
    let (mut w, _) = farm_days();
    w.show_settings(Tab::Tags);
    w.file_pages.tags.naming = Some(crate::tags::page::Naming {
        id: None,
        typed: "Door".into(),
        focus: false,
    });
    let Some(picture) = part_of(w, |_| {}, [220, 30], [1220, 600]) else {
        return;
    };
    matches_approved("tags-page", &picture);
}

#[test]
fn an_entity_s_tags_with_the_add_list_open() {
    let (w, _) = farm_days();
    let Some(picture) = part_of(
        w,
        |h| {
            h.get_by_label("Add tag").click();
        },
        [1120, 100],
        [320, 320],
    ) else {
        return;
    };
    matches_approved("tags-properties", &picture);
}

#[test]
fn the_level_panel_shows_tags_after_names() {
    let (w, _) = farm_days();
    let Some(picture) = part_of(w, |_| {}, [0, 100], [252, 200]) else {
        return;
    };
    matches_approved("tags-level-panel", &picture);
}
