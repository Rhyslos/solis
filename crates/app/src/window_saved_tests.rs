//! Settings saved by builds before the design's look still start the
//! editor (CONTRIBUTING.md, rule 23): the three files in
//! `tests/fixtures/editor-settings/before-g-layout/`, written by the last
//! build before decision 0100, each read by this build's own reader.

use super::super::tests::Memory;
use super::super::{
    ACCENT_KEY, AUTO_UPDATE_KEY, EDITOR_SIZE_KEY, FIRST_SIZE, GUIDE_BROWSER_KEY,
    INTERFACE_LOOK_KEY, OLD_ACCENT_KEY, SHELL_KEY,
};
use super::*;
use editor::theme::{Accent, Density, Scale};
use eframe::Storage as _;
use eframe::egui;
use std::collections::HashMap;
use std::path::Path;

/// The settings in one of the files, as eframe would hand them over.
fn settings(name: &str) -> Memory {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("editor-settings")
        .join("before-g-layout")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    // eframe's file is a map of keys to values in its own text form, and
    // eframe's reader is the one that knows it: the whole file is handed
    // to it as one value.
    let mut whole = Memory::default();
    whole.set_string("file", text);
    let pairs: HashMap<String, String> = eframe::get_value(&whole, "file")
        .unwrap_or_else(|| panic!("{name} isn't a settings file eframe can read"));
    assert!(pairs.len() >= 8, "{name} holds only {} keys", pairs.len());
    let mut storage = Memory::default();
    for (key, value) in pairs {
        storage.set_string(&key, value);
    }
    storage
}

fn info() -> AppInfo {
    AppInfo {
        name: crate::branding::APP_NAME.to_string(),
        version: "0.0.1".to_string(),
    }
}

/// Starts the editor from `storage` as the window does, without a window.
fn started(storage: &Memory) -> (Saved, Workbench) {
    let mut saved = Saved::read(Some(storage));
    let mut workbench = saved.workbench(info());
    saved.settle(&mut workbench);
    (saved, workbench)
}

#[test]
fn a_first_run_s_settings_start_the_new_look_with_everything_else_kept() {
    let storage = settings("standard-blue.ron");
    let (saved, workbench) = started(&storage);
    // The standard theme and its blue become Neutral dark and its blue.
    assert_eq!(saved.look, InterfaceLook::default());
    assert_eq!(workbench.interface_look(), InterfaceLook::default());
    assert_eq!(workbench.accent(), Accent::Blue);
    // What the look doesn't touch is as it was saved.
    assert_eq!(saved.editor_size, FIRST_SIZE);
    assert_eq!(saved.known.saved().len(), 2);
    assert!(saved.auto_update);
    assert_eq!(saved.skipped, None);
    assert!(workbench.hints_on());
    assert_eq!(workbench.camera(), Camera::default());
    assert_eq!(workbench.look(), Look::default());
    assert_eq!(workbench.snapping(), editor::Snapping::default());
    assert_eq!(workbench.overlays(), Overlays::default());
    assert!(workbench.console_history().is_empty());
    starts_with_the_standard_frame_and_says_so_once(&workbench);
}

/// Started from settings with a docked panels' arrangement from before
/// the frame: every tab is as the design draws it, and the Console says
/// once that the arrangement wasn't carried over.
fn starts_with_the_standard_frame_and_says_so_once(workbench: &Workbench) {
    assert_eq!(workbench.ribbon_tab(), editor::RibbonTab::Home);
    assert!(!workbench.ribbon_collapsed());
    // The standard panel layout (decision 0109): Home's lists beside the
    // area, the Console in front on the shelf.
    assert_eq!(
        workbench.interface_look().layout,
        editor::interface_look::PanelLayout::AreaLeftAssetsBottom
    );
    assert_eq!(workbench.shelf_tab(), Some(editor::ShelfTab::Console));
    assert!(!workbench.shelf_folded());
    let said = workbench
        .log_lines()
        .iter()
        .filter(|l| l.text.contains("wasn't carried over"))
        .count();
    assert_eq!(said, 1);
}

/// One frame of `app`, as eframe asks for it, with `events` in it.
fn frame(ctx: &egui::Context, app: &mut super::super::App, events: Vec<egui::Event>) {
    let mut frame = eframe::Frame::_new_kittest();
    let input = egui::RawInput {
        events,
        ..egui::RawInput::default()
    };
    let mut output = ctx.run_ui(input, |ui| eframe::App::ui(app, ui, &mut frame));
    output.textures_delta.clear();
}

/// The second review pass's N-04: every start opens on File › Recent
/// projects, the frame saved last time behind it, through `App::new` in
/// the window's own order. The saved tab used to be put back after
/// Recent projects was chosen, and covered it at every start but the
/// first. Search projects doesn't take the keys at the start, so Esc
/// carries on into the editor at once, on the tab left in front.
#[test]
fn every_start_opens_on_recent_projects_with_the_saved_frame_behind_it() {
    let mut storage = Memory::default();
    // Nothing is asked of the network: checking for updates is off.
    eframe::set_value(&mut storage, AUTO_UPDATE_KEY, &false);
    let mut last_run = Workbench::new(info());
    last_run.show_tab(editor::RibbonTab::Audio);
    eframe::set_value(&mut storage, SHELL_KEY, &last_run.shell_state());
    let ctx = egui::Context::default();
    let mut cc = eframe::CreationContext::_new_kittest(ctx.clone());
    cc.storage = Some(&storage);
    let mut app = super::super::App::new(
        &cc,
        crate::app_info(),
        Vec::new(),
        crate::updates::PendingSlot::default(),
        None,
    );
    app.sound = crate::sound::Sound::with_opener(
        || Err("there's no sound card".to_owned()),
        crate::sound::RETRY,
    );
    assert!(app.workbench.file_page_open(), "on Recent projects");
    app.splashing = false;
    frame(&ctx, &mut app, Vec::new());
    frame(&ctx, &mut app, Vec::new());
    assert!(app.workbench.file_page_open(), "still on Recent projects");
    assert_eq!(
        ctx.memory(egui::Memory::focused),
        None,
        "Search projects doesn't have the keys"
    );
    let esc = egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&ctx, &mut app, vec![esc]);
    assert_eq!(app.workbench.ribbon_tab(), editor::RibbonTab::Audio);
}

#[test]
fn graphite_rounded_panels_and_plum_become_the_one_theme_and_purple() {
    let storage = settings("graphite-rounded-plum.ron");
    let (saved, workbench) = started(&storage);
    // Any theme becomes Neutral dark, rounded panels mean nothing now,
    // and Plum is mapped to the nearest of the five.
    assert_eq!(
        saved.look,
        InterfaceLook {
            accent: Accent::Purple,
            ..InterfaceLook::default()
        }
    );
    assert_eq!(workbench.accent(), Accent::Purple);
    assert_eq!(workbench.interface_look().scale, Scale::default());
    assert_eq!(workbench.interface_look().density, Density::Default);
    // Everything else that was changed is still changed.
    assert_eq!(saved.editor_size, [1600.0, 1000.0]);
    assert_eq!(saved.skipped, Some(79));
    assert!(!saved.auto_update);
    assert!(!workbench.auto_update());
    let known = saved.known.saved();
    assert_eq!(known.len(), 2);
    assert!(known.iter().any(|k| k.pinned), "the pin");
    assert!(!workbench.hints_on());
    assert_eq!(
        workbench.camera().position,
        glam::Vec3::new(12.0, -7.5, 3.25)
    );
    assert_eq!(workbench.look().kind, editor::LookKind::Stylized);
    assert_eq!(workbench.look().bands, 4);
    assert_eq!(workbench.snapping().grid_step, 0.25);
    assert_eq!(workbench.snapping().angle_step, 45.0);
    assert_ne!(workbench.overlays(), Overlays::default());
    assert_eq!(workbench.console_history(), ["help", "time 18"]);
    // The rearranged panels (the Animate workspace in use) aren't carried
    // over: the frame starts as the design draws it, and says so once.
    // (The camera and look above are read through the saved views, which
    // win over the single camera and look of older settings.)
    starts_with_the_standard_frame_and_says_so_once(&workbench);
    assert!(storage.get_string(super::super::VIEWS_KEY).is_some());
}

#[test]
fn settings_from_before_the_second_accent_key_start_too() {
    let storage = settings("before-accent2.ron");
    assert!(storage.get_string(ACCENT_KEY).is_none());
    assert!(storage.get_string(OLD_ACCENT_KEY).is_some());
    let (saved, workbench) = started(&storage);
    // Amber under the oldest key was the standard of its day: nobody
    // chose it, so it becomes today's standard.
    assert_eq!(saved.look, InterfaceLook::default());
    assert_eq!(workbench.accent(), Accent::Blue);
    // The recent projects of that time become the known projects.
    assert_eq!(saved.known.saved().len(), 2);
    assert_eq!(saved.editor_size, FIRST_SIZE, "never saved then");
    assert!(saved.auto_update, "never saved then");
    assert_eq!(
        workbench.camera().position,
        glam::Vec3::new(12.0, -7.5, 3.25)
    );
    assert_eq!(workbench.console_history(), ["help"]);
    assert!(workbench.hints_on());
    starts_with_the_standard_frame_and_says_so_once(&workbench);
}

#[test]
fn the_old_keys_are_read_once_and_then_left_exactly_as_they_were() {
    for name in [
        "standard-blue.ron",
        "graphite-rounded-plum.ron",
        "before-accent2.ron",
    ] {
        let mut storage = settings(name);
        let before: HashMap<String, String> = storage.0.clone();
        let (saved, workbench) = started(&storage);
        assert_eq!(storage.0, before, "{name}: reading changed the settings");
        // The new build saves its look and its frame under keys of their
        // own...
        eframe::set_value(&mut storage, INTERFACE_LOOK_KEY, &saved.look);
        eframe::set_value(&mut storage, SHELL_KEY, &workbench.shell_state());
        for (key, value) in &before {
            assert_eq!(storage.0.get(key), Some(value), "{name}: {key} was touched");
        }
        // ...and from then on the old accent is not looked at again: a
        // choice made in the new build is not overruled by it.
        let chosen = InterfaceLook {
            accent: Accent::Grey,
            scale: Scale::from(125),
            density: Density::Compact,
            ..InterfaceLook::default()
        };
        eframe::set_value(&mut storage, INTERFACE_LOOK_KEY, &chosen);
        assert_eq!(Saved::read(Some(&storage)).look, chosen, "{name}");
        // Nor is the old arrangement: the frame saved is taken in, and
        // the Console has nothing more to say about it.
        let (_, again) = started(&storage);
        assert!(
            !again
                .log_lines()
                .iter()
                .any(|l| l.text.contains("wasn't carried over")),
            "{name}: said once only"
        );
    }
}

#[test]
fn every_earlier_accent_under_either_key_is_mapped() {
    for (key, saved, accent) in [
        (ACCENT_KEY, "Blue", Accent::Blue),
        (ACCENT_KEY, "PetrolBlue", Accent::Teal),
        (ACCENT_KEY, "Green", Accent::Teal),
        (ACCENT_KEY, "Olive", Accent::Teal),
        (ACCENT_KEY, "Red", Accent::Orange),
        (ACCENT_KEY, "Copper", Accent::Orange),
        (ACCENT_KEY, "Orange", Accent::Orange),
        (ACCENT_KEY, "Amber", Accent::Orange),
        (ACCENT_KEY, "Gold", Accent::Orange),
        (ACCENT_KEY, "Plum", Accent::Purple),
        (OLD_ACCENT_KEY, "Amber", Accent::Blue),
        (OLD_ACCENT_KEY, "Red", Accent::Orange),
        (OLD_ACCENT_KEY, "Green", Accent::Teal),
        (OLD_ACCENT_KEY, "Plum", Accent::Purple),
    ] {
        let mut storage = Memory::default();
        storage.set_string(key, saved.to_string());
        assert_eq!(
            Saved::read(Some(&storage)).look.accent,
            accent,
            "{key}: {saved}"
        );
    }
    // The newer of the two keys wins.
    let mut storage = Memory::default();
    storage.set_string(OLD_ACCENT_KEY, "Red".to_string());
    storage.set_string(ACCENT_KEY, "Plum".to_string());
    assert_eq!(Saved::read(Some(&storage)).look.accent, Accent::Purple);
}

#[test]
fn no_settings_and_damaged_settings_both_start_as_standard() {
    let fresh = Saved::read(None);
    assert_eq!(fresh.look, InterfaceLook::default());
    assert_eq!(fresh.editor_size, FIRST_SIZE);
    assert!(fresh.auto_update);
    // Every key holding nonsense: nothing is an error, everything is its
    // standard value.
    let mut storage = Memory::default();
    for key in [
        INTERFACE_LOOK_KEY,
        ACCENT_KEY,
        OLD_ACCENT_KEY,
        AUTO_UPDATE_KEY,
        EDITOR_SIZE_KEY,
        super::super::OLD_LAYOUT_KEYS[0],
        super::super::OLD_LAYOUT_KEYS[1],
        super::super::OLD_LAYOUT_KEYS[2],
        SHELL_KEY,
        super::super::CAMERA_KEY,
        super::super::KNOWN_PROJECTS_KEY,
        super::super::VIEWS_KEY,
    ] {
        storage.set_string(key, "(((not a value".to_string());
    }
    let (saved, workbench) = started(&storage);
    assert_eq!(saved.look, InterfaceLook::default());
    assert_eq!(saved.editor_size, FIRST_SIZE);
    assert!(saved.known.saved().is_empty());
    assert_eq!(workbench.camera(), Camera::default());
    assert_eq!(workbench.accent(), Accent::Blue);
}

/// A small generator of numbers that is the same on every run.
struct Dice(u32);

impl Dice {
    fn roll(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

#[test]
fn damaged_looks_and_accents_never_stop_the_editor_starting() {
    let good = {
        let mut storage = Memory::default();
        let look = InterfaceLook {
            accent: Accent::Teal,
            scale: Scale::from(175),
            density: Density::Comfortable,
            ribbon_labels: false,
            hide_unfinished: true,
            shelf_opens_on: editor::interface_look::ShelfOpensOn::Entity,
            layout: editor::interface_look::PanelLayout::AreaAndAssetsLeft,
        };
        eframe::set_value(&mut storage, INTERFACE_LOOK_KEY, &look);
        storage.get_string(INTERFACE_LOOK_KEY).unwrap()
    };
    let mut dice = Dice(0x9e37_79b9);
    let mut kept = 0;
    for round in 0..4000 {
        let mut bytes = good.clone().into_bytes();
        for _ in 0..1 + dice.roll() % 3 {
            if bytes.is_empty() {
                break;
            }
            let at = dice.roll() as usize % bytes.len();
            match dice.roll() % 5 {
                0 => bytes[at] = (dice.roll() >> 8) as u8,
                1 => bytes.truncate(at),
                2 => bytes.insert(at, b"(,:9"[dice.roll() as usize % 4]),
                // Gentler damage, which often still reads: another digit,
                // a switch flipped.
                3 => {
                    if let Some(digit) = bytes.iter_mut().find(|b| b.is_ascii_digit()) {
                        *digit = b'0' + (dice.roll() % 10) as u8;
                    }
                }
                _ => {
                    let text = String::from_utf8_lossy(&bytes).replace("true", "false");
                    bytes = text.into_bytes();
                }
            }
        }
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let mut storage = Memory::default();
        // The same damage under the look's key or an old accent's.
        let key = [INTERFACE_LOOK_KEY, ACCENT_KEY, OLD_ACCENT_KEY][round % 3];
        storage.set_string(key, text);
        let look = Saved::read(Some(&storage)).look;
        assert!(Scale::STEPS.contains(&look.scale.percent()));
        assert!(Accent::ALL.contains(&look.accent));
        kept += usize::from(look != InterfaceLook::default());
    }
    assert!(kept > 50, "only {kept} damaged looks still read");
    assert!(kept < 4000, "no damage was refused");
}

/// The program the user guide opens in (decision 0112): saved under its
/// own key, the default when it's missing (every file from before it),
/// and the default again when what's saved is damaged or couldn't be a
/// program on this computer (rule 9).
#[test]
fn the_guide_s_browser_is_kept_and_a_damaged_one_is_the_default() {
    let program = if cfg!(windows) {
        std::path::PathBuf::from("C:\\Browsers\\example.exe")
    } else {
        std::path::PathBuf::from("/usr/bin/example")
    };
    let chosen = editor::GuideBrowser::Installed {
        name: "Example Browser".into(),
        program,
    };
    assert_eq!(
        Saved::read(None).guide_browser,
        editor::GuideBrowser::Default
    );
    let mut storage = Memory::default();
    eframe::set_value(&mut storage, GUIDE_BROWSER_KEY, &chosen);
    let (saved, workbench) = started(&storage);
    assert_eq!(saved.guide_browser, chosen);
    assert_eq!(workbench.guide_browser(), &chosen);
    let good = storage.get_string(GUIDE_BROWSER_KEY).unwrap();
    let mut dice = Dice(0x5151_2026);
    let mut kept = 0;
    for _ in 0..3000 {
        let mut bytes = good.clone().into_bytes();
        for _ in 0..1 + dice.roll() % 3 {
            if bytes.is_empty() {
                break;
            }
            let at = dice.roll() as usize % bytes.len();
            match dice.roll() % 3 {
                0 => bytes[at] = (dice.roll() >> 8) as u8,
                1 => bytes.truncate(at),
                _ => bytes.insert(at, b"(\\\"/:."[dice.roll() as usize % 6]),
            }
        }
        let mut storage = Memory::default();
        storage.set_string(
            GUIDE_BROWSER_KEY,
            String::from_utf8_lossy(&bytes).into_owned(),
        );
        let read = Saved::read(Some(&storage)).guide_browser;
        assert_eq!(crate::user_guide::checked(read.clone()), read);
        kept += usize::from(read != editor::GuideBrowser::Default);
    }
    assert!(kept > 50, "only {kept} damaged choices still read");
}
