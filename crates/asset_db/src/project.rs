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

/// The project file's version and upgraders (Phase 8, C1): each step
/// turns the file's table at one version into the next, before it's
/// read. Every version ever released keeps an example file in
/// `tests/fixtures/project/`.
pub static PROJECT_FORMAT: reflect::upgrade::Format<toml::Table> = reflect::upgrade::Format {
    name: "project",
    current: FORMAT,
    version_of: format_key_version,
    steps: &[],
};

/// The `format = N` key of a TOML file (the project file, sidecars): a
/// format's `version_of`.
pub(crate) fn format_key_version(text: &str) -> Option<u32> {
    reflect::upgrade::toml_key_version(text, "format").flatten()
}

/// Reads a TOML file whose version is its `format` key: brought up to
/// date as a table by `format`'s upgraders, then read as `T`. A newer file
/// is refused with why, and never read on.
pub(crate) fn read_toml_versioned<T: serde::de::DeserializeOwned>(
    text: &str,
    format: &reflect::upgrade::Format<toml::Table>,
) -> Result<T, String> {
    let mut table: toml::Table = toml::from_str(text).map_err(|e| e.message().to_string())?;
    let version = match table.get("format") {
        Some(v) => v
            .as_integer()
            .and_then(|v| u32::try_from(v).ok())
            .ok_or("its format isn't a whole number")?,
        None => return Err("it has no format version".into()),
    };
    format
        .upgrade(version, &mut table)
        .map_err(|e| e.to_string())?;
    table.insert(
        "format".into(),
        toml::Value::Integer(i64::from(format.current)),
    );
    table
        .try_into()
        .map_err(|e: toml::de::Error| e.message().to_string())
}

/// New projects ignore the cache and builds in Git.
pub const GITIGNORE: &str = "\
# Solis's cache: rebuilt whenever it's missing. Never commit it.
/.solis/
# Exported games.
/builds/
# Save games made while playing in the editor (decision 0038).
/saves/
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
*.solinput text eol=lf
*.solrec text eol=lf
*.solpix text eol=lf
*.lst text eol=lf
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
        // Solis's own folder, where the cache is written: never a link.
        own_folder(root, CACHE_FOLDER)?;
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
        let name = Self::read_file(&text)?;
        std::fs::create_dir_all(root.join(ASSETS_FOLDER))?;
        // Solis's own folder, where the cache is written: never a link.
        own_folder(root, CACHE_FOLDER)?;
        Ok(Self {
            root: root.to_path_buf(),
            name,
        })
    }

    /// The project's name from `project.solis`'s text, refusing a damaged
    /// file or one from a newer Solis. (Fuzzed: a project can be anyone's.)
    pub fn read_file(text: &str) -> Result<String, ProjectError> {
        // Newer: refused with the plain reason the editor shows.
        if let Some(found) = format_key_version(text).filter(|v| *v > FORMAT) {
            return Err(ProjectError::TooNew(found));
        }
        let file: ProjectFile =
            read_toml_versioned(text, &PROJECT_FORMAT).map_err(ProjectError::Damaged)?;
        Ok(file.name)
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
        // A new file, never one already there (a link planted under the
        // same name would send the write elsewhere).
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
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

/// Reads a text file, refusing one bigger than `limit` bytes.
pub(crate) fn read_text_limited(path: &Path, limit: u64) -> std::io::Result<String> {
    let bytes = read_limited(path, limit)?;
    String::from_utf8(bytes)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "it isn't text"))
}

/// Reads a file of at most `limit` bytes. Only an ordinary file is read
/// (a link to a device or a pipe, which never ends, is refused), and never
/// more than `limit` bytes of it, even if it grows while being read.
pub fn read_limited(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let not_a_file =
        || std::io::Error::new(std::io::ErrorKind::InvalidData, "it isn't an ordinary file");
    let too_big = |size: u64| {
        std::io::Error::new(
            std::io::ErrorKind::FileTooLarge,
            format!("it's {size} bytes; the most read is {limit}"),
        )
    };
    // Checked before opening too: opening a pipe waits for a writer.
    let before = std::fs::metadata(path)?;
    if !before.is_file() {
        return Err(not_a_file());
    }
    if before.len() > limit {
        return Err(too_big(before.len()));
    }
    let file = std::fs::File::open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(not_a_file());
    }
    if meta.len() > limit {
        return Err(too_big(meta.len()));
    }
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    file.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(too_big(bytes.len() as u64));
    }
    Ok(bytes)
}

/// Makes the folder `name` (one or more plain parts) inside `root` if it
/// isn't there, and gives back its path, refusing it if it or a folder on
/// the way is a link (or a junction): Solis writes in it, and a project
/// could ship a link that sends those writes anywhere on the computer.
pub fn own_folder(root: &Path, name: &str) -> std::io::Result<PathBuf> {
    let mut path = root.to_path_buf();
    for part in name.split('/') {
        if part.is_empty() || part == "." || part == ".." || part.contains(['\\', ':']) {
            return Err(std::io::Error::other(format!(
                "{name} isn't a plain folder name"
            )));
        }
        path.push(part);
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(std::io::Error::other(format!(
                    "{} is a link to somewhere else; Solis only writes in a real folder there",
                    path.display()
                )));
            }
            Ok(meta) if !meta.is_dir() => {
                return Err(std::io::Error::other(format!(
                    "{} is a file, not a folder",
                    path.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => std::fs::create_dir(&path)?,
            Err(e) => return Err(e),
        }
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_folder;

    #[test]
    fn a_limited_read_takes_only_ordinary_files_within_the_limit() {
        // The third security audit: the size was checked, then the whole
        // file read, so a link to a device never ended.
        let folder = test_folder("limited");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("a.txt");
        std::fs::write(&path, b"twelve bytes").unwrap();
        assert_eq!(read_limited(&path, 12).unwrap(), b"twelve bytes");
        let err = read_limited(&path, 11).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::FileTooLarge);
        assert!(read_limited(&folder, 1 << 20).is_err(), "a folder");
        assert_eq!(
            read_limited(&folder.join("none"), 10).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
        #[cfg(unix)]
        {
            let link = folder.join("zero");
            std::os::unix::fs::symlink("/dev/zero", &link).unwrap();
            assert!(read_limited(&link, 1 << 20).is_err(), "a device");
        }
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn solis_s_own_folders_are_made_and_never_links() {
        let root = test_folder("own-folder");
        std::fs::create_dir_all(&root).unwrap();
        let made = own_folder(&root, "solis/kit").unwrap();
        assert!(made.is_dir() && made.ends_with("solis/kit"));
        assert_eq!(own_folder(&root, "solis/kit").unwrap(), made, "again");
        for bad in ["", "../out", "a//b", "C:/x", "a\\b"] {
            assert!(own_folder(&root, bad).is_err(), "{bad}");
        }
        std::fs::write(root.join("file"), b"").unwrap();
        assert!(own_folder(&root, "file").is_err());
        #[cfg(unix)]
        {
            let outside = test_folder("own-folder-outside");
            std::fs::create_dir_all(&outside).unwrap();
            std::os::unix::fs::symlink(&outside, root.join("saves")).unwrap();
            assert!(own_folder(&root, "saves").is_err());
            std::os::unix::fs::symlink(&outside, made.join("deeper")).unwrap();
            assert!(own_folder(&root, "solis/kit/deeper/x").is_err());
            assert!(std::fs::read_dir(&outside).unwrap().next().is_none());
            let _ = std::fs::remove_dir_all(&outside);
        }
        let _ = std::fs::remove_dir_all(&root);
    }

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
        assert!(ignore.contains("/saves/"));
        let attributes = std::fs::read_to_string(root.join(".gitattributes")).unwrap();
        assert!(attributes.contains("*.solarea text eol=lf"));
        assert!(attributes.contains("*.lst text eol=lf"), "Lust's files");
        assert!(attributes.contains("*.solinput text eol=lf"));
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
