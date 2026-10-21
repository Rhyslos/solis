//! Writes the sample model (a textured cube on a red base) to a `.glb`
//! file, for trying the importer: `cargo run -p assets --example sample -- out.glb`.

#![allow(clippy::print_stderr)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("Usage: sample <out.glb>");
        return ExitCode::from(2);
    };
    match std::fs::write(&path, assets::sample::sample_glb()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Couldn't write {}: {e}", path.to_string_lossy());
            ExitCode::FAILURE
        }
    }
}
