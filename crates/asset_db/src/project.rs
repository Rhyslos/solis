//! The project folder ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)):
//! `project.solis` naming it, `assets/` holding everything the user owns,
//! and `.solis/`, a cache that can be deleted at any time.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// The file that marks a folder as a project.
pub const PROJECT_FILE: &str = "project.solis";
/// The folder of the user's assets, watched for changes.
pub const ASSETS_FOLDER: &str = "assets";
/// The rebuildable cache, never committed.
pub const CACHE_FOLDER: &str = ".solis";
/// The project file's format version this build writes and reads.
pub const FORMAT: u32 = 1;

/// New projects ignore the cache and builds in Git.
pub const GITIGNORE: &str = "\
# Solis's cache: rebuilt whenever it's missing. Never commit it.
/.solis/
# Exported games.
/builds/
";

/// New projects keep Solis's text files with Unix line endings, so Git
/// merges them line by line on every computer, and store big binary files
/// (models, images, sound) with Git LFS (decision 0006).
pub const GITATTRIBUTES: &str = "\
# Solis's own text files: the same line endings everywhere, so they merge.
*.solis text eol=lf
*.solarea text eol=lf
*.solassembly text eol=lf
*.solmat text eol=lf
*.solmeta text eol=lf
*.lust text eol=lf
*.lua text eol=lf
# Big files go through Git LFS.
*.glb filter=lfs diff=lfs merge=lfs -text
*.gltf text eol=lf
*.bin filter=lfs diff=lfs merge=lfs -text
*.png filter=lfs diff=lfs merge=lfs -text
*.jpg filter=lfs diff=lfs merge=lfs -text
*.jpeg filter=lfs diff=lfs merge=lfs -text
*.ktx2 filter=lfs diff=lfs merge=lfs -text
*.wav filter=lfs diff=lfs merge=lfs -text
*.ogg filter=lfs diff=lfs merge=lfs -text
*.mp3 filter=lfs diff=lfs merge=lfs -text
*.flac filter=lfs diff=lfs merge=lfs -text
";

/// What `project.solis` holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ProjectFile {
    /// The project's name, shown in the title bar.
    name: String,
    /// The format version.
    format: u32,
}

/// An open project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    root: PathBuf,
    name: String,
}

/// Why a project couldn't be made or opened, in plain words.
#[derive(Debug)]
pub enum ProjectError {
    /// The folder has no `project.solis`.
    NotAProject(PathBuf),
    /// The folder already is a project.
    AlreadyAProject(PathBuf),
    /// `project.solis` couldn't be read.
    Damaged(String),
    /// Made by a newer Solis.
    TooNew(u32),
    /// The disk refused.
    Io(std::io::Error),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAProject(p) => write!(f, "{} has no {PROJECT_FILE}", p.display()),
            Self::AlreadyAProject(p) => write!(f, "{} is already a project", p.display()),
            Self::Damaged(why) => write!(f, "{PROJECT_FILE} couldn't be read ({why})"),
            Self::TooNew(v) => write!(
                f,
                "it was made by a newer version (format {v}); update to open it"
            ),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ProjectError {}

impl From<std::io::Error> for ProjectError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl Project {
    /// Makes `root` a new project called `name`: its project file, the
    /// assets and cache folders, and Git's `.gitignore` and
    /// `.gitattributes` (kept if the folder already has them).
    pub fn create(root: &Path, name: &str) -> Result<Self, ProjectError> {
        if root.join(PROJECT_FILE).exists() {
            return Err(ProjectError::AlreadyAProject(root.to_path_buf()));
        }
        std::fs::create_dir_all(root.join(ASSETS_FOLDER))?;
        std::fs::create_dir_all(root.join(CACHE_FOLDER))?;
        for (file, text) in [(".gitignore", GITIGNORE), (".gitattributes", GITATTRIBUTES)] {
            let path = root.join(file);
            if !path.exists() {
                write_atomically(&path, text.as_bytes())?;
            }
        }
        let file = ProjectFile {
            name: name.to_string(),
            format: FORMAT,
        };
        let text = toml::to_string(&file).map_err(|e| ProjectError::Damaged(e.to_string()))?;
        write_atomically(&root.join(PROJECT_FILE), text.as_bytes())?;
        Ok(Self {
            root: root.to_path_buf(),
            name: name.to_string(),
        })
    }

    /// Opens the project in `root`, making its assets and cache folders if
    /// they're missing (a fresh clone has no cache).
    pub fn open(root: &Path) -> Result<Self, ProjectError> {
        let path = root.join(PROJECT_FILE);
        if !path.is_file() {
            return Err(ProjectError::NotAProject(root.to_path_buf()));
        }
        let text = read_text_limited(&path, 64 * 1024)?;
        let file: ProjectFile =
            toml::from_str(&text).map_err(|e| ProjectError::Damaged(e.message().to_string()))?;
        if file.format > FORMAT {
            return Err(ProjectError::TooNew(file.format));
        }
        std::fs::create_dir_all(root.join(ASSETS_FOLDER))?;
        std::fs::create_dir_all(root.join(CACHE_FOLDER))?;
        Ok(Self {
            root: root.to_path_buf(),
            name: file.name,
        })
    }

    /// Its name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Its top folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The folder of its assets.
    pub fn assets(&self) -> PathBuf {
        self.root.join(ASSETS_FOLDER)
    }

    /// The import cache's folder.
    pub fn cache(&self) -> PathBuf {
        self.root.join(CACHE_FOLDER).join("cache")
    }
}

/// Writes `bytes` to `path` through a temporary file beside it, so a crash,
/// a power cut or a full disk never leaves half a file (CONTRIBUTING.md,
/// rule 11): the temporary file is written to the disk itself before it
/// takes the real one's place. Each write has a temporary file of its own,
/// so two writing the same file at once never mix.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(format!(
        ".{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let temporary = PathBuf::from(temporary);
    let written = (|| {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    written?;
    // The new name itself is kept by the folder, which is flushed too
    // where the system allows (on Windows, renaming takes care of it).
    #[cfg(unix)]
    if let Some(folder) = path.parent()
        && let Ok(folder) = std::fs::File::open(if folder.as_os_str().is_empty() {
            Path::new(".")
        } else {
            folder
        })
    {
        let _ = folder.sync_all();
    }
    Ok(())
}

/// Reads a text file, refusing one bigger than `limit` bytes before
/// reading it.
pub(crate) fn read_text_limited(path: &Path, limit: u64) -> std::io::Result<String> {
    let size = std::fs::metadata(path)?.len();
    if size > limit {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("it's {size} bytes; the most read is {limit}"),
        ));
    }
    let bytes = std::fs::read(path)?;
    String::from_utf8(bytes)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "it isn't text"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_folder;

    #[test]
    fn atomic_writes_replace_the_file_and_leave_nothing_behind() {
        let folder = test_folder("atomic");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("harbour.solarea");
        write_atomically(&path, b"first").unwrap();
        write_atomically(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        // Many writing the same file at once each use their own temporary
        // file: one of them wins whole, and no temporary file is left.
        std::thread::scope(|s| {
            for i in 0..8 {
                let path = &path;
                s.spawn(move || write_atomically(path, format!("writer {i}").as_bytes()));
            }
        });
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("writer "), "{text}");
        let left: Vec<_> = std::fs::read_dir(&folder)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect();
        assert_eq!(left, ["harbour.solarea"], "{left:?}");
        // A write into a folder that isn't there fails, leaving nothing.
        assert!(write_atomically(&folder.join("gone/x.solarea"), b"x").is_err());
    }

    #[test]
    fn a_new_project_has_its_folders_and_git_files_and_opens_again() {
        let root = test_folder("new-project");
        let made = Project::create(&root, "Harbour").unwrap();
        assert!(root.join("assets").is_dir() && root.join(".solis").is_dir());
        let ignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(ignore.contains("/.solis/"));
        let attributes = std::fs::read_to_string(root.join(".gitattributes")).unwrap();
        assert!(attributes.contains("*.solarea text eol=lf"));
        assert!(attributes.contains("*.glb filter=lfs"));
        let opened = Project::open(&root).unwrap();
        assert_eq!(opened, made);
        assert_eq!(opened.name(), "Harbour");
        assert!(matches!(
            Project::create(&root, "Again"),
            Err(ProjectError::AlreadyAProject(_))
        ));
    }

    #[test]
    fn existing_git_files_are_kept() {
        let root = test_folder("keeps-git");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".gitignore"), "mine\n").unwrap();
        Project::create(&root, "Mine").unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join(".gitignore")).unwrap(),
            "mine\n"
        );
    }

    #[test]
    fn bad_project_files_are_explained() {
        let root = test_folder("bad-project");
        std::fs::create_dir_all(&root).unwrap();
        assert!(matches!(
            Project::open(&root),
            Err(ProjectError::NotAProject(_))
        ));
        std::fs::write(root.join(PROJECT_FILE), "name = ").unwrap();
        assert!(matches!(
            Project::open(&root),
            Err(ProjectError::Damaged(_))
        ));
        std::fs::write(root.join(PROJECT_FILE), "name = \"X\"\nformat = 99\n").unwrap();
        assert!(matches!(
            Project::open(&root),
            Err(ProjectError::TooNew(99))
        ));
        std::fs::write(root.join(PROJECT_FILE), vec![b'#'; 100_000]).unwrap();
        assert!(Project::open(&root).is_err(), "too big to read");
        // A fresh clone has no cache folder: it's made.
        std::fs::write(root.join(PROJECT_FILE), "name = \"X\"\nformat = 1\n").unwrap();
        let project = Project::open(&root).unwrap();
        assert!(project.root().join(".solis").is_dir());
        assert!(project.cache().starts_with(project.root()));
    }
}
