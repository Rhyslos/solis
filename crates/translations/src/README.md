# translations/src

A game's text in several languages (decision 0131). **Start reading at** `book.rs`, then `ftl.rs`.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers. | |
| `language.rs` | A language's code, its name for a list, its option for game code, the computer's language, and the best of the project's for a player. | `Language`, `best`, `system`, `known`, `of_option` |
| `keys.rs` | What a key may be, how it's written in a Fluent file, the file a new key goes in, and a line with values as a method of its group's object. | `key_problem`, `fluent_id`, `key_of`, `group_object`, `method_name`, `value_name`, `nearest` |
| `ftl.rs` | One `.ftl` file read within limits: its lines, their places, text and values, and its mistakes. | `read`, `Line`, `ValueKind`, `FileProblem` |
| `book.rs` | Every language's text, found by key with the main language as the fallback; what a running game reads. | `Translations`, `Texts`, `TableText`, `TextValue` |
| `edit.rs` | One line set or taken out of a file's text, keeping the rest; changes made into files. | `set_line`, `remove_line`, `apply` |
| `check.rs` | Problems: lines missing per language, lines nothing uses, values that don't match. | `check`, `note_uses` |
| `sheet.rs` | The translators' spreadsheet out and back in. | `export`, `import`, `Change` |
| `*_tests.rs` | Each file's tests: plurals in English, French and Polish, values, the fallback, mistakes at their lines, round trips of edits and spreadsheets, and property tests of damaged files and anything typed. | |
