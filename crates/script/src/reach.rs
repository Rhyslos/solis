//! Which of the editor's own files a program reaches. The editor writes a
//! Lust file for each thing made in it (an animator's, a tile map's:
//! decisions 0078 and 0079) into the engine's folder, and each fetches the
//! engine object its methods call (`Animators`, `Maps`). Lust makes every
//! file in that folder part of every program, but keeps an engine object
//! only once the program reaches its file from the entry: so in a game
//! that fetched neither the map's file nor `solis/tiles`, the map's class
//! stayed and its `Maps` didn't, and the game was told "there's nothing
//! called Maps here" in a file it never wrote. Handed to Lust without the
//! files no fetch reaches, every program checks whatever the editor wrote,
//! and a game reaches a thing's file, and so its engine object, exactly
//! as before: by fetching it. Nothing else can use a file left out: a
//! component holds a map's class only by fetching its file.

use std::borrow::Cow;
use std::collections::HashMap;

/// Whether `path` (inside the assets, `/` between folders) is a file the
/// editor writes for a thing made in it: `solis/animators/…` or
/// `solis/maps/…`.
pub(crate) fn made_in_an_editor(path: &str) -> bool {
    let Some((folder, rest)) = path
        .strip_prefix(area::lust_data::ENGINE_FOLDER)
        .and_then(|rest| rest.strip_prefix('/'))
        .and_then(|rest| rest.split_once('/'))
    else {
        return false;
    };
    [crate::animator_code::FOLDER, crate::map_code::FOLDER].contains(&folder)
        && rest.ends_with(".lst")
}

/// `files` as Lust is handed them for the program that starts at `entry`:
/// without the editor's files ([`made_in_an_editor`]) that no fetch
/// reaches from `entry`, on and on. Borrowed when there's none to leave
/// out.
pub(crate) fn for_lust<'a>(
    files: &'a [(String, String)],
    entry: &str,
) -> Cow<'a, [(String, String)]> {
    if !files.iter().any(|(path, _)| made_in_an_editor(path)) {
        return Cow::Borrowed(files);
    }
    let place: HashMap<&str, usize> = files
        .iter()
        .enumerate()
        .map(|(at, (path, _))| (path.as_str(), at))
        .collect();
    let mut reached = vec![false; files.len()];
    let mut todo: Vec<usize> = place.get(entry).copied().into_iter().collect();
    while let Some(at) = todo.pop() {
        if std::mem::replace(&mut reached[at], true) {
            continue;
        }
        for path in area::lust_guard::fetched(&files[at].1) {
            if let Some(&target) = place.get(file_of(path).as_str()) {
                todo.push(target);
            }
        }
    }
    let kept = files
        .iter()
        .zip(reached)
        .filter(|((path, _), reached)| *reached || !made_in_an_editor(path))
        .map(|(file, _)| file.clone())
        .collect();
    Cow::Owned(kept)
}

/// The file a fetch's path names, as Lust finds it among the files:
/// `.lst` added, and `.` and `..` worked out (`a/./b/../c` is `a/c.lst`).
fn file_of(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            // Lust's own way: a `..` takes away the part before it,
            // whatever it is, and stays only when there's none.
            ".." => {
                if parts.pop().is_none() {
                    parts.push("..");
                }
            }
            part => parts.push(part),
        }
    }
    format!("{}.lst", parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, text: &str) -> (String, String) {
        (path.to_string(), text.to_string())
    }

    fn kept(files: &[(String, String)], entry: &str) -> Vec<String> {
        for_lust(files, entry)
            .iter()
            .map(|(path, _)| path.clone())
            .collect()
    }

    #[test]
    fn only_the_files_made_in_an_editor_are_its_own() {
        for (path, made) in [
            ("solis/animators/farmer.lst", true),
            ("solis/animators/people/farmer.lst", true),
            ("solis/maps/maps/farm.lst", true),
            ("solis/maps/cave.lst", true),
            ("solis/world.lst", false),
            ("solis/project.lst", false),
            ("solis/kit/cameras.lst", false),
            ("solis/data/tilemap.lst", false),
            ("solis/maps", false),
            ("solis/maps/", false),
            ("solis/maps/notes.txt", false),
            ("maps/farm.lst", false),
            ("game/solis/maps/farm.lst", false),
            ("solisx/maps/farm.lst", false),
            ("", false),
        ] {
            assert_eq!(made_in_an_editor(path), made, "{path}");
        }
    }

    #[test]
    fn the_editor_s_files_no_fetch_reaches_are_left_out() {
        let files = [
            file("main.lst", "fetch \"farming/field\":Field;\nend\n"),
            file(
                "farming/field.lst",
                "fetch \"solis/maps/farm\":FarmMap;\nend\n",
            ),
            file("loose.lst", "fetch \"solis/maps/cave\":CaveMap;\nend\n"),
            file("solis/maps/farm.lst", "fetch \"solis/tiles\":Maps;\nend\n"),
            file("solis/maps/cave.lst", "fetch \"solis/tiles\":Maps;\nend\n"),
            file("solis/animators/cow.lst", "end\n"),
            file("solis/tiles.lst", "end\n"),
        ];
        // The game's own files and the engine's stay, reached or not.
        assert_eq!(
            kept(&files, "main.lst"),
            [
                "main.lst",
                "farming/field.lst",
                "loose.lst",
                "solis/maps/farm.lst",
                "solis/tiles.lst"
            ]
        );
        // Another entry reaches others.
        assert_eq!(
            kept(&files, "loose.lst"),
            [
                "main.lst",
                "farming/field.lst",
                "loose.lst",
                "solis/maps/cave.lst",
                "solis/tiles.lst"
            ]
        );
        // An entry that isn't there reaches nothing.
        assert_eq!(kept(&files, "gone.lst").len(), 4);
    }

    #[test]
    fn with_none_of_the_editor_s_files_nothing_is_copied() {
        let files = [file("main.lst", "end\n"), file("solis/world.lst", "end\n")];
        assert!(matches!(for_lust(&files, "main.lst"), Cow::Borrowed(_)));
    }

    #[test]
    fn a_path_is_worked_out_as_lust_works_it_out() {
        assert_eq!(file_of("solis/maps/farm"), "solis/maps/farm.lst");
        assert_eq!(file_of("./solis//maps/./farm"), "solis/maps/farm.lst");
        assert_eq!(file_of("solis/kit/../maps/farm"), "solis/maps/farm.lst");
        assert_eq!(file_of("../farm"), "../farm.lst");
        assert_eq!(file_of("../../a/../b"), "b.lst");
        assert_eq!(file_of(""), ".lst");
        let files = [
            file(
                "main.lst",
                "fetch \"solis/kit/../maps/./farm\":FarmMap;\nend\n",
            ),
            file("solis/maps/farm.lst", "end\n"),
        ];
        assert_eq!(kept(&files, "main.lst").len(), 2);
    }

    #[test]
    fn files_fetching_each_other_in_a_ring_end() {
        let files = [
            file("main.lst", "fetch \"solis/maps/a\":A;\nend\n"),
            file("solis/maps/a.lst", "fetch \"solis/maps/b\":B;\nend\n"),
            file(
                "solis/maps/b.lst",
                "fetch \"solis/maps/a\":A;\nfetch \"main\":M;\nend\n",
            ),
        ];
        assert_eq!(kept(&files, "main.lst").len(), 3);
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(256))]
        /// Any files at all: no panic, the game's and the engine's own are
        /// all kept, and the entry's every fetch of an editor's file is
        /// kept too.
        #[test]
        fn any_files_keep_the_game_s_own_and_what_the_entry_fetches(
            texts in proptest::collection::vec(".{0,80}", 1..6),
            fetches in proptest::collection::vec(0usize..6, 0..4),
        ) {
            let paths = ["main.lst", "solis/maps/a.lst", "solis/animators/b/c.lst", "solis/world.lst", "game.lst", "solis/maps/d.lst"];
            let mut files: Vec<(String, String)> = texts
                .iter()
                .zip(paths)
                .map(|(text, path)| (path.to_string(), text.clone()))
                .collect();
            let wanted: Vec<&str> = fetches.iter().filter_map(|at| paths.get(*at).copied()).collect();
            // Before the text, which may leave a comment or text open.
            let fetching: String = wanted
                .iter()
                .map(|path| format!("fetch \"{}\":X;\n", path.trim_end_matches(".lst")))
                .collect();
            files[0].1.insert_str(0, &fetching);
            let kept = kept(&files, "main.lst");
            for (path, _) in &files {
                if !made_in_an_editor(path) || (wanted.contains(&path.as_str())) {
                    proptest::prop_assert!(kept.contains(path), "{} left out", path);
                }
            }
        }
    }
}
