# 0124: A dialogue system in the engine

- **Status:** Accepted (the project lead, 2026-11-18). This amends 0122: dialogue is no longer a game made with tools.
- **Date:** 2026-11-18

## Problem

Decision 0122 and CLAUDE.md kept every game system out of the engine. Dialogue, inventories, quests and shops were to be made with Solis's tools, such as text effects, tables and tags, and stage 1's sample was to write a little dialogue in Lust.

The project lead has since specified a dialogue system for a prototype of their own ([docs/plans/dialogue/prototype-spec.md](../plans/dialogue/prototype-spec.md)), and wants it **in Solis, as a full system**. Beyond the prototype's conversations, it should work with triggers, timers and states, and deliver sound at exact moments. It's meant to be one central place to manage all of a game's dialogue, with audio and subtitles kept in step. Their reasons: it sets Solis apart and gives it a clear edge for storytelling.

## Options

**Where it goes:**
1. Stage 1, after text effects.
2. Stage 1's last milestones.
3. A storytelling stage of its own after stage 1.

**Translations and subtitles**, today in the late "finishing and sharing games" stage:
1. Brought into stage 1 with dialogue.
2. Only translation keys now, with languages loaded later.

**What joins the engine with dialogue:**
1. Dialogue and a store of story values (flags and numbers).
2. Dialogue and quests.
3. Dialogue only.

## Decision

- **Solis gets a full dialogue system in the engine.** Everything in the prototype's specification stays. Solis adds:
  - triggers, timers and states;
  - voice lines and other sounds at moments inside a line;
  - subtitles in step with the voice;
  - one place for all of a game's dialogue.

  The outline is [docs/plans/dialogue/README.md](../plans/dialogue/README.md).
- **Stage 1, right after text layout and text effects.** If planning finds it needs something that comes later, it moves behind the latest thing it needs (the project lead's answer).
- **Translations and subtitles come into stage 1,** built just before dialogue (the project lead's answer). They move out of the finishing stage. Button pictures stay there.
- **Dialogue only** (the project lead's answer). Its conditions and effects call game code, and there's no separate store of story values. Inventories, quests, shops and crafting stay games made with tools.
- Every part with UI is mocked up first, and each milestone is planned in detail only when it's next.

## Consequences

- CLAUDE.md's rule "The project lead's games are made with Solis, never built into it" names dialogue as the one exception, by this decision. The game stages' "Tools, never game systems" says the same.
- Stage 1 gains these milestones:
  - translations and subtitles;
  - dialogue's (conversations and the runtime, the dialogue box, the editor, starting from the world, voice and moments, one place for all dialogue).

  Its sample uses the real dialogue system.
- ROADMAP.md gains a Dialogue section in its catalogue. Shared 5's translations and subtitles point to it.
- Conversations follow Solis's rules:
  - they're Lust data files (decision 0081), reached from game code through the editor's Lust file (decision 0065);
  - a conversation in progress is game state, saved and the same on any number of cores;
  - the editor uses the shared graph editor and timeline (rule 21).
