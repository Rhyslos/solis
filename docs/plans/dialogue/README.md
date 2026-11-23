# Dialogue: a full storytelling system in Solis

The project lead, 18 November 2026 ([decision 0124](../../decisions/0124-a-dialogue-system-in-the-engine.md)): Solis gets a full dialogue system built into the engine. It is the one central place where a game's conversations, voice lines and subtitles are written, timed and kept in step. It's meant to set Solis apart for storytelling.

This folder:

| File | What it is |
| --- | --- |
| `README.md` | This outline: what the system covers, how it fits Solis, and the milestones in rough order |
| `prototype-spec.md` | The project lead's specification of their first prototype: data, runtime, dialogue box and graph editor. The starting point, kept as written |
| [`milestone-2.md`](milestone-2.md) | The detailed plan for milestone 2, conversations and their runtime: the file, game code's calls and events, conditions and effects, holding the game, saves, the steps and their tests, and the questions for the project lead. It also links the mock-ups of milestones 3 and 4 |

**Only the next milestone is planned in detail** ([the game stages](../game-stages.md)). This outline is the shape. Each milestone gets its research, its questions and its mock-up when it comes up, and **every part with UI is mocked up first** (the project lead: "as always, you mock it up first when we get to it").

## Where it comes in the order

Stage 1, **right after text layout and text effects**, with **translations and subtitles** built just before it (the project lead's answers). It needs:
- triggers (built);
- timers;
- states that start and stop;
- text layout and effects (the typewriter, coloured and wavy words);
- sound (built);
- translations and subtitles.

All of these come earlier in stage 1. If planning finds it needs something that comes later, it moves to after the latest thing it needs (the project lead). Stage 1's sample then uses the real dialogue system.

## What the prototype gives, kept

Everything in [the prototype's specification](prototype-spec.md) stays:
- **Conversations as data:** a graph of Line, Choice, Branch and End nodes, linked by ID, with speakers in their own reusable files.
- **The runtime:** one conversation at a time. Branches are resolved at once, and Lines and Choices wait for the player. There's a step limit for branches that loop, and the four notifications (Started, LineShown, ChoicesShown, Ended).
- **Conditions and effects** on nodes, options, routes and entries. Effects run on entering a node, and an option's effects run before moving on.
- **Pausing** by requests, so a conversation and a menu can each pause the game without unpausing the other.
- **The dialogue box:**
  - typewriter text, with 0 meaning instant;
  - choices with focus, and locked choices said in words;
  - interact and accept as remappable actions, so the press that starts a conversation never also advances it;
  - a box that grows upward.
- **Starting a conversation from the world:** an area round the thing you talk to, and an ordered list of entries. The first one whose conditions are met plays, and the last entry is a fallback.
- **Translation keys:** made once, never changed, with English working with no translation files at all.
- **The graph editor:** node boxes with ports and wires, the port mapping, add, connect, disconnect, delete, move, set start and arrange. It has a Problems list (missing links, unreachable nodes, empty text) and undo.
- **The rebuild order** and "things that are easy to miss".

## What Solis adds

The project lead's expansion, from simple settings to a full system:
- **Built with triggers, timers and states:**
  - a conversation can start when something walks into a zone;
  - lines can move on by themselves after a time;
  - a character's state can follow the conversation.
- **Sound at exact moments:**
  - a voice line per line, and per language;
  - other sounds and cues placed at moments inside a line, such as a laugh, a door, or a pause before the last word.
- **Subtitles kept in step with the voice,** in the same place, with the same timing, for every language.
- **One central place for a game's dialogue:**
  - every conversation, speaker and line;
  - which lines have voice and which have translations;
  - what's missing;
  - search;
  - the spreadsheets for translators and voice actors.

How Solis fits it to its own rules (to settle at each milestone, with the project lead):
- **Conditions and effects call game code** (the project lead's answer: dialogue only, no separate store of story values). A condition asks the game a question, and an effect tells the game something happened. Game code in Lust, Rust or Lua answers. The names are checked before the game runs, like every name game code uses (rule 25).
- **Conversations are Lust data files** (decision 0081), reached from game code through the Lust file the editor writes, never by a string name (decision 0065).
- **A conversation in progress is game state:** saved, in the checksum, and the same on any number of cores.
- **The editor uses the shared pieces** (rule 21): the one graph editor (decision 0050), the timeline (decision 0064) for moments in a line, and the usual Properties.
- **The dialogue box is game UI,** a screen game makers can restyle or replace.
- **Inventories, quests and shops stay games made with tools** (the project lead's answer). Only dialogue joins the engine.

## Milestones, in rough order

Translations and subtitles come first, then dialogue. The split and the names will be settled when each is planned.

1. **Translations and subtitles:**
   - the game's text in several languages, with a check for missing lines;
   - captions for sounds and speech;
   - brought forward from Shared 5.
2. **Conversations and their runtime** ([the plan](milestone-2.md)):
   - the conversation file and speakers;
   - the four node kinds;
   - conditions and effects through game code;
   - the runtime and its events;
   - pause requests;
   - saves.
3. **The dialogue box** ([mock-up, for approval](https://claude.ai/artifact/77CrihfvXyDwM318Q4fEFp)):
   - a game UI screen on text layout and effects;
   - choices;
   - interact and accept;
   - restyled or replaced by the game maker.
4. **The conversation editor** ([mock-up, for approval](https://claude.ai/artifact/N2YvGVm2oZ8G9j7jN4LXfD)):
   - on the shared graph editor;
   - Problems;
   - Properties;
   - undo;
   - Arrange.
5. **Starting from the world:**
   - talking to things, built on trigger zones and tags;
   - ordered entries;
   - lines that move on by themselves on timers;
   - states that follow the conversation.
6. **Voice and moments:**
   - voice lines per language;
   - sounds and cues placed at moments in a line, on the timeline;
   - subtitles in step with the voice.
7. **One place for all of a game's dialogue:**
   - the overview, its search and what's missing;
   - spreadsheets out to translators and voice actors, and back in.
