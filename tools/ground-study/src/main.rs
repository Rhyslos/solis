//! Makes the ground study's numbers and pictures (Tools 1b, G0); see the
//! library's own documentation and `docs/reports/ground-study.md`.
//!
//! - no arguments: every measurement, as Markdown tables;
//! - `uploads` (with the `gpu` feature): uploads through the renderer;
//! - `pictures <folder>` (with the `gpu` feature): the report's pictures.
//! - `worlds [levels|view|streaming|units|disk]`: the open-worlds study's
//!   measurements, all or one part.
//! - `worlds-pictures <folder>` (with the `gpu` feature): its pictures.

// A tool that prints its report.
#![allow(clippy::print_stdout)]

mod study;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => study::run(),
        Some("worlds") => study::worlds::run(args.get(1).map(String::as_str)),
        #[cfg(feature = "gpu")]
        Some("worlds-pictures") => {
            let folder = std::path::PathBuf::from(args.get(1).map_or(".", String::as_str));
            if let Err(e) = study::worlds::pictures::all(&folder) {
                println!("No pictures: {e}");
                return std::process::ExitCode::FAILURE;
            }
        }
        #[cfg(feature = "gpu")]
        Some("uploads") => study::uploads(),
        #[cfg(feature = "gpu")]
        Some("pictures") => {
            let folder = std::path::PathBuf::from(args.get(1).map_or(".", String::as_str));
            if let Err(e) = study::gpu::pictures(&folder) {
                println!("No pictures: {e}");
                return std::process::ExitCode::FAILURE;
            }
        }
        Some(other) => {
            println!(
                "Unknown command {other}: run with nothing, `uploads`, `pictures <folder>`, `worlds` or `worlds-pictures <folder>`."
            );
            return std::process::ExitCode::FAILURE;
        }
    }
    std::process::ExitCode::SUCCESS
}
