# docs

Everything written about the engine that isn't code: why things are the way they are, how to use it, the libraries it relies on, and the working plans, reviews and reports along the way. The main plan and rules live at the top of the repository (`DESIGN.md`, `ROADMAP.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md`).

| File or folder | What it holds |
| --- | --- |
| `dependencies.md` | The register of every outside library: what it's for, its licence, and the safeguards against dependency trouble (decision 0052), including "Patches", the stand-ins in [`patches/`](../patches/README.md). A test in `tools/layer-check` checks the register matches the crates. |
| [`decisions/`](decisions/README.md) | One file per major choice, numbered in order. Check here before changing anything a decision covers. |
| `manual/` | The user manual, for people new to game engines. Its contents page is [`manual/SUMMARY.md`](manual/SUMMARY.md); new pages follow [`manual/_template.md`](manual/_template.md) and link terms to [`manual/concepts.md`](manual/concepts.md). It describes only what exists in `main`. It has no README of its own, because every page in that folder is part of the manual. |
| [`plans/`](plans/README.md) | Working plans for each phase: the choices made before coding and a plan for each milestone, so a later session can carry on. |
| [`reports/`](reports/README.md) | Reports written for the project lead: research before a phase, and what a milestone found. |
| [`reviews/`](reviews/README.md) | Code reviews written up at the end of a phase. |

**Read next:** [`manual/SUMMARY.md`](manual/SUMMARY.md) to see what the engine can do today, or [`decisions/`](decisions/README.md) to see why.
