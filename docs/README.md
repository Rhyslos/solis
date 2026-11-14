# docs

Everything written about the engine that isn't code: why things are the way they are, how to use it, the libraries it relies on, and the working plans, reviews and reports along the way. The main plan and rules live at the top of the repository (`DESIGN.md`, `ROADMAP.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md`).

| File or folder | What it holds |
| --- | --- |
| `game-api-guidelines.md` | The project lead's guidelines for every call game code makes into Solis (decision 0065), with the checklist each new call answers. The source of truth for game code's API. |
| `game-code-interface.txt` | Everything game code can call (Lust's natives, Lua's `solis`, the Rust traits, the ready-made components and their fields), written by a test in `crates/script/src/interface.rs`. A change to what games call fails that test until this file is updated in the same pull request, where a reviewer sees it (Phase 8, C3). |
| `dependencies.md` | The register of every outside library: what it's for, its licence, and the safeguards against dependency trouble (decision 0052), including "Patches", the stand-ins in [`patches/`](../patches/README.md). A test in `tools/layer-check` checks the register matches the crates. |
| [`decisions/`](decisions/README.md) | One file per major choice, numbered in order. Check here before changing anything a decision covers. |
| [`design/`](design/README.md) | The project lead's own designs. [`design/g-layout/`](design/g-layout/README.md) is the editor's interface: the specification and its thirteen screens, the source of truth for how the editor looks (decision 0099). |
| `manual/` | The user manual, for people new to game engines, and the one source of the user guide. Its contents page is [`manual/SUMMARY.md`](manual/SUMMARY.md): a `##` heading per group of pages and the pages under it, which is the guide's chapters, in order and in groups (every page must be listed there once, or the checks fail). New pages follow [`manual/_template.md`](manual/_template.md), whose last part is the convention for keys, boxes and steps, and link terms to [`manual/concepts.md`](manual/concepts.md). It describes only what exists in `main`. Its pictures are in `manual/pictures/`: PNGs of the editor, taken by [`tools/guide-pictures`](../tools/guide-pictures/README.md) and never by hand, each shown by one page (the checks fail when one is missing, shown by no page, or older than the interface). It has no README of its own, because every page in that folder is part of the manual, nor has `pictures/`, which holds only pictures. |
| [`user-guide/`](user-guide/README.md) | The user guide: the manual built into one offline HTML page by [`tools/user-guide`](../tools/user-guide/README.md) (decision 0112), the same file the Windows build ships beside the program. Never edited by hand: `cargo run -p user-guide` makes it again after a change to the manual, and a test fails when it is out of date. |
| [`plans/`](plans/README.md) | Working plans for each phase: the choices made before coding and a plan for each milestone, so a later session can carry on. |
| [`reports/`](reports/README.md) | Reports written for the project lead: research before a phase, and what a milestone found. |
| [`reviews/`](reviews/README.md) | Code reviews written up at the end of a phase. |

**Read next:** [`manual/SUMMARY.md`](manual/SUMMARY.md) to see what the engine can do today, or [`decisions/`](decisions/README.md) to see why.
