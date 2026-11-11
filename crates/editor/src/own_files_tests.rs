//! The files the editor changes itself, in the whole editor (the review,
//! BUG-09): a change outlasts whatever the app hands over until its save
//! is answered, the editor's own save is never taken for a change on
//! disk, and a save that fails is said and taken back as one undo step.
//! Each for every kind of file: a clips file, an animator, a tileset, an
//! atlas, a tile map. The app's part (its store of the files, kept in
//! step with each save) is played by `test_support::save_as_the_app`; the
//! app's own tests are in `app/src/window_own_files_tests.rs`.

use super::*;
use crate::console::LogLine;
use crate::library::ProjectInfo;
use crate::panel::Panel;
use crate::test_support::*;
use crate::tileset_editor::TilesetEditor;

const CLIPS: &str = "hero.solframes";
const ANIMATOR: &str = "hero.solanim";
const TILESET: &str = "farm.tileset.lst";
const ATLAS: &str = "hero.atlas.lst";
const MAP: &str = "farm.tilemap.lst";

/// A kind of file the editor changes itself.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Clips,
    Animator,
    Tileset,
    Atlas,
    Map,
}

const KINDS: [Kind; 5] = [
    Kind::Clips,
    Kind::Animator,
    Kind::Tileset,
    Kind::Atlas,
    Kind::Map,
];

impl Kind {
    fn path(self) -> &'static str {
        match self {
            Kind::Clips => CLIPS,
            Kind::Animator => ANIMATOR,
            Kind::Tileset => TILESET,
            Kind::Atlas => ATLAS,
            Kind::Map => MAP,
        }
    }

    /// The file as the project starts with it.
    fn first(self) -> OwnFile {
        match self {
            Kind::Clips => OwnFile::Frames(Arc::default()),
            Kind::Animator => {
                let id = reflect::Id::random();
                OwnFile::Animator(Arc::new(animation::AnimatorFile {
                    entry: Some(id),
                    states: vec![animation::State {
                        id,
                        name: "Idle".into(),
                        place: [0.0, 0.0],
                        plays: animation::Plays::Clip(String::new()),
                        speed: 1.0,
                        speed_by: String::new(),
                    }],
                    ..animation::AnimatorFile::default()
                }))
            }
            Kind::Tileset => OwnFile::Tileset(Arc::new(crate::workbench_tiles::starter_tileset())),
            Kind::Atlas => OwnFile::Atlas(Arc::new(animation::atlas::Atlas::new("hero.png"))),
            Kind::Map => OwnFile::TileMap(Arc::new(tiles::TileMap::new([4, 4], None, None))),
        }
    }

    /// Makes a change to the file as its tab would, saved at once: a
    /// different one each time.
    fn change(self, w: &mut Workbench) {
        match self {
            Kind::Clips => {
                let editor = w.open.frames_editor.as_mut().unwrap();
                assert!(editor.change(|f| {
                    f.clips.push(animation::frames_file::OwnClip {
                        name: format!("clip {}", f.clips.len()),
                        speed: 1.0,
                        direction: animation::Direction::Forward,
                        repeats: 0,
                        frames: Vec::new(),
                    });
                }));
                w.save_frames();
            }
            Kind::Animator => {
                let editor = w.open.animator_editor.as_mut().unwrap();
                let x = editor.file.states.len() as f32 * 200.0;
                assert!(editor.add_state([x, 0.0]));
                w.save_animator();
            }
            Kind::Tileset => assert!(w.change_tileset(TILESET, |e| e.add_terrain())),
            Kind::Atlas => {
                let editor = w.open.atlas_editor.as_mut().unwrap();
                let grid = editor.atlas.grid.map(|g| g + 1);
                assert!(editor.set_grid(grid));
                w.save_open_atlas();
            }
            Kind::Map => assert!(w.change_map(MAP, |m| {
                let name = format!("Layer{}", m.layers().len());
                m.add_layer(&name, false).is_ok()
            })),
        }
    }

    /// Ctrl+Z over the file's tab (the Map tab, for a tile map).
    fn undo(self, w: &mut Workbench) {
        match self {
            Kind::Clips => w.frames_undo(false, true),
            Kind::Animator => w.animator_undo(false, true),
            Kind::Tileset => w.tileset_undo(TILESET, false, true),
            Kind::Atlas => w.atlas_undo(false),
            Kind::Map => w.map_undo(false, true),
        }
    }

    /// The file as its editor has it (a tile map has none: as it's
    /// shown), or `None` once its editor is closed.
    fn open(self, w: &Workbench) -> Option<OwnFile> {
        let open = &w.open;
        Some(match self {
            Kind::Clips => OwnFile::Frames(Arc::new(open.frames_editor.as_ref()?.file.clone())),
            Kind::Animator => {
                OwnFile::Animator(Arc::new(open.animator_editor.as_ref()?.file.clone()))
            }
            Kind::Tileset => OwnFile::Tileset(Arc::new(open.tilesets.tab()?.file.clone())),
            Kind::Atlas => OwnFile::Atlas(Arc::new(open.atlas_editor.as_ref()?.atlas.clone())),
            Kind::Map => OwnFile::TileMap(w.map_file(MAP)?),
        })
    }

    /// The file as the editor shows it everywhere else (on sprites, on
    /// maps, to the running game).
    fn shown(self, w: &Workbench) -> Held {
        w.viewport.sheets.held(self.path())
    }
}

/// A project with one file of each kind, each open in its editor; and the
/// project's files as the app has them.
fn with_files() -> (Workbench, SpriteSheets) {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    let mut store = SpriteSheets::default();
    for kind in KINDS {
        store.put(kind.path(), kind.first());
    }
    w.set_sprite_sheets(store.clone());
    w.open_frames(CLIPS);
    w.open_animator(ANIMATOR);
    w.open_tileset(TILESET);
    w.open_atlas("hero.png");
    for kind in KINDS {
        assert_eq!(kind.open(&w), store_file(&store, kind), "{kind:?} is open");
    }
    assert!(w.open.outbox.is_empty() && said(&w).is_empty());
    (w, store)
}

/// The file of `kind` as the app's files hold it.
fn store_file(store: &SpriteSheets, kind: Kind) -> Option<OwnFile> {
    match store.held(kind.path()) {
        Held::Read(file) => Some(file),
        _ => None,
    }
}

/// What the Console has been told, but for the project opening.
fn said(w: &Workbench) -> Vec<&LogLine> {
    w.console
        .lines()
        .iter()
        .filter(|l| !l.text.starts_with("Opened the project"))
        .collect()
}

#[test]
fn a_file_s_saves_are_answered_in_order_and_the_last_sent_is_the_newest() {
    let first = Kind::Atlas.first();
    let second = OwnFile::Atlas(Arc::new(animation::atlas::Atlas::new("other.png")));
    let mut waiting = Unanswered::default();
    assert_eq!(waiting.answered(ATLAS), None, "nothing was sent");
    waiting.sent(ATLAS, first);
    waiting.sent(ATLAS, second.clone());
    waiting.sent(MAP, Kind::Map.first());
    // Laid over the app's files, the newest of each shows.
    let mut sheets = SpriteSheets::default();
    waiting.lay_over(&mut sheets);
    assert_eq!(sheets.held(ATLAS), Held::Read(second.clone()));
    assert!(matches!(sheets.held(MAP), Held::Read(OwnFile::TileMap(_))));
    // One answer of two: still waiting. The second: the file as last sent.
    assert_eq!(waiting.answered(ATLAS), None);
    assert_eq!(waiting.answered(ATLAS), Some(second));
    assert_eq!(waiting.answered(ATLAS), None, "all answered");
    let mut sheets = SpriteSheets::default();
    waiting.lay_over(&mut sheets);
    assert_eq!(sheets.held(ATLAS), Held::Gone, "nothing of it to lay over");
}

#[test]
fn an_editor_takes_in_only_a_file_that_differs_as_one_undo_step() {
    let OwnFile::Tileset(first) = Kind::Tileset.first() else {
        unreachable!()
    };
    let mut editor = TilesetEditor::new(TILESET.into(), (*first).clone());
    let mut sheets = SpriteSheets::default();
    assert_eq!(follow(&mut editor, &sheets), Followed::Gone);
    // The same file, read again into another allocation: nothing to do.
    sheets.put(TILESET, OwnFile::Tileset(Arc::new((*first).clone())));
    assert_eq!(follow(&mut editor, &sheets), Followed::Same);
    assert!(!editor.undo(), "no undo step");
    // Changed: taken in, as one step.
    let mut by_hand = (*first).clone();
    by_hand.tile = [24, 24];
    sheets.put(TILESET, OwnFile::Tileset(Arc::new(by_hand.clone())));
    assert_eq!(follow(&mut editor, &sheets), Followed::TookIn);
    assert_eq!(editor.file, by_hand);
    assert_eq!(follow(&mut editor, &sheets), Followed::Same);
    assert!(editor.undo());
    assert_eq!(editor.file, *first);
    assert!(!editor.undo(), "one step, not two");
    // A file that can't be read is left alone: the editor keeps its own.
    sheets
        .tilesets
        .insert(TILESET.into(), TilesetState::Failed("line 3".into()));
    assert_eq!(follow(&mut editor, &sheets), Followed::Same);
    assert_eq!(editor.file, *first);
}

/// The review, BUG-09 (b): the app's files were handed over whole whenever
/// anything was read, and held the file as it was before a change the app
/// hadn't saved yet, so the change was undone on screen (and a tileset or
/// atlas said it had "changed on disk").
#[test]
fn a_change_not_saved_yet_outlasts_whatever_the_app_hands_over() {
    for kind in KINDS {
        let (mut w, store) = with_files();
        let first = kind.open(&w);
        kind.change(&mut w);
        let mine = kind.open(&w).unwrap();
        assert_ne!(Some(&mine), first.as_ref());
        // Something else was read before the app saved this: its files
        // are still the older ones.
        w.set_sprite_sheets(store.clone());
        assert_eq!(kind.open(&w), Some(mine.clone()), "{kind:?}: kept");
        assert_eq!(kind.shown(&w), Held::Read(mine), "{kind:?}: and shown");
        assert_eq!(said(&w).len(), 0, "{kind:?}: {:?}", said(&w));
        // Its undo history is as it was: one step back to the first file.
        kind.undo(&mut w);
        assert_eq!(kind.open(&w), first, "{kind:?}");
        kind.undo(&mut w);
        assert_eq!(kind.open(&w), first, "{kind:?}: only one step");
    }
}

/// The review, BUG-09 (c): an older save of the editor's own, read back
/// after a newer change, was reported as "changed on disk" and took the
/// newer change back.
#[test]
fn its_own_save_read_back_is_never_a_change_on_disk() {
    for kind in KINDS {
        let (mut w, mut store) = with_files();
        let first = kind.open(&w);
        kind.change(&mut w);
        let older = kind.open(&w);
        assert_eq!(save_as_the_app(&mut w, &mut store).len(), 1);
        kind.change(&mut w);
        let newer = kind.open(&w).unwrap();
        // The first save is read back, with the second not saved yet.
        assert_eq!(store_file(&store, kind), older);
        w.set_sprite_sheets(store.clone());
        assert_eq!(kind.open(&w), Some(newer.clone()), "{kind:?}");
        assert_eq!(kind.shown(&w), Held::Read(newer.clone()), "{kind:?}");
        // Then the second is saved, and read back as well.
        assert_eq!(save_as_the_app(&mut w, &mut store).len(), 1);
        w.set_sprite_sheets(store.clone());
        w.set_sprite_sheets(store.clone());
        assert_eq!(kind.open(&w), Some(newer), "{kind:?}");
        assert_eq!(said(&w).len(), 0, "{kind:?}: {:?}", said(&w));
        // Two changes, two undo steps, and no more.
        kind.undo(&mut w);
        assert_eq!(kind.open(&w), older, "{kind:?}");
        kind.undo(&mut w);
        assert_eq!(kind.open(&w), first, "{kind:?}");
        kind.undo(&mut w);
        assert_eq!(kind.open(&w), first, "{kind:?}: nothing more to undo");
    }
}

/// The review, BUG-09 (a): a save that failed left the change on screen
/// with its undo step, and the next thing read took it away without a
/// word.
#[test]
fn a_save_that_fails_is_said_and_taken_back_as_one_undo_step() {
    for kind in KINDS {
        let (mut w, mut store) = with_files();
        let first = kind.open(&w);
        kind.change(&mut w);
        let mine = kind.open(&w).unwrap();
        // The app tried, and couldn't.
        w.open.outbox.clear();
        w.file_not_saved(kind.path(), "the disk is full", store.clone());
        // Shown as the file is, everywhere: the change isn't in it.
        assert_eq!(kind.open(&w), first, "{kind:?}");
        assert_eq!(Some(kind.shown(&w)), first.clone().map(Held::Read));
        // Said once, as an error: the file, why, and how to get it back.
        let lines = said(&w);
        assert_eq!(lines.len(), 1, "{kind:?}: {lines:?}");
        assert_eq!(lines[0].level, LogLevel::Error);
        let text = &lines[0].text;
        let begins = format!("Couldn't save {}: the disk is full. ", kind.path());
        assert!(text.starts_with(&begins), "{kind:?}: {text}");
        assert!(text.contains("without your change"), "{kind:?}: {text}");
        assert!(text.contains("Ctrl+Z over the "), "{kind:?}: {text}");
        // Reading the files again changes nothing more, and says nothing.
        w.set_sprite_sheets(store.clone());
        assert_eq!(kind.open(&w), first, "{kind:?}");
        assert_eq!(said(&w).len(), 1, "{kind:?}");
        // Ctrl+Z brings the change back and saves it again: this time the
        // app can, and the file holds it.
        kind.undo(&mut w);
        assert_eq!(kind.open(&w), Some(mine.clone()), "{kind:?}");
        assert_eq!(save_as_the_app(&mut w, &mut store).len(), 1, "{kind:?}");
        assert_eq!(store_file(&store, kind), Some(mine.clone()), "{kind:?}");
        w.set_sprite_sheets(store.clone());
        assert_eq!(kind.open(&w), Some(mine), "{kind:?}");
        assert_eq!(said(&w).len(), 1, "{kind:?}: nothing more to say");
    }
}

#[test]
fn a_failed_save_with_a_later_one_on_its_way_takes_nothing_back() {
    for kind in KINDS {
        let (mut w, mut store) = with_files();
        kind.change(&mut w);
        kind.change(&mut w);
        let newest = kind.open(&w).unwrap();
        // The first save fails; the second is still to be tried.
        let first = w.open.outbox.remove(0);
        assert!(matches!(first, WorkbenchAction::SaveFile { .. }));
        w.file_not_saved(kind.path(), "the file was in use", store.clone());
        assert_eq!(kind.open(&w), Some(newest.clone()), "{kind:?}: kept");
        assert_eq!(kind.shown(&w), Held::Read(newest.clone()), "{kind:?}");
        let lines = said(&w);
        assert_eq!(lines.len(), 1, "{kind:?}: {lines:?}");
        assert_eq!(
            lines[0].text,
            format!("Couldn't save {}: the file was in use.", kind.path())
        );
        // The second holds both changes, and is saved.
        assert_eq!(save_as_the_app(&mut w, &mut store).len(), 1, "{kind:?}");
        assert_eq!(store_file(&store, kind), Some(newest.clone()), "{kind:?}");
        w.set_sprite_sheets(store.clone());
        assert_eq!(kind.open(&w), Some(newest), "{kind:?}");
        assert_eq!(said(&w).len(), 1, "{kind:?}");
    }
}

#[test]
fn a_new_file_that_couldn_t_be_made_closes_its_tab_and_says_so() {
    let (mut w, store) = with_files();
    w.new_animator("sprites");
    let path = "sprites/animator.solanim";
    assert_eq!(w.open.animator_editor.as_ref().unwrap().path, path);
    w.open.outbox.clear();
    w.file_not_saved(path, "the folder isn't there", store.clone());
    assert!(w.open.animator_editor.is_none());
    assert!(w.dock.find_tab(&Panel::Animator).is_none());
    assert_eq!(w.viewport.sheets.held(path), Held::Gone);
    let lines = said(&w);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(
        lines[0].text,
        "Couldn't save sprites/animator.solanim: the folder isn't there. The file isn't \
         there, so the Animator tab closed."
    );
    // A tile map has no tab of its own: it's only said.
    w.new_tile_map("");
    let made: Vec<String> = saved_files(&w.open.outbox, |f| matches!(f, OwnFile::TileMap(_)))
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(made.len(), 1);
    w.file_not_saved(&made[0], "the disk is full", store);
    assert_eq!(w.viewport.sheets.held(&made[0]), Held::Gone);
    let last = said(&w).pop().unwrap().text.clone();
    assert!(last.ends_with("the disk is full. The file isn't there."));
}

#[test]
fn a_tileset_changed_from_properties_whose_save_fails_says_where_to_undo() {
    let (mut w, store) = with_files();
    w.close_tileset();
    Kind::Tileset.change(&mut w);
    w.file_not_saved(TILESET, "the disk is full", store);
    let lines = said(&w);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].text.contains("Ctrl+Z over its Properties"));
}

#[test]
fn an_answer_no_save_is_waiting_for_changes_nothing_but_is_said() {
    let (mut w, store) = with_files();
    w.file_saved(MAP);
    w.file_not_saved(MAP, "the disk is full", store);
    for kind in KINDS {
        assert_eq!(kind.open(&w), Some(kind.shown(&w)).and_then(read));
    }
    let lines = said(&w);
    assert_eq!(lines.len(), 1);
    assert_eq!(
        lines[0].text,
        "Couldn't save farm.tilemap.lst: the disk is full."
    );
}

fn read(held: Held) -> Option<OwnFile> {
    match held {
        Held::Read(file) => Some(file),
        _ => None,
    }
}
