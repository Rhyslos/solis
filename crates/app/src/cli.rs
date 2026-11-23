//! The command line: `--version`, `--help`, or nothing to open the
//! launcher (decision 0118). A project's folder named on its own opens
//! that project in the editor straight away, as does `--sample <which>
//! <folder>`, which makes a sample project and opens it, as the
//! launcher's Learn does (the guide's picture script and the window test
//! start the editor this way); `--import <file>` brings models in as it
//! opens.

use std::ffi::OsString;
use std::path::PathBuf;

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Open the editor.
    Run(Launch),
    /// Print the version and exit.
    Version,
    /// Print how to use the program and exit.
    Help,
    /// Check the project in this folder without opening the editor, and
    /// exit (Phase 8, C4).
    CheckProject(CheckProject),
    /// Print this many new IDs, one a line, for files written by hand or
    /// by a script, and exit (Fixes 4, D3).
    NewIds(u32),
}

/// The most IDs `--new-id` prints at once.
pub const MAX_NEW_IDS: u32 = 10_000;

/// What `--check-project` checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckProject {
    /// The project's folder.
    pub folder: PathBuf,
    /// How many ticks to play its start level for, with no input
    /// (`--smoke <ticks>`, Fixes 1): none plays nothing.
    pub smoke: Option<u32>,
}

/// The most ticks `--smoke` plays: ten minutes of a game.
pub const MAX_SMOKE_TICKS: u32 = 36_000;

/// What the editor does as it opens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Launch {
    /// Models to import.
    pub imports: Vec<PathBuf>,
    /// A sample project to make in an empty folder and open.
    pub sample: Option<(Sample, PathBuf)>,
    /// A project's folder to open in the editor, skipping the launcher.
    pub project: Option<PathBuf>,
}

impl Launch {
    /// Whether the editor opens straight away, with a project: one is
    /// named, or a sample is to be made. Otherwise the launcher opens.
    pub fn skips_the_launcher(&self) -> bool {
        self.project.is_some() || self.sample.is_some()
    }
}

/// The sample projects File › Learn makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sample {
    /// The turret sample (Make the sample game…).
    Game,
    /// The UI sample.
    Ui,
    /// The playground.
    Playground,
    /// The benchmark.
    Benchmark,
}

impl Sample {
    /// Every sample, by the word the command line takes for it.
    pub const ALL: [(&'static str, Sample); 4] = [
        ("game", Sample::Game),
        ("ui", Sample::Ui),
        ("playground", Sample::Playground),
        ("benchmark", Sample::Benchmark),
    ];

    fn named(word: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(name, _)| *name == word)
            .map(|(_, sample)| *sample)
    }
}

/// Reads the arguments after the program's name. Anything unknown is
/// explained in the error.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut version = false;
    let mut launch = Launch::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--version" | "-V") => version = true,
            Some("--help" | "-h") => return Ok(Command::Help),
            Some("--new-id") => return new_ids(args),
            Some("--check-project") => match args.next() {
                Some(folder) => return check_project(PathBuf::from(folder), args),
                None => {
                    return Err(
                        "--check-project needs a project's folder after it. Try --help."
                            .to_string(),
                    );
                }
            },
            Some("--import") => match args.next() {
                Some(file) => launch.imports.push(PathBuf::from(file)),
                None => return Err("--import needs a file after it. Try --help.".to_string()),
            },
            Some("--sample") => {
                let which = args.next();
                let sample = which
                    .as_ref()
                    .and_then(|w| w.to_str())
                    .and_then(Sample::named);
                match (sample, args.next()) {
                    (Some(sample), Some(folder)) => {
                        launch.sample = Some((sample, PathBuf::from(folder)));
                    }
                    _ => {
                        return Err(
                            "--sample needs which sample (game, ui, playground or benchmark) \
                             and an empty folder after it. Try --help."
                                .to_string(),
                        );
                    }
                }
            }
            Some(word) if !word.starts_with('-') && !word.trim().is_empty() => {
                if launch.project.is_some() {
                    return Err(format!(
                        "Only one project opens at a time: {word} is a second. Try --help."
                    ));
                }
                launch.project = Some(PathBuf::from(word));
            }
            _ => {
                return Err(format!(
                    "Unknown option {}. Try --help.",
                    arg.to_string_lossy()
                ));
            }
        }
    }
    Ok(if version {
        Command::Version
    } else {
        Command::Run(launch)
    })
}

/// `--new-id`, then how many (one when nothing follows).
fn new_ids(mut rest: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let wrong = || {
        format!(
            "--new-id takes how many IDs to print, from 1 to {MAX_NEW_IDS}, or nothing for one. \
             Try --help."
        )
    };
    let count = match rest.next() {
        None => 1,
        Some(n) => n
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
            .filter(|n| (1..=MAX_NEW_IDS).contains(n))
            .ok_or_else(wrong)?,
    };
    if rest.next().is_some() {
        return Err(wrong());
    }
    Ok(Command::NewIds(count))
}

/// `count` new IDs, one a line, as files write them.
pub fn new_id_lines(count: u32) -> String {
    (0..count)
        .map(|_| format!("{}\n", reflect::Id::random()))
        .collect()
}

/// `--check-project <folder>`, then what's after it: `--smoke <ticks>`
/// or nothing.
fn check_project(
    folder: PathBuf,
    mut rest: impl Iterator<Item = OsString>,
) -> Result<Command, String> {
    let mut smoke = None;
    while let Some(arg) = rest.next() {
        if arg.to_str() != Some("--smoke") || smoke.is_some() {
            return Err(format!(
                "--check-project takes a folder and --smoke <ticks>, not {}. Try --help.",
                arg.to_string_lossy()
            ));
        }
        let ticks = rest
            .next()
            .and_then(|t| t.to_str().and_then(|t| t.parse::<u32>().ok()))
            .filter(|t| (1..=MAX_SMOKE_TICKS).contains(t));
        match ticks {
            Some(ticks) => smoke = Some(ticks),
            None => {
                return Err(format!(
                    "--smoke needs how many ticks to play after it, from 1 to \
                     {MAX_SMOKE_TICKS} (60 is a second). Try --help."
                ));
            }
        }
    }
    Ok(Command::CheckProject(CheckProject { folder, smoke }))
}

/// How to use the program, for `--help`.
pub fn usage(program: &str) -> String {
    format!(
        "Usage: {program} [<project folder>] [--import <file>]... [--sample <which> <folder>] [--version | --help]\n       \
         {program} --check-project <folder> [--smoke <ticks>]\n       \
         {program} --new-id [<count>]\n\n\
         Opens the {name} launcher, or with a project's folder, that project in the editor.\n\n\
         Options:\n  \
         <project folder> Open this project in the editor straight away\n  \
         --import <file>  Import a glTF model (.gltf or .glb) as the editor opens\n  \
         --sample <which> <folder>\n                   \
         Make a sample project (game, ui, playground or benchmark) in an\n                   \
         empty folder and open it, as the launcher's Learn does\n  \
         --check-project <folder>\n                   \
         Check a project without opening the editor: read every file, check\n                   \
         the game's code and play back recordings/checks/; the report is\n                   \
         printed and kept in .solis/check-report.txt. Exits 1 if anything failed\n  \
         --smoke <ticks>  With --check-project: also play the game's start level for\n                   \
         this many ticks (60 a second) with no input, and fail on any error\n                   \
         or warning in the Console, said with its tick\n  \
         --new-id [<count>]\n                   \
         Print new IDs, one a line (one, or up to {MAX_NEW_IDS}), for files\n                   \
         written by hand or by a script, and exit\n  \
         -V, --version    Print the version and exit\n  \
         -h, --help       Print this help and exit\n",
        name = crate::branding::APP_NAME
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn nothing_opens_the_editor() {
        assert_eq!(parse(args(&[])), Ok(Command::Run(Launch::default())));
    }

    #[test]
    fn check_project_takes_a_folder() {
        assert_eq!(
            parse(args(&["--check-project", "games/farm"])),
            Ok(Command::CheckProject(CheckProject {
                folder: PathBuf::from("games/farm"),
                smoke: None
            }))
        );
        assert_eq!(
            parse(args(&["--check-project", "games/farm", "--smoke", "60"])),
            Ok(Command::CheckProject(CheckProject {
                folder: PathBuf::from("games/farm"),
                smoke: Some(60)
            }))
        );
        for bad in [
            &["--check-project", "f", "--smoke"][..],
            &["--check-project", "f", "--smoke", "0"],
            &["--check-project", "f", "--smoke", "ten"],
            &["--check-project", "f", "--smoke", "-5"],
            &["--check-project", "f", "--smoke", "36001"],
            &["--check-project", "f", "--smoke", "5", "--smoke", "6"],
            &["--check-project", "f", "--import", "a.glb"],
        ] {
            assert!(parse(args(bad)).is_err(), "{bad:?}");
        }
        assert!(usage("solis").contains("--smoke <ticks>"));
        assert!(
            parse(args(&["--check-project"]))
                .unwrap_err()
                .contains("folder")
        );
        assert!(usage("solis").contains("--check-project <folder>"));
    }

    #[test]
    fn new_id_prints_one_or_many() {
        assert_eq!(parse(args(&["--new-id"])), Ok(Command::NewIds(1)));
        assert_eq!(parse(args(&["--new-id", "20"])), Ok(Command::NewIds(20)));
        for bad in [
            &["--new-id", "0"][..],
            &["--new-id", "ten"],
            &["--new-id", "10001"],
            &["--new-id", "2", "3"],
        ] {
            assert!(parse(args(bad)).unwrap_err().contains("--help"), "{bad:?}");
        }
        let lines = new_id_lines(50);
        let ids: std::collections::BTreeSet<&str> = lines.lines().collect();
        assert_eq!(ids.len(), 50, "every ID is new");
        for id in ids {
            // Each reads back as an ID, as a file holding it would.
            assert!(id.parse::<reflect::Id>().is_ok(), "{id}");
        }
        assert!(usage("solis").contains("--new-id [<count>]"));
    }

    #[test]
    fn version_and_help_are_understood() {
        assert_eq!(parse(args(&["--version"])), Ok(Command::Version));
        assert_eq!(parse(args(&["-V"])), Ok(Command::Version));
        assert_eq!(parse(args(&["--help"])), Ok(Command::Help));
        assert_eq!(parse(args(&["--version", "-h"])), Ok(Command::Help));
    }

    #[test]
    fn models_can_be_imported_from_the_command_line() {
        assert_eq!(
            parse(args(&["--import", "a.glb", "--import", "b c.gltf"])),
            Ok(Command::Run(Launch {
                imports: vec![PathBuf::from("a.glb"), PathBuf::from("b c.gltf")],
                sample: None,
                project: None,
            }))
        );
        let err = parse(args(&["--import"])).unwrap_err();
        assert!(err.contains("needs a file"), "{err}");
    }

    #[test]
    fn a_sample_can_be_made_from_the_command_line() {
        for (word, sample) in Sample::ALL {
            assert_eq!(
                parse(args(&["--sample", word, "games/try"])),
                Ok(Command::Run(Launch {
                    imports: Vec::new(),
                    sample: Some((sample, PathBuf::from("games/try"))),
                    project: None,
                }))
            );
        }
        for wrong in [
            &["--sample"][..],
            &["--sample", "game"],
            &["--sample", "farm", "x"],
        ] {
            let err = parse(args(wrong)).unwrap_err();
            assert!(
                err.contains("--sample needs") && err.contains("--help"),
                "{err}"
            );
        }
        assert!(usage("solis").contains("--sample <which> <folder>"));
    }

    #[test]
    fn a_project_named_on_its_own_skips_the_launcher() {
        let Ok(Command::Run(launch)) = parse(args(&["games/farm", "--import", "a.glb"])) else {
            panic!("a project to open");
        };
        assert_eq!(launch.project, Some(PathBuf::from("games/farm")));
        assert!(launch.skips_the_launcher());
        let Ok(Command::Run(plain)) = parse(args(&[])) else {
            panic!("the launcher");
        };
        assert!(!plain.skips_the_launcher(), "nothing named: the launcher");
        let Ok(Command::Run(sample)) = parse(args(&["--sample", "ui", "x"])) else {
            panic!("a sample");
        };
        assert!(sample.skips_the_launcher());
        let err = parse(args(&["a", "b"])).unwrap_err();
        assert!(
            err.contains("Only one project") && err.contains("--help"),
            "{err}"
        );
        assert!(usage("solis").contains("<project folder>"));
    }

    #[test]
    fn unknown_options_are_explained() {
        let err = parse(args(&["--fly"])).unwrap_err();
        assert!(err.contains("--fly") && err.contains("--help"));
    }

    #[test]
    fn usage_names_the_program() {
        assert!(usage("solis").starts_with("Usage: solis"));
    }

    #[test]
    fn usage_takes_the_name_from_branding() {
        assert!(usage("x").contains(crate::branding::APP_NAME));
    }

    /// Thousands of random argument lists, including broken text: the
    /// parser always answers, and never crashes.
    #[test]
    fn random_arguments_never_crash_the_parser() {
        let pieces = [
            "--version",
            "-V",
            "--help",
            "-h",
            "-",
            "--",
            "",
            " ",
            "--VERSION",
            "-vh",
            "é",
            "--help=1",
            "\u{0}",
            "solis",
        ];
        let mut seed: u64 = 0x5eed;
        let mut next = |n: usize| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) as usize % n
        };
        for _ in 0..5_000 {
            let count = next(5);
            let list: Vec<OsString> = (0..count)
                .map(|_| OsString::from(pieces[next(pieces.len())]))
                .collect();
            // Every argument is a known option, the file after --import,
            // or the one project to open (a word not starting with -).
            let mut only_known = true;
            let mut after_import = false;
            let mut projects = 0;
            for a in &list {
                let known = matches!(
                    a.to_str(),
                    Some("--version" | "-V" | "--help" | "-h" | "--import")
                );
                let project = a
                    .to_str()
                    .is_some_and(|w| !w.starts_with('-') && !w.trim().is_empty());
                if !after_import && project {
                    projects += 1;
                }
                if !after_import && !known && !project {
                    only_known = false;
                }
                after_import = !after_import && a.to_str() == Some("--import");
            }
            only_known &= projects <= 1;
            let help = list
                .iter()
                .any(|a| matches!(a.to_str(), Some("--help" | "-h")));
            match parse(list.clone()) {
                Ok(_) => assert!(only_known || help, "{list:?}"),
                Err(message) => assert!(message.contains("--help"), "{message}"),
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn arguments_that_are_not_text_are_explained() {
        use std::os::unix::ffi::OsStringExt;
        let broken = OsString::from_vec(vec![0x66, 0x6f, 0xff, 0xfe]);
        assert!(parse(vec![broken]).is_err());
    }
}
