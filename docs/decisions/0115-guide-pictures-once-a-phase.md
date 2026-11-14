# 0115: The guide's pictures are taken again once a phase

- **Status:** Accepted (the project lead, 2026-11-14, 12:56; amends [0113](0113-pictures-and-three-languages-in-the-guide.md))
- **Date:** 2026-11-14

## Problem

Decision 0113 made a test in the normal suite fail whenever the editor's approved picture-test images changed since the guide's pictures were last taken. Nearly every milestone changes the interface a little, so every milestone took all the pictures again: a long run on a virtual screen, and a pull request carrying dozens of changed pictures, most of them changed by a few pixels.

## Options

1. **Keep 0113:** pictures always as new as the interface; the cost paid every milestone.
2. **Take them again once a phase** (the project lead's choice): between phases the guide may show a slightly older interface, and the check says so as a note; at the end of a phase it fails until they are taken again.
3. **Take them only by hand when someone notices:** cheapest, but they drift with nothing to say so.

## Decision

- The check that the pictures are older than the interface (`tools/user-guide/tests/manual.rs`, against the stamp `tools/guide-pictures/interface.txt`) **no longer fails the normal suite**: it prints a note naming the command.
- It **fails only when `GUIDE_PICTURES_STRICT=1` is set**. That run is one of the end-of-phase steps, beside the scale check and the code review (CLAUDE.md, CONTRIBUTING.md and ROADMAP.md's phase gate say so), and the pictures are taken again then (`tools/guide-pictures/take-pictures.sh`).
- **The other picture checks stay strict, every run:** a picture a page shows that's missing, a picture no page shows, one that isn't a PNG or is over 200 KB, and one without its caption all stop the guide being built.
- A milestone may still take the pictures again when its change would make the guide misleading (a screen that's gone, a button renamed).

## Consequences

- Milestones' pull requests are smaller and quicker; the guide's pictures may lag the interface by up to a phase, and the note says so whenever the suite runs.
- The end of each phase gains one step: `GUIDE_PICTURES_STRICT=1 cargo test -p user-guide`, after taking the pictures again.
