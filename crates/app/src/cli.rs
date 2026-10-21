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
         Opens the Solis editor.\n\n\
         Options:\n  \
         -V, --version   Print the version and exit\n  \
         -h, --help      Print this help and exit\n"
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
}
