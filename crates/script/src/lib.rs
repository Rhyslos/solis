//! Game code ([decision 0028](../../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)):
//! a project's Lust, checked and turned into Lua by the Lust compiler, and
//! run in Luau while the game plays.

pub mod animator_code;
mod check_turns;
pub mod code;
pub mod component_code;
mod convert;
pub mod describe;
pub mod events;
mod exact_maths;
mod field_hints;
pub mod game;
mod hint_comments;
pub mod host;
pub mod interface;
#[allow(unsafe_code)]
mod interrupt;
mod lua_components;
pub mod luau;
mod lust_program;
mod many_names;
pub mod map_code;
mod native_values;
mod natives;
mod objects;
pub mod old_names;
pub mod project_names;
mod reach;
pub mod readings;
pub mod runtime;
pub mod rust_component;
pub mod screens;
pub mod settings;
pub mod state_names;
pub mod states;
mod syncing;
pub mod table_code;
pub mod tables;
pub mod tag_view;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod text_code;
mod texts;
mod ticking;
pub mod timer_names;
pub mod timers;

pub use code::{
    Bytecode, Code, CodeProblem, Compiled, NotStarted, check, check_unfinished, compile, gather,
    renamed_uses, try_compile,
};
pub use events::Event;
pub use game::{Change, Game, GameWorld, SpawnValues, ViewIndex, WorldView};
pub use host::{AnimatorValue, Host, MapPlace, NoHost, Shot, SoundAt};
/// Game code turned into Lua, ready for a [`Runtime`].
pub use lust::LuaChunk as Chunk;
pub use objects::ObjectValues;
pub use readings::Readings;
pub use runtime::{Failure, Runtime, Swapped, Ticked};
pub use rust_component::{Context, RustComponent, RustKind};
pub use screens::{NoReader, ScreenCode, ScreenReader, Screens};
pub use states::{Started, StateChange, still_in};
pub use tables::{RowRef, TableRef, Tables};
pub use tag_view::TagView;
pub use ticking::Order;
pub use timers::{Timers, Timing};

/// The Lust compiler's version.
pub const LUST_VERSION: &str = lust::VERSION;

/// Whether `slot` is a usable save slot's name (decision 0038): letters,
/// digits, `_` and `-`, at most 64 long, and not a name Windows keeps
/// for a device (`CON`, `NUL`, `COM1` …). It becomes a file's name, so
/// nothing that could reach outside the saves folder.
pub fn valid_slot(slot: &str) -> bool {
    const DEVICES: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];
    let upper = slot.to_ascii_uppercase();
    let device = DEVICES.contains(&upper.as_str())
        || ["COM", "LPT"].iter().any(|d| {
            upper
                .strip_prefix(d)
                .is_some_and(|n| n.len() == 1 && n.chars().all(|c| c.is_ascii_digit()))
        });
    !slot.is_empty()
        && slot.len() <= 64
        && !device
        && slot
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod slot_tests {
    #[test]
    fn save_slots_are_plain_names_and_never_a_device() {
        // The hardening milestone's mutation testing: the slot check was
        // only tested from the app.
        for good in [
            "quick",
            "slot_1",
            "before-boss",
            "Z",
            &"a".repeat(64),
            "COM",
            "COM10",
            "CONSOLE",
        ] {
            assert!(super::valid_slot(good), "{good}");
        }
        for bad in [
            "",
            &"a".repeat(65),
            "../up",
            "a/b",
            "a\\b",
            "has space",
            "dot.ted",
            "ünïcode",
            "CON",
            "con",
            "Nul",
            "PRN",
            "AUX",
            "COM1",
            "com9",
            "LPT1",
            "lpt0",
        ] {
            assert!(!super::valid_slot(bad), "{bad}");
        }
    }
}

#[cfg(test)]
#[path = "editor_files_tests.rs"]
mod editor_files_tests;
#[cfg(test)]
#[path = "field_hints_code_tests.rs"]
mod field_hints_code_tests;
#[cfg(test)]
#[path = "manual_examples_tests.rs"]
mod manual_examples_tests;
