//! Every version of every file format opens (Phase 8, C1; decision 0053).
//!
//! `tests/fixtures/<format>/v<N>/` holds files as each released version
//! wrote them. Each is read with the current build, which upgrades it in
//! memory; a build that can't open one can't be merged. A format whose
//! version goes up without an example of its old version fails here too.

use reflect::upgrade::Versioned;
use std::path::{Path, PathBuf};

/// The fixtures' folder.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

/// A format's folder, how it's versioned, and how a file of it is read
/// with the current build (an error says why it couldn't be).
type Reader = fn(&str) -> Result<(), String>;

fn every_format() -> Vec<(&'static str, &'static dyn Versioned, Reader)> {
    vec![
        ("area", &area::text::FORMAT, |text| {
            let registry = area::builtin::registry().map_err(|e| e.to_string())?;
            area::text::read_file(text, &registry)
                .map(drop)
                .map_err(|e| e.to_string())
        }),
        ("sound-event", &sound::event::FORMAT, |text| {
            let read = sound::event::read(text).map_err(|e| e.to_string())?;
            if read.notes.is_empty() {
                Ok(())
            } else {
                Err(format!("notes: {:?}", read.notes))
            }
        }),
        ("mixer", &sound::mixer::FORMAT, |text| {
            let read = sound::mixer::read(text).map_err(|e| e.to_string())?;
            if read.notes.is_empty() {
                Ok(())
            } else {
                Err(format!("notes: {:?}", read.notes))
            }
        }),
        ("input-actions", &input::BINDINGS_FORMAT, |text| {
            input::Bindings::read(text)
                .map(drop)
                .map_err(|e| e.to_string())
        }),
        ("recording", &input::RECORDING_FORMAT, |text| {
            input::Recording::read(text)
                .map(drop)
                .map_err(|e| e.to_string())
        }),
        ("project", &asset_db::PROJECT_FORMAT, |text| {
            asset_db::Project::read_file(text)
                .map(drop)
                .map_err(|e| e.to_string())
        }),
        ("sidecar", &asset_db::SIDECAR_FORMAT, |text| {
            asset_db::Sidecar::parse(text).map(drop)
        }),
    ]
}

/// The files in a folder.
fn files_in(folder: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
        .unwrap_or_else(|e| panic!("{}: {e}", folder.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    files
}

#[test]
fn every_version_of_every_format_opens() {
    let formats = every_format();
    assert_eq!(
        formats.len(),
        crate::upgrades::tests::formats().len(),
        "every format has its examples"
    );
    for (folder, format, read) in formats {
        for version in 1..=format.current() {
            let dir = fixtures().join(folder).join(format!("v{version}"));
            let files = files_in(&dir);
            assert!(
                !files.is_empty(),
                "{}: no example of version {version} (a format's every version keeps one)",
                dir.display()
            );
            for file in files {
                let text = std::fs::read_to_string(&file).unwrap();
                assert_eq!(
                    format.version_of(&text),
                    Some(version),
                    "{} says another version",
                    file.display()
                );
                read(&text).unwrap_or_else(|why| panic!("{}: {why}", file.display()));
                // Older ones are noted in the Console; current ones aren't.
                let note = crate::upgrades::older_note("it", &text, format);
                assert_eq!(
                    note.is_some(),
                    version < format.current(),
                    "{}",
                    file.display()
                );
            }
        }
        // Nothing newer than this build lies around unread.
        let newer = fixtures()
            .join(folder)
            .join(format!("v{}", format.current() + 1));
        assert!(
            !newer.exists(),
            "{} is newer than this build",
            newer.display()
        );
    }
}

/// A project folder in the temporary folder, made fresh.
fn project(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "solis-fixtures-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn an_old_input_actions_file_is_upgraded_and_kept_when_saved() {
    // The gate's case: a project's input actions from before version 2.
    let root = project("input");
    let old = std::fs::read(fixtures().join("input-actions/v1/input.solinput")).unwrap();
    std::fs::write(root.join(input::FILE), &old).unwrap();
    // Opened: upgraded in memory, with a note, and the file untouched.
    let (bindings, note) = crate::play_files::load_bindings(&root);
    let note = note.expect("noted");
    assert!(note.contains("older version (format 1)"), "{note}");
    assert_eq!(bindings.get("Pan").unwrap().up, ["Up"]);
    assert_eq!(std::fs::read(root.join(input::FILE)).unwrap(), old);
    // Saved: a copy of the old one first, then the new format.
    let kept = crate::play_files::save_bindings(&root, &bindings.write()).unwrap();
    let Some(crate::play_files::Kept::Older(copy)) = kept else {
        panic!("not kept: {kept:?}");
    };
    assert_eq!(std::fs::read(&copy).unwrap(), old);
    assert!(
        copy.starts_with(asset_db::backup::folder(&root)),
        "{copy:?}"
    );
    assert!(copy.ends_with(input::FILE));
    let now = std::fs::read_to_string(root.join(input::FILE)).unwrap();
    assert_eq!(input::BINDINGS_FORMAT.version_of(&now), Some(2));
    assert_eq!(input::Bindings::read(&now).unwrap(), bindings);
    let note = crate::upgrades::kept_note(&root, input::FILE, &copy);
    assert!(note.contains(".solis"), "{note}");
    // Saved again: current now, so no second copy.
    assert_eq!(
        crate::play_files::save_bindings(&root, &bindings.write()).unwrap(),
        None
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_upgrade_that_would_lose_something_leaves_the_old_file() {
    let root = project("refused");
    let old = std::fs::read(fixtures().join("input-actions/v1/input.solinput")).unwrap();
    let path = root.join(input::FILE);
    std::fs::write(&path, &old).unwrap();
    // A save whose text doesn't read back as itself (as if an upgrader
    // dropped something the writer can't write) is refused.
    let refused = crate::upgrades::save(
        &root,
        &path,
        "version = 2\n",
        &input::BINDINGS_FORMAT,
        |_| Ok("version = 2\n[[action]]\nname = \"Lost\"\n".into()),
    );
    assert!(refused.unwrap_err().contains("read back"));
    assert_eq!(std::fs::read(&path).unwrap(), old);
    assert!(!asset_db::backup::folder(&root).exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_older_save_in_a_slot_is_kept_before_it_s_replaced() {
    // The saves folder isn't in Git, so an older save is copied first.
    let root = project("save");
    let old = std::fs::read(fixtures().join("save/v2/harbour.solsave")).unwrap();
    let slot = crate::play_files::save_path(&root, "quick").unwrap();
    std::fs::create_dir_all(slot.parent().unwrap()).unwrap();
    std::fs::write(&slot, &old).unwrap();
    let current = std::fs::read_to_string(fixtures().join("save/v3/harbour.solsave")).unwrap();
    let (path, copy) = crate::play_files::save_game(&root, "quick", &current).unwrap();
    let copy = copy.expect("kept");
    assert_eq!(std::fs::read(&copy).unwrap(), old);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), current);
    // Now current: replaced without another copy.
    let (_, again) = crate::play_files::save_game(&root, "quick", &current).unwrap();
    assert_eq!(again, None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_newer_save_in_a_slot_is_kept_before_it_s_replaced() {
    // Regression (the Phase 8 review): a save from a newer Solis (tried,
    // then gone back from) was overwritten with no copy.
    let root = project("newer-save");
    let current = std::fs::read_to_string(fixtures().join("save/v3/harbour.solsave")).unwrap();
    let newer = current.replacen("solis save 3", "solis save 9", 1);
    let slot = crate::play_files::save_path(&root, "quick").unwrap();
    std::fs::create_dir_all(slot.parent().unwrap()).unwrap();
    std::fs::write(&slot, &newer).unwrap();
    let (_, copy) = crate::play_files::save_game(&root, "quick", &current).unwrap();
    assert_eq!(std::fs::read_to_string(copy.expect("kept")).unwrap(), newer);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn input_actions_from_a_newer_solis_are_never_overwritten() {
    let root = project("newer-input");
    let newer = "version = 9\n[[action]]\nname = \"Jump\"\n";
    std::fs::write(root.join(input::FILE), newer).unwrap();
    let refused = crate::play_files::save_bindings(&root, &input::Bindings::default().write());
    assert!(refused.unwrap_err().to_string().contains("newer version"));
    assert_eq!(
        std::fs::read_to_string(root.join(input::FILE)).unwrap(),
        newer
    );
    let _ = std::fs::remove_dir_all(&root);
}
