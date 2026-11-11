//! Who the keys belong to, in the whole editor: with the pointer over a
//! tab that edits a file of its own, a key acts on what that tab shows
//! and never on the area as well; everywhere else it's the area's.

use crate::layout;
use crate::library::{LibraryItem, LibraryKind, LibraryRequest, ProjectInfo};
use crate::panel::Panel;
use crate::screen_doc::tests::MENU;
use crate::sprite_sheets::{
    AnimatorState, ArtState, FramesState, MapFileState, SheetState, SpriteSheets, TilesetState,
};
use crate::test_support::*;
use crate::viewport::Tool;
use crate::workbench::Workbench;
use egui::Modifiers;
use egui_kittest::Harness;
use reflect::AssetId;
use std::sync::Arc;

const SET: &str = "maps/town.tileset.lst";
const MAP: &str = "maps/town.tilemap.lst";
const SHEET: &str = "walker.aseprite";
const CLIPS: &str = "walker.solframes";
const ANIMATOR: &str = "farmer.solanim";
const PICTURE: &str = "sprites/tools.png";
const SCREEN: &str = "ui/pauseMenu.lst";

/// A project with a file for every tab that edits one (a tileset and a
/// map of it, a sheet with a clips file and an animator, a picture to
/// crop, a screen), none of them open yet, and an area of a Barrel and a
/// Crate, the Crate selected. Returns the map's ID with it.
fn farm() -> (Workbench, AssetId) {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    w.set_program(Some(crate::workbench_screens::tests::program(MENU)));
    let [set, map, sheet, clips, animator, picture] = [(); 6].map(|()| AssetId::random());
    let item = |path: &str, kind, id| LibraryItem {
        id: Some(id),
        kind,
        name: crate::sprite_sheets::name_of(path),
        path: path.into(),
        thumbnail: None,
    };
    w.set_library(vec![
        item(SET, LibraryKind::Tileset, set),
        item(MAP, LibraryKind::TileMap, map),
        item(SHEET, LibraryKind::SpriteSheet, sheet),
        item(CLIPS, LibraryKind::SpriteClips, clips),
        item(ANIMATOR, LibraryKind::Animator, animator),
        item(PICTURE, LibraryKind::Texture, picture),
    ]);
    let wave = animation::FramesFile {
        sheet: Some(sheet),
        first: "wave".into(),
        tags: Vec::new(),
        clips: vec![animation::OwnClip {
            name: "wave".into(),
            speed: 1.0,
            direction: animation::Direction::Forward,
            repeats: 0,
            frames: (0..2)
                .map(|frame| animation::ClipFrame {
                    frame,
                    ms: 100,
                    events: Vec::new(),
                })
                .collect(),
        }],
    };
    let idle = animation::State {
        id: reflect::Id::random(),
        name: "Idle".into(),
        place: [0.0, 0.0],
        plays: animation::Plays::Clip("wave".into()),
        speed: 1.0,
        speed_by: String::new(),
    };
    let farmer = animation::AnimatorFile {
        sheet: Some(clips),
        entry: Some(idle.id),
        states: vec![idle],
        ..animation::AnimatorFile::default()
    };
    // The picture: 32 × 16, all of it drawn.
    let art = assets::tile_art::TileArt {
        size: [32, 16],
        frames: vec![vec![255; 32 * 16 * 4]],
        durations: vec![100],
    };
    let mut sheets = SpriteSheets::default();
    sheets.tilesets.insert(
        SET.into(),
        TilesetState::Read(
            Arc::new(crate::workbench_tiles::starter_tileset()),
            Vec::new(),
        ),
    );
    sheets.tile_maps.insert(
        MAP.into(),
        MapFileState::Read(
            Arc::new(crate::workbench_tiles::tests::farm_map(set)),
            Vec::new(),
        ),
    );
    sheets.by_path.insert(
        SHEET.into(),
        SheetState::Read(Arc::new(crate::sprite_sheets::tests::test_sheet())),
    );
    sheets
        .frames
        .insert(CLIPS.into(), FramesState::Read(Arc::new(wave), Vec::new()));
    sheets.animators.insert(
        ANIMATOR.into(),
        AnimatorState::Read(Arc::new(farmer), Vec::new()),
    );
    sheets
        .tile_art
        .insert(PICTURE.into(), ArtState::Read(Arc::new(art)));
    for (id, path) in [
        (set, SET),
        (map, MAP),
        (sheet, SHEET),
        (clips, CLIPS),
        (animator, ANIMATOR),
        (picture, PICTURE),
    ] {
        sheets.paths.insert(id, path.into());
    }
    w.set_sprite_sheets(sheets);
    w.spawn_selected(area::EntityData::new("Barrel"));
    w.spawn_selected(area::EntityData::new("Crate"));
    (w, map)
}

/// The farm drawn, with what `panel` shows open in it, `panel` in front
/// and the pointer resting over the middle of it.
fn pointer_over(panel: Panel) -> Harness<'static, State> {
    let (mut w, map) = farm();
    let mut actions = Vec::new();
    match panel {
        // The Map tab comes with a selected map: the map placed is.
        Panel::Map => {
            w.place_tile_map(map, None).expect("placed");
        }
        Panel::Clips | Panel::Frames => w.open_frames(CLIPS),
        Panel::Animator => w.open_animator(ANIMATOR),
        Panel::Tileset => w.open_tileset(SET),
        Panel::AtlasCrop => {
            w.library_request(LibraryRequest::AtlasCrop(PICTURE.into()), &mut actions);
        }
        Panel::Canvas | Panel::Elements => open_the_screen(&mut w),
        _ => {}
    }
    rest_over(harness(w), panel)
}

/// Opens the farm's screen on the canvas (in the UI workspace).
fn open_the_screen(w: &mut Workbench) {
    let mut actions = Vec::new();
    w.library_request(
        LibraryRequest::Open(LibraryKind::Code, SCREEN.into()),
        &mut actions,
    );
}

/// Draws the editor with `panel` in front and the pointer resting over
/// the middle of it.
fn rest_over(mut h: Harness<'static, State>, panel: Panel) -> Harness<'static, State> {
    h.run();
    layout::bring_to_front(&mut h.state_mut().0.dock, panel);
    h.run();
    let path = h.state().0.dock.find_tab(&panel).expect("the tab is open");
    let body = h.state().0.dock[path.surface][path.node]
        .get_leaf()
        .expect("a tab group")
        .viewport;
    h.hover_at(body.center());
    h.run();
    h
}

/// What the area's own keys change.
#[derive(Debug, Clone, PartialEq)]
struct AreaNow {
    entities: usize,
    selected: usize,
    tool: Tool,
    x_locked: bool,
}

fn area_now(w: &Workbench) -> AreaNow {
    AreaNow {
        entities: w.open.document.area().len(),
        selected: w.open.document.selection.iter().count(),
        tool: w.viewport.tool,
        x_locked: w.viewport.snapping.locked[0],
    }
}

/// A key of the area's, and what it does to an area whose one selected
/// entity was the last thing added.
#[derive(Debug, Clone, Copy)]
enum AreaKey {
    Delete,
    Duplicate,
    SelectNothing,
    SelectAll,
    MoveTool,
    LockX,
    Undo,
}

impl AreaKey {
    const ALL: [Self; 7] = [
        Self::Delete,
        Self::Duplicate,
        Self::SelectNothing,
        Self::SelectAll,
        Self::MoveTool,
        Self::LockX,
        Self::Undo,
    ];

    fn press(self, h: &Harness<'static, State>) {
        match self {
            Self::Delete => h.key_press(egui::Key::Delete),
            Self::Duplicate => h.key_press_modifiers(Modifiers::COMMAND, egui::Key::D),
            Self::SelectNothing => h.key_press(egui::Key::Escape),
            Self::SelectAll => h.key_press_modifiers(Modifiers::COMMAND, egui::Key::A),
            Self::MoveTool => h.key_press(egui::Key::W),
            Self::LockX => h.key_press(egui::Key::X),
            Self::Undo => h.key_press_modifiers(Modifiers::COMMAND, egui::Key::Z),
        }
    }

    /// The area after this key acted on it as it was `before`.
    fn done_to(self, before: &AreaNow) -> AreaNow {
        let mut after = before.clone();
        match self {
            Self::Delete => {
                after.entities -= 1;
                after.selected = 0;
            }
            Self::Duplicate => after.entities += 1,
            Self::SelectNothing => after.selected = 0,
            Self::SelectAll => after.selected = before.entities,
            Self::MoveTool => after.tool = Tool::Move,
            Self::LockX => after.x_locked = !before.x_locked,
            // The last step added the selected entity; before it, the
            // one added earlier was selected.
            Self::Undo => after.entities -= 1,
        }
        after
    }
}

/// Every panel the pointer can rest over, whether the area's keys are
/// acted on there, and whether Ctrl+Z there is the area's.
const OWNERS: [(Panel, bool, bool); 12] = [
    (Panel::Viewport, true, true),
    (Panel::Area, true, true),
    (Panel::Properties, true, true),
    (Panel::Library, true, true),
    // Beside Properties, for the selected entity's map: only undo is its.
    (Panel::Map, true, false),
    (Panel::Clips, false, false),
    (Panel::Frames, false, false),
    (Panel::Animator, false, false),
    (Panel::Tileset, false, false),
    (Panel::AtlasCrop, false, false),
    (Panel::Canvas, false, false),
    (Panel::Elements, false, false),
];

#[test]
fn each_key_of_the_area_s_acts_on_it_only_where_the_area_owns_the_keys() {
    for (panel, area_keys, area_undo) in OWNERS {
        for key in AreaKey::ALL {
            let mut h = pointer_over(panel);
            let before = area_now(&h.state().0);
            assert_eq!(before.selected, 1, "{panel:?}: one entity selected");
            key.press(&h);
            h.run();
            let acts = match key {
                AreaKey::Undo => area_undo,
                _ => area_keys,
            };
            let expected = if acts {
                key.done_to(&before)
            } else {
                before.clone()
            };
            assert_eq!(
                area_now(&h.state().0),
                expected,
                "{key:?} with the pointer over {panel:?}"
            );
        }
    }
}

/// The screen's text with the pointer over `panel` (the Canvas or
/// Elements) and the piece at `piece` picked.
fn screen_with(panel: Panel, piece: &[usize]) -> Harness<'static, State> {
    let mut h = pointer_over(panel);
    let doc = h.state_mut().0.open.screen_doc.as_mut().expect("open");
    doc.picked = vec![piece.to_vec()];
    h
}

fn screen_text(h: &Harness<'static, State>) -> String {
    h.state().0.open.screen_doc.as_ref().unwrap().text.clone()
}

#[test]
fn delete_over_the_canvas_or_elements_removes_the_piece_and_not_the_entity() {
    // The review (BUG-06): Delete was read twice, by the canvas and by the
    // area's keys, so a button and the selected entity both went.
    for panel in [Panel::Canvas, Panel::Elements] {
        let mut h = screen_with(panel, &[0, 0, 1]);
        let before = area_now(&h.state().0);
        assert!(screen_text(&h).contains("Resume"));
        h.key_press(egui::Key::Delete);
        h.run();
        assert!(
            !screen_text(&h).contains("Resume"),
            "{panel:?}: the piece went"
        );
        assert_eq!(
            area_now(&h.state().0),
            before,
            "{panel:?}: the area is as it was"
        );
    }
}

#[test]
fn ctrl_d_over_the_canvas_or_elements_copies_the_piece_and_not_the_entity() {
    // The review (BUG-06): the area's keys took Ctrl+D first, so it never
    // reached the canvas and the selected entity was duplicated instead.
    for panel in [Panel::Canvas, Panel::Elements] {
        let mut h = screen_with(panel, &[0, 0, 0]);
        let before = area_now(&h.state().0);
        h.key_press_modifiers(Modifiers::COMMAND, egui::Key::D);
        h.run();
        assert_eq!(
            screen_text(&h).matches("text \"Paused\"").count(),
            2,
            "{panel:?}: the piece was copied"
        );
        assert_eq!(
            area_now(&h.state().0),
            before,
            "{panel:?}: the area is as it was"
        );
    }
}

#[test]
fn esc_over_the_canvas_picks_no_piece_and_keeps_the_area_s_selection() {
    let mut h = screen_with(Panel::Canvas, &[0, 0, 1]);
    let before = area_now(&h.state().0);
    h.key_press(egui::Key::Escape);
    h.run();
    let w = &h.state().0;
    assert!(w.open.screen_doc.as_ref().unwrap().picked.is_empty());
    assert_eq!(area_now(w), before);
}

/// The Atlas Crop tab under the pointer, with two crops, the first picked.
fn atlas_with_crops() -> Harness<'static, State> {
    let mut h = pointer_over(Panel::AtlasCrop);
    let editor = h.state_mut().0.open.atlas_editor.as_mut().expect("open");
    assert!(editor.add([0, 0, 16, 16]) && editor.add([16, 0, 16, 16]));
    editor.pick(Some(0));
    h
}

fn crops(h: &Harness<'static, State>) -> usize {
    let editor = h.state().0.open.atlas_editor.as_ref().unwrap();
    editor.atlas.crops.len()
}

#[test]
fn delete_over_the_atlas_crop_tab_removes_the_crop_and_not_the_entity() {
    // The review (BUG-06): the picked crop and the selected entity both went.
    let mut h = atlas_with_crops();
    let before = area_now(&h.state().0);
    h.key_press(egui::Key::Delete);
    h.run();
    assert_eq!(crops(&h), 1, "the picked crop went");
    assert_eq!(area_now(&h.state().0), before, "the area is as it was");
}

#[test]
fn ctrl_d_over_the_atlas_crop_tab_duplicates_nothing() {
    // The tab has no Ctrl+D of its own, and the area's isn't for it.
    let mut h = atlas_with_crops();
    let before = area_now(&h.state().0);
    h.key_press_modifiers(Modifiers::COMMAND, egui::Key::D);
    h.run();
    assert_eq!(crops(&h), 2);
    assert_eq!(area_now(&h.state().0), before);
}

#[test]
fn delete_and_ctrl_d_over_the_viewport_are_still_the_area_s() {
    // With a screen open and a piece picked: the pointer decides, not
    // what happens to be open.
    let (mut w, _) = farm();
    open_the_screen(&mut w);
    let mut h = rest_over(harness(w), Panel::Viewport);
    let doc = h.state_mut().0.open.screen_doc.as_mut().expect("open");
    doc.picked = vec![vec![0, 0, 1]];
    let before = area_now(&h.state().0);
    h.key_press_modifiers(Modifiers::COMMAND, egui::Key::D);
    h.run();
    assert_eq!(
        area_now(&h.state().0).entities,
        before.entities + 1,
        "the Crate was duplicated"
    );
    h.key_press(egui::Key::Delete);
    h.run();
    assert_eq!(area_now(&h.state().0).entities, before.entities);
    assert!(screen_text(&h).contains("Resume"), "the piece is untouched");
    assert_eq!(screen_text(&h).matches("text \"Paused\"").count(), 1);
}

/// A tab with a file of its own: a change to make in it, and whether the
/// change is there.
type OwnChange = (Panel, fn(&mut Workbench), fn(&Workbench) -> bool);

const OWN_CHANGES: [OwnChange; 8] = [
    (
        Panel::Map,
        |w| w.map_requests_done(vec![crate::map_tab::MapRequest::AddLayer]),
        |w| w.map_file(MAP).unwrap().layers().len() == 4,
    ),
    (Panel::Clips, speed_the_clip_up, the_clip_is_faster),
    (Panel::Frames, speed_the_clip_up, the_clip_is_faster),
    (
        Panel::Animator,
        |w| {
            let editor = w.open.animator_editor.as_mut().unwrap();
            let idle = editor.file.states[0].id;
            assert!(editor.rename_state(idle, "Hoe"));
        },
        |w| w.open.animator_editor.as_ref().unwrap().file.states[0].name == "Hoe",
    ),
    (
        Panel::Tileset,
        |w| assert!(w.open.tileset_editor.as_mut().unwrap().add_terrain()),
        |w| w.open.tileset_editor.as_ref().unwrap().file.terrains.len() == 4,
    ),
    (
        Panel::AtlasCrop,
        |w| assert!(w.open.atlas_editor.as_mut().unwrap().add([0, 0, 16, 16])),
        |w| w.open.atlas_editor.as_ref().unwrap().atlas.crops.len() == 1,
    ),
    (Panel::Canvas, take_resume_out, resume_is_out),
    (Panel::Elements, take_resume_out, resume_is_out),
];

fn speed_the_clip_up(w: &mut Workbench) {
    assert!(w.open.frames_editor.as_mut().unwrap().set_speed(2.0));
}

fn the_clip_is_faster(w: &Workbench) -> bool {
    w.open.frames_editor.as_ref().unwrap().file.clips[0].speed == 2.0
}

fn take_resume_out(w: &mut Workbench) {
    let doc = w.open.screen_doc.as_mut().unwrap();
    doc.picked = vec![vec![0, 0, 1]];
    assert!(doc.delete_picked());
}

fn resume_is_out(w: &Workbench) -> bool {
    !w.open.screen_doc.as_ref().unwrap().text.contains("Resume")
}

#[test]
fn ctrl_z_and_ctrl_y_over_a_tab_are_for_that_tab_s_own_changes() {
    for (panel, change, changed) in OWN_CHANGES {
        let mut h = pointer_over(panel);
        change(&mut h.state_mut().0);
        assert!(changed(&h.state().0), "{panel:?}: changed");
        let before = area_now(&h.state().0);
        h.key_press_modifiers(Modifiers::COMMAND, egui::Key::Z);
        h.run();
        assert!(!changed(&h.state().0), "{panel:?}: undone over its tab");
        h.key_press_modifiers(Modifiers::COMMAND, egui::Key::Y);
        h.run();
        assert!(changed(&h.state().0), "{panel:?}: redone over its tab");
        assert_eq!(
            area_now(&h.state().0),
            before,
            "{panel:?}: the area is as it was"
        );
    }
}

#[test]
fn space_plays_the_clip_over_the_frames_tab_and_opens_the_search_elsewhere() {
    let playing = |h: &Harness<'static, State>| {
        let w = &h.state().0;
        (
            w.open.frames_editor.as_ref().unwrap().playing,
            w.search.open,
        )
    };
    let mut h = pointer_over(Panel::Frames);
    h.key_press(egui::Key::Space);
    // One frame: a playing clip asks to be drawn again and again.
    h.step();
    assert_eq!(playing(&h), (true, false), "over Frames: the clip plays");
    // The Clips tab has no Space of its own, so it's the search's there.
    let mut h = pointer_over(Panel::Clips);
    h.key_press(egui::Key::Space);
    h.step();
    assert_eq!(playing(&h), (false, true), "over Clips: the search opens");
}

#[test]
fn a_key_pressed_to_be_bound_is_bound_and_does_nothing_else() {
    // Delete pressed to bind it to Jump deleted the selected entity too.
    let (mut w, _) = farm();
    w.open_input_window();
    let mut h = harness(w);
    h.run();
    let jumps = |h: &Harness<'static, State>| {
        let jump = h.state().0.bindings().get("Jump").unwrap();
        jump.inputs.len()
    };
    let (before, bound) = (area_now(&h.state().0), jumps(&h));
    // Jump's + (the third action); the click itself isn't bound, so a
    // frame or two pass first.
    let window = h.state_mut().0.open.input_window.as_mut().unwrap();
    window.listening = Some((2, crate::input_window::Slot::Inputs, 0));
    for _ in 0..3 {
        h.run();
    }
    h.key_press(egui::Key::Delete);
    h.run();
    assert_eq!(jumps(&h), bound + 1, "bound to Jump");
    assert_eq!(area_now(&h.state().0), before, "and nothing deleted");
}

#[test]
fn esc_with_the_library_s_drawer_up_tucks_it_away_and_keeps_the_selection() {
    let (mut w, _) = farm();
    w.set_library_pinned(false);
    w.drawer_open = true;
    let mut h = harness(w);
    h.run();
    let before = area_now(&h.state().0);
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(!h.state().0.drawer_open(), "tucked away");
    assert_eq!(area_now(&h.state().0), before, "the selection is kept");
    // With the drawer away, Esc is the area's again.
    h.key_press(egui::Key::Escape);
    h.run();
    assert_eq!(area_now(&h.state().0).selected, 0);
}

#[test]
fn a_tab_has_the_keys_only_while_its_file_is_open() {
    use crate::key_owner::KeyOwner;
    let ctx = egui::Context::default();
    // An empty Canvas (no screen open) has nothing to act on.
    let (mut w, _) = farm();
    w.pointer_over = Some(Panel::Canvas);
    assert_eq!(w.key_owner(&ctx), KeyOwner::Area);
    open_the_screen(&mut w);
    assert_eq!(w.key_owner(&ctx), KeyOwner::Canvas);
    // A key being bound comes before wherever the pointer rests, and the
    // game having the keys before everything.
    w.open_input_window();
    let window = w.open.input_window.as_mut().unwrap();
    window.listening = Some((2, crate::input_window::Slot::Inputs, 0));
    assert_eq!(w.key_owner(&ctx), KeyOwner::Binding);
    w.game_input = true;
    assert_eq!(w.key_owner(&ctx), KeyOwner::Game);
}
