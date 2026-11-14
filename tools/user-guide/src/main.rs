//! Builds the user guide from the manual and writes it.
//!
//! ```text
//! cargo run -p user-guide                  # docs/user-guide/index.html
//! cargo run -p user-guide -- --out <file>  # anywhere else, such as beside a build
//! cargo run -p user-guide -- --stamp-pictures
//! ```
//!
//! `--stamp-pictures` is for the picture script (`tools/guide-pictures`):
//! once the pictures are taken, it writes down the interface they show
//! (`pictures::stamp`), so a test can tell when they are older.
//!
//! Run from anywhere: the repository is found from where this tool's own
//! source is. A problem (a page missing from SUMMARY.md, a broken link)
//! is listed and nothing is written.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut args = std::env::args().skip(1);
    let mut out = root.join(user_guide::KEPT_COPY);
    let mut shown = user_guide::KEPT_COPY.to_string();
    while let Some(arg) = args.next() {
        if arg == "--stamp-pictures" {
            return stamp_pictures(&root);
        }
        match (arg.as_str(), args.next()) {
            ("--out", Some(path)) => {
                out = PathBuf::from(&path);
                shown = path;
            }
            _ => {
                return said(
                    "Usage: user-guide [--out <file> | --stamp-pictures]",
                    ExitCode::from(2),
                );
            }
        }
    }
    let manual = match user_guide::read(&root) {
        Ok(manual) => manual,
        Err(why) => return said(&why, ExitCode::FAILURE),
    };
    let html = match user_guide::build(&manual) {
        Ok(html) => html,
        Err(problems) => {
            let list = problems.join("\n  ");
            return said(
                &format!("The guide wasn't built:\n  {list}"),
                ExitCode::FAILURE,
            );
        }
    };
    if let Some(folder) = out.parent().filter(|f| !f.as_os_str().is_empty())
        && let Err(why) = std::fs::create_dir_all(folder)
    {
        return said(
            &format!("Couldn't make {}: {why}", folder.display()),
            ExitCode::FAILURE,
        );
    }
    match std::fs::write(&out, html) {
        Ok(()) => said(&format!("Wrote {shown}"), ExitCode::SUCCESS),
        Err(why) => said(
            &format!("Couldn't write {}: {why}", out.display()),
            ExitCode::FAILURE,
        ),
    }
}

/// Writes the stamp of the interface the pictures show.
fn stamp_pictures(root: &Path) -> ExitCode {
    let path = root.join(user_guide::pictures::STAMP);
    let written = user_guide::pictures::stamp(root)
        .and_then(|stamp| std::fs::write(&path, stamp).map_err(|e| e.to_string()));
    match written {
        Ok(()) => said(
            &format!("Wrote {}", user_guide::pictures::STAMP),
            ExitCode::SUCCESS,
        ),
        Err(why) => said(
            &format!("Couldn't write {}: {why}", path.display()),
            ExitCode::FAILURE,
        ),
    }
}

/// Says `words` on the terminal and gives `code` back.
fn said(words: &str, code: ExitCode) -> ExitCode {
    let mut terminal = std::io::stderr();
    let _ = writeln!(terminal, "{words}");
    code
}
