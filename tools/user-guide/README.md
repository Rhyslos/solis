# user-guide

**Tool · not part of the program, but what it makes ships with it**

Builds the **user guide** from the user manual ([decision 0112](../../docs/decisions/0112-the-user-guide-is-the-manual-built.md)): the manual's pages (`docs/manual/*.md`), in the order and groups of its contents page (`docs/manual/SUMMARY.md`), as one HTML file with its style, script, fonts, logo and pictures written into it. It opens from the disk with no network: the Windows build puts it beside the program as `user-guide/index.html`, where the editor's **Documentation** button opens it, and a copy is kept in the repository as [`docs/user-guide/index.html`](../../docs/user-guide/README.md).

```
cargo run -p user-guide                  # writes docs/user-guide/index.html
cargo run -p user-guide -- --out <file>  # writes it anywhere else, such as beside a build of your own
cargo run -p user-guide -- --stamp-pictures  # for the picture script: notes the interface the pictures show
```

## Purpose

- Read the manual, the program's name and repository (`APP_NAME` and `UPDATE_REPO` in `crates/app/src/branding.rs`, as text, never built), the logo (`branding/logo.svg`) and the editor's fonts (Inter and JetBrains Mono, `crates/editor/assets/fonts`, whose licences are in `licenses/fonts`).
- Turn each page into a numbered chapter: its title, its opening sentence, its sections (each with a `#` link), "Good to know" and "Careful" boxes, numbered steps, keys drawn as keys, tables, code, the manual's pictures as figures that open larger, and each game-code example in Lust, Rust and Lua with a switch between them (the conventions are in `docs/manual/_template.md`; [decision 0113](../../docs/decisions/0113-pictures-and-three-languages-in-the-guide.md)).
- Refuse to build, listing every problem, when a page isn't in `SUMMARY.md`, a link goes nowhere (another page, a heading in it, or a file in the repository), a picture a page shows is missing or too large, a picture is shown by no page, or a game-code example isn't in all three languages.
- Write the same bytes for the same manual, so the check below can compare.

**The check:** `tests/manual.rs` runs with `cargo test --workspace` and fails when the kept copy isn't what this tool makes now, when a page is missing from the contents, when a link is broken, when the pictures are older than the editor's interface (see [`tools/guide-pictures`](../guide-pictures/README.md)), or when an example's Rust doesn't balance its brackets or its Lua or Rust calls something game code can't. CI runs it on every pull request, documentation-only ones too (`.github/workflows/user-guide.yml`).

| Folder | What it holds |
| --- | --- |
| [`src/`](src/README.md) | The tool: the Markdown reader, the links, the contents and the page. |
| [`assets/`](assets/README.md) | The page around the chapters, its style and its script. |
| [`tests/`](tests/README.md) | The checks on the real manual. |

## Must never

- Fetch anything from the network, or make a page that does.
- Name the program itself: the name comes from `branding.rs`.
- Depend on any of the program's crates: it reads what it needs as text, so building the guide builds nothing else.

## Depends on

Nothing: no libraries (its Markdown reader is its own, written for what the manual uses) and none of the program's crates.

## Status

In use since decision 0112.
