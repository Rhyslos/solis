//! Writes a sample model to a `.glb` file, for trying the importer:
//! `cargo run -p assets --example sample -- out.glb` writes the small one (a
//! textured cube on a red base); add `--scene` before the file name for the
//! bigger sample scene (spheres, a grooved wall, a glowing stripe).

#![allow(clippy::print_stderr)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let (scene, path) = match args.as_slice() {
        [flag, path] if flag == "--scene" => (true, path),
        [path] => (false, path),
        _ => {
            eprintln!("Usage: sample [--scene] <out.glb>");
            return ExitCode::from(2);
        }
    };
    let bytes = if scene {
        assets::sample::showcase_glb()
    } else {
        assets::sample::sample_glb()
    };
    match std::fs::write(path, bytes) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Couldn't write {}: {e}", path.to_string_lossy());
            ExitCode::FAILURE
        }
    }
}
