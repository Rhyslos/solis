//! What every fuzz target shares.

use std::panic::PanicHookInfo;

/// Keeps quiet about the panics Solis catches on purpose, and only those.
///
/// A library's panic that Solis catches and turns into a refused file
/// (the glTF parser's, say) isn't a failure, and printing each one would
/// bury the fuzzer's output. Every other panic is shown as usual: a
/// target's own `assert!` message, or a panic in Solis's code, says what
/// went wrong in the crash's log. libFuzzer sees every panic that escapes
/// either way. Call it first in each target; it does its work once.
pub fn quiet_caught_panics() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let show = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info: &PanicHookInfo<'_>| {
            let in_a_library = info.location().is_some_and(|at| from_a_library(at.file()));
            if !in_a_library {
                show(info);
            }
        }));
    });
}

/// Whether `file`, a panic's source file, is in a library from crates.io
/// (built from Cargo's registry folder) rather than in Solis or a target.
fn from_a_library(file: &str) -> bool {
    file.replace('\\', "/").contains("/registry/src/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_library_s_panics_are_quiet() {
        let library =
            "/home/me/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gltf-1.4.1/src/lib.rs";
        assert!(from_a_library(library));
        assert!(from_a_library(
            r"C:\Users\me\.cargo\registry\src\index.crates.io-1\gltf-1.4.1\src\lib.rs"
        ));
        for ours in [
            "fuzz_targets/model.rs",
            "../crates/assets/src/reader.rs",
            "/home/me/solis/crates/assets/src/reader.rs",
            "/rustc/2d8144b7880597b6e6d3dfd63a9a9efae3f533d3/library/core/src/option.rs",
        ] {
            assert!(!from_a_library(ours), "{ours}");
        }
    }
}
