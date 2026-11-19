//! A game's text in several languages (stage 1, translations;
//! [decision 0131](../../../docs/decisions/0131-translations-and-captions.md)).
//!
//! Each line of text has a key made once and never changed
//! (`Shop.Welcome`), which game code names as `Text.Shop.Welcome`. Its
//! text in each language is in Fluent files, `locale/<language>/*.ftl`, so
//! a translator writes plurals and values in the shape their language
//! needs; a data table's translated cells take their main-language text
//! from the table. The project's **main language** (English unless it says
//! otherwise) is shown wherever a language lacks a line, so a game works
//! with no translations at all.
//!
//! Layer 1 (engine). See this crate's README for its purpose and limits.

mod book;
pub mod check;
pub mod edit;
pub mod ftl;
pub mod keys;
pub mod language;
pub mod sheet;

pub use book::{FOLDER, TableText, TextValue, Texts, Translations, in_language};
pub use ftl::{FileProblem, Line, ValueKind};
pub use language::Language;
