//! New project's name rules: the name typed becomes the project's folder
//! (`<Create in>\<Name>`, decision 0118), so it must be a folder name
//! Windows takes. Each rule broken has its own plain reason, shown under
//! the field; the app checks again when Create is pressed.

use std::path::{Path, PathBuf};

/// The most letters a project's name has (as Rename… allows).
pub(crate) const MAX_NAME: usize = 100;

/// The longest path Windows takes everywhere, in characters. The
/// project's own files go deeper, so its folder keeps well under it.
pub(crate) const MAX_PATH: usize = 260;

/// Room left under [`MAX_PATH`] for the project's own files and folders
/// (`.solis\cache\` and a file's name, `assets\…`).
const ROOM_INSIDE: usize = 60;

/// Characters Windows never allows in a file or folder's name.
const FORBIDDEN: &str = "<>:\"/\\|?*";

/// Why `name` can't be a new project's name (and so its folder's), if it
/// can't: empty, too long, holding a character Windows forbids or a
/// line break, a name Windows keeps for itself (`CON`, `NUL`, `COM1`…),
/// starting with a dot, or ending in a dot or a space.
pub fn name_problem(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() {
        return Some("Give the project a name: its folder is called the same.".into());
    }
    if name.chars().count() > MAX_NAME {
        return Some(format!("Names are at most {MAX_NAME} letters."));
    }
    if name.chars().any(char::is_control) {
        return Some("A name can't hold line breaks or tabs.".into());
    }
    if let Some(bad) = name.chars().find(|c| FORBIDDEN.contains(*c)) {
        return Some(format!(
            "A folder's name can't hold {bad} on Windows (nor any of < > : \" / \\ | ? *)."
        ));
    }
    if name.starts_with('.') {
        return Some(
            "A name starting with a dot makes a hidden folder: start with a letter.".into(),
        );
    }
    if name.ends_with('.') {
        return Some("Windows drops a dot at the end of a folder's name: leave it off.".into());
    }
    if assets::reserved_on_windows(name) {
        let stem = name.split('.').next().unwrap_or(name).trim();
        return Some(format!(
            "Windows keeps {} for itself (as it does CON, NUL, COM1 and LPT1): choose another name.",
            stem.to_uppercase()
        ));
    }
    None
}

/// The folder a project called `name` gets inside `create_in`.
pub(crate) fn folder_for(create_in: &Path, name: &str) -> PathBuf {
    create_in.join(name.trim())
}

/// Why the folder `folder` is too deep for Windows, if it is: its path,
/// with room for the project's own files inside it, must stay under
/// [`MAX_PATH`] characters.
pub(crate) fn path_problem(folder: &Path) -> Option<String> {
    let length = folder.as_os_str().to_string_lossy().chars().count();
    (length + ROOM_INSIDE > MAX_PATH).then(|| {
        format!(
            "That folder's path is {length} characters long: Windows needs it under {}. Choose \
             a shorter name, or a folder higher up with Browse….",
            MAX_PATH - ROOM_INSIDE
        )
    })
}

/// What is at the folder a new project would be made in, as the app found
/// it (it looks on a worker, never on the window's thread).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderState {
    /// Nothing there yet: it is made.
    Free,
    /// An empty folder: the project goes in it.
    Empty,
    /// A folder with things in it: never used, so nothing is overwritten.
    Taken,
    /// A file of that name.
    AFile,
    /// The disk couldn't say (and why).
    Unknown(String),
}

impl FolderState {
    /// Why the project can't be made there, if it can't.
    pub fn problem(&self) -> Option<String> {
        match self {
            Self::Free | Self::Empty => None,
            Self::Taken => Some(
                "That folder already exists and isn't empty: choose another name, so nothing \
                 in it is overwritten."
                    .into(),
            ),
            Self::AFile => {
                Some("A file of that name is already there: choose another name.".into())
            }
            Self::Unknown(why) => Some(format!("Couldn't look at that folder: {why}.")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_names_pass() {
        for name in [
            "Farm Days",
            "Tiny Kart",
            "ゲーム",
            "a",
            "Cave-Run_2 (copy)",
            "  Spaced  ",
        ] {
            assert_eq!(name_problem(name), None, "{name}");
        }
        assert_eq!(
            name_problem(&"x".repeat(MAX_NAME)),
            None,
            "just long enough"
        );
    }

    #[test]
    fn every_broken_rule_says_why_in_plain_words() {
        let cases: [(&str, &str); 14] = [
            ("", "Give the project a name"),
            ("   ", "Give the project a name"),
            ("a\nb", "line breaks"),
            ("tab\there", "line breaks"),
            ("Farm: Days", "can't hold :"),
            ("a/b", "can't hold /"),
            ("a\\b", "can't hold \\"),
            ("what?", "can't hold ?"),
            ("<x>", "can't hold <"),
            (".hidden", "hidden folder"),
            ("Ends.", "dot at the end"),
            ("CON", "keeps CON"),
            ("nul.txt", "keeps NUL"),
            ("com1", "keeps COM1"),
        ];
        for (name, why) in cases {
            let said = name_problem(name).unwrap_or_else(|| panic!("{name:?} passed"));
            assert!(said.contains(why), "{name:?}: {said}");
            assert!(said.ends_with('.'), "a sentence: {said}");
        }
        let long = name_problem(&"x".repeat(MAX_NAME + 1)).unwrap();
        assert!(long.contains("at most 100"), "{long}");
        assert!(name_problem("LPT9").is_some() && name_problem("Console").is_none());
    }

    #[test]
    fn the_folder_is_the_name_inside_create_in() {
        let base = Path::new("projects");
        assert_eq!(folder_for(base, " Farm Days "), base.join("Farm Days"));
    }

    #[test]
    fn a_folder_too_deep_for_windows_is_explained() {
        assert_eq!(
            path_problem(Path::new("C:/Users/Sam/Documents/Projects/Farm")),
            None
        );
        let deep: PathBuf = (0..30).map(|_| "folder").collect();
        let said = path_problem(&deep).unwrap();
        assert!(
            said.contains("characters long") && said.contains("Browse"),
            "{said}"
        );
    }

    #[test]
    fn only_a_free_or_empty_folder_can_take_a_project() {
        assert_eq!(FolderState::Free.problem(), None);
        assert_eq!(FolderState::Empty.problem(), None);
        assert!(
            FolderState::Taken
                .problem()
                .unwrap()
                .contains("isn't empty")
        );
        assert!(FolderState::AFile.problem().unwrap().contains("file"));
        let why = FolderState::Unknown("no drive".into()).problem().unwrap();
        assert!(why.contains("no drive"));
    }

    /// Thousands of random names: each is passed or refused with a reason,
    /// and one passed is a usable folder name, never a crash.
    #[test]
    fn random_names_never_crash_and_passed_ones_are_usable() {
        let pieces = [
            "a", "Z", " ", ".", "CON", "nul", "com", "1", ":", "/", "\\", "?", "*", "\"", "<", ">",
            "|", "\n", "\u{0}", "é", "ゲ", "-", "_", "(", "lpt", "9", "\t",
        ];
        let mut seed: u64 = 0x0118;
        let mut next = |n: usize| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) as usize % n
        };
        for _ in 0..10_000 {
            let name: String = (0..next(8)).map(|_| pieces[next(pieces.len())]).collect();
            if name_problem(&name).is_none() {
                let trimmed = name.trim();
                assert!(!trimmed.is_empty(), "{name:?}");
                assert!(
                    !trimmed
                        .chars()
                        .any(|c| FORBIDDEN.contains(c) || c.is_control())
                );
                assert!(!assets::reserved_on_windows(trimmed), "{name:?}");
                assert!(
                    !trimmed.starts_with('.') && !trimmed.ends_with('.'),
                    "{name:?}"
                );
            }
        }
    }
}
