//! Tidying projects from the launcher ([decision
//! 0086](../../../docs/decisions/0086-the-launcher.md)): renaming one (its
//! name in the project file, never its folder), making a copy of its
//! folder, and finding projects already on the disk. Nothing here deletes
//! anything, follows a link, or writes outside the folder it was given.

use crate::project::{CACHE_FOLDER, PROJECT_FILE, Project, ProjectError, write_atomically};
use std::path::{Path, PathBuf};

/// The longest project name.
pub const MAX_NAME: usize = 100;
/// How many folders deep a scan looks below the folder chosen.
pub const SCAN_DEPTH: usize = 4;
/// The most folders a scan looks in, so a whole drive can't keep it busy.
pub const SCAN_FOLDERS: usize = 20_000;

/// `name` tidied for a project's name, or why it can't be one.
pub fn project_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("a project needs a name".into());
    }
    if name.chars().count() > MAX_NAME {
        return Err(format!("names are at most {MAX_NAME} letters"));
    }
    if name.chars().any(char::is_control) {
        return Err("a name can't hold line breaks or other control characters".into());
    }
    Ok(name.to_string())
}

/// Gives the project in `root` the name `name` in its project file,
/// keeping everything else the file says. Its folder keeps its name.
pub fn rename(root: &Path, name: &str) -> Result<String, ProjectError> {
    let name = project_name(name).map_err(ProjectError::Damaged)?;
    let path = root.join(PROJECT_FILE);
    let text = crate::project::read_text_limited(&path, 64 * 1024)?;
    // Refused as opening refuses: damaged, or from a newer Solis.
    Project::read_summary(&text)?;
    let mut table: toml::Table =
        toml::from_str(&text).map_err(|e| ProjectError::Damaged(e.to_string()))?;
    table.insert("name".into(), toml::Value::String(name.clone()));
    let text = toml::to_string(&table).map_err(|e| ProjectError::Damaged(e.to_string()))?;
    write_atomically(&path, text.as_bytes())?;
    Ok(name)
}

/// What a copy did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copied {
    /// Files copied.
    pub files: u64,
    /// Bytes copied.
    pub bytes: u64,
    /// Links left out (a project's links could point anywhere).
    pub links_skipped: u64,
}

/// Copies the project in `from` into `to`, an empty or new folder, and
/// names the copy `name`. Everything is copied but the import cache
/// (`.solis/cache`, made again when the copy opens) and links. A folder
/// inside `from` can't be the copy's place.
pub fn copy_project(from: &Path, to: &Path, name: &str) -> Result<Copied, String> {
    Project::summary(from).map_err(|e| e.to_string())?;
    let name = project_name(name)?;
    let from_real = from.canonicalize().map_err(|e| e.to_string())?;
    if let Ok(meta) = std::fs::symlink_metadata(to) {
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(format!("{} isn't a folder", to.display()));
        }
        if std::fs::read_dir(to)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
        {
            return Err(format!(
                "{} isn't empty: choose an empty folder for the copy",
                to.display()
            ));
        }
    }
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    let to_real = to.canonicalize().map_err(|e| e.to_string())?;
    if to_real.starts_with(&from_real) {
        return Err("the copy can't go inside the project it copies".into());
    }
    let mut copied = Copied {
        files: 0,
        bytes: 0,
        links_skipped: 0,
    };
    copy_folder(&from_real, &to_real, &from_real, &mut copied).map_err(|e| e.to_string())?;
    rename(&to_real, &name).map_err(|e| e.to_string())?;
    Ok(copied)
}

fn copy_folder(from: &Path, to: &Path, top: &Path, copied: &mut Copied) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            copied.links_skipped += 1;
            continue;
        }
        if kind.is_dir() {
            if path == top.join(CACHE_FOLDER).join("cache") {
                continue;
            }
            copy_folder(&path, &to.join(entry.file_name()), top, copied)?;
        } else if kind.is_file() {
            copied.bytes += std::fs::copy(&path, to.join(entry.file_name()))?;
            copied.files += 1;
        }
    }
    Ok(())
}

/// Every project in `folder` or the folders inside it, up to
/// [`SCAN_DEPTH`] deep, never inside a project (its assets aren't
/// projects), hidden folders, or links. Stops after [`SCAN_FOLDERS`]
/// folders. Returns the projects found and whether it stopped early.
pub fn find_projects(folder: &Path) -> (Vec<PathBuf>, bool) {
    let mut found = Vec::new();
    let mut looked = 0;
    let mut waiting = vec![(folder.to_path_buf(), 0)];
    while let Some((at, depth)) = waiting.pop() {
        looked += 1;
        if looked > SCAN_FOLDERS {
            found.sort();
            return (found, true);
        }
        if at.join(PROJECT_FILE).is_file() {
            found.push(at);
            continue;
        }
        if depth >= SCAN_DEPTH {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let hidden = entry.file_name().to_string_lossy().starts_with('.');
            if kind.is_dir() && !kind.is_symlink() && !hidden {
                waiting.push((entry.path(), depth + 1));
            }
        }
    }
    found.sort();
    (found, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_folder;

    #[test]
    fn names_are_tidied_or_refused() {
        assert_eq!(project_name("  Farm days ").unwrap(), "Farm days");
        assert!(project_name("   ").is_err());
        assert!(project_name("a\nb").is_err());
        assert!(project_name(&"x".repeat(MAX_NAME + 1)).is_err());
        assert!(
            project_name(&"é".repeat(MAX_NAME)).is_ok(),
            "letters, not bytes"
        );
    }

    #[test]
    fn renaming_changes_the_name_and_keeps_the_rest() {
        let root = test_folder("tidy-rename");
        let project = Project::create(&root, "Farm").unwrap();
        project.record_build(57).unwrap();
        assert_eq!(rename(&root, " Farm days ").unwrap(), "Farm days");
        let summary = Project::summary(&root).unwrap();
        assert_eq!(summary.name, "Farm days");
        assert_eq!(summary.saved_by, Some(57));
        assert!(root.is_dir(), "the folder keeps its name");
        assert!(rename(&root, "").is_err());
        assert!(rename(&test_folder("tidy-rename-none"), "X").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_copy_leaves_out_the_cache_and_links_and_is_named() {
        let root = test_folder("tidy-copy-from");
        Project::create(&root, "Farm").unwrap();
        std::fs::write(root.join("assets/main.lst"), "end\n").unwrap();
        std::fs::create_dir_all(root.join("assets/art")).unwrap();
        std::fs::write(root.join("assets/art/a.txt"), "art").unwrap();
        std::fs::create_dir_all(root.join(".solis/cache")).unwrap();
        std::fs::write(root.join(".solis/cache/big.solbin"), "cache").unwrap();
        std::fs::create_dir_all(root.join(".solis/backups")).unwrap();
        std::fs::write(root.join(".solis/backups/old.lst"), "kept").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc", root.join("assets/outside")).unwrap();
        let to = test_folder("tidy-copy-to");
        let copied = copy_project(&root, &to, "Farm copy").unwrap();
        assert_eq!(Project::summary(&to).unwrap().name, "Farm copy");
        assert_eq!(Project::summary(&root).unwrap().name, "Farm", "untouched");
        assert_eq!(
            std::fs::read_to_string(to.join("assets/art/a.txt")).unwrap(),
            "art"
        );
        assert!(to.join(".solis/backups/old.lst").is_file());
        assert!(!to.join(".solis/cache").exists());
        assert!(!to.join("assets/outside").exists());
        assert!(copied.files >= 4);
        #[cfg(unix)]
        assert_eq!(copied.links_skipped, 1);
        // Never over something, never inside itself, never from a non-project.
        let err = copy_project(&root, &to, "Again").unwrap_err();
        assert!(err.contains("isn't empty"), "{err}");
        let err = copy_project(&root, &root.join("assets/inside"), "In").unwrap_err();
        assert!(err.contains("inside"), "{err}");
        assert!(copy_project(&to.join("assets"), &test_folder("tidy-copy-3"), "X").is_err());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&to);
    }

    #[test]
    fn a_scan_finds_projects_but_not_inside_them_or_hidden_ones() {
        let top = test_folder("tidy-scan");
        for at in ["a", "games/b", "games/deep/1/2/3/too-deep", ".hidden/c"] {
            Project::create(&top.join(at), "P").unwrap();
        }
        // A project inside a project's folder isn't looked for.
        Project::create(&top.join("a/assets/inner"), "Inner").unwrap();
        let (found, stopped) = find_projects(&top);
        assert!(!stopped);
        assert_eq!(found, vec![top.join("a"), top.join("games/b")]);
        let (found, _) = find_projects(&top.join("a"));
        assert_eq!(found, vec![top.join("a")], "the folder chosen can be one");
        assert_eq!(find_projects(&top.join("none")), (vec![], false));
        let _ = std::fs::remove_dir_all(&top);
    }
}
