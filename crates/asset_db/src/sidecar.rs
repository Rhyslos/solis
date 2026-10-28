//! Sidecars: `<file>.solmeta` beside each asset, holding its ID, its
//! importer settings and the stable IDs of its parts
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)).
//! They're small TOML files made to be committed with the asset, so the ID
//! travels with it through Git and renames.

use crate::project::{read_text_limited, write_atomically};
use reflect::AssetId;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The sidecar's extension, added after the asset's own.
pub const EXTENSION: &str = "solmeta";
/// The format version this build writes and reads.
pub const FORMAT: u32 = 1;

/// Sidecars' version and upgraders (Phase 8, C1): each step turns a
/// sidecar's table at one version into the next, before it's read. Every
/// version ever released keeps an example file in `tests/fixtures/sidecar/`.
pub static SIDECAR_FORMAT: reflect::upgrade::Format<toml::Table> = reflect::upgrade::Format {
    name: "sidecar",
    current: FORMAT,
    version_of: crate::project::format_key_version,
    steps: &[],
};
/// The biggest sidecar read (a model with tens of thousands of parts fits).
pub const MAX_BYTES: u64 = 8 << 20;

/// The sidecar of the asset at `asset`.
pub fn path_for(asset: &Path) -> PathBuf {
    let mut path = asset.as_os_str().to_owned();
    path.push(".");
    path.push(EXTENSION);
    PathBuf::from(path)
}

/// The asset a sidecar at `sidecar` belongs to, if it is one.
pub fn asset_for(sidecar: &Path) -> Option<PathBuf> {
    let name = sidecar.file_name()?.to_str()?;
    let stem = name.strip_suffix(&format!(".{EXTENSION}"))?;
    (!stem.is_empty()).then(|| sidecar.with_file_name(stem))
}

/// What kind of part of an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PartKind {
    /// A node of a model's tree.
    Node,
    /// A mesh.
    Mesh,
    /// A material.
    Material,
}

/// One part, as the file names it, before it has an ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartKey {
    /// What kind.
    pub kind: PartKind,
    /// Its name with its parents' (nodes: "Ship/Mast/Sail"), or its own.
    pub path: String,
    /// Its place among the parts of its kind in the file.
    pub index: usize,
}

/// One part's stable ID, and how it was last found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    /// The ID, unique within the asset, never reused.
    pub id: u32,
    /// What kind.
    pub kind: PartKind,
    /// Its name path when last imported.
    pub path: String,
    /// Its place among its kind when last imported.
    pub index: usize,
    /// Whether the last import no longer had it. Kept, so references say
    /// "missing" instead of pointing at something else.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub missing: bool,
}

/// What a sidecar holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sidecar {
    /// The format version.
    pub format: u32,
    /// The asset's ID.
    pub id: AssetId,
    /// The asset's content hash when last seen (hex), so it can be found
    /// again after a rename outside Solis.
    pub hash: String,
    /// Importer settings (none yet).
    #[serde(default)]
    pub import: toml::Table,
    /// The parts' IDs.
    #[serde(default, rename = "part")]
    pub parts: Vec<Part>,
}

impl Sidecar {
    /// A new sidecar for content with `hash`, with a fresh ID.
    pub fn new(hash: u64) -> Self {
        Self {
            format: FORMAT,
            id: AssetId::random(),
            hash: format!("{hash:016x}"),
            import: toml::Table::new(),
            parts: Vec::new(),
        }
    }

    /// The content hash it recorded.
    pub fn content_hash(&self) -> Option<u64> {
        u64::from_str_radix(&self.hash, 16).ok()
    }

    /// Reads the sidecar at `path`. Damaged, oversized or newer ones are
    /// refused with a plain reason.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = read_text_limited(path, MAX_BYTES).map_err(|e| e.to_string())?;
        Self::parse(&text)
    }

    /// Reads a sidecar from its text.
    pub fn parse(text: &str) -> Result<Self, String> {
        // A current sidecar (nearly all of them) is read straight into
        // its fields; only another version goes through the upgrade path,
        // as a table (the Phase 8 review: opening a big project read every
        // sidecar twice over).
        let current = crate::project::format_key_version(text) == Some(FORMAT);
        let sidecar: Self = match current {
            true => toml::from_str(text).map_err(|e| e.message().to_string())?,
            false => crate::project::read_toml_versioned(text, &SIDECAR_FORMAT)?,
        };
        let mut ids = std::collections::HashSet::new();
        if let Some(part) = sidecar.parts.iter().find(|p| !ids.insert(p.id)) {
            return Err(format!("two parts share the ID {}", part.id));
        }
        Ok(sidecar)
    }

    /// The sidecar as text, with a note for whoever opens it.
    pub fn to_text(&self) -> String {
        let body = toml::to_string(self).unwrap_or_default();
        format!(
            "# Kept by Solis beside its asset: commit it with the asset, and move or\n# rename them together (or let Solis do it).\n{body}"
        )
    }

    /// Writes it to `path`, only if that changes the file.
    pub fn write(&self, path: &Path) -> std::io::Result<bool> {
        let text = self.to_text();
        if read_text_limited(path, MAX_BYTES).is_ok_and(|old| old == text) {
            return Ok(false);
        }
        write_atomically(path, text.as_bytes())?;
        Ok(true)
    }

    /// [`Sidecar::write`] for a sidecar already there, in the project at
    /// `root`: one in an older format is copied to the backups first
    /// (Phase 8; the review found sidecars rewritten without a copy).
    pub fn write_in(&self, root: &Path, path: &Path) -> std::io::Result<bool> {
        let text = self.to_text();
        if read_text_limited(path, MAX_BYTES).is_ok_and(|old| old == text) {
            return Ok(false);
        }
        crate::backup::save(root, path, text.as_bytes(), &SIDECAR_FORMAT, |new| {
            let again = Self::parse(new)?.to_text();
            if again == new {
                Ok(())
            } else {
                Err("it read back differently".into())
            }
        })
        .map_err(std::io::Error::other)?;
        Ok(true)
    }

    /// Gives each of `keys` a part ID: an old part with the same kind and
    /// name path first; then, among parts sharing a name, the same place in
    /// order; then the same index; otherwise a new ID. Old parts not found
    /// are kept, marked missing. Returns each key's ID, in order.
    pub fn assign_parts(&mut self, keys: &[PartKey]) -> Vec<u32> {
        use std::collections::HashMap;
        let mut taken = vec![false; self.parts.len()];
        let mut ids: Vec<Option<u32>> = vec![None; keys.len()];
        // Every lookup through a map, so a model with many thousands of
        // parts is matched in one pass, not one pass per part.
        let mut old_by_path: HashMap<(PartKind, &str), Vec<usize>> = HashMap::new();
        let mut old_by_index: HashMap<(PartKind, usize), Vec<usize>> = HashMap::new();
        for (j, p) in self.parts.iter().enumerate() {
            old_by_path.entry((p.kind, &p.path)).or_default().push(j);
            old_by_index.entry((p.kind, p.index)).or_default().push(j);
        }
        let mut new_count: HashMap<(PartKind, &str), usize> = HashMap::new();
        for k in keys {
            *new_count.entry((k.kind, &k.path)).or_default() += 1;
        }
        // 1. A unique name path on both sides.
        for (i, key) in keys.iter().enumerate() {
            let at = (key.kind, key.path.as_str());
            if new_count.get(&at) == Some(&1)
                && let Some([j]) = old_by_path.get(&at).map(Vec::as_slice)
            {
                taken[*j] = true;
                ids[i] = Some(self.parts[*j].id);
            }
        }
        // 2. Duplicated names: the nth with this name gets the old nth.
        let mut seen: HashMap<(PartKind, &str), usize> = HashMap::new();
        for (i, key) in keys.iter().enumerate() {
            let at = (key.kind, key.path.as_str());
            let nth = seen.entry(at).or_default();
            let this = *nth;
            *nth += 1;
            if ids[i].is_some() {
                continue;
            }
            if let Some(&j) = old_by_path.get(&at).and_then(|old| old.get(this))
                && !taken[j]
            {
                taken[j] = true;
                ids[i] = Some(self.parts[j].id);
            }
        }
        // 3. The same index, for renamed parts.
        for (i, key) in keys.iter().enumerate() {
            if ids[i].is_some() {
                continue;
            }
            if let Some(&j) = old_by_index
                .get(&(key.kind, key.index))
                .and_then(|old| old.iter().find(|&&j| !taken[j]))
            {
                taken[j] = true;
                ids[i] = Some(self.parts[j].id);
            }
        }
        // 4. New IDs, never reusing an old one (nor running past the
        // biggest number: a damaged or hand-made sidecar may use it).
        let mut used: std::collections::HashSet<u32> = self.parts.iter().map(|p| p.id).collect();
        let mut next = self
            .parts
            .iter()
            .map(|p| p.id)
            .max()
            .map_or(Some(0), |m| m.checked_add(1));
        let mut fresh = || -> u32 {
            let id = match next {
                Some(id) => {
                    next = id.checked_add(1);
                    id
                }
                // Past the end: the first number not yet used.
                None => (0..=u32::MAX).find(|n| !used.contains(n)).unwrap_or(0),
            };
            used.insert(id);
            id
        };
        let ids: Vec<u32> = ids
            .into_iter()
            .map(|id| id.unwrap_or_else(&mut fresh))
            .collect();
        // The table now: every part found, then those that went missing.
        let mut parts: Vec<Part> = keys
            .iter()
            .zip(&ids)
            .map(|(key, &id)| Part {
                id,
                kind: key.kind,
                path: key.path.clone(),
                index: key.index,
                missing: false,
            })
            .collect();
        for (j, old) in self.parts.iter().enumerate() {
            if !taken[j] {
                parts.push(Part {
                    missing: true,
                    ..old.clone()
                });
            }
        }
        self.parts = parts;
        ids
    }
}

/// The longest name path kept as it is; a longer one (only a very deep
/// tree has one) is kept as its last name and a fingerprint of the rest, so
/// a damaged or hostile model can't make paths that fill the memory.
const MAX_PATH: usize = 1024;

/// `parent/name`, or a short stand-in when that would be too long.
fn joined(parent: &str, name: &str) -> String {
    if parent.len() + name.len() < MAX_PATH {
        return format!("{parent}/{name}");
    }
    // The project's own hash: the same in every version of Solis, as the
    // path is kept in the sidecar.
    let mut hash = crate::hash::ContentHash::default();
    hash.update(parent.as_bytes());
    hash.update(b"/");
    hash.update(name.as_bytes());
    let short: String = name.chars().take(64).collect();
    format!("…{:016x}/{short}", hash.finish())
}

/// The parts of a model, as keys: its nodes by name path, its meshes by
/// the first node showing them, its materials by name.
pub fn model_parts(model: &assets::Model) -> Vec<PartKey> {
    let mut paths: Vec<String> = Vec::with_capacity(model.nodes.len());
    let mut keys = Vec::new();
    for (index, node) in model.nodes.iter().enumerate() {
        let path = match node.parent.and_then(|p| paths.get(p)) {
            Some(parent) => joined(parent, &node.name),
            None => node.name.clone(),
        };
        keys.push(PartKey {
            kind: PartKind::Node,
            path: path.clone(),
            index,
        });
        paths.push(path);
    }
    // Each mesh's first node, found in one pass.
    let mut owner: Vec<Option<usize>> = vec![None; model.meshes.len()];
    for (n, node) in model.nodes.iter().enumerate() {
        for &mesh in &node.meshes {
            if let Some(slot) = owner.get_mut(mesh)
                && slot.is_none()
            {
                *slot = Some(n);
            }
        }
    }
    for (index, owner) in owner.into_iter().enumerate() {
        let owner = owner
            .and_then(|n| paths.get(n))
            .map_or_else(String::new, Clone::clone);
        keys.push(PartKey {
            kind: PartKind::Mesh,
            path: format!("{owner}#mesh"),
            index,
        });
    }
    for (index, material) in model.materials.iter().enumerate() {
        keys.push(PartKey {
            kind: PartKind::Material,
            path: material.name.clone(),
            index,
        });
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_ids_never_run_past_the_biggest_number_or_repeat() {
        // A hand-edited sidecar already using the biggest ID.
        let mut sidecar = Sidecar::new(1);
        sidecar.parts.push(Part {
            id: u32::MAX,
            kind: PartKind::Node,
            path: "Old".into(),
            index: 0,
            missing: false,
        });
        let ids = sidecar.assign_parts(&[
            key(PartKind::Node, "Old", 0),
            key(PartKind::Node, "New", 1),
            key(PartKind::Node, "Newer", 2),
        ]);
        assert_eq!(ids[0], u32::MAX, "kept");
        let unique: std::collections::HashSet<u32> = ids.iter().copied().collect();
        assert_eq!(unique.len(), 3, "{ids:?}");
        // Two parts sharing an ID make the sidecar damaged, not quietly
        // wrong.
        let mut text = sidecar.to_text();
        text = text.replace(&format!("id = {}", ids[1]), &format!("id = {}", ids[2]));
        assert!(Sidecar::parse(&text).unwrap_err().contains("share the ID"));
    }

    #[test]
    fn a_very_deep_tree_keeps_short_stable_paths_and_matches_quickly() {
        // A hostile model: a chain of 20,000 nodes. Paths stay short (not
        // 20,000 names each), the same every time, and IDs are found in
        // one pass.
        let mut model = assets::load_bytes(
            &assets::sample::sample_glb(),
            None,
            "deep",
            &assets::Limits::default(),
        )
        .unwrap();
        model.nodes = (0..20_000usize)
            .map(|i| assets::Node {
                name: format!("n{i}"),
                parent: i.checked_sub(1),
                local: glam::Mat4::IDENTITY,
                world: glam::Mat4::IDENTITY,
                meshes: Vec::new(),
            })
            .collect();
        let keys = model_parts(&model);
        assert!(keys.iter().all(|k| k.path.len() <= MAX_PATH + 100));
        assert_eq!(model_parts(&model), keys, "the same every time");
        let nodes: Vec<&str> = keys
            .iter()
            .filter(|k| k.kind == PartKind::Node)
            .map(|k| k.path.as_str())
            .collect();
        let unique: std::collections::HashSet<&str> = nodes.iter().copied().collect();
        assert_eq!(unique.len(), nodes.len(), "every node told apart");
        let mut sidecar = Sidecar::new(1);
        let first = sidecar.assign_parts(&keys);
        let again = sidecar.assign_parts(&keys);
        assert_eq!(first, again, "a re-import keeps every ID");
    }

    fn key(kind: PartKind, path: &str, index: usize) -> PartKey {
        PartKey {
            kind,
            path: path.into(),
            index,
        }
    }

    #[test]
    fn sidecars_round_trip_through_text() {
        let mut sidecar = Sidecar::new(0xABCD);
        sidecar.assign_parts(&[key(PartKind::Node, "Ship", 0)]);
        let text = sidecar.to_text();
        assert!(text.starts_with("# Kept by Solis"));
        assert_eq!(Sidecar::parse(&text).unwrap(), sidecar);
        assert_eq!(sidecar.content_hash(), Some(0xABCD));
        assert_eq!(
            path_for(Path::new("a/crate.glb")),
            Path::new("a/crate.glb.solmeta")
        );
        assert_eq!(
            asset_for(Path::new("a/crate.glb.solmeta")),
            Some(PathBuf::from("a/crate.glb"))
        );
        assert_eq!(asset_for(Path::new("a/.solmeta")), None);
        assert_eq!(asset_for(Path::new("a/crate.glb")), None);
    }

    #[test]
    fn damaged_or_newer_sidecars_are_refused() {
        for text in [
            "",
            "id = 5",
            "format = 1\nid = \"not an id\"\nhash = \"0\"",
            "format = 9\nid = \"0000000000001\"\nhash = \"0\"",
            "format = 1\nid = \"0000000000000\"\nhash = \"0\"",
        ] {
            assert!(Sidecar::parse(text).is_err(), "{text}");
        }
        assert!(
            Sidecar::parse("format = 1\nid = \"0000000000001\"\nhash = \"0\"").is_ok(),
            "parts and settings are optional"
        );
    }

    #[test]
    fn rubbish_never_crashes_the_reader() {
        // A small fuzz: every prefix and many corruptions of a real one.
        let mut sidecar = Sidecar::new(7);
        sidecar.assign_parts(&[
            key(PartKind::Node, "A", 0),
            key(PartKind::Material, "Paint", 0),
        ]);
        let text = sidecar.to_text();
        for end in 0..text.len() {
            if text.is_char_boundary(end) {
                let _ = Sidecar::parse(&text[..end]);
            }
        }
        let bytes = text.as_bytes();
        let mut seed = 12345u64;
        for _ in 0..2000 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mut broken = bytes.to_vec();
            let at = (seed as usize) % broken.len();
            broken[at] = (seed >> 32) as u8;
            if let Ok(text) = std::str::from_utf8(&broken) {
                let _ = Sidecar::parse(text);
            }
        }
    }

    #[test]
    fn parts_keep_their_ids_through_a_reexport_with_nodes_reordered() {
        let mut sidecar = Sidecar::new(1);
        let first = sidecar.assign_parts(&[
            key(PartKind::Node, "Ship", 0),
            key(PartKind::Node, "Ship/Mast", 1),
            key(PartKind::Node, "Ship/Wheel", 2),
            key(PartKind::Material, "Wood", 0),
        ]);
        // Blender exported again: the nodes come out in another order and
        // the material got a new index.
        let again = sidecar.assign_parts(&[
            key(PartKind::Node, "Ship/Wheel", 0),
            key(PartKind::Node, "Ship", 1),
            key(PartKind::Node, "Ship/Mast", 2),
            key(PartKind::Material, "Wood", 3),
        ]);
        assert_eq!(again, [first[2], first[0], first[1], first[3]]);
    }

    #[test]
    fn duplicate_names_match_in_order_renames_by_index_and_lost_parts_stay() {
        let mut sidecar = Sidecar::new(1);
        let first = sidecar.assign_parts(&[
            key(PartKind::Node, "Crate", 0),
            key(PartKind::Node, "Crate", 1),
            key(PartKind::Node, "Barrel", 2),
            key(PartKind::Node, "Lamp", 3),
        ]);
        // The barrel was renamed, the lamp deleted, a rope added.
        let again = sidecar.assign_parts(&[
            key(PartKind::Node, "Crate", 0),
            key(PartKind::Node, "Crate", 1),
            key(PartKind::Node, "Keg", 2),
            key(PartKind::Node, "Rope", 4),
        ]);
        assert_eq!(&again[..3], &first[..3]);
        assert!(!first.contains(&again[3]), "a new part gets a new ID");
        let lamp = sidecar.parts.iter().find(|p| p.id == first[3]).unwrap();
        assert!(lamp.missing, "kept, marked missing");
        // It comes back with its old ID.
        let back = sidecar.assign_parts(&[key(PartKind::Node, "Lamp", 3)]);
        assert_eq!(back, [first[3]]);
    }

    #[test]
    fn a_sidecar_not_in_this_format_is_copied_before_it_s_rewritten() {
        // Regression (the Phase 8 review): sidecars were rewritten with no
        // copy when a model was imported again.
        let root = std::env::temp_dir().join(format!("solis-sidecar-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("assets")).unwrap();
        let path = root.join("assets/crate.glb.solmeta");
        std::fs::write(&path, "format = 9\nid = \"0000000000001\"\nhash = \"0\"\n").unwrap();
        assert!(Sidecar::new(1).write_in(&root, &path).unwrap());
        let copied = crate::backup::folder(&root);
        assert!(copied.exists(), "kept in the backups");
        // Current now: written again with no second copy.
        let mut again = Sidecar::read(&path).unwrap();
        again.hash = "2".into();
        assert!(again.write_in(&root, &path).unwrap());
        assert_eq!(std::fs::read_dir(&copied).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
