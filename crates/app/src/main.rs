//! The program's entry point: reads the command line, then opens the editor
//! (or prints the version or usage).

// On Windows, a release build opens without a black console window behind
// the editor. Debug builds keep it, for messages while developing. (So
// `solis --version` prints nothing from a release build on Windows.)
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use app::cli::{self, Command};
use std::io::Write;
use std::process::ExitCode;

/// Writes text to standard output or standard error; `false` if it failed.
fn say(text: &str, to_error: bool) -> bool {
    let written = if to_error {
        std::io::stderr().lock().write_all(text.as_bytes())
    } else {
        std::io::stdout().lock().write_all(text.as_bytes())
    };
    written.is_ok()
}

fn main() -> ExitCode {
    let mut args = std::env::args_os();
    let program = args
        .next()
        .and_then(|p| {
            std::path::Path::new(&p)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| app::branding::APP_ID.to_string());

    match cli::parse(args) {
        Ok(Command::Version) => exit_with(say(&format!("{}\n", app::version_banner()), false)),
        Ok(Command::Help) => exit_with(say(&cli::usage(&program), false)),
        Ok(Command::CheckProject(folder)) => {
            let report = app::check_project::check(&folder, &app::version_banner());
            let text = report.text();
            say(&text, false);
            // Kept in the project too: a Windows build has no console to
            // print to.
            let _ = asset_db::own_folder(&folder, asset_db::CACHE_FOLDER).and_then(|own| {
                asset_db::write_atomically(&own.join("check-report.txt"), text.as_bytes())
            });
            if report.failed == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Ok(Command::Run(imports)) => match app::window::run(app::app_info(), imports) {
            Ok(()) => ExitCode::SUCCESS,
            Err(reason) => {
                say(
                    &format!(
                        "{} could not open its window: {reason}\n",
                        app::branding::APP_NAME
                    ),
                    true,
                );
                ExitCode::FAILURE
            }
        },
        Err(reason) => {
            say(&format!("{reason}\n"), true);
            ExitCode::from(2)
        }
    }
}

/// Success if the text was written, failure otherwise.
fn exit_with(written: bool) -> ExitCode {
    if written {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
