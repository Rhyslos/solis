# Feature name

> One sentence: what it does.

## When to use it
The situations this is for, in plain language. Mention the alternative if something else is better for some cases.

## How to use it
1. Step one (which panel, what to select).
2. Step two: press `Ctrl + S`.
3. Step three.

> **Good to know:** something worth stopping for, such as a choice that can't be undone or a reason that isn't obvious.

## Settings
| Setting | What it does |
|---|---|
| Example | Explain the effect, not just the name. Give a typical value where it helps. |

## Shortcuts
| Action | Shortcut |
|---|---|
| Example | `Ctrl + S` |

## Tips and common problems
- **Problem:** what you see. **Why:** what causes it. **Fix:** what to do.

## Related
- Links to related pages, and to [Concepts](concepts.md) terms used on this page.

## How the manual becomes the user guide (delete this part)
Every page here is also a chapter of the user guide (`docs/user-guide/index.html`, built by `cargo run -p user-guide`; see [The user guide](user-guide.md)). Write so it reads well in both:

- **Keys:** write a key, or keys pressed together, in backticks with ` + ` between them: `` `Ctrl + K` ``, `` `Esc` ``, `` `F2` ``, `` `Shift + click` ``. The guide draws them as keys. Code in backticks that isn't key names (`` `world.lst` ``) stays code. `<kbd>K</kbd>` works too.
- **Steps:** a numbered list is drawn as numbered steps.
- **Game code in three languages:** every example of game code is written in Lust, Rust and Lua, as three fenced blocks one after another, `` ```lust ``, `` ```rust `` and `` ```lua ``. The guide shows them as one example with a switch above it (Lust first; the reader's choice is remembered). Each is a whole file that follows [the API guidelines](../game-api-guidelines.md) and calls only what the engine has (`docs/game-code-interface.txt`): the checks run every Lust example through Lust and every Lua one through Luau, balance each Rust one's brackets, and fail when an example lacks a language. A Rust example may leave out `values()`, `set()` and the `RustKind` with a comment saying so, since [Game code](game-code.md#components) shows them once. Where a language can't do the same (tweens and game-wide objects are Lust's own; screens are written in Lust), its block says how it's done there. A file the example is (`-- hero.lst: the player.`) is named on its first line. Code that isn't game code (commands, files, settings) is fenced without a language, or with `text`, and has no switch.
- **Pictures:** a picture of the editor is `![What it shows](pictures/name.png)` alone on its line, with its caption on the line straight under it, as one paragraph. The guide shows it as a figure that opens larger when clicked. Pictures are never made by hand: add the picture's steps to `tools/guide-pictures/pictures.list` (what it shows, the page, how to make it) and run `tools/guide-pictures/take-pictures.sh`. Crop it to what it shows. The checks fail when a page shows a picture that isn't there, a picture is shown by no page, a picture is over 200 KB, or the pictures are older than the interface.
- **Boxes:** a quote starting **Good to know:** (or **Note:**) is a blue box; one starting **Careful:** (or **Warning:**) an amber one. The quote straight under the title is the page's opening sentence.
- **Links:** to another page (`physics.md`), a heading (`physics.md#how-to-use-it`, the heading's words in lower case with hyphens) or a file elsewhere in the repository (`../decisions/0112-the-user-guide-is-the-manual-built.md`, shown on GitHub). A link to nothing fails the checks.
- **A new page** goes in [SUMMARY.md](SUMMARY.md), in its group: that list is the guide's chapters, in order. A page missing from it fails the checks.
- After changing any page, build the guide again (`cargo run -p user-guide`) and commit it with the change.
