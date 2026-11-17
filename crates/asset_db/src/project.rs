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
/// Version 2 is a project whose levels are `.sollevel` files (decision
/// 0118: areas renamed levels).
pub const FORMAT: u32 = 2;

/// The project file's version and upgraders (Phase 8, C1): each step
/// turns the file's table at one version into the next, before it's
/// read. Every version ever released keeps an example file in
/// `tests/fixtures/project/`.
pub static PROJECT_FORMAT: reflect::upgrade::Format<toml::Table> = reflect::upgrade::Format {
    name: "project",
    current: FORMAT,
    version_of: format_key_version,
    steps: &[levels_not_areas],
};

/// Version 1 to 2 (decision 0118): nothing in the file itself changes.
/// A project at version 2 keeps its levels as `.sollevel` files, moved
/// from `.solarea` when it was opened, which a build from before can't
/// see: such a build refuses the project as one from a newer Solis,
/// rather than opening it with no levels.
fn levels_not_areas(_: &mut toml::Table) -> Result<(), String> {
    Ok(())
}

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
*.sollevel text eol=lf
*.solarea text eol=lf
*.solassembly text eol=lf
*.solmat text eol=lf
*.solmeta text eol=lf
*.solinput text eol=lf
*.solrec text eol=lf
*.solpix text eol=lf
*.solframes text eol=lf
*.solanim text eol=lf
*.soltileset text eol=lf
*.soltilemap text eol=lf
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

/// What a project makes: 2D, 3D or both (decision 0118). Asked when the
/// project is made and changed in Project settings. Projects from before
/// it was asked have no `kind` and are both, so nothing they use is
/// hidden from them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum ProjectKind {
    /// Sprites and tile maps.
    TwoD,
    /// Models, lights and cameras.
    ThreeD,
    /// 2D and 3D: everything.
    #[default]
    Both,
}

impl ProjectKind {
    /// Every kind, in the order they're offered.
    pub const ALL: [Self; 3] = [Self::TwoD, Self::ThreeD, Self::Both];

    /// The word the project file keeps.
    pub fn word(self) -> &'static str {
        match self {
            Self::TwoD => "2d",
            Self::ThreeD => "3d",
            Self::Both => "both",
        }
    }

    /// The kind a project file's word names (any case), if any.
    pub fn from_word(word: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.word().eq_ignore_ascii_case(word.trim()))
    }
}

impl Serialize for ProjectKind {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.word())
    }
}

impl<'de> Deserialize<'de> for ProjectKind {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let word = String::deserialize(d)?;
        Self::from_word(&word).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "its kind \"{}\" isn't 2d, 3d or both",
                word.chars().take(20).collect::<String>()
            ))
        })
    }
}

/// What `project.solis` holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ProjectFile {
    /// The project's name, shown in the title bar.
    name: String,
    /// The format version.
    format: u32,
    /// The build of Solis that last opened it (decision 0086), for the
    /// launcher's list. Optional: older files and copies built from source
    /// leave it out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    saved_by: Option<u64>,
    /// 2D, 3D or both (decision 0118). An optional key, like `saved_by`:
    /// older files leave it out and are both, and a Solis that didn't know
    /// it passed it by and kept it when it wrote the file (it changes keys
    /// through the file's table), so the key needed no version of its own.
    #[serde(default)]
    kind: ProjectKind,
}

/// What the launcher shows of a project, read from its project file alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// Its name.
    pub name: String,
    /// The build of Solis that last opened it, if recorded.
    pub saved_by: Option<u64>,
    /// 2D, 3D or both (both for a project from before it was asked).
    pub kind: ProjectKind,
}

/// An open project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    root: PathBuf,
    name: String,
    kind: ProjectKind,
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
    /// Makes `root` a new project called `name`, of both kinds: see
    /// [`Project::create_kind`].
    pub fn create(root: &Path, name: &str) -> Result<Self, ProjectError> {
        Self::create_kind(root, name, ProjectKind::Both)
    }

    /// Makes `root` (and the folders above it) a new project called
    /// `name`, of `kind`: its project file, the cache folder, and Git's
    /// `.gitignore` and `.gitattributes` (kept if the folder already has
    /// them). Only what it needs to open: the assets folder is made when
    /// something is first put in it (decision 0118).
    pub fn create_kind(root: &Path, name: &str, kind: ProjectKind) -> Result<Self, ProjectError> {
        if root.join(PROJECT_FILE).exists() {
            return Err(ProjectError::AlreadyAProject(root.to_path_buf()));
        }
        std::fs::create_dir_all(root)?;
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
            saved_by: None,
            kind,
        };
        let text = toml::to_string(&file).map_err(|e| ProjectError::Damaged(e.to_string()))?;
        write_atomically(&root.join(PROJECT_FILE), text.as_bytes())?;
        Ok(Self {
            root: root.to_path_buf(),
            name: name.to_string(),
            kind,
        })
    }

    /// Opens the project in `root`, making its cache folder if it's
    /// missing (a fresh clone has no cache). A missing assets folder is
    /// left missing (decision 0118): it's made when something is first
    /// put in it ([`Project::assets_folder`]).
    pub fn open(root: &Path) -> Result<Self, ProjectError> {
        let path = root.join(PROJECT_FILE);
        if !path.is_file() {
            return Err(ProjectError::NotAProject(root.to_path_buf()));
        }
        let text = read_text_limited(&path, 64 * 1024)?;
        let summary = Self::read_summary(&text)?;
        // Solis's own folder, where the cache is written: never a link.
        own_folder(root, CACHE_FOLDER)?;
        Ok(Self {
            root: root.to_path_buf(),
            name: summary.name,
            kind: summary.kind,
        })
    }

    /// The project's name from `project.solis`'s text, refusing a damaged
    /// file or one from a newer Solis. (Fuzzed: a project can be anyone's.)
    pub fn read_file(text: &str) -> Result<String, ProjectError> {
        Self::read_summary(text).map(|s| s.name)
    }

    /// The project's name and the build that last opened it, from
    /// `project.solis`'s text (refused as [`Project::read_file`] refuses).
    pub fn read_summary(text: &str) -> Result<Summary, ProjectError> {
        // Newer: refused with the plain reason the editor shows.
        if let Some(found) = format_key_version(text).filter(|v| *v > FORMAT) {
            return Err(ProjectError::TooNew(found));
        }
        let file: ProjectFile =
            read_toml_versioned(text, &PROJECT_FORMAT).map_err(ProjectError::Damaged)?;
        Ok(Summary {
            name: file.name,
            saved_by: file.saved_by,
            kind: file.kind,
        })
    }

    /// What the launcher shows of the project in `root`, reading only its
    /// project file: nothing is made or changed.
    pub fn summary(root: &Path) -> Result<Summary, ProjectError> {
        let path = root.join(PROJECT_FILE);
        if !path.is_file() {
            return Err(ProjectError::NotAProject(root.to_path_buf()));
        }
        Self::read_summary(&read_text_limited(&path, 64 * 1024)?)
    }

    /// The project file's text brought up to this build's format, keeping
    /// everything else it says, when it's older: `None` when it's current.
    pub fn upgraded_file(&self) -> Result<Option<String>, ProjectError> {
        let text = read_text_limited(&self.root.join(PROJECT_FILE), 64 * 1024)?;
        let Some(found) = format_key_version(&text).filter(|v| *v < FORMAT) else {
            return Ok(None);
        };
        let mut table: toml::Table =
            toml::from_str(&text).map_err(|e| ProjectError::Damaged(e.to_string()))?;
        PROJECT_FORMAT
            .upgrade(found, &mut table)
            .map_err(|e| ProjectError::Damaged(e.to_string()))?;
        table.insert("format".into(), toml::Value::Integer(i64::from(FORMAT)));
        toml::to_string(&table)
            .map(Some)
            .map_err(|e| ProjectError::Damaged(e.to_string()))
    }

    /// Saves the project file in this build's format when it's older,
    /// keeping a copy of the older one in the backups first
    /// (`crate::backup::save`, rule 23), once the project's older files
    /// have moved to their new names (decision 0118: format 2 once its
    /// levels are `.sollevel`). Returns where the copy went, or `None`
    /// when the file was already current.
    pub fn upgrade_file(&self) -> Result<Option<PathBuf>, String> {
        let Some(text) = self.upgraded_file().map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let read_back = |new: &str| {
            let table: toml::Table = toml::from_str(new).map_err(|e| e.to_string())?;
            let again = toml::to_string(&table).map_err(|e| e.to_string())?;
            Self::read_summary(&again).map_err(|e| e.to_string())?;
            if again == new {
                Ok(())
            } else {
                Err("it read back differently".into())
            }
        };
        let path = self.root.join(PROJECT_FILE);
        crate::backup::save(
            &self.root,
            &path,
            text.as_bytes(),
            &PROJECT_FORMAT,
            read_back,
        )
        .map_err(|e| e.to_string())
    }

    /// Notes in the project file that `build` opened it last, keeping
    /// everything else the file says. Returns whether the file changed:
    /// it's left alone when it already says so.
    pub fn record_build(&self, build: u64) -> Result<bool, ProjectError> {
        let number = i64::try_from(build)
            .map_err(|_| ProjectError::Damaged("the build number is too big".into()))?;
        self.change_file("saved_by", toml::Value::Integer(number), |s| {
            s.saved_by == Some(build)
        })
    }

    /// Gives the project the kind `kind` in its project file (Project
    /// settings), keeping everything else the file says. Returns whether
    /// the file changed: it's left alone when it already says so.
    pub fn set_kind(&mut self, kind: ProjectKind) -> Result<bool, ProjectError> {
        let changed = self.change_file("kind", toml::Value::String(kind.word().into()), |s| {
            s.kind == kind
        })?;
        self.kind = kind;
        Ok(changed)
    }

    /// Sets `key` to `value` in the project file, keeping everything else
    /// it says, unless `already` says the file has it so.
    fn change_file(
        &self,
        key: &str,
        value: toml::Value,
        already: impl Fn(&Summary) -> bool,
    ) -> Result<bool, ProjectError> {
        let path = self.root.join(PROJECT_FILE);
        let text = read_text_limited(&path, 64 * 1024)?;
        if already(&Self::read_summary(&text)?) {
            return Ok(false);
        }
        let mut table: toml::Table =
            toml::from_str(&text).map_err(|e| ProjectError::Damaged(e.to_string()))?;
        table.insert(key.into(), value);
        let text = toml::to_string(&table).map_err(|e| ProjectError::Damaged(e.to_string()))?;
        write_atomically(&path, text.as_bytes())?;
        Ok(true)
    }

    /// Its name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 2D, 3D or both.
    pub fn kind(&self) -> ProjectKind {
        self.kind
    }

    /// Its top folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The folder of its assets. It may not be there yet: a new project
    /// gets it when something is first put in it
    /// ([`Project::assets_folder`]).
    pub fn assets(&self) -> PathBuf {
        self.root.join(ASSETS_FOLDER)
    }

    /// The folder of its assets, made now if it isn't there (never
    /// through a link), for something about to be put in it.
    pub fn assets_folder(&self) -> std::io::Result<PathBuf> {
        own_folder(&self.root, ASSETS_FOLDER)
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
/// more than `limit` bytes of it, even if it grows while being read
/// (`reflect::ordinary_file`, the one place files are opened). One that
/// isn't an ordinary file is `InvalidData`; a bigger one, `FileTooLarge`.
pub fn read_limited(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    Ok(reflect::ordinary_file::read_limited(path, limit)?)
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
        // A folder, a device or a pipe named like a file: refused at once,
        // as text too.
        for odd in crate::odd_files(&folder, "b.txt") {
            let err = read_limited(&odd, 1 << 20).unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "{odd:?}");
            assert_eq!(err.to_string(), "it isn't an ordinary file", "{odd:?}");
            assert!(read_text_limited(&odd, 1 << 20).is_err(), "{odd:?}");
        }
        assert_eq!(read_text_limited(&path, 12).unwrap(), "twelve bytes");
    }

    #[test]
    fn the_build_that_opened_a_project_is_recorded_and_read_back() {
        let root = test_folder("saved-by");
        let project = Project::create(&root, "Farm").unwrap();
        let none = Summary {
            name: "Farm".into(),
            saved_by: None,
            kind: ProjectKind::Both,
        };
        assert_eq!(
            Project::summary(&root).unwrap(),
            none,
            "a new one says none"
        );
        // Something a later Solis wrote is kept.
        let path = root.join(PROJECT_FILE);
        let text = std::fs::read_to_string(&path).unwrap() + "later = \"kept\"\n";
        std::fs::write(&path, text).unwrap();
        assert!(project.record_build(57).unwrap());
        assert!(
            !project.record_build(57).unwrap(),
            "the same build: unchanged"
        );
        assert_eq!(Project::summary(&root).unwrap().saved_by, Some(57));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("later = \"kept\""), "{text}");
        assert!(
            project.record_build(52).unwrap(),
            "an older build opening it"
        );
        assert_eq!(Project::summary(&root).unwrap().saved_by, Some(52));
        assert_eq!(Project::open(&root).unwrap().name(), "Farm");
    }

    #[test]
    fn a_summary_refuses_what_opening_refuses() {
        let root = test_folder("summary-refused");
        assert!(matches!(
            Project::summary(&root),
            Err(ProjectError::NotAProject(_))
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(PROJECT_FILE), "name = \"X\"\nformat = 99\n").unwrap();
        assert!(matches!(
            Project::summary(&root),
            Err(ProjectError::TooNew(99))
        ));
        std::fs::write(root.join(PROJECT_FILE), "name = 3\nformat = 1\n").unwrap();
        assert!(matches!(
            Project::summary(&root),
            Err(ProjectError::Damaged(_))
        ));
        // A build number that isn't a number is a damaged file, not a crash.
        std::fs::write(
            root.join(PROJECT_FILE),
            "name = \"X\"\nformat = 1\nsaved_by = -4\n",
        )
        .unwrap();
        assert!(Project::summary(&root).is_err());
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
        }
    }

    #[test]
    fn atomic_writes_replace_the_file_and_leave_nothing_behind() {
        let folder = test_folder("atomic");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("harbour.sollevel");
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
        assert_eq!(left, ["harbour.sollevel"], "{left:?}");
        // A write into a folder that isn't there fails, leaving nothing.
        assert!(write_atomically(&folder.join("gone/x.sollevel"), b"x").is_err());
    }

    #[test]
    fn a_new_project_has_its_folders_and_git_files_and_opens_again() {
        let root = test_folder("new-project");
        let made = Project::create(&root, "Harbour").unwrap();
        assert!(root.join(".solis").is_dir());
        // Only what it needs to open (decision 0118): no assets folder
        // until something is put in it.
        assert!(!root.join("assets").exists());
        let ignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(ignore.contains("/.solis/"));
        assert!(ignore.contains("/saves/"));
        let attributes = std::fs::read_to_string(root.join(".gitattributes")).unwrap();
        assert!(attributes.contains("*.sollevel text eol=lf"), "levels");
        assert!(
            attributes.contains("*.solarea text eol=lf"),
            "levels before 0118"
        );
        assert!(attributes.contains("*.lst text eol=lf"), "Lust's files");
        assert!(attributes.contains("*.solinput text eol=lf"));
        assert!(attributes.contains("*.solanim text eol=lf"), "animators");
        assert!(attributes.contains("*.soltilemap text eol=lf"), "tile maps");
        assert!(attributes.contains("*.glb filter=lfs"));
        let opened = Project::open(&root).unwrap();
        assert_eq!(opened, made);
        assert_eq!(opened.name(), "Harbour");
        assert!(!root.join("assets").exists(), "opening makes none either");
        let assets = opened.assets_folder().unwrap();
        assert!(assets.is_dir() && assets == opened.assets());
        assert_eq!(opened.assets_folder().unwrap(), assets, "again");
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

    #[test]
    fn a_project_of_format_1_is_brought_to_format_2_keeping_what_it_says() {
        // Decision 0118: format 2 is a project whose levels are .sollevel.
        assert_eq!(FORMAT, 2);
        assert!(PROJECT_FORMAT.is_complete());
        let root = test_folder("format-2");
        let old = "name = \"Harbour\"\nformat = 1\nsaved_by = 211\nkind = \"2d\"\nlater = 1\n";
        std::fs::write(root.join(PROJECT_FILE), old).unwrap();
        let project = Project::open(&root).unwrap();
        assert_eq!(project.name(), "Harbour");
        let new = project.upgraded_file().unwrap().expect("format 1 is older");
        assert_eq!(format_key_version(&new), Some(2));
        let mut kept: toml::Table = toml::from_str(&new).unwrap();
        let mut was: toml::Table = toml::from_str(old).unwrap();
        kept.remove("format");
        was.remove("format");
        assert_eq!(kept, was, "nothing else changes");
        // Opening alone never writes it (the app saves it, with a backup).
        assert_eq!(
            std::fs::read_to_string(root.join(PROJECT_FILE)).unwrap(),
            old
        );
        std::fs::write(root.join(PROJECT_FILE), &new).unwrap();
        assert_eq!(Project::read_summary(&new).unwrap().kind, ProjectKind::TwoD);
        assert!(project.upgraded_file().unwrap().is_none(), "current");
        // A new project is made at format 2.
        let made = Project::create(&root.join("new"), "New").unwrap();
        assert!(made.upgraded_file().unwrap().is_none());
    }

    #[test]
    fn a_build_from_before_levels_refuses_a_format_2_project_as_newer() {
        // The reader of the build before decision 0118: format 1 at most.
        let before = reflect::upgrade::Format::<toml::Table> {
            name: "project",
            current: 1,
            version_of: format_key_version,
            steps: &[],
        };
        let text = "name = \"Harbour\"\nformat = 2\n";
        let mut table: toml::Table = toml::from_str(text).unwrap();
        let refused = before.upgrade(2, &mut table).unwrap_err().to_string();
        assert!(refused.contains("saved by a newer version"), "{refused}");
        // And how that build's project reader said it (its check of the
        // format key before reading on), word for word.
        assert_eq!(
            ProjectError::TooNew(2).to_string(),
            "it was made by a newer version (format 2); update to open it"
        );
        assert!(matches!(
            Project::read_summary("name = \"X\"\nformat = 3\n"),
            Err(ProjectError::TooNew(3))
        ));
    }

    #[test]
    fn a_project_s_kind_is_kept_and_old_projects_are_both() {
        let root = test_folder("kind");
        let made = Project::create_kind(&root.join("deeper/Farm"), "Farm", ProjectKind::TwoD);
        let mut project = made.unwrap();
        assert_eq!(project.kind(), ProjectKind::TwoD);
        let summary = Project::summary(project.root()).unwrap();
        assert_eq!(summary.kind, ProjectKind::TwoD, "read back");
        let text = std::fs::read_to_string(project.root().join(PROJECT_FILE)).unwrap();
        assert!(text.contains("kind = \"2d\""), "{text}");
        // Changed in Project settings, keeping what else the file says.
        let path = project.root().join(PROJECT_FILE);
        std::fs::write(&path, text + "later = 1\n").unwrap();
        assert!(project.set_kind(ProjectKind::ThreeD).unwrap());
        assert!(!project.set_kind(ProjectKind::ThreeD).unwrap(), "the same");
        assert_eq!(project.kind(), ProjectKind::ThreeD);
        let opened = Project::open(project.root()).unwrap();
        assert_eq!(opened.kind(), ProjectKind::ThreeD);
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("later = 1")
        );
        // A project from before the kind was asked is both.
        std::fs::write(&path, "name = \"Old\"\nformat = 1\n").unwrap();
        assert_eq!(
            Project::open(project.root()).unwrap().kind(),
            ProjectKind::Both
        );
        // Any case; anything else is explained, never guessed.
        for (word, kind) in [("2D", ProjectKind::TwoD), (" Both ", ProjectKind::Both)] {
            std::fs::write(
                &path,
                format!("name = \"X\"\nformat = 1\nkind = \"{word}\"\n"),
            )
            .unwrap();
            assert_eq!(
                Project::summary(project.root()).unwrap().kind,
                kind,
                "{word}"
            );
        }
        for wrong in ["\"4d\"", "3", "[]", "\"\""] {
            std::fs::write(&path, format!("name = \"X\"\nformat = 1\nkind = {wrong}\n")).unwrap();
            let err = Project::summary(project.root()).unwrap_err();
            assert!(matches!(err, ProjectError::Damaged(_)), "{wrong}: {err}");
        }
        let err = Project::read_summary("name = \"X\"\nformat = 1\nkind = \"vr\"\n");
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("isn't 2d, 3d or both")
        );
    }

    #[test]
    fn every_kind_is_written_and_read_back() {
        for kind in ProjectKind::ALL {
            assert_eq!(ProjectKind::from_word(kind.word()), Some(kind));
            let root = test_folder(&format!("kind-{}", kind.word()));
            Project::create_kind(&root, "Round", kind).unwrap();
            assert_eq!(Project::open(&root).unwrap().kind(), kind);
        }
        assert_eq!(ProjectKind::default(), ProjectKind::Both);
    }

    /// Thousands of damaged project files, around the kind especially:
    /// each is read or refused in plain words, never a crash (rule 9).
    #[test]
    fn damaged_project_files_never_crash_the_reader() {
        let good = "name = \"Farm\"\nformat = 1\nsaved_by = 57\nkind = \"2d\"\n";
        assert_eq!(Project::read_summary(good).unwrap().kind, ProjectKind::TwoD);
        let pieces = [
            "kind", "=", "\"", "2d", "3d", "both", "[", "]", "{", "}", "\n", "format", "1", "99",
            "name", "-", "é", "\u{0}", "#", "saved_by", ".", "'", "\\", "true",
        ];
        let mut seed: u64 = 0x50_115;
        let mut next = |n: usize| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) as usize % n
        };
        for _ in 0..5_000 {
            let mut text: Vec<char> = good.chars().collect();
            for _ in 0..=next(4) {
                let at = next(text.len() + 1);
                match next(3) {
                    0 if at < text.len() => drop(text.remove(at)),
                    1 => {
                        let piece = pieces[next(pieces.len())];
                        for (i, c) in piece.chars().enumerate() {
                            text.insert((at + i).min(text.len()), c);
                        }
                    }
                    _ => text.truncate(at),
                }
            }
            let text: String = text.into_iter().collect();
            if let Err(err) = Project::read_summary(&text) {
                assert!(!err.to_string().is_empty(), "{text:?}");
            }
        }
    }
}
