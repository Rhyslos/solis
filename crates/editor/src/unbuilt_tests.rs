//! The mechanism for unfinished parts, kept true:
//!
//! - **the table is sound**, and names exactly what the plan's inventory
//!   says is unbuilt;
//! - **hiding works on lists**: the unbuilt entries go, then what is
//!   left empty;
//! - **walking the editor** meets exactly the parts it should, and none
//!   with the setting on.
//!
//! How a part is drawn, and that the look is the approved one, is tested
//! on the placeholder sheet, in `unbuilt/sheet_tests.rs`.

use super::*;
use crate::theme::{self, InterfaceLook};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, NodeT as _, Queryable as _};
use std::path::Path;

/// The look with unfinished parts hidden.
fn hiding() -> InterfaceLook {
    InterfaceLook {
        hide_unfinished: true,
        ..InterfaceLook::default()
    }
}

/// Puts the standard look back for the tests that share this thread.
fn standard_again() {
    crate::interface_look::use_look(InterfaceLook::default());
}

/// A text file of the repository, with its line ends made plain.
fn repository_file(path: &[&str]) -> String {
    let mut file = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    for part in path {
        file = file.join(part);
    }
    std::fs::read_to_string(&file)
        .unwrap_or_else(|e| panic!("{}: {e}", file.display()))
        .replace("\r\n", "\n")
}

#[test]
fn the_table_is_in_the_enum_s_order_and_every_line_is_filled_in() {
    let mut keys = Vec::new();
    for (index, part) in ALL.iter().enumerate() {
        assert_eq!(part.id as usize, index, "{}", part.key);
        assert_eq!(get(part.id), part);
        assert!(!part.name.trim().is_empty(), "{}", part.key);
        assert!(part.what.len() > 12, "{}: say what it will do", part.key);
        // One plain sentence: it starts with a capital and the full stop
        // is added where it is shown.
        assert!(
            part.what
                .starts_with(|c: char| c.is_ascii_uppercase() || c.is_ascii_digit()),
            "{}: {}",
            part.key,
            part.what
        );
        assert!(!part.what.ends_with('.'), "{}: {}", part.key, part.what);
        // The id is the key, with its dots and dashes taken out.
        let from_key: String = part
            .key
            .split(['.', '-'])
            .map(|word| {
                let mut letters = word.chars();
                let first = letters.next().unwrap().to_ascii_uppercase();
                format!("{first}{}", letters.as_str())
            })
            .collect();
        assert_eq!(format!("{:?}", part.id), from_key, "{}", part.key);
        keys.push(part.key);
    }
    let count = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), count, "two parts share a key");
}

/// The roadmap's headings.
fn roadmap_headings() -> Vec<String> {
    repository_file(&["ROADMAP.md"])
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .map(|heading| heading.trim().to_owned())
        .collect()
}

#[test]
fn every_phase_is_a_heading_of_the_roadmap() {
    let headings = roadmap_headings();
    assert!(headings.len() > 20, "{headings:?}");
    for phase in Phase::ALL {
        let Some(title) = phase.title() else {
            assert_eq!(phase, Phase::NotPlanned);
            continue;
        };
        // The whole heading; or, for the one phase whose name on the
        // roadmap holds the program's own name, its number alone.
        let whole = headings.iter().any(|heading| heading == title);
        let number_only = !title.contains(':')
            && headings
                .iter()
                .any(|h| h.starts_with(&format!("{title}: ")));
        assert!(
            whole || number_only,
            "\"{title}\" isn't a heading of ROADMAP.md: was the phase renamed?"
        );
    }
    assert_eq!(Phase::Shared13.title(), Some("Shared 13"));
    assert_eq!(Phase::ThreeD5.title(), Some("3D 5: Bounced light"));
}

/// One row of the inventory's part (b) that says something is unbuilt:
/// its key and its "Filled in" cell.
fn inventory_rows() -> Vec<(String, String)> {
    let text = repository_file(&["docs", "plans", "shared-1-ui-remake-inventory.md"]);
    let from = text.find("## (b) Every element").expect("part (b)");
    let to = text.find("### b.15 Counts").expect("the counts");
    let mut rows = Vec::new();
    for line in text[from..to]
        .lines()
        .filter(|line| line.starts_with("| `"))
    {
        let cells: Vec<&str> = line.trim_matches('|').split(" | ").map(str::trim).collect();
        assert!(cells.len() >= 4, "{line}");
        let key = cells[0].trim_matches('`');
        let when = cells.get(4).copied().unwrap_or_default();
        match cells[2] {
            "PLACEHOLDER" => rows.push((key.to_owned(), when.to_owned())),
            "PARTLY" => {
                for piece in unbuilt_pieces(cells[3]) {
                    rows.push((piece, when.to_owned()));
                }
            }
            "EXISTS" => {}
            other => panic!("{key} is marked {other}"),
        }
    }
    rows
}

/// The ids a PARTLY row names as its unbuilt pieces: the backticked ids
/// that follow "unbuilt piece" (or "pieces") in the same sentence, with
/// whatever stands in brackets after each.
fn unbuilt_pieces(cell: &str) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut rest = cell;
    while let Some(at) = rest.to_lowercase().find("unbuilt piece") {
        rest = &rest[at + "unbuilt piece".len()..];
        let Some(tick) = rest.find('`') else { break };
        if rest[..tick].contains('.') {
            continue;
        }
        let mut list = &rest[tick..];
        while let Some(id) = list.strip_prefix('`') {
            let Some(end) = id.find('`') else { break };
            pieces.push(id[..end].to_owned());
            list = &id[end + 1..];
            if list.starts_with(" (") {
                let close = list.find(')').map_or(list.len(), |i| i + 1);
                list = &list[close..];
            }
            list = [", ", " and ", "; "]
                .iter()
                .find_map(|joiner| list.strip_prefix(joiner))
                .unwrap_or("");
        }
    }
    pieces
}

/// What the inventory lists as unbuilt and the table leaves out, each
/// with why.
const LEFT_OUT: [(&str, &str); 11] = [
    (
        "shell.search.settings",
        "settings as search results are built in this phase: no placeholder is needed",
    ),
    (
        "shell.groups.second-area",
        "a second area's tab simply isn't drawn: there is nothing to show",
    ),
    (
        "debug.graph.select",
        "choosing a frame in the graph is built in this phase",
    ),
    (
        "recent.template.vr",
        "decided against: Solis is for Windows desktops",
    ),
    (
        "proj.game-mode",
        "decided against: game-wide objects do that job",
    ),
    (
        "proj.units",
        "decided against as a setting: units and axes are fixed",
    ),
    (
        "input.players-rebind",
        "decided against as a setting: a game's own menu rebinds",
    ),
    (
        "home.code",
        "the code editor group isn't drawn for now, with placeholders or without",
    ),
    ("home.status.cursor", "part of the code editor group"),
    ("place.pill.code-hidden", "part of the code editor group"),
    ("cross.error-pins", "part of the code editor group"),
];

#[test]
fn the_table_names_exactly_what_the_inventory_says_is_unbuilt() {
    let rows = inventory_rows();
    assert!(rows.len() > 150, "only {} rows were read", rows.len());
    let mut wanted: Vec<&str> = rows.iter().map(|(key, _)| key.as_str()).collect();
    wanted.sort_unstable();
    wanted.dedup();
    for (key, why) in LEFT_OUT {
        assert!(
            wanted.contains(&key),
            "{key} is left out ({why}) but isn't in the inventory"
        );
        assert!(
            ALL.iter().all(|part| part.key != key),
            "{key} is left out and in the table"
        );
    }
    wanted.retain(|key| LEFT_OUT.iter().all(|(left_out, _)| left_out != key));
    let mut have: Vec<&str> = ALL.iter().map(|part| part.key).collect();
    have.sort_unstable();
    let missing: Vec<&&str> = wanted.iter().filter(|key| !have.contains(key)).collect();
    let extra: Vec<&&str> = have.iter().filter(|key| !wanted.contains(key)).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "docs/plans/shared-1-ui-remake-inventory.md and unbuilt/table.rs disagree.\n\
         In the inventory, not in the table: {missing:?}\n\
         In the table, not in the inventory: {extra:?}"
    );
}

#[test]
fn each_part_s_phase_is_the_one_the_inventory_gives() {
    let rows = inventory_rows();
    for part in ALL {
        // A part may be listed twice (as a row and as another's piece).
        let cells: Vec<&str> = rows
            .iter()
            .filter(|(key, when)| key == part.key && !when.is_empty())
            .map(|(_, when)| when.as_str())
            .collect();
        assert!(
            !cells.is_empty(),
            "{} has no phase in the inventory",
            part.key
        );
        let named = |cell: &str| match part.phase.title() {
            // "Shared 7: Materials and shaders" is written "Shared 7".
            Some(title) => cell.contains(title.split(':').next().unwrap()),
            None => cell.starts_with("not on the roadmap"),
        };
        assert!(
            cells.iter().any(|cell| named(cell)),
            "{} is {:?} in the table; the inventory says {cells:?}",
            part.key,
            part.phase
        );
    }
}

#[test]
fn the_reader_of_the_inventory_finds_pieces_however_they_are_listed() {
    assert_eq!(
        unbuilt_pieces("Real for one. Unbuilt piece: `lib.used-by.all-kinds` (which areas)."),
        ["lib.used-by.all-kinds"]
    );
    assert_eq!(
        unbuilt_pieces("Drawn as a pair; unbuilt piece `audio.meters.stereo`."),
        ["audio.meters.stereo"]
    );
    assert_eq!(
        unbuilt_pieces("Unbuilt pieces: `debug.stats.draw-calls`, `debug.stats.memory`."),
        ["debug.stats.draw-calls", "debug.stats.memory"]
    );
    assert_eq!(
        unbuilt_pieces(
            "Unbuilt pieces, each a chip until its key is agreed (rule 17): `place.keep-placing` \
             (Shift), `place.free` (Alt), `place.rotate-ghost` (R)."
        ),
        ["place.keep-placing", "place.free", "place.rotate-ghost"]
    );
    // An id named for another reason isn't a piece.
    assert!(unbuilt_pieces("The warning needs `lib.used-by.all-kinds`.").is_empty());
    assert!(unbuilt_pieces("No unbuilt piece here. See `home.code`.").is_empty());
}

#[test]
fn what_a_part_says_names_it_what_it_will_do_and_when() {
    let bake = get(UnbuiltId::HomeLightingBake);
    assert_eq!(
        bake.hover_text(),
        "Not built yet: Bake. Works out bounced light for the area. Planned for 3D 5: Bounced \
         light."
    );
    assert_eq!(
        bake.click_text(),
        "Bake isn't built yet (3D 5: Bounced light)"
    );
    assert_eq!(
        bake.page_line(),
        "Not built yet \u{b7} planned for 3D 5: Bounced light"
    );
    let yaw = get(UnbuiltId::PlaceRandomYaw);
    assert_eq!(
        yaw.hover_text(),
        "Not built yet: Random yaw. Turns each placed thing a random way. Not planned yet."
    );
    assert_eq!(
        yaw.click_text(),
        "Random yaw isn't built yet (not planned yet)"
    );
    assert_eq!(yaw.page_line(), "Not built yet \u{b7} not planned yet");
    // Every part's words are whole sentences.
    for part in ALL {
        let hover = part.hover_text();
        assert!(
            hover.starts_with("Not built yet: ") && hover.ends_with('.'),
            "{hover}"
        );
        assert!(!hover.contains(".."), "{hover}");
    }
}

#[test]
fn hiding_takes_the_unbuilt_entries_out_and_then_what_is_left_empty() {
    use Entry::{Built, Unbuilt};
    let list = || {
        vec![
            Built("Save"),
            Unbuilt(UnbuiltId::HomeLightingBake),
            Built("Build"),
            Unbuilt(UnbuiltId::ShellPlaySimulate),
        ]
    };
    // Shown: nothing changes, in content or in order.
    assert_eq!(filter_when(list(), false), list());
    assert_eq!(
        filter_when(list(), true),
        vec![Built("Save"), Built("Build")]
    );
    assert_eq!(filter_when(Vec::<Entry<()>>::new(), true), vec![]);
    let all_unbuilt = vec![Entry::<()>::Unbuilt(UnbuiltId::ShellTabVfx)];
    assert!(filter_when(all_unbuilt.clone(), true).is_empty());
    assert_eq!(filter_when(all_unbuilt, false).len(), 1);
    // Groups: each is filtered, and one left with nothing goes too, so
    // its neighbours touch.
    let groups = || {
        vec![
            (
                "Create",
                vec![Built("Model"), Unbuilt(UnbuiltId::HomeCreateTrigger)],
            ),
            ("Lighting", vec![Unbuilt(UnbuiltId::HomeLightingBake)]),
            ("Project", vec![Built("Save")]),
            ("Empty by design", vec![]),
        ]
    };
    assert_eq!(filter_groups_when(groups(), false), groups());
    assert_eq!(
        filter_groups_when(groups(), true),
        vec![
            ("Create", vec![Built("Model")]),
            ("Project", vec![Built("Save")])
        ]
    );
}

#[test]
fn the_setting_in_use_is_what_filter_reads() {
    use Entry::{Built, Unbuilt};
    let list = || vec![Built(1), Unbuilt(UnbuiltId::ShellTabVfx)];
    crate::interface_look::use_look(InterfaceLook {
        hide_unfinished: true,
        ..InterfaceLook::default()
    });
    assert!(hidden());
    assert_eq!(filter(list()), vec![Built(1)]);
    crate::interface_look::use_look(InterfaceLook::default());
    assert!(!hidden());
    assert_eq!(filter(list()).len(), 2);
}

/// Parts that are ways of working or drawings over something else, with
/// no control of their own to draw: the plan's inventory says "not
/// drawn" of each. They are in the table so they are counted and
/// planned, and the walk below doesn't expect to meet them.
const NOTHING_TO_DRAW: [(UnbuiltId, &str); 16] = [
    (
        UnbuiltId::ShadingPreview,
        "a group of an unbuilt screen, which is one page card (D2)",
    ),
    (
        UnbuiltId::VfxStack,
        "a group of an unbuilt screen, which is one page card (D2)",
    ),
    (
        UnbuiltId::MeshViewport,
        "a group of an unbuilt screen, which is one page card (D2)",
    ),
    (
        UnbuiltId::MeshUv,
        "a group of an unbuilt screen, which is one page card (D2)",
    ),
    (
        UnbuiltId::HomeViewMotionCurve,
        "a drawing over the viewport, not a control",
    ),
    (
        UnbuiltId::PlaceClickToPlace,
        "a way of placing: click a tile, then the area",
    ),
    (UnbuiltId::LibMultiSelect, "a way of selecting"),
    (UnbuiltId::LibDragToHome, "a way of dragging"),
    (
        UnbuiltId::DebugStatus,
        "its place in the status bar says what is real",
    ),
    (
        UnbuiltId::RecentCardBadge,
        "a project is neither 2D nor 3D: no badge is drawn",
    ),
    (UnbuiltId::InputGroups, "the actions are one flat list"),
    (
        UnbuiltId::CrossBakeNow,
        "the problem it belongs to can't arise yet",
    ),
    // The toolbox column went: the tools float in the view (decision
    // 0114), and these come back where their own work puts them.
    (UnbuiltId::HomeToolPose, "the toolbox went (decision 0114)"),
    (
        UnbuiltId::HomeToolMotionPath,
        "the toolbox went (decision 0114)",
    ),
    (
        UnbuiltId::HomeToolAddKey,
        "the toolbox went (decision 0114)",
    ),
    (
        UnbuiltId::HomeToolMeasure,
        "the toolbox went (decision 0114)",
    ),
];

/// The unfinished parts the editor draws somewhere today. Each milestone
/// that builds a screen adds that screen's parts here, until this is the
/// whole table less [`NOTHING_TO_DRAW`]; the last milestone of the phase
/// replaces the list with that sum.
const DRAWN_SO_FAR: [UnbuiltId; 139] = [
    // The frame.
    UnbuiltId::ShellTabShading,
    UnbuiltId::ShellTabVfx,
    UnbuiltId::ShellPlaySimulate,
    UnbuiltId::ShellStatusBranch,
    // The unbuilt screens' shelves, and the model viewer.
    UnbuiltId::ShadingShelfParameters,
    UnbuiltId::ShadingShelfNode,
    UnbuiltId::ShadingShelfPalette,
    UnbuiltId::ShadingShelfInstances,
    UnbuiltId::VfxShelfModule,
    UnbuiltId::VfxShelfParameters,
    UnbuiltId::VfxShelfPerformance,
    UnbuiltId::MeshTab,
    UnbuiltId::MeshViewLit,
    UnbuiltId::MeshViewUnlit,
    UnbuiltId::MeshViewWire,
    UnbuiltId::MeshViewNormals,
    UnbuiltId::MeshViewUvCheck,
    UnbuiltId::MeshShowBounds,
    UnbuiltId::MeshShowPivot,
    UnbuiltId::MeshShowCollision,
    UnbuiltId::MeshLightPreset,
    UnbuiltId::MeshLightRotate,
    UnbuiltId::MeshLightBackground,
    UnbuiltId::MeshLodAuto,
    UnbuiltId::MeshLodPick,
    UnbuiltId::MeshLodDensity,
    UnbuiltId::MeshToolsReimport,
    UnbuiltId::MeshToolsPivot,
    UnbuiltId::MeshToolsShot,
    UnbuiltId::MeshUseShowInLevel,
    UnbuiltId::MeshShelfOverview,
    UnbuiltId::MeshShelfLods,
    UnbuiltId::MeshShelfMaterials,
    UnbuiltId::MeshShelfCollision,
    UnbuiltId::MeshShelfSockets,
    UnbuiltId::MeshShelfImport,
    // Build.
    UnbuiltId::BuildTargets,
    UnbuiltId::BuildTargetWindows,
    UnbuiltId::BuildTargetPlatforms,
    UnbuiltId::BuildFull,
    UnbuiltId::BuildInstaller,
    UnbuiltId::BuildRustCode,
    UnbuiltId::BuildLog,
    // The shelves' tabs and tools.
    UnbuiltId::HomeShelfAnimation,
    UnbuiltId::HomeShelfTransport,
    UnbuiltId::HomeShelfFrameCounter,
    UnbuiltId::LibShelfImportQueue,
    UnbuiltId::AnimShelfSequencer,
    UnbuiltId::AnimShelfKey,
    UnbuiltId::DebugShelfHottest,
    UnbuiltId::DebugShelfMemory,
    // Home's ribbon.
    UnbuiltId::HomeCreateTrigger,
    UnbuiltId::HomeLightingBake,
    UnbuiltId::HomeLightingQuality,
    UnbuiltId::HomeLightingLightmapView,
    // Library's ribbon.
    UnbuiltId::LibImportAnyKind,
    UnbuiltId::LibFilterTags,
    UnbuiltId::LibFilterMissing,
    UnbuiltId::LibSourceCheckIn,
    UnbuiltId::LibSourceHistory,
    UnbuiltId::LibUseShowInLevel,
    // Shading: its ribbon and its page.
    UnbuiltId::ShadingMaterialApply,
    UnbuiltId::ShadingMaterialSave,
    UnbuiltId::ShadingAddTexture,
    UnbuiltId::ShadingAddMath,
    UnbuiltId::ShadingAddParam,
    UnbuiltId::ShadingGraphAutoLayout,
    UnbuiltId::ShadingGraphComment,
    UnbuiltId::ShadingGraphWires,
    UnbuiltId::ShadingPreviewShape,
    UnbuiltId::ShadingPreviewLit,
    UnbuiltId::ShadingPreviewLive,
    UnbuiltId::ShadingGraph,
    // Animate's ribbon.
    UnbuiltId::AnimKeysAdd,
    UnbuiltId::AnimKeysAuto,
    UnbuiltId::AnimInterpStep,
    UnbuiltId::AnimInterpLinear,
    UnbuiltId::AnimInterpSmooth,
    UnbuiltId::AnimTracksTrack,
    UnbuiltId::AnimTracksCurves,
    // Debug's ribbon.
    UnbuiltId::DebugCaptureRecord,
    UnbuiltId::DebugCapturePause,
    UnbuiltId::DebugCaptureSave,
    UnbuiltId::DebugTargetSource,
    UnbuiltId::DebugTargetDeep,
    UnbuiltId::DebugShowMemory,
    UnbuiltId::DebugToolsDebugger,
    UnbuiltId::DebugToolsMemory,
    // Debug's profiler.
    UnbuiltId::DebugTimelineThreads,
    UnbuiltId::DebugStatsMemory,
    // File's column, and Recent projects.
    UnbuiltId::FilePlugins,
    UnbuiltId::RecentCardThumbnail,
    UnbuiltId::RecentTemplateFirstPerson,
    UnbuiltId::RecentTemplateTopDown,
    UnbuiltId::RecentTemplatePlatformer,
    UnbuiltId::RecentTemplateAll,
    // Preferences.
    UnbuiltId::PrefsTabInput,
    UnbuiltId::PrefsTabSourceControl,
    UnbuiltId::PrefsCodeSplit,
    UnbuiltId::PrefsShortcutsEdit,
    // Project settings, and its Input.
    UnbuiltId::ProjTabPhysics,
    UnbuiltId::ProjTabAudio,
    UnbuiltId::ProjTabPlatforms,
    UnbuiltId::ProjTabPlugins,
    UnbuiltId::ProjCompany,
    UnbuiltId::ProjVersion,
    UnbuiltId::ProjIcon,
    UnbuiltId::ProjEditorStart,
    UnbuiltId::ProjFrameRate,
    UnbuiltId::ProjVersionControl,
    UnbuiltId::InputContexts,
    UnbuiltId::InputTrigger,
    UnbuiltId::InputBuffer,
    UnbuiltId::InputDeadZone,
    UnbuiltId::InputInvertY,
    // Audio's ribbon.
    UnbuiltId::AudioMixerSend,
    UnbuiltId::AudioMixerSnapshot,
    UnbuiltId::AudioMonitorMeters,
    UnbuiltId::AudioListenChoose,
    UnbuiltId::AudioFxEq,
    UnbuiltId::AudioFxCompress,
    UnbuiltId::AudioFxReverb,
    UnbuiltId::AudioFxLimiter,
    // Audio's strips.
    UnbuiltId::AudioStripInserts,
    UnbuiltId::AudioStripPan,
    // VFX: its ribbon and its page.
    UnbuiltId::VfxEmitterAdd,
    UnbuiltId::VfxEmitterDuplicate,
    UnbuiltId::VfxModuleSpawn,
    UnbuiltId::VfxModuleVelocity,
    UnbuiltId::VfxModuleColor,
    UnbuiltId::VfxModuleSize,
    UnbuiltId::VfxModuleCollide,
    UnbuiltId::VfxSimLoop,
    UnbuiltId::VfxSimSpeed,
    UnbuiltId::VfxSimBounds,
    UnbuiltId::VfxPreviewBackground,
    UnbuiltId::VfxPreviewTime,
    UnbuiltId::VfxPreviewFollow,
    UnbuiltId::VfxPreview,
];

/// Walks the editor in `look`, with a project open: every workspace,
/// each sub-tab of Preferences and Project settings, and Recent
/// projects. Gives every unfinished part noted
/// as drawn on the way, and the name of everything on screen described
/// as not built.
fn walk(look: InterfaceLook) -> (BTreeSet<UnbuiltId>, Vec<String>) {
    use crate::file_pages::Tab;
    use crate::test_support::{harness, info};
    use crate::workbench::Workbench;
    let mut w = Workbench::new(info()).with_interface_look(look);
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    w.set_known_projects(vec![crate::KnownProject {
        root: "/projects/farm".into(),
        name: "Farm".into(),
        problem: None,
        pinned: false,
        last_opened: None,
        saved_by: None,
    }]);
    let mut h = harness(w);
    theme::restyle(&h.ctx, look);
    let mut noted = BTreeSet::new();
    let mut described = Vec::new();
    let mut look_around = |h: &mut Harness<'static, crate::test_support::State>| {
        h.run();
        h.run();
        noted.extend(drawn(&h.ctx));
        described.extend(
            h.query_all(
                By::new().predicate(|node| node.description().as_deref() == Some(DESCRIPTION)),
            )
            .map(|node| node.accesskit_node().label().unwrap_or_default()),
        );
    };
    for tab in crate::shell::RibbonTab::ALL {
        h.state_mut().0.show_tab(tab);
        look_around(&mut h);
        // A control's list of kinds is drawn only while it is open.
        let lists: Vec<egui::Rect> = h
            .query_all_by_label_contains(": more kinds")
            .map(|node| node.rect())
            .collect();
        for arrow in lists {
            crate::test_support::click(&mut h, arrow.center(), egui::Modifiers::NONE);
            look_around(&mut h);
            h.key_press(egui::Key::Escape);
            h.run();
        }
    }
    // The model viewer, from a model's right-click in the Library.
    let mut asked = Vec::new();
    h.state_mut().0.library_request(
        crate::library::LibraryRequest::ViewModel("props/crate.glb".into()),
        &mut asked,
    );
    look_around(&mut h);
    h.state_mut().0.show_tab(crate::shell::RibbonTab::Home);
    for tab in [
        Tab::General,
        Tab::Appearance,
        Tab::Viewport,
        Tab::Shortcuts,
        Tab::Project,
        Tab::TwoD,
        Tab::Input,
        Tab::Build,
    ] {
        h.state_mut().0.show_settings(tab);
        look_around(&mut h);
    }
    h.state_mut().0.start_on_recent_projects();
    look_around(&mut h);
    described.sort();
    described.dedup();
    standard_again();
    (noted, described)
}

#[test]
fn walking_the_editor_meets_every_part_it_should_and_nothing_else_says_not_built() {
    let (noted, described) = walk(InterfaceLook::default());
    let mut wanted: Vec<UnbuiltId> = DRAWN_SO_FAR.to_vec();
    wanted.sort();
    assert_eq!(noted.into_iter().collect::<Vec<_>>(), wanted);
    // Everything on screen that says it isn't built came from the
    // mechanism: its names are exactly the names of the parts drawn.
    // (Two parts may share a name: Save, on two screens.)
    let mut names: Vec<String> = wanted.iter().map(|id| get(*id).name.to_owned()).collect();
    names.sort();
    names.dedup();
    assert_eq!(described, names);
    // A part with nothing to draw is never one the walk expects.
    for (id, why) in NOTHING_TO_DRAW {
        assert!(!DRAWN_SO_FAR.contains(&id), "{id:?}: {why}");
        assert!(!why.is_empty());
    }
}

#[test]
fn walking_the_editor_with_them_hidden_meets_none() {
    let (noted, described) = walk(hiding());
    assert!(noted.is_empty(), "{noted:?}");
    assert_eq!(described, Vec::<String>::new());
}
