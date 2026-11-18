//! Tags in the editor: the project's list made, renamed everywhere and
//! deleted (asking first when it's used) on Project settings › Tags; an
//! entity's tags added and taken off in Properties, each one undo step,
//! for several picked entities at once; a prefab's tags on every copy, a
//! copy's own change with its dot and ↶; the Level panel showing tags and
//! finding entities by them; and a zone's Only tags.

use super::*;
use crate::file_pages::Tab;
use crate::properties::PropertyRequest;
use crate::test_support::*;
use egui_kittest::kittest::Queryable;
use level::EntityData;
use level::builtin::{Transform, TriggerZone};

/// A workbench in a project with the tags Enemy and Flying, and a bat and
/// a crow in its level.
fn with_tags() -> (Workbench, TagId, TagId, EntityId, EntityId) {
    let mut w = Workbench::new(info());
    w.set_project(Some(crate::ProjectInfo {
        name: "Farm Days".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm".into(),
    }));
    let mut tags = TagList::default();
    let enemy = tags.add("Enemy").unwrap();
    let flying = tags.add("Flying").unwrap();
    w.set_tags(tags);
    let spawn = |name: &str| {
        let id = EntityId::random();
        (
            id,
            level::Edit::Spawn {
                entities: vec![(id, EntityData::new(name).with(&Transform::default()))],
            },
        )
    };
    let (bat, a) = spawn("Bat");
    let (crow, b) = spawn("Crow");
    w.open.document.edit("Add", vec![a, b], None, None).unwrap();
    w.open.outbox.clear();
    (w, enemy, flying, bat, crow)
}

fn tags_of(w: &Workbench, id: EntityId) -> Vec<TagId> {
    carried(&w.open.tags, w.open.document.level().get(id).unwrap())
}

fn saves(w: &Workbench) -> Vec<TagList> {
    w.open
        .outbox
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::SaveTags(tags) => Some(tags.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn tags_are_made_with_names_checked_and_saved() {
    let (mut w, ..) = with_tags();
    let door = w.make_tag("Door").unwrap();
    assert_eq!(w.tags().name_of(door), Some("Door"));
    assert_eq!(
        saves(&w).last().unwrap().names(),
        ["Enemy", "Flying", "Door"]
    );
    for (bad, reason) in [
        ("door", "capital"),
        ("Big door", "no spaces"),
        ("ENEMY", "all capitals"),
        ("enemy", "capital"),
        ("Enemy", "already"),
        ("", "needs a name"),
    ] {
        let why = w.make_tag(bad).unwrap_err();
        assert!(why.contains(reason), "{bad}: {why}");
    }
    assert_eq!(saves(&w).len(), 1, "refused names save nothing");
}

#[test]
fn a_rename_reaches_every_level_prefab_and_game_code() {
    let (mut w, enemy, _, bat, _) = with_tags();
    w.change_tags(&[bat], TagAsk::Add(enemy));
    // A prefab carrying it.
    let (asset, file) = lamp_post_file();
    let mut file = (*file).clone();
    let root = match file.kind {
        level::text::FileKind::Prefab { root } => root,
        level::text::FileKind::Level => unreachable!(),
    };
    file.level.get_mut(root).unwrap().set(&Tags::of([enemy]));
    w.set_prefab(asset, Arc::new(file));
    w.rename_tag(enemy, "Monster").unwrap();
    // The level and the prefab keep it by its ID, so both follow.
    assert_eq!(tags_of(&w, bat), [enemy]);
    assert_eq!(w.tags().name_of(enemy), Some("Monster"));
    let used = w.used_by(enemy);
    assert_eq!((used.entities, used.prefabs.len()), (1, 1));
    // Game code's name follows: Tag.Monster checks, Tag.Enemy doesn't.
    let names = script::project_names::Names::standard().and_tags(
        &w.tags()
            .names()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    let (text, _) = script::project_names::project_file("Test 1.0", &names);
    assert!(text.contains("state Tag = [Monster, Flying];"), "{text}");
    assert_eq!(saves(&w).last().unwrap(), w.tags());
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("Tag.Monster now") && l.text.contains("Tag.Enemy"))
    );
    // A rename to a name in use is refused, and changes nothing.
    assert!(w.rename_tag(enemy, "Flying").is_err());
    assert_eq!(w.tags().name_of(enemy), Some("Monster"));
}

#[test]
fn adding_and_taking_off_a_tag_is_one_undo_step_each() {
    let (mut w, enemy, flying, bat, _) = with_tags();
    w.change_tags(&[bat], TagAsk::Add(enemy));
    w.change_tags(&[bat], TagAsk::Add(flying));
    assert_eq!(tags_of(&w, bat), [enemy, flying]);
    w.change_tags(&[bat], TagAsk::Add(enemy));
    assert_eq!(tags_of(&w, bat), [enemy, flying], "never twice");
    w.change_tags(&[bat], TagAsk::Remove(enemy));
    assert_eq!(tags_of(&w, bat), [flying]);
    w.undo();
    assert_eq!(tags_of(&w, bat), [enemy, flying]);
    w.undo();
    assert_eq!(tags_of(&w, bat), [enemy]);
    w.undo();
    assert!(tags_of(&w, bat).is_empty());
    assert!(
        w.open
            .document
            .level()
            .get(bat)
            .unwrap()
            .get::<Tags>()
            .is_none(),
        "the first tag gave it its Tags; undone, it has none"
    );
    // New tag… makes one in the project and gives it, in one go.
    w.change_tags(&[bat], TagAsk::New("Door".into()));
    let door = w.tags().id_of("Door").unwrap();
    assert_eq!(tags_of(&w, bat), [door]);
    w.change_tags(&[bat], TagAsk::New("bad name".into()));
    assert_eq!(tags_of(&w, bat), [door]);
}

#[test]
fn several_picked_entities_show_and_change_the_tags_they_share() {
    let (mut w, enemy, flying, bat, crow) = with_tags();
    w.change_tags(&[bat, crow], TagAsk::Add(enemy));
    assert_eq!(tags_of(&w, bat), [enemy]);
    assert_eq!(tags_of(&w, crow), [enemy]);
    w.undo();
    assert!(
        tags_of(&w, bat).is_empty() && tags_of(&w, crow).is_empty(),
        "one step"
    );
    w.change_tags(&[bat, crow], TagAsk::Add(enemy));
    w.change_tags(&[bat], TagAsk::Add(flying));
    // Shown: only Enemy, which both carry.
    w.select(bat);
    w.open.document.selection.toggle(crow);
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(h.query_by_label("Remove tag Enemy").is_some());
    assert!(
        h.query_by_label("Remove tag Flying").is_none(),
        "only the bat flies"
    );
    h.get_by_label("Remove tag Enemy").click();
    h.run();
    let w = &h.state().0;
    assert!(tags_of(w, crow).is_empty());
    assert_eq!(tags_of(w, bat), [flying]);
}

#[test]
fn the_add_list_offers_the_project_s_tags_and_makes_a_new_one() {
    let (mut w, enemy, _, bat, _) = with_tags();
    w.select(bat);
    let mut h = harness(w);
    h.run();
    h.run();
    h.get_by_label("Add tag").click();
    h.run();
    h.get_by_label("Enemy").click();
    h.run();
    assert_eq!(tags_of(&h.state().0, bat), [enemy]);
    // Typed, a name no tag has: Enter makes it and gives it.
    h.get_by_label("Add tag").click();
    h.run();
    h.get_by_label("Search tags").type_text("Collectible");
    h.run();
    assert!(
        h.query_by_label_contains("New tag \u{201c}Collectible")
            .is_some()
    );
    h.key_press(egui::Key::Enter);
    h.run();
    let w = &h.state().0;
    let collectible = w.tags().id_of("Collectible").expect("made");
    assert_eq!(tags_of(w, bat), [enemy, collectible]);
}

#[test]
fn a_prefab_s_tags_are_on_every_copy_and_a_copy_s_own_change_wins() {
    let (mut w, enemy, flying, ..) = with_tags();
    let (asset, file) = lamp_post_file();
    let mut tagged = (*file).clone();
    let root_of = |f: &level::text::LevelFile| match f.kind {
        level::text::FileKind::Prefab { root } => root,
        level::text::FileKind::Level => unreachable!(),
    };
    let root = root_of(&tagged);
    tagged.level.get_mut(root).unwrap().set(&Tags::of([enemy]));
    w.set_prefab(asset, Arc::new(tagged.clone()));
    w.place_prefab(asset);
    let first = w.selection().unwrap();
    w.place_prefab(asset);
    let second = w.selection().unwrap();
    assert_eq!(
        tags_of(&w, first),
        [enemy],
        "the prefab's tag is the copy's"
    );
    assert!(!w.tags_changed_from_prefab(
        w.open.document.level().get(first).unwrap(),
        w.part_of(first).as_ref()
    ));
    // The first copy adds one of its own: changed, with the dot.
    w.change_tags(&[first], TagAsk::Add(flying));
    assert_eq!(tags_of(&w, first), [enemy, flying]);
    let changed = |w: &Workbench, id| {
        w.tags_changed_from_prefab(
            w.open.document.level().get(id).unwrap(),
            w.part_of(id).as_ref(),
        )
    };
    assert!(changed(&w, first));
    assert!(!changed(&w, second));
    // The prefab changes its tags: the second copy follows, the first
    // keeps its own.
    tagged.level.get_mut(root).unwrap().set(&Tags::default());
    w.set_prefab(asset, Arc::new(tagged));
    assert!(
        tags_of(&w, second).is_empty(),
        "the copy follows its prefab"
    );
    assert_eq!(tags_of(&w, first), [enemy, flying], "its own change wins");
    // ↶ puts the prefab's back.
    w.change_tags(&[first], TagAsk::Revert);
    assert!(tags_of(&w, first).is_empty());
    assert!(!changed(&w, first));
    // Shown in Properties with the dot and ↶ while it's changed.
    w.change_tags(&[first], TagAsk::Add(flying));
    w.select(first);
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(h.query_by_label("Remove tag Flying").is_some());
    h.get_by_label("\u{21b6}").click();
    h.run();
    assert!(tags_of(&h.state().0, first).is_empty());
}

#[test]
fn the_level_panel_shows_tags_and_its_search_finds_them() {
    let (mut w, enemy, flying, bat, crow) = with_tags();
    w.change_tags(&[bat], TagAsk::Add(enemy));
    w.change_tags(&[bat], TagAsk::Add(flying));
    w.change_tags(&[crow], TagAsk::Add(flying));
    let note = |w: &Workbench, id| {
        crate::level_panel::tags_note(&w.open.tags, w.open.document.level().get(id).unwrap())
    };
    assert_eq!(
        note(&w, bat).as_deref(),
        Some("Enemy \u{b7} Flying"),
        "after its name"
    );
    assert_eq!(note(&w, crow).as_deref(), Some("Flying"));
    let state = &mut w;
    state.open.level_panel.search = "enem".into();
    let rows = state.open.level_panel.rows(
        &state.open.document,
        (&state.open.tags, state.open.tags_revision),
    );
    assert_eq!(rows.iter().map(|(id, _)| *id).collect::<Vec<_>>(), [bat]);
    state.open.level_panel.search = "fly".into();
    let rows = state.open.level_panel.rows(
        &state.open.document,
        (&state.open.tags, state.open.tags_revision),
    );
    let mut found: Vec<EntityId> = rows.iter().map(|(id, _)| *id).collect();
    found.sort();
    let mut both = vec![bat, crow];
    both.sort();
    assert_eq!(found, both);
    // A renamed tag is found by its new name at once.
    state.rename_tag(enemy, "Monster").unwrap();
    state.open.level_panel.search = "monst".into();
    let rows = state.open.level_panel.rows(
        &state.open.document,
        (&state.open.tags, state.open.tags_revision),
    );
    assert_eq!(rows.len(), 1);
}

#[test]
fn deleting_a_tag_asks_first_when_it_s_used_saying_what() {
    let (mut w, enemy, flying, bat, _) = with_tags();
    w.change_tags(&[bat], TagAsk::Add(enemy));
    w.show_settings(Tab::Tags);
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(
        h.state().1.actions.contains(&WorkbenchAction::CountTagUse),
        "the levels are counted"
    );
    assert!(h.query_by_label_contains("Used by 1 entity").is_some());
    assert!(h.query_by_label_contains("Not used yet").is_some());
    // Flying is used by nothing: it goes at once.
    h.get_by_label("Flying's menu").click();
    h.run();
    h.get_by_label("Delete").click();
    h.run();
    assert!(h.state().0.tags().get(flying).is_none());
    // Enemy is on the bat: Delete asks, saying what uses it.
    h.get_by_label("Enemy's menu").click();
    h.run();
    h.get_by_label("Delete").click();
    h.run();
    h.run();
    assert!(h.query_by_label_contains("Enemy is on 1 entity").is_some());
    assert!(h.state().0.tags().get(enemy).is_some(), "not yet");
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Cancel")
        .click();
    h.run();
    assert!(h.state().0.tags().get(enemy).is_some(), "kept");
    h.get_by_label("Enemy's menu").click();
    h.run();
    h.get_by_label("Delete").click();
    h.run();
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Delete")
        .click();
    h.run();
    let w = &h.state().0;
    assert!(w.tags().is_empty());
    assert!(tags_of(w, bat).is_empty(), "nothing carries it now");
}

#[test]
fn the_page_makes_and_renames_tags_by_name() {
    let (mut w, enemy, ..) = with_tags();
    w.show_settings(Tab::Tags);
    let mut h = harness(w);
    h.run();
    h.run();
    h.get_by_label("New tag").click();
    h.run();
    h.get_by_label("Tag name").type_text("big door");
    h.run();
    assert!(
        h.query_by_label_contains("Try BigDoor").is_some(),
        "a refused name says why"
    );
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state().0.tags().len(), 2, "not kept");
    h.get_by_label("Tag name").focus();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_by_label("Tag name").type_text("Door");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state().0.tags().names(), ["Enemy", "Flying", "Door"]);
    // Double-click a name to rename it everywhere.
    let at = h.get_by_label("Enemy").rect().center();
    double_click(&mut h, at);
    h.get_by_label("Tag name").focus();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_by_label("Tag name").type_text("Monster");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state().0.tags().name_of(enemy), Some("Monster"));
}

#[test]
fn a_zone_s_only_tags_are_chosen_on_its_card() {
    let (mut w, enemy, ..) = with_tags();
    let zone = w.add_trigger_zone();
    w.apply_property(
        PropertyRequest::ZoneOnly {
            entity: zone,
            ask: TagAsk::Add(enemy),
        },
        1,
    );
    let only = |w: &Workbench| {
        w.open
            .document
            .level()
            .get(zone)
            .and_then(|e| e.get::<TriggerZone>())
            .unwrap()
            .only
    };
    assert_eq!(only(&w), [enemy]);
    w.undo();
    assert!(only(&w).is_empty());
    w.select(zone);
    let mut h = harness(w);
    h.run();
    h.run();
    // The second + Tag: the first is the zone entity's own tags.
    h.query_all_by_label("Add tag").nth(1).unwrap().click();
    h.run();
    h.get_by_label("Enemy").click();
    h.run();
    assert_eq!(only(&h.state().0), [enemy]);
}

#[test]
fn the_tags_file_in_the_library_opens_project_settings_tags() {
    let (mut w, ..) = with_tags();
    let mut actions = Vec::new();
    w.library_request(
        crate::library::LibraryRequest::Open(crate::library::LibraryKind::Tags, "tags.lst".into()),
        &mut actions,
    );
    assert_eq!(w.file_pages.page, crate::file_pages::Page::ProjectSettings);
    assert_eq!(w.file_pages.project_tab, Tab::Tags);
    assert!(actions.is_empty(), "nothing is opened in another program");
    assert!(crate::library::LibraryKind::Tags.opens_on_double_click());
    assert!(!crate::library::LibraryKind::Tags.opens_in_its_program("tags.lst"));
}

#[test]
fn several_zones_show_their_only_tags_by_name_or_not_at_all() {
    let (mut w, enemy, flying, ..) = with_tags();
    let a = w.add_trigger_zone();
    let b = w.add_trigger_zone();
    for zone in [a, b] {
        w.change_zone_tags(zone, TagAsk::Add(enemy));
        w.change_zone_tags(zone, TagAsk::Add(flying));
    }
    w.select(a);
    w.open.document.selection.toggle(b);
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(
        h.query_by_label("Enemy \u{b7} Flying").is_some(),
        "the shared tags by name"
    );
    let ids = enemy.to_string();
    assert!(
        h.query_all_by_label_contains(&ids).next().is_none(),
        "never the IDs"
    );
    // They differ: the row is left out.
    h.state_mut().0.change_zone_tags(b, TagAsk::Remove(flying));
    h.run();
    h.run();
    assert!(h.query_by_label("Enemy \u{b7} Flying").is_none());
    assert!(h.query_all_by_label_contains(&ids).next().is_none());
}
