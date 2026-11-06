//! The playtest export (decision 0093).

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("solis-playtest-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The UI sample made in `dir/project`, and a stand-in for the program.
fn project(dir: &Path) -> (PathBuf, PathBuf) {
    let root = dir.join("project");
    crate::ui_sample::make(&root).unwrap();
    // What the editor leaves in a project that an export doesn't take.
    for left in ["saves", "recordings", ".solis/backups", "builds"] {
        std::fs::create_dir_all(root.join(left)).unwrap();
        std::fs::write(root.join(left).join("x.txt"), "the editor's").unwrap();
    }
    let program = dir.join("solis-program");
    std::fs::write(&program, b"the program").unwrap();
    (root, program)
}

fn farm() -> Playtest {
    Playtest {
        name: "Turret: test".into(),
        area: "ui.solarea".into(),
        fullscreen: false,
        size: [1600, 900],
        folder: None,
    }
}

#[test]
fn an_export_holds_the_program_renamed_the_game_and_what_to_play() {
    let dir = scratch("export");
    let (root, program) = project(&dir);
    let folder = dir.join("out").join("Turret playtest");
    let made = export(&root, &program, &farm(), &folder).unwrap();
    assert!(made.files > 5, "{made:?}");
    let exe = folder.join(program_name("Turret test"));
    assert_eq!(std::fs::read(&exe).unwrap(), b"the program");
    assert!(folder.join("game/project.solis").is_file());
    assert!(folder.join("game/assets/main.lst").is_file());
    assert!(folder.join("game/assets/ui/hud.lst").is_file());
    for left in ["saves", "recordings", ".solis", "builds"] {
        assert!(!folder.join("game").join(left).exists(), "{left} left out");
    }
    // The program, started from there, finds the game.
    let found = beside(&exe).unwrap().unwrap();
    assert_eq!(found.root, folder.join("game"));
    assert_eq!(found.playtest.name, "Turret test");
    assert_eq!(found.playtest.area, "ui.solarea");
    assert_eq!(found.playtest.folder, None, "where it went isn't shipped");
    // And the editor itself, anywhere else, doesn't.
    assert!(beside(&program).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_export_never_mixes_with_or_replaces_anything() {
    let dir = scratch("refused");
    let (root, program) = project(&dir);
    let full = dir.join("full");
    std::fs::create_dir_all(&full).unwrap();
    std::fs::write(full.join("mine.txt"), "keep").unwrap();
    let why = export(&root, &program, &farm(), &full).unwrap_err();
    assert!(why.contains("already has files"), "{why}");
    assert_eq!(std::fs::read_to_string(full.join("mine.txt")).unwrap(), "keep");
    // An empty folder is fine.
    let empty = dir.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    assert!(export(&root, &program, &farm(), &empty).is_ok());
    // Inside the project, it would copy itself.
    let why = export(&root, &program, &farm(), &root.join("builds/new")).unwrap_err();
    assert!(why.contains("inside the project"), "{why}");
    // An area that isn't there, or one outside the assets.
    let gone = Playtest {
        area: "gone.solarea".into(),
        ..farm()
    };
    assert!(export(&root, &program, &gone, &dir.join("a")).is_err());
    let out = Playtest {
        area: "../project.solis".into(),
        ..farm()
    };
    assert!(export(&root, &program, &out, &dir.join("b")).is_err());
    assert!(export(&root, &program, &farm(), Path::new("relative/folder")).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn links_in_the_project_are_left_out() {
    let dir = scratch("links");
    let (root, program) = project(&dir);
    let secret = dir.join("secret.txt");
    std::fs::write(&secret, "not the game's").unwrap();
    std::os::unix::fs::symlink(&secret, root.join("assets/secret.txt")).unwrap();
    let folder = dir.join("out");
    export(&root, &program, &farm(), &folder).unwrap();
    assert!(!folder.join("game/assets/secret.txt").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_damaged_playtest_file_is_explained_not_ignored() {
    let dir = scratch("damaged");
    std::fs::write(dir.join(PLAYTEST_FILE), "nonsense").unwrap();
    let why = beside(&dir.join("Farm.exe")).unwrap().unwrap_err();
    assert!(why.contains("damaged"), "{why}");
    std::fs::write(dir.join(PLAYTEST_FILE), farm().text()).unwrap();
    let why = beside(&dir.join("Farm.exe")).unwrap().unwrap_err();
    assert!(why.contains("game folder"), "{why}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_player_s_settings_read_back_and_start_from_the_export_s_window() {
    let dir = scratch("settings");
    let start = load_settings(Some(&dir), &farm());
    assert_eq!(start.resolution, [1600, 900], "the export's window");
    assert!(!start.fullscreen);
    let changed = start
        .with(Setting::TextSize(1.5))
        .with(Setting::Fullscreen(true))
        .with(Setting::Resolution(1920, 1080));
    save_settings(&dir, &changed).unwrap();
    assert_eq!(load_settings(Some(&dir), &farm()), changed);
    assert_eq!(load_settings(None, &farm()), start, "no folder: as exported");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn settings_files_survive_being_damaged() {
    let text = settings_text(
        &PlayerSettings::default()
            .with(Setting::TextSize(1.25))
            .with(Setting::Resolution(2560, 1440)),
    );
    let bytes = text.as_bytes();
    let mut seed = 0xBEEF_CAFEu64;
    for _ in 0..3_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut broken = bytes.to_vec();
        broken.truncate(1 + (seed as usize) % bytes.len());
        let at = (seed >> 20) as usize % broken.len();
        broken[at] = b" \n.-e0123456789NaNinf"[(seed >> 40) as usize % 21];
        let s = parse_settings(&String::from_utf8_lossy(&broken), PlayerSettings::default());
        assert_eq!(s, s.sanitized(), "always within range");
        assert!(s.text_size.is_finite() && s.ui_size.is_finite());
    }
    let wild = parse_settings(
        "text size NaN\nui size 1e300\nresolution 0 99999999\n",
        PlayerSettings::default(),
    );
    assert_eq!(wild, wild.sanitized());
}

#[test]
fn the_crash_log_says_what_happened_and_what_was_said() {
    let dir = scratch("crash");
    let said: Vec<String> = (0..100).map(|i| format!("line {i}")).collect();
    let path = write_crash_log(&dir.join("Farm"), "Farm stopped unexpectedly.", &said).unwrap();
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.starts_with("Farm stopped unexpectedly."));
    assert!(text.contains("When: "));
    assert!(text.contains("line 99") && !text.contains("line 10\n"), "the last 40");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_project_remembers_how_it_was_exported() {
    let dir = scratch("remember");
    assert_eq!(last_export(&dir), None);
    let p = Playtest {
        folder: Some(dir.join("out")),
        ..farm()
    };
    remember_export(&dir, &p).unwrap();
    assert_eq!(last_export(&dir), Some(Playtest { name: "Turret test".into(), ..p }));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn saves_go_in_the_player_s_own_folder_named_after_the_game() {
    let folder = saved_games("Farm: 2").unwrap();
    assert!(folder.ends_with("Farm 2"), "{}", folder.display());
    assert!(folder.is_absolute());
    assert!(program_name("Farm").starts_with("Farm"));
}
