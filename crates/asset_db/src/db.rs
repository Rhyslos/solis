//! The asset database: every asset in the project's `assets/` folder, by
//! its ID from the sidecar beside it
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)).
//! The folder is the truth: the database is worked out from it on opening
//! and checked again whenever the watcher says something changed.

use crate::hash::hash_file;
use crate::project::Project;
use crate::sidecar::{self, Sidecar};
use reflect::AssetId;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What kind of asset a file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetKind {
    /// A glTF model.
    Model,
    /// An assembly (`.solassembly`): entities to place many times.
    Assembly,
}

/// The extension of assembly files.
pub const ASSEMBLY_EXTENSION: &str = "solassembly";

impl AssetKind {
    /// The kind of the file at `path`, by its extension, if Solis reads it.
    pub fn of(path: &Path) -> Option<Self> {
        if assets::is_model_file(path) {
            return Some(Self::Model);
        }
        path.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(ASSEMBLY_EXTENSION))
            .then_some(Self::Assembly)
    }
}

/// A file's size and time of last change, to notice changes cheaply. Two
/// versions with the same size written within one tick of the file
/// system's clock look the same; real saves are far enough apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    /// Bytes.
    pub size: u64,
    /// Last changed.
    pub modified: Option<SystemTime>,
}

impl Stamp {
    fn of(path: &Path) -> Option<Self> {
        let meta = std::fs::metadata(path).ok()?;
        Some(Self {
            size: meta.len(),
            modified: meta.modified().ok(),
        })
    }
}

/// One asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRecord {
    /// Its ID, from its sidecar.
    pub id: AssetId,
    /// Where it is, inside `assets/`.
    pub path: PathBuf,
    /// What kind it is.
    pub kind: AssetKind,
    /// Its own file's stamp, and those of the files it reads (a `.gltf`'s
    /// `.bin` and images), to notice a change to any of them.
    pub stamps: Vec<(PathBuf, Option<Stamp>)>,
}

impl AssetRecord {
    /// Its name: the file's name without its extension.
    pub fn name(&self) -> String {
        self.path.file_stem().map_or_else(
            || self.path.display().to_string(),
            |s| s.to_string_lossy().into_owned(),
        )
    }
}

/// How the folder changed since the last look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A new asset.
    Added(AssetId),
    /// An asset's content (or a file it reads) changed: reload it.
    Changed(AssetId),
    /// An asset was moved or renamed (its ID stays).
    Moved {
        /// The asset.
        id: AssetId,
        /// Where it was.
        from: PathBuf,
    },
    /// An asset is gone. Its sidecar, if left behind, is kept.
    Removed {
        /// The asset.
        id: AssetId,
        /// Where it was.
        path: PathBuf,
    },
}

/// The project's assets.
#[derive(Debug)]
pub struct AssetDb {
    project: Project,
    records: BTreeMap<AssetId, AssetRecord>,
    limits: assets::Limits,
}

impl AssetDb {
    /// The database of `project`, worked out from its folder. Returns it,
    /// every asset as added, and any problems in plain words.
    pub fn open(project: Project, limits: assets::Limits) -> (Self, Vec<Change>, Vec<String>) {
        let mut db = Self {
            project,
            records: BTreeMap::new(),
            limits,
        };
        let (changes, notes) = db.rescan();
        (db, changes, notes)
    }

    /// The project.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Every asset, by ID.
    pub fn records(&self) -> &BTreeMap<AssetId, AssetRecord> {
        &self.records
    }

    /// The asset `id`.
    pub fn get(&self, id: AssetId) -> Option<&AssetRecord> {
        self.records.get(&id)
    }

    /// Where the asset `id`'s file is on disk.
    pub fn file(&self, id: AssetId) -> Option<PathBuf> {
        self.records
            .get(&id)
            .map(|r| self.project.assets().join(&r.path))
    }

    /// The asset whose file is at `path` (anywhere on disk), if any.
    pub fn id_at(&self, path: &Path) -> Option<AssetId> {
        let relative = path.strip_prefix(self.project.assets()).ok()?;
        self.records
            .values()
            .find(|r| r.path == relative)
            .map(|r| r.id)
    }

    /// Looks at the whole folder again and returns what changed. Sidecars
    /// are written for new assets, follow a file renamed outside Solis when
    /// its content shows which it is, and are never deleted.
    pub fn rescan(&mut self) -> (Vec<Change>, Vec<String>) {
        let root = self.project.assets();
        let mut notes = Vec::new();
        let (files, sidecars) = walk(&root);
        // Orphans: sidecars whose asset isn't there, by recorded hash.
        let orphans: Vec<(PathBuf, Sidecar)> = sidecars
            .iter()
            .filter(|s| sidecar::asset_for(s).is_some_and(|a| !a.exists()))
            .filter_map(|s| Sidecar::read(s).ok().map(|c| (s.clone(), c)))
            .collect();
        let mut found: BTreeMap<AssetId, AssetRecord> = BTreeMap::new();
        let mut unpaired: Vec<PathBuf> = Vec::new();
        let mut known: Vec<(PathBuf, AssetKind, Sidecar)> = Vec::new();
        for file in files {
            let Some(kind) = AssetKind::of(&file) else {
                continue;
            };
            let sidecar_path = sidecar::path_for(&file);
            if !sidecar_path.exists() {
                unpaired.push(file);
                continue;
            }
            let relative = relative(&root, &file);
            match Sidecar::read(&sidecar_path) {
                Ok(sidecar) => known.push((relative, kind, sidecar)),
                Err(why) => notes.push(format!(
                    "{}'s sidecar couldn't be read ({why}). Fix it, or delete it to give the \
                     file a new ID; it's left out until then.",
                    relative.display()
                )),
            }
        }
        // A copy that brought its sidecar along shares an ID: the file
        // already known by it keeps it (or the first, by name), and the
        // others get IDs of their own.
        let mut by_id: BTreeMap<AssetId, Vec<usize>> = BTreeMap::new();
        for (i, (_, _, sidecar)) in known.iter().enumerate() {
            by_id.entry(sidecar.id).or_default().push(i);
        }
        for (id, sharing) in by_id {
            let keeper = sharing
                .iter()
                .copied()
                .find(|&i| self.records.get(&id).is_some_and(|r| r.path == known[i].0))
                .unwrap_or(sharing[0]);
            let (path, kind, _) = &known[keeper];
            found.insert(id, self.record(id, path.clone(), *kind));
            for &copy in sharing.iter().filter(|&&i| i != keeper) {
                let (path, kind, sidecar) = &known[copy];
                let mut fresh = sidecar.clone();
                fresh.id = AssetId::random();
                fresh.parts.clear();
                match fresh.write(&sidecar::path_for(&root.join(path))) {
                    Ok(_) => {
                        notes.push(format!(
                            "{} is a copy, so it got a new ID of its own.",
                            path.display()
                        ));
                        found.insert(fresh.id, self.record(fresh.id, path.clone(), *kind));
                    }
                    Err(e) => notes.push(format!(
                        "Couldn't give {} its own ID ({e}); it's left out.",
                        path.display()
                    )),
                }
            }
        }
        // New files: a renamed one finds its orphaned sidecar by content.
        let mut orphan_taken = vec![false; orphans.len()];
        let hashes: Vec<Option<u64>> = unpaired.iter().map(|f| hash_file(f).ok()).collect();
        for (file, hash) in unpaired.iter().zip(&hashes) {
            let Some(kind) = AssetKind::of(file) else {
                continue;
            };
            let relative = relative(&root, file);
            let matching: Vec<usize> = orphans
                .iter()
                .enumerate()
                .filter(|(i, (_, o))| {
                    !orphan_taken[*i] && hash.is_some() && o.content_hash() == *hash
                })
                .map(|(i, _)| i)
                .collect();
            let same_content_files = hashes.iter().filter(|h| **h == *hash).count();
            let sidecar_path = sidecar::path_for(file);
            if let ([only], 1) = (matching.as_slice(), same_content_files) {
                let (orphan_path, orphan) = &orphans[*only];
                if !found.contains_key(&orphan.id)
                    && std::fs::rename(orphan_path, &sidecar_path).is_ok()
                {
                    orphan_taken[*only] = true;
                    found.insert(orphan.id, self.record(orphan.id, relative, kind));
                    continue;
                }
            }
            let new = Sidecar::new(hash.unwrap_or(0));
            match new.write(&sidecar_path) {
                Ok(_) => {
                    found.insert(new.id, self.record(new.id, relative, kind));
                }
                Err(e) => notes.push(format!(
                    "Couldn't write {}'s sidecar ({e}); it's left out.",
                    relative.display()
                )),
            }
        }
        // What changed since last time.
        let mut changes = Vec::new();
        for (id, record) in &found {
            match self.records.get(id) {
                None => changes.push(Change::Added(*id)),
                Some(old) => {
                    if old.path != record.path {
                        changes.push(Change::Moved {
                            id: *id,
                            from: old.path.clone(),
                        });
                    }
                    if old
                        .stamps
                        .iter()
                        .map(|(_, s)| s)
                        .ne(record.stamps.iter().map(|(_, s)| s))
                    {
                        changes.push(Change::Changed(*id));
                    }
                }
            }
        }
        for (id, old) in &self.records {
            if !found.contains_key(id) {
                changes.push(Change::Removed {
                    id: *id,
                    path: old.path.clone(),
                });
            }
        }
        self.records = found;
        (changes, notes)
    }

    fn record(&self, id: AssetId, path: PathBuf, kind: AssetKind) -> AssetRecord {
        let file = self.project.assets().join(&path);
        let mut stamps = vec![(path.clone(), Stamp::of(&file))];
        if file
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("gltf"))
        {
            for dependency in assets::referenced_files(&file, &self.limits).unwrap_or_default() {
                let stamp = Stamp::of(&dependency);
                stamps.push((dependency, stamp));
            }
        }
        AssetRecord {
            id,
            path,
            kind,
            stamps,
        }
    }

    /// Copies the model at `source` (and the files it reads) into the
    /// project's assets, under a name not yet taken, and returns where it
    /// went. The next rescan adds it.
    pub fn copy_in(&self, source: &Path) -> std::io::Result<PathBuf> {
        let assets = self.project.assets();
        let name = source
            .file_name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no file name"))?;
        let mut target = assets.join(name);
        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let extension = source
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut n = 2;
        while target.exists() {
            target = assets.join(format!("{stem} {n}.{extension}"));
            n += 1;
        }
        let from_folder = source.parent().unwrap_or(Path::new(""));
        for dependency in assets::referenced_files(source, &self.limits).unwrap_or_default() {
            // Kept at the same place relative to the model, so it still
            // finds them; one already there is left as it is.
            if let Ok(inside) = dependency.strip_prefix(from_folder) {
                let to = assets.join(inside);
                if !to.exists() {
                    if let Some(folder) = to.parent() {
                        std::fs::create_dir_all(folder)?;
                    }
                    std::fs::copy(&dependency, &to)?;
                }
            }
        }
        std::fs::copy(source, &target)?;
        Ok(target)
    }

    /// Gives the parts of `model` (the asset `id`, just imported) stable
    /// IDs in its sidecar, writing it only if they changed. Returns each
    /// part's ID, in the order of [`sidecar::model_parts`].
    pub fn update_parts(&self, id: AssetId, model: &assets::Model) -> std::io::Result<Vec<u32>> {
        let Some(file) = self.file(id) else {
            return Ok(Vec::new());
        };
        let path = sidecar::path_for(&file);
        let mut sidecar = Sidecar::read(&path)
            .map_err(|why| std::io::Error::new(std::io::ErrorKind::InvalidData, why))?;
        let ids = sidecar.assign_parts(&sidecar::model_parts(model));
        if let Ok(hash) = hash_file(&file) {
            sidecar.hash = format!("{hash:016x}");
        }
        sidecar.write(&path)?;
        Ok(ids)
    }
}

/// `path` inside `root`.
fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map_or_else(|_| path.to_path_buf(), Path::to_path_buf)
}

/// Every file under `root` that's an asset or a sidecar, sorted, skipping
/// hidden folders and temporary files. A folder that loops back (a link)
/// is walked once.
fn walk(root: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut files = Vec::new();
    let mut sidecars = Vec::new();
    let mut todo = vec![root.to_path_buf()];
    let mut seen = HashMap::new();
    while let Some(folder) = todo.pop() {
        let Ok(real) = std::fs::canonicalize(&folder) else {
            continue;
        };
        if seen.insert(real, ()).is_some() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || name.ends_with(".tmp") {
                continue;
            }
            match entry.file_type() {
                Ok(t) if t.is_dir() => todo.push(path),
                Ok(_) if name.ends_with(&format!(".{}", sidecar::EXTENSION)) => sidecars.push(path),
                Ok(_) if AssetKind::of(&path).is_some() => files.push(path),
                _ => {}
            }
        }
    }
    files.sort();
    sidecars.sort();
    (files, sidecars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_folder;

    fn project(name: &str) -> (Project, PathBuf) {
        let root = test_folder(name);
        let project = Project::create(&root, "Test").unwrap();
        let assets = project.assets();
        (project, assets)
    }

    fn glb(path: &Path, size: f32) {
        let mut b = assets::sample::Builder::default();
        b.add_cube_mesh(size, None);
        b.json_nodes = vec![r#"{"mesh":0,"name":"Box"}"#.into()];
        std::fs::write(path, b.glb()).unwrap();
    }

    fn open(project: Project) -> (AssetDb, Vec<Change>, Vec<String>) {
        AssetDb::open(project, assets::Limits::default())
    }

    #[test]
    fn new_assets_get_sidecars_and_keep_their_ids_on_reopening() {
        let (project, assets) = project("db-new");
        glb(&assets.join("crate.glb"), 0.5);
        std::fs::create_dir_all(assets.join("props")).unwrap();
        glb(&assets.join("props/barrel.glb"), 0.4);
        std::fs::write(assets.join("notes.txt"), "not an asset").unwrap();
        let (db, changes, notes) = open(project.clone());
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(changes.len(), 2);
        assert!(assets.join("crate.glb.solmeta").exists());
        let ids: Vec<AssetId> = db.records().keys().copied().collect();
        let (again, changes, _) = open(project);
        assert_eq!(again.records().keys().copied().collect::<Vec<_>>(), ids);
        assert_eq!(changes.len(), 2, "all added, as it's a fresh database");
        let barrel = again.id_at(&assets.join("props/barrel.glb")).unwrap();
        assert_eq!(again.get(barrel).unwrap().name(), "barrel");
    }

    #[test]
    fn assemblies_are_assets_too() {
        let (project, assets) = project("db-assemblies");
        std::fs::create_dir_all(assets.join("assemblies")).unwrap();
        std::fs::write(
            assets.join("assemblies/lamp post.solassembly"),
            "assembly version 1 root 0000000000001;\nentity 0000000000001 { name = \"Post\"; }\n",
        )
        .unwrap();
        let (db, changes, _) = open(project);
        assert_eq!(changes.len(), 1);
        let id = db
            .id_at(&assets.join("assemblies/lamp post.solassembly"))
            .unwrap();
        assert_eq!(db.get(id).unwrap().kind, AssetKind::Assembly);
        assert_eq!(db.get(id).unwrap().name(), "lamp post");
    }

    #[test]
    fn a_rename_outside_solis_keeps_the_id() {
        let (project, assets) = project("db-rename");
        glb(&assets.join("crate.glb"), 0.5);
        let (mut db, ..) = open(project);
        let id = db.id_at(&assets.join("crate.glb")).unwrap();
        // Explorer renames the file and leaves the sidecar behind.
        std::fs::create_dir_all(assets.join("props")).unwrap();
        std::fs::rename(assets.join("crate.glb"), assets.join("props/box.glb")).unwrap();
        let (changes, notes) = db.rescan();
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(
            changes,
            [Change::Moved {
                id,
                from: PathBuf::from("crate.glb")
            }]
        );
        assert_eq!(db.id_at(&assets.join("props/box.glb")), Some(id));
        assert!(assets.join("props/box.glb.solmeta").exists());
        assert!(
            !assets.join("crate.glb.solmeta").exists(),
            "the sidecar followed"
        );
    }

    #[test]
    fn a_copy_with_its_sidecar_gets_a_new_id_and_the_original_keeps_its_own() {
        let (project, assets) = project("db-copy");
        glb(&assets.join("crate.glb"), 0.5);
        let (mut db, ..) = open(project);
        let id = db.id_at(&assets.join("crate.glb")).unwrap();
        // Copied in Explorer, sidecar and all, under a name sorting first.
        std::fs::copy(assets.join("crate.glb"), assets.join("a crate.glb")).unwrap();
        std::fs::copy(
            assets.join("crate.glb.solmeta"),
            assets.join("a crate.glb.solmeta"),
        )
        .unwrap();
        let (changes, notes) = db.rescan();
        assert_eq!(
            db.id_at(&assets.join("crate.glb")),
            Some(id),
            "the original keeps it"
        );
        let copy = db.id_at(&assets.join("a crate.glb")).unwrap();
        assert_ne!(copy, id);
        assert_eq!(changes, [Change::Added(copy)]);
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn edits_moves_and_removals_are_noticed_and_sidecars_kept() {
        let (project, assets) = project("db-changes");
        glb(&assets.join("crate.glb"), 0.5);
        let (mut db, ..) = open(project);
        let id = db.id_at(&assets.join("crate.glb")).unwrap();
        assert!(db.rescan().0.is_empty(), "nothing changed");
        // Saved again, bigger.
        glb(&assets.join("crate.glb"), 0.9);
        std::fs::write(assets.join("crate.glb"), {
            let mut bytes = std::fs::read(assets.join("crate.glb")).unwrap();
            bytes.extend_from_slice(&[0; 4]);
            bytes
        })
        .unwrap();
        assert_eq!(db.rescan().0, [Change::Changed(id)]);
        std::fs::remove_file(assets.join("crate.glb")).unwrap();
        let (changes, _) = db.rescan();
        assert_eq!(
            changes,
            [Change::Removed {
                id,
                path: PathBuf::from("crate.glb")
            }]
        );
        assert!(assets.join("crate.glb.solmeta").exists(), "never deleted");
    }

    #[test]
    fn a_save_by_rename_is_a_change_not_a_new_asset() {
        // Many programs save by writing a temporary file and renaming it
        // over the old one.
        let (project, assets) = project("db-save-rename");
        glb(&assets.join("crate.glb"), 0.5);
        let (mut db, ..) = open(project);
        let id = db.id_at(&assets.join("crate.glb")).unwrap();
        glb(&assets.join("crate.glb.tmp"), 0.7);
        // A different size too: Windows can give two quick writes the same
        // time, and a file the same size and time looks unchanged (a real
        // save comes long after the last).
        let mut bytes = std::fs::read(assets.join("crate.glb.tmp")).unwrap();
        bytes.extend_from_slice(&[0; 4]);
        std::fs::write(assets.join("crate.glb.tmp"), bytes).unwrap();
        assert!(db.rescan().0.is_empty(), "temporary files are ignored");
        std::fs::rename(assets.join("crate.glb.tmp"), assets.join("crate.glb")).unwrap();
        assert_eq!(db.rescan().0, [Change::Changed(id)]);
    }

    #[test]
    fn a_damaged_sidecar_is_explained_and_left_alone() {
        let (project, assets) = project("db-damaged");
        glb(&assets.join("crate.glb"), 0.5);
        std::fs::write(assets.join("crate.glb.solmeta"), "id = ").unwrap();
        let (db, _, notes) = open(project);
        assert!(db.records().is_empty());
        assert_eq!(notes.len(), 1);
        assert!(notes[0].contains("crate.glb"));
        assert_eq!(
            std::fs::read_to_string(assets.join("crate.glb.solmeta")).unwrap(),
            "id = ",
            "not overwritten"
        );
    }

    #[test]
    fn copying_in_takes_the_files_a_model_reads_and_a_free_name() {
        let (project, assets) = project("db-copy-in");
        let outside = test_folder("db-copy-in-source");
        std::fs::create_dir_all(&outside).unwrap();
        let mut b = assets::sample::Builder::default();
        b.add_cube_mesh(0.5, None);
        b.json_nodes = vec![r#"{"mesh":0}"#.into()];
        std::fs::write(outside.join("crate.gltf"), b.json(Some("crate.bin"))).unwrap();
        std::fs::write(outside.join("crate.bin"), &b.bin).unwrap();
        let (mut db, ..) = open(project);
        let first = db.copy_in(&outside.join("crate.gltf")).unwrap();
        let second = db.copy_in(&outside.join("crate.gltf")).unwrap();
        assert_eq!(first, assets.join("crate.gltf"));
        assert_eq!(second, assets.join("crate 2.gltf"));
        assert!(assets.join("crate.bin").exists());
        let (changes, _) = db.rescan();
        assert_eq!(changes.len(), 2);
        // Changing the .bin changes the model.
        let id = db.id_at(&first).unwrap();
        let mut bin = b.bin.clone();
        bin.extend_from_slice(&[0; 8]);
        std::fs::write(assets.join("crate.bin"), &bin).unwrap();
        let (changes, _) = db.rescan();
        assert!(changes.contains(&Change::Changed(id)));
    }

    #[test]
    fn part_ids_are_written_once_and_kept() {
        let (project, assets) = project("db-parts");
        glb(&assets.join("crate.glb"), 0.5);
        let (db, ..) = open(project);
        let id = db.id_at(&assets.join("crate.glb")).unwrap();
        let file = db.file(id).unwrap();
        let model = assets::load(&file, &assets::Limits::default()).unwrap();
        let ids = db.update_parts(id, &model).unwrap();
        assert!(!ids.is_empty());
        let written = std::fs::metadata(sidecar::path_for(&file))
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(db.update_parts(id, &model).unwrap(), ids);
        let again = std::fs::metadata(sidecar::path_for(&file))
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(written, again, "unchanged sidecars aren't rewritten");
    }
}
