//! The mechanism for unfinished parts, kept true:
//!
//! - **the table is sound**, and names exactly what the plan's inventory
//!   says is unbuilt;
//! - **every shape draws** a part in place, named and described for
//!   screen readers, and a click only says so in the status bar;
//! - **hidden means gone**: with the setting on nothing registered is
//!   drawn, nothing on screen is described as not built, and what is
//!   left closes up;
//! - **the look is the approved one**: the sheet's colours are those of
//!   the pictures the project lead approved, and the sheet itself is an
//!   approved picture.

use super::sheet::{self, Asked};
use super::*;
use crate::picture_support::{
    DESIGN_SCALE, Picture, can_draw, drawn as drawn_picture, matches_approved,
};
use crate::theme::{self, InterfaceLook};
use crate::tokens::{self, size};
use egui::{Key, Rect, accesskit::Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, NodeT as _, Queryable as _};
use std::path::Path;

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

/// The parts on the placeholder sheet: every shape the frame has.
const ON_THE_SHEET: [UnbuiltId; 20] = [
    UnbuiltId::ShellTabShading,
    UnbuiltId::ShellTabVfx,
    UnbuiltId::PlacePaintScatter,
    UnbuiltId::HomeLightingBake,
    UnbuiltId::HomeLightingQuality,
    UnbuiltId::HomeLightingLightmapView,
    UnbuiltId::ShellPlaySimulate,
    UnbuiltId::FilePlugins,
    UnbuiltId::HomeToolPose,
    UnbuiltId::HomeToolMeasure,
    UnbuiltId::PrefsTabInput,
    UnbuiltId::PrefsCodeSplit,
    UnbuiltId::PlaceChipRecent,
    UnbuiltId::RecentTemplateAll,
    UnbuiltId::ShadingGraph,
    UnbuiltId::PlaceRotation,
    UnbuiltId::ProjVersionControl,
    UnbuiltId::LibShelfImportQueue,
    UnbuiltId::HomeShelfAnimation,
    UnbuiltId::HomeShelfTransport,
];

/// The placeholder sheet in a test window, in `look`; `draws` asks for
/// real frames.
fn sheet_in(look: InterfaceLook, draws: bool) -> Harness<'static, Asked> {
    let mut themed = false;
    let mut builder = Harness::builder().with_size(vec2(sheet::SIZE[0], sheet::SIZE[1]));
    if draws {
        builder = builder.renderer(egui_kittest::wgpu::WgpuTestRenderer::new());
    }
    let mut harness = builder.build_ui_state(
        move |ui, asked: &mut Asked| {
            if !themed {
                // The fonts are in force from the next frame.
                theme::apply(ui.ctx(), look);
                themed = true;
                return;
            }
            let whole = ui.ctx().content_rect();
            let mut root = ui.new_child(egui::UiBuilder::new().max_rect(whole));
            root.set_clip_rect(whole);
            let now = sheet::show(&mut root);
            asked.hide |= now.hide;
        },
        Asked::default(),
    );
    harness.run();
    harness
}

fn shown() -> Harness<'static, Asked> {
    sheet_in(InterfaceLook::default(), false)
}

fn hiding() -> InterfaceLook {
    InterfaceLook {
        hide_unfinished: true,
        ..InterfaceLook::default()
    }
}

fn standard_again() {
    crate::interface_look::use_look(InterfaceLook::default());
}

/// The part `id` on screen: the one thing with its name that is
/// described as not built (a working part may share the name).
fn part<'h>(h: &'h Harness<'_, Asked>, id: UnbuiltId) -> egui_kittest::Node<'h> {
    let name = get(id).name;
    h.get(By::new().predicate(move |node| {
        node.description().as_deref() == Some(DESCRIPTION) && node.label().as_deref() == Some(name)
    }))
}

/// The names of everything on screen described as not built.
fn described_as_unbuilt(h: &Harness<'_, Asked>) -> Vec<String> {
    let mut names: Vec<String> = h
        .query_all(By::new().predicate(|node| node.description().as_deref() == Some(DESCRIPTION)))
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect();
    names.sort();
    names
}

#[test]
fn every_shape_draws_its_part_and_notes_it() {
    let h = shown();
    let noted = drawn(&h.ctx);
    let mut wanted: Vec<UnbuiltId> = ON_THE_SHEET.to_vec();
    wanted.push(UnbuiltId::ShellStatusBranch);
    wanted.sort();
    assert_eq!(noted.into_iter().collect::<Vec<_>>(), wanted);
    // And each is on screen under its name, described as not built:
    // nothing else is.
    let mut names: Vec<String> = wanted.iter().map(|id| get(*id).name.to_owned()).collect();
    names.sort();
    assert_eq!(described_as_unbuilt(&h), names);
}

#[test]
fn a_part_is_in_place_at_the_real_thing_s_size() {
    let h = shown();
    let size_of = |id: UnbuiltId| {
        let rect = part(&h, id).rect();
        [rect.width(), rect.height()]
    };
    assert_eq!(size_of(UnbuiltId::HomeLightingBake), size::RIBBON_BIG);
    assert_eq!(size_of(UnbuiltId::ShellPlaySimulate), size::RIBBON_BIG);
    assert_eq!(
        size_of(UnbuiltId::HomeLightingQuality)[1],
        size::RIBBON_ROW_H
    );
    assert_eq!(size_of(UnbuiltId::HomeToolPose), [size::TOOL_BUTTON; 2]);
    assert_eq!(
        size_of(UnbuiltId::HomeShelfTransport),
        [size::ICON_BUTTON; 2]
    );
    assert_eq!(size_of(UnbuiltId::ShellTabShading)[1], size::RIBBON_TAB_H);
    assert_eq!(size_of(UnbuiltId::HomeShelfAnimation)[1], 26.0);
    assert_eq!(size_of(UnbuiltId::PrefsTabInput)[1], size::SUB_TAB_H);
    assert_eq!(size_of(UnbuiltId::FilePlugins)[1], size::FILE_MENU_ITEM_H);
    assert_eq!(size_of(UnbuiltId::PlaceChipRecent)[1], 22.0);
    assert_eq!(size_of(UnbuiltId::ShellStatusBranch)[1], size::STATUS_BAR_H);
    assert_eq!(size_of(UnbuiltId::PlaceRotation)[1], 24.0);
    // An unbuilt large button stands in the row of real ones, level with
    // them and the ribbon's 2 px from its neighbour.
    let (light, scatter) = (
        h.get_by_label("Light").rect(),
        part(&h, UnbuiltId::PlacePaintScatter).rect(),
    );
    assert_eq!(scatter.top(), light.top());
    assert_eq!(scatter.left() - light.right(), size::RIBBON_GAP);
    // An unbuilt setting is a row like the real one over it: as wide,
    // and as tall when its sentence takes one line too.
    let real = h
        .get_by_role_and_label(Role::CheckBox, "Show ribbon labels")
        .rect();
    let row = part(&h, UnbuiltId::PrefsCodeSplit).rect();
    assert_eq!(row.right(), real.right());
    assert_eq!(row.left(), sheet::MENU_W + size::FILE_PAGE_PAD[0]);
    assert!((row.height() - 60.0).abs() <= 1.0, "{}", row.height());
    // The page's card is 400 wide, in the middle of its room.
    let card = part(&h, UnbuiltId::ShadingGraph).rect();
    assert_eq!(card.width(), size::UNBUILT_CARD_W);
    let room = Rect::from_min_max(
        egui::pos2(sheet::RIGHT_X, size::TAB_BAR_H + size::RIBBON_H),
        egui::pos2(sheet::SIZE[0], sheet::PAGE_FOOT),
    );
    assert!((card.center().x - room.center().x).abs() <= 0.5);
    assert!((card.center().y - room.center().y).abs() <= 0.5);
}

#[test]
fn a_click_only_says_so_in_the_status_bar_for_three_seconds() {
    let mut h = shown();
    assert_eq!(said(&h.ctx), None);
    part(&h, UnbuiltId::HomeLightingBake).click();
    h.step();
    let words = "Bake isn't built yet (3D 5: Bounced light)";
    assert_eq!(said(&h.ctx).as_deref(), Some(words));
    h.step();
    assert!(h.query_by_label(words).is_some(), "the status bar says so");
    // Nothing else happened.
    assert_eq!(*h.state(), Asked::default());
    // Each step of the test window is a quarter of a second.
    for _ in 0..9 {
        h.step();
    }
    assert!(h.query_by_label(words).is_some(), "still there after 2.5 s");
    for _ in 0..4 {
        h.step();
    }
    assert_eq!(said(&h.ctx), None);
    assert!(h.query_by_label(words).is_none(), "gone after 3 s");
    // Another click says the other part's words.
    part(&h, UnbuiltId::PrefsCodeSplit).click();
    h.step();
    assert_eq!(
        said(&h.ctx).as_deref(),
        Some("Code editor split isn't built yet (not planned yet)")
    );
}

#[test]
fn the_keyboard_reaches_a_part_and_space_says_the_same() {
    let mut h = shown();
    part(&h, UnbuiltId::ShellPlaySimulate).focus();
    h.step();
    h.key_press(Key::Space);
    h.step();
    h.step();
    assert_eq!(
        said(&h.ctx).as_deref(),
        Some("Simulate isn't built yet (not planned yet)")
    );
}

#[test]
fn pointing_at_a_part_says_what_it_will_do_and_when() {
    let mut h = shown();
    part(&h, UnbuiltId::HomeLightingBake).hover();
    // Hover texts wait half a second for the pointer to rest.
    for _ in 0..6 {
        h.step();
    }
    let words = get(UnbuiltId::HomeLightingBake).hover_text();
    assert!(h.query_by_label(&words).is_some(), "{words}");
}

#[test]
fn the_page_s_link_asks_to_hide_unfinished_parts() {
    let mut h = shown();
    assert!(!h.state().hide);
    h.get_by_label("Hide unfinished parts").click();
    h.run();
    assert!(h.state().hide);
    // The card says what, and when.
    for words in [
        "The material as boxes joined by wires, to build and change",
        "Not built yet \u{b7} planned for Shared 7: Materials and shaders",
    ] {
        assert!(h.query_by_label(words).is_some(), "{words}");
    }
}

#[test]
fn hidden_nothing_registered_is_drawn_and_what_is_left_closes_up() {
    let before = shown();
    let gap_between_groups = 2.0 * size::RIBBON_GROUP_PAD[1];
    let edges = |h: &Harness<'_, Asked>, a: &str, b: &str| {
        h.get_by_label(b).rect().left() - h.get_by_label(a).rect().right()
    };
    // Shown: unbuilt parts stand between the real ones.
    assert!(edges(&before, "Library", "Animate") > 40.0);
    assert!(edges(&before, "Light", "Save") > 100.0);
    drop(before);
    let h = sheet_in(hiding(), false);
    assert!(drawn(&h.ctx).is_empty(), "{:?}", drawn(&h.ctx));
    assert_eq!(described_as_unbuilt(&h), Vec::<String>::new());
    for gone in [
        "Bake",
        "Shading",
        "VFX",
        "Code editor split",
        "Plugins",
        "Animation",
    ] {
        assert!(
            h.query_by_label(gone).is_none(),
            "{gone} is still on screen"
        );
    }
    assert!(h.query_by_label("Hide unfinished parts").is_none());
    // The real parts are all still there.
    for real in [
        "Home",
        "Library",
        "Animate",
        "Model",
        "Light",
        "Save",
        "Build",
        "Play",
        "Preferences",
        "General",
        "Appearance",
        "Show ribbon labels",
        "Harbour",
        "Entity",
        "Console",
        "Ready",
    ] {
        assert!(
            h.query_all_by_label(real).next().is_some(),
            "{real} went too"
        );
    }
    // No holes: a tab touches the tab before it, and where the whole
    // Lighting group went, Project follows Create at one group's gap.
    assert_eq!(edges(&h, "Library", "Animate"), 0.0);
    assert_eq!(edges(&h, "Model", "Light"), size::RIBBON_GAP);
    assert_eq!(edges(&h, "Light", "Save"), gap_between_groups);
    assert_eq!(edges(&h, "Build", "Play"), gap_between_groups);
    assert_eq!(edges(&h, "Entity", "Console"), size::TAB_BAR_GAP);
    standard_again();
}

/// The colour at a point of a drawn sheet.
fn at(picture: &Picture, point: egui::Pos2) -> egui::Color32 {
    picture.at([point.x, point.y], 1.0)
}

/// One of the pictures the project lead approved the look on.
fn approved_round_1(file: &str) -> Picture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("design")
        .join("g-layout")
        .join("additions")
        .join("round-1")
        .join(file);
    Picture::read(&path).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_look_is_the_one_the_project_lead_approved() {
    // What the approved pictures have, at named points.
    let preferences = approved_round_1("01-placeholder-prefs-A.png");
    let design = |picture: &Picture, point: [f32; 2]| picture.at(point, DESIGN_SCALE);
    let track = design(&preferences, [1009.0, 616.0]);
    let knob = design(&preferences, [994.0, 616.0]);
    let mark = design(&preferences, [360.25, 608.5]);
    let in_mark = design(&preferences, [362.5, 608.5]);
    assert_eq!(
        [track, knob, mark, in_mark],
        [
            tokens::line(),
            tokens::text_separator(),
            tokens::text_faint(),
            tokens::shelf_bar()
        ]
    );
    let shading = approved_round_1("01-placeholder-shading-A.png");
    let page = design(&shading, [500.0, 400.0]);
    let card = design(&shading, [530.0, 400.0]);
    let border = design(&shading, [520.25, 450.0]);
    assert_eq!(
        [page, card, border],
        [tokens::shelf_bar(), tokens::panel(), tokens::card_border()]
    );
    let home = approved_round_1("01-placeholder-home-A.png");
    assert_eq!(design(&home, [685.25, 46.5]), tokens::text_faint());
    assert_eq!(design(&home, [687.5, 46.5]), tokens::ribbon());

    // The sheet, drawn for real, has the same in the same places.
    if !can_draw() {
        return;
    }
    let mut h = sheet_in(InterfaceLook::default(), true);
    h.run();
    let picture = drawn_picture(&mut h);
    matches_approved("placeholder-sheet", &picture);
    // The unbuilt setting's faint switch, at the row's right edge.
    let row = part(&h, UnbuiltId::PrefsCodeSplit).rect();
    let switch = Rect::from_min_size(
        egui::pos2(
            row.right() - size::TOGGLE[0],
            row.center().y - size::TOGGLE[1] / 2.0 - 0.5,
        ),
        vec2(size::TOGGLE[0], size::TOGGLE[1]),
    );
    assert_eq!(at(&picture, switch.center() + vec2(8.0, 0.0)), track);
    assert_eq!(at(&picture, switch.left_center() + vec2(9.0, 0.0)), knob);
    // The mark in the corner of an unbuilt large button: 4 in from its
    // top and its right, a 1 px outline round the ribbon's own grey.
    let bake = part(&h, UnbuiltId::HomeLightingBake).rect();
    let corner = egui::pos2(
        bake.right() - size::UNBUILT_CORNER - size::UNBUILT_MARK,
        bake.top() + size::UNBUILT_CORNER,
    );
    assert_eq!(at(&picture, corner + vec2(0.5, 2.5)), mark);
    assert_eq!(at(&picture, corner + vec2(4.5, 2.5)), mark);
    assert_eq!(at(&picture, corner + vec2(2.5, 0.5)), mark);
    assert_eq!(at(&picture, corner + vec2(2.5, 4.5)), mark);
    assert_eq!(at(&picture, corner + vec2(2.5, 2.5)), tokens::ribbon());
    assert_eq!(at(&picture, corner + vec2(-1.5, 2.5)), tokens::ribbon());
    // The page: its grey, its card and the card's border.
    let page_card = part(&h, UnbuiltId::ShadingGraph).rect();
    assert_eq!(
        at(&picture, page_card.left_center() + vec2(-10.0, 0.0)),
        page
    );
    assert_eq!(
        at(&picture, page_card.left_center() + vec2(10.0, 0.0)),
        card
    );
    assert_eq!(
        at(&picture, page_card.left_center() + vec2(0.5, 0.0)),
        border
    );
}

/// The filled rectangles the last frame painted that cover `rect`: what
/// a hover or a press would add under a control.
fn fills_over(h: &Harness<'_, Asked>, rect: Rect) -> usize {
    crate::test_support::rects_drawn(h)
        .iter()
        .filter(|(drawn, fill, _)| {
            *fill != egui::Color32::TRANSPARENT
                && drawn.contains_rect(rect.shrink(1.0))
                && drawn.area() < rect.area() * 2.0
        })
        .count()
}

#[test]
fn a_part_does_not_light_up_under_the_pointer_or_when_pressed() {
    let mut h = shown();
    for id in [
        UnbuiltId::HomeLightingBake,
        UnbuiltId::HomeLightingQuality,
        UnbuiltId::ShellTabShading,
        UnbuiltId::HomeToolPose,
        UnbuiltId::HomeShelfAnimation,
        UnbuiltId::HomeShelfTransport,
        UnbuiltId::FilePlugins,
        UnbuiltId::PrefsTabInput,
        UnbuiltId::PlaceChipRecent,
    ] {
        let rect = part(&h, id).rect();
        let before = fills_over(&h, rect);
        h.hover_at(rect.center());
        h.step();
        assert_eq!(fills_over(&h, rect), before, "{id:?} under the pointer");
        h.drag_at(rect.center());
        h.step();
        assert_eq!(fills_over(&h, rect), before, "{id:?} pressed");
        h.drop_at(rect.center());
        h.step();
    }
}

#[test]
fn hidden_what_is_real_has_the_room_an_unbuilt_page_had() {
    if !can_draw() {
        return;
    }
    let mut h = sheet_in(hiding(), true);
    h.run();
    let picture = drawn_picture(&mut h);
    // Where the unbuilt page was, what is real has the room.
    let middle = egui::pos2((sheet::RIGHT_X + sheet::SIZE[0]) / 2.0, 220.0);
    assert_eq!(at(&picture, middle), tokens::viewport());
    standard_again();
}
