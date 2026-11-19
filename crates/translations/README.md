# translations

**Layer 1 · Engine**

A game's text in several languages (stage 1, translations; [decision 0131](../../docs/decisions/0131-translations-and-captions.md)): each line of text found by a key made once and never changed, written per language in Fluent files, with the project's main language shown wherever another lacks a line.

## Purpose

- **Languages** (`language.rs`): `Language`, a short code (`en`, `nb`, `pt-BR`) kept inline so the player's settings stay cheap to copy; its name for a list (`Norsk`, `Deutsch`; in English where the editor's letters can't write it), its option for game code (`Language.PtBr`), the computer's own language (`system`, through `sys-locale`), and `best`, which picks one of the project's languages for a player (the same code, else the same language, else the main one). The project's list and main language are `level::languages` (`languages.lst`).
- **Keys** (`keys.rs`): a key is a group and a name (`Shop.Welcome`), each part a name Lust can use, so game code names it `Text.Shop.Welcome`; in a Fluent file its dots are dashes (`Shop-Welcome`). A new key's lines go in a file named after its group (`shop.ftl`). A line with values is a method on its group's object (`ShopText:welcome(name)`), its values named for Lust (`player-name` is `playerName`). `nearest` suggests a key for a misspelt one.
- **Reading a file** (`ftl.rs`): one `.ftl` file read with `fluent-syntax` within limits (4 MB a file, 10,000 characters a line, 100,000 lines a language): each line's key, where it's written, its text as written (indents off, attributes left out) and the values it takes (a value that picks a plural or goes through `NUMBER` is a number, others text); each mistake at its line in plain words.
- **The translations** (`book.rs`): `Translations`, every language's lines from `locale/<language>/*.ftl` (other files and languages the project hasn't are said in its problems) and the data tables' translated cells (`TableText`: their main-language text is the table's), with each language's text put together by `fluent-bundle` (values, each language's own plural rules, no direction marks around values). `text` gives a key's text in a language: that language's line, else the main language's, else the key itself, so a game works with no translations at all. `missing`, `count` and `keys` for the Translations tab and the check. `Texts` is what a running game reads: the translations and the player's language, cheap to copy.
- **Changing a file** (`edit.rs`): `set_line` writes one line's text into a file's text where it was (keeping comments, order and its attributes) or at the end, refusing text that isn't Fluent in plain words; empty text takes the line out. `apply` makes a list of changes into the files they write.
- **The check** (`check.rs`): for Problems, each file's mistakes, the lines each language is missing (a count and the first few), lines of the main language nothing uses (`note_uses` reads game code, screens and sound events once each for `Text.Shop.Welcome`, `ShopText:welcome` and `"Shop.Welcome"`), and lines whose values don't match the main language's.
- **The spreadsheet** (`sheet.rs`): `export` writes `translations.csv`, a key, the main language and a column per language, every field quoted, line breaks kept; `import` reads one back against the translations and gives the changes (a table's main text stays the table's), with notes for columns and keys the project hasn't.

## Must never

- Stop on any file, however damaged: a reader past its limits leaves the rest out with a note.
- Be game state: the text shown follows the player's language, which two players of one game may set differently. Nothing here is saved or checksummed.
- Change a key: a key is made once, and renaming a line means making a new key.

## Depends on

`level` (the project's languages, CSV reading), `fluent-syntax` and `fluent-bundle` (with `unic-langid`) for Fluent, and `sys-locale` for the computer's language (proptest for its tests).

## Status

Stage 1, translations and subtitles: built.
