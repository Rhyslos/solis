# 0047. The effects suite is called Solstice

- **Status:** Accepted
- **Date:** 2026-10-26

## Problem

The effects suite was called "Solis Eclipse" (DESIGN.md §1). When light got its own phase (decision 0046), the name was checked:
- "Eclipse" is a registered US trademark for software development tools, and another game engine with heavy particle effects is being built under the name;
- the "Solis" prefix does little to set it apart.

## Options

1. **Eclipse:** accept the trademark risk.
2. **Ember:** sits right next to a leading fire and smoke tool for game effects with a near-identical name.
3. **Aura** (suggested by the project lead): used by an AI tool that makes particle effects for game engines, by an award-winning volumetric lighting and fog tool for games, and by a hardware maker's lighting software. It is also a common kind of effect, so it is hard to search for.
4. **Aurora:** the name of an established game engine, and "the aurora" is a sky feature in the Phase 8 plan.
5. **Solstice:** its only trademark found is for meeting-room screen sharing, a different market. No game tool by that name was found.

## Decision

Option 5, chosen by the project lead: **Solstice**, on its own. "Solis Solstice" is a tongue twister. In text it is "Solstice, Solis's effects suite" where that helps.

The checks were quick searches of US trademark copies and the web; EU, UK and WIPO registers weren't checked. Before the name goes on anything sold or registered, a trademark professional should confirm it.

## Consequences

- DESIGN.md, ROADMAP.md and the effects plan say Solstice.
- When the name first appears in the program (Phase 10), it is kept in one place, `branding.rs`, like the program's own name, and `RENAMING.md` gains a line for it.
