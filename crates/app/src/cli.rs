//! The command line: `--version`, `--help`, or nothing to open the editor.

use std::ffi::OsString;

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Open the editor.
    Run,
    /// Print the version and exit.
    Version,
    /// Print how to use the program and exit.
    Help,
}

/// Reads the arguments after the program's name. Anything unknown is
/// explained in the error.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut command = Command::Run;
    for arg in args {
        match arg.to_str() {
            Some("--version" | "-V") => command = Command::Version,
            Some("--help" | "-h") => return Ok(Command::Help),
            _ => {
                return Err(format!(
                    "Unknown option {}. Try --help.",
                    arg.to_string_lossy()
                ));
            }
        }
    }
    Ok(command)
}

/// How to use the program, for `--help`.
pub fn usage(program: &str) -> String {
    format!(
        "Usage: {program} [--version | --help]\n\n\
         Opens the {name} editor.\n\n\
         Options:\n  \
         -V, --version   Print the version and exit\n  \
         -h, --help      Print this help and exit\n",
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
        assert_eq!(parse(args(&[])), Ok(Command::Run));
    }

    #[test]
    fn version_and_help_are_understood() {
        assert_eq!(parse(args(&["--version"])), Ok(Command::Version));
        assert_eq!(parse(args(&["-V"])), Ok(Command::Version));
        assert_eq!(parse(args(&["--help"])), Ok(Command::Help));
        assert_eq!(parse(args(&["--version", "-h"])), Ok(Command::Help));
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
            let only_known = list
                .iter()
                .all(|a| matches!(a.to_str(), Some("--version" | "-V" | "--help" | "-h")));
            match parse(list.clone()) {
                Ok(_) => assert!(
                    only_known
                        || list
                            .iter()
                            .any(|a| matches!(a.to_str(), Some("--help" | "-h")))
                ),
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
