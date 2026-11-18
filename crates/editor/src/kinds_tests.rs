//! The kind table walked: every part has a row, and a project of one
//! kind shows nothing of the other's. The ribbons, the tab bar, the
//! settings and the reach of each kind are walked where they're drawn
//! (`shell/kind_tests.rs`, `file_pages/kind_tests.rs`,
//! `shell_reach_tests.rs`).

use super::*;
use crate::launcher::ProjectKind as K;
use crate::shell::ShellCommand;
use crate::test_support::info;
use crate::workbench::Workbench;

const KINDS: [PartKind; 3] = [PartKind::TwoD, PartKind::ThreeD, PartKind::Shared];

#[test]
fn a_project_shows_its_own_kind_and_what_is_shared() {
    use PartKind as P;
    for (project, part, shows) in [
        (K::TwoD, P::TwoD, true),
        (K::TwoD, P::ThreeD, false),
        (K::TwoD, P::Shared, true),
        (K::ThreeD, P::TwoD, false),
        (K::ThreeD, P::ThreeD, true),
        (K::ThreeD, P::Shared, true),
        (K::Both, P::TwoD, true),
        (K::Both, P::ThreeD, true),
        (K::Both, P::Shared, true),
    ] {
        assert_eq!(project.shows(part), shows, "{project:?} shows {part:?}");
    }
}

#[test]
fn every_built_in_component_has_its_row() {
    let registry = level::builtin::registry().unwrap();
    let mut names: Vec<&str> = registry.iter().map(|s| s.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names, COMPONENTS,
        "a new built-in needs a row in `component`"
    );
    // Board 08's 3D-only components, and the 2D-only ones.
    for three in ["Model", "Brush", "RigidBody", "Collider", "CharacterBody"] {
        assert_eq!(component(three), PartKind::ThreeD, "{three}");
    }
    for two in ["TileMap", "Collider2D", "Parallax"] {
        assert_eq!(component(two), PartKind::TwoD, "{two}");
    }
    // Lights are lamps on sprites in 2D (decision 0092).
    for shared in [
        "Transform",
        "Sprite",
        "Camera",
        "ReverbZone",
        "Light",
        "Environment",
        "Turret",
    ] {
        assert_eq!(component(shared), PartKind::Shared, "{shared}");
    }
}

#[test]
fn the_board_s_parts_are_of_the_kind_it_says() {
    use crate::shell::RibbonTab;
    use crate::unbuilt::UnbuiltId as U;
    assert_eq!(tab(RibbonTab::Shading), PartKind::ThreeD);
    for shared in [
        RibbonTab::Animate,
        RibbonTab::Ui,
        RibbonTab::Vfx,
        RibbonTab::Audio,
    ] {
        assert_eq!(tab(shared), PartKind::Shared, "{shared:?} is shared");
    }
    for lighting in [
        U::HomeLightingBake,
        U::HomeLightingQuality,
        U::HomeLightingLightmapView,
        U::ShellTabShading,
    ] {
        assert_eq!(unbuilt(lighting), PartKind::ThreeD, "{lighting:?}");
    }
    assert_eq!(unbuilt(U::HomeCreateTrigger), PartKind::Shared, "zones");
    for three in [
        ShellCommand::ImportModel,
        ShellCommand::ChooseShapeBrush,
        ShellCommand::EditBrushInsides,
    ] {
        assert_eq!(command(three), PartKind::ThreeD, "{three:?}");
    }
    for two in [
        ShellCommand::NewTileMap,
        ShellCommand::PixelPerfect,
        ShellCommand::SnapToTiles,
        ShellCommand::PaintWith(crate::tile_paint::PaintTool::Fill),
    ] {
        assert_eq!(command(two), PartKind::TwoD, "{two:?}");
    }
    for shared in [
        ShellCommand::AddSprite,
        ShellCommand::AddPointLight,
        ShellCommand::AddCamera,
        ShellCommand::AddReverbZone,
        ShellCommand::AddEntity,
    ] {
        assert_eq!(command(shared), PartKind::Shared, "{shared:?}");
    }
    assert_eq!(view(ViewKind::TwoD), PartKind::TwoD);
    assert_eq!(view(ViewKind::Perspective), PartKind::ThreeD);
}

#[test]
fn every_3d_key_start_names_some_unfinished_part() {
    // A start that names nothing would be a row for a part that's gone.
    for id in crate::unbuilt::ALL.iter().map(|part| part.id) {
        // Every part has a kind: the call can't fail, so walking it is
        // the check.
        assert!(KINDS.contains(&unbuilt(id)));
    }
    let three: Vec<_> = crate::unbuilt::ALL
        .iter()
        .filter(|part| unbuilt(part.id) == PartKind::ThreeD)
        .map(|part| part.key)
        .collect();
    for start in [
        "shell.tab.shading",
        "shading.",
        "mesh.",
        "home.lighting.",
        "home.tool.pose",
        "home.object.model-",
        "cross.bake-now",
    ] {
        assert!(three.iter().any(|key| key.starts_with(start)), "{start}");
    }
}

#[test]
fn the_search_offers_nothing_of_the_kind_the_project_hides() {
    for (project, hidden) in [(K::TwoD, PartKind::ThreeD), (K::ThreeD, PartKind::TwoD)] {
        let mut w = Workbench::new(info());
        w.apply_project_kind(project);
        for each in ShellCommand::ALL {
            if command(each) == hidden {
                assert_eq!(
                    each.why_not(&w),
                    hidden.use_note(),
                    "{each:?} in {project:?}"
                );
            }
        }
        let listed: Vec<String> = w.search_items().into_iter().map(|i| i.name).collect();
        for each in ShellCommand::ALL
            .into_iter()
            .filter(|c| command(*c) == hidden)
        {
            let name = each.name(&w);
            assert!(!listed.contains(&name), "{name} listed in {project:?}");
        }
    }
    // Both refuses nothing for its kind.
    let w = Workbench::new(info());
    for each in ShellCommand::ALL {
        assert!(
            each.why_not(&w)
                .is_none_or(|why| !why.contains("switch the project")),
            "{each:?}"
        );
    }
}

#[test]
fn templates_agree_with_the_table() {
    use crate::launcher::Starter;
    for starter in Starter::ALL {
        for project in K::ALL {
            assert_eq!(
                starter.suits(project),
                project.shows(super::starter(starter)),
                "{starter:?} in {project:?}"
            );
        }
        let marked = starter.made_for();
        match super::starter(starter) {
            PartKind::Shared => assert_eq!(marked, None, "{starter:?}"),
            PartKind::ThreeD => assert_eq!(marked, Some(K::ThreeD), "{starter:?}"),
            PartKind::TwoD => assert_eq!(marked, Some(K::TwoD), "{starter:?}"),
        }
    }
    // Learn's examples are 3D: greyed for 2D (R1), as the table hides.
    assert!(!Starter::Playground.suits(K::TwoD));
    assert!(Starter::Empty.suits(K::TwoD));
}

#[test]
fn an_entity_is_of_the_kind_of_its_components() {
    use level::EntityData;
    let ship = EntityData::new("Ship").with(&level::builtin::Model::default());
    assert_eq!(entity(&ship), PartKind::ThreeD);
    let lamp = EntityData::new("Lamp").with(&level::builtin::Light::default());
    assert_eq!(entity(&lamp), PartKind::Shared, "lamps light sprites");
    let map = EntityData::new("Ground").with(&level::builtin::TileMap::default());
    assert_eq!(entity(&map), PartKind::TwoD);
    let empty = EntityData::new("Empty").with(&level::builtin::Transform::default());
    assert_eq!(entity(&empty), PartKind::Shared);
    // Both kinds at once: shown everywhere, never hidden.
    let both = map.with(&level::builtin::Model::default());
    assert_eq!(entity(&both), PartKind::Shared);
    assert_eq!(
        PartKind::ThreeD.note().as_deref(),
        Some("3D only: switch the project to Both to edit it")
    );
}

#[test]
fn changing_the_kind_turns_hidden_views_into_the_kind_s_own() {
    let mut w = Workbench::new(info());
    assert_eq!(w.viewport.camera.kind, ViewKind::Perspective);
    w.apply_project_kind(K::TwoD);
    assert_eq!(w.viewport.camera.kind, ViewKind::TwoD);
    w.apply_project_kind(K::Both);
    assert_eq!(w.viewport.camera.kind, ViewKind::TwoD, "Both keeps it");
    w.apply_project_kind(K::ThreeD);
    assert_eq!(w.viewport.camera.kind, ViewKind::Perspective);
}

#[test]
fn opening_or_switching_to_2d_or_3d_counts_towards_the_note() {
    let mut w = Workbench::new(info());
    w.set_project_kind(K::Both);
    assert_eq!(w.interface.kind_note_seen, 0, "Both has no note");
    w.set_project_kind(K::TwoD);
    w.set_project_kind(K::ThreeD);
    assert_eq!(w.interface.kind_note_seen, 2);
    w.dismiss_kind_note();
    w.set_project_kind(K::TwoD);
    assert_eq!(
        w.interface.kind_note_seen,
        crate::interface_look::KIND_NOTE_GONE,
        "gone for good"
    );
}

#[test]
fn the_kind_in_use_is_the_frame_s() {
    use_kind(K::TwoD);
    assert!(!shown(PartKind::ThreeD));
    assert!(shown(PartKind::TwoD));
    use_kind(K::Both);
    assert!(shown(PartKind::ThreeD));
}
