//! What Solis makes is kept in Lust data files
//! ([decision 0081](../../../docs/decisions/0081-what-solis-makes-is-lust.md)),
//! each named for what it holds: `farm.tilemap.lst`, `ground.tileset.lst`.
//! Every one ends in `.lst`, so Ledger opens and colours them all; the word
//! before it says what it is. Game code is any other `.lst` file.
//!
//! This is the one place the kinds are listed: the asset database, the
//! Library and the game code gatherer (which leaves data files out of the
//! game's program) all ask here.

use std::path::Path;

/// Lust's file extension.
pub const LUST_EXTENSION: &str = "lst";

/// Each kind of data file Solis writes, by the word before `.lst`.
pub const KINDS: &[&str] = &["atlas", "tilemap", "tileset"];

/// The kind of data file at `path` (`tilemap` for `farm.tilemap.lst`), if
/// it is one. Capitals in the name don't matter.
pub fn kind_of(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    let (stem, ext) = name.rsplit_once('.')?;
    if !ext.eq_ignore_ascii_case(LUST_EXTENSION) {
        return None;
    }
    let (before, kind) = stem.rsplit_once('.')?;
    if before.is_empty() {
        return None;
    }
    KINDS.iter().copied().find(|k| k.eq_ignore_ascii_case(kind))
}

/// Whether `path` is a data file of `kind`.
pub fn is(path: &Path, kind: &str) -> bool {
    kind_of(path).is_some_and(|k| k.eq_ignore_ascii_case(kind))
}

/// `path`'s name without what says its kind: `farm` for `farm.tilemap.lst`
/// (and for any other file, its name without its last extension).
pub fn stem(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    if let Some(kind) = kind_of(path) {
        let cut = name.len() - kind.len() - LUST_EXTENSION.len() - 2;
        return Some(name[..cut].to_string());
    }
    Some(path.file_stem()?.to_str()?.to_string())
}

/// `path` without the end that says its kind (`maps/farm` for
/// `maps/farm.tilemap.lst`; any other file without its last extension),
/// with `/` between folders.
pub fn path_stem(path: &str) -> String {
    let path = path.replace('\\', "/");
    let (folder, name) = path
        .rsplit_once('/')
        .map_or(("", path.as_str()), |(f, n)| (f, n));
    let stem = stem(Path::new(name)).unwrap_or_else(|| name.to_string());
    if folder.is_empty() {
        stem
    } else {
        format!("{folder}/{stem}")
    }
}

/// The file name for a data file of `kind` called `stem`: `farm.tilemap.lst`.
pub fn file_name(stem: &str, kind: &str) -> String {
    format!("{stem}.{kind}.{LUST_EXTENSION}")
}

/// The end of a data file of `kind`'s name, after its stem: `.tilemap.lst`.
pub fn ending(kind: &str) -> String {
    format!(".{kind}.{LUST_EXTENSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_data_file_says_its_kind_before_lst() {
        assert_eq!(kind_of(Path::new("maps/farm.tilemap.lst")), Some("tilemap"));
        assert_eq!(kind_of(Path::new("Farm.TileMap.LST")), Some("tilemap"));
        assert_eq!(kind_of(Path::new("ground.tileset.lst")), Some("tileset"));
        for not in [
            "main.lst",
            "farm.lst",
            ".tilemap.lst",
            "farm.tilemap",
            "farm.soltilemap",
        ] {
            assert_eq!(kind_of(Path::new(not)), None, "{not}");
        }
        assert_eq!(
            kind_of(Path::new("farm.other.lst")),
            None,
            "not a kind Solis makes"
        );
        assert!(is(Path::new("a.tilemap.lst"), "tilemap"));
        assert!(!is(Path::new("a.tileset.lst"), "tilemap"));
    }

    #[test]
    fn stems_leave_the_kind_out() {
        assert_eq!(stem(Path::new("farm.tilemap.lst")).unwrap(), "farm");
        assert_eq!(stem(Path::new("main.lst")).unwrap(), "main");
        assert_eq!(stem(Path::new("hero.aseprite")).unwrap(), "hero");
        assert_eq!(path_stem("maps/farm.tilemap.lst"), "maps/farm");
        assert_eq!(path_stem("maps\\old.soltilemap"), "maps/old");
        assert_eq!(path_stem("farm.tilemap.lst"), "farm");
        assert_eq!(file_name("farm", "tilemap"), "farm.tilemap.lst");
        assert_eq!(ending("tileset"), ".tileset.lst");
    }
}
