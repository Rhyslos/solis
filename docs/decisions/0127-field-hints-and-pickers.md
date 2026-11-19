# 0127. Field hints in a comment, pickers for the project's names, lists when Lust has them

- **Status:** Accepted (builds on 0019, 0050, 0065, 0077, 0081 and 0122)
- **Date:** 2026-11-19

## Problem

Stage 1's next milestone (decision 0122; [report 06](../reports/after-the-rethink/06-shared-4.md), M1 and M2) is field hints and more kinds of settings: a component's own settings in Properties with sliders, limits, steps, units and a sentence of help; fields that pick a sound, a sprite, a prefab, a picture; colours; and lists. The project lead approved a quick mock-up on 2026-11-18 (`docs/design/overnight/mock-1.png`) and asked for every choice to take the recommended answer while they slept.

Two parts need Lust: Lust has no attributes on a field (`float speed = 4 [range 0 10];` is a mistake), and a component's field can't be an array (`event_kind` in Lust's checker: a field holds `bool`, `int`, `float`, `string`, a state, `vec3`, `vec2`, `rotation`, `colour` or `entity`). Checked against the pinned Lust, alpha 74 (`9fcd48c`).

## Options

**Where a Lust field's hints are written:**
1. **A comment straight after the field**, in the `[ ]` words of Lust's screens: `float speed = 4.0; -- [range 0 10, unit "m/s"]`. No Lust change; moving to real attributes later is taking away the `--`.
2. A side file the editor keeps beside the code (the quick mock-up's note). Hints apart from the field they describe, edited somewhere else, and a second file to keep in step.
3. Wait for Lust's attributes. Nothing for stage 1.

**How a field says it holds one of the project's names:** by its kind, the state from `solis/project.lst` it's written with (`Sound alertSound = Sound.Sfx.Alert;`), or by a hint (`-- [picks Sound]`). The kind already says it, and Lust already checks the value.

**Lists:** build them now in Rust components only; lists as child entities; or wait for Lust.

## Decision

- **Hints are a comment after the field** (option 1). Solis reads them from the source (`level::lust_fields` finds each `has` field and its line as Lust reads the text; `script::hint_comments` reads the comment). The words: `range a b`, `step s`, `unit "…"`, `tip "…"`, `group "…"`, `multiline`, `hidden`. A comment that starts with `[` straight after a field's `;` is its hints.
- **Lua:** a hints table as the fifth entry of a field (`{ "speed", "float", 4.0, nil, { range = { 0, 10 }, step = 0.5 } }`). **Rust:** `reflect::Hint` gains `step`, `tip`, `group`, `slider`, `multiline` and `hidden`, and its `unit` and `help` become `Cow<'static, str>` so hints read from code can be held. One reader of words (`script::field_hints`) puts them on a field for all three, so each word means the same everywhere.
- **Properties:** a number with a `range` is the kit's slider with its number beside it and "0 to 10" under the field; dragged and typed values keep to the range and the step, never a value only shown, so game code may set anything; `tip` is a sentence under the field (and on its name); `group` a heading in spaced capitals over its fields; `multiline` a box of three lines; `hidden` not shown but kept. A slider's drag is one undo step, as every field's is.
- **Mistakes in a hint** (an unknown word, `range 10 0`, a range on a sound) and **a starting value outside its range** go to Problems with the file and line, as `Compiled::hint_problems`; the code runs, and the field shows without that hint. Hints are the code's: never saved in a level, changed when new code arrives.
- **Pickers by kind:** a state field of `Sound`, `Clip`, `Picture`, `Prefab`, `Screen`, `Level` or `Tag` gets `Offer::Names(kind)`. In Lua the kind is written as the field's kind (`{ "alertSound", "Sound", "Sfx.Alert" }`), its options read from `solis/project.lst`. The picker is the kit's drop-down with the file's thumbnail or the kind's icon before the name; its list has a search box, the project's folders as headings, and ▶ on a sound (in the list and after the field, playing as the Library does). A Library file dragged onto the field lights it when it stands for one of its names, and dropping sets it. The value is the option's name (decision 0077), so **renaming or moving a file in the Library changes the open level's fields naming it**, as one undo step.
- **New project names:** `state Level` (the project's `.sollevel` files) and **every picture** in `state Picture` (PNG and JPEG files, with Atlas Crop's crops as options inside their picture, no longer a note). `solis/project.lst` passes `Level` too.
- **Colours** keep the swatch and colour picker they had, with the hex after it.
- **Lists wait for Lust** (the recommended answer when Lust can't carry arrays in a component): no list field, no format change. The request is written in [the plan's As built note](../plans/game-stages.md#the-next-milestone-field-hints-and-more-kinds-of-settings).
- **Keys:** none new. Drag from the Library onto a field is the only new mouse action (drag to reorder came with lists, so it isn't built).

## Consequences

- No file format changes; save games, levels and prefabs keep a picked name as text, as a state's value always was.
- Rust code that built a `Hint` with `unit: " m"` writes `unit: " m".into()` (a Breaking line in CHANGELOG).
- A project with pictures whose names can't be Lust options now gets a note for each, as prefabs and sounds always did.
- A hint comment that isn't meant as one (`-- [see below]`) is read as hints and listed in Problems: the manual says so.
- When Lust has attributes on fields, the reader moves to Lust's schema and the comments' `--` goes; when it has arrays in components, lists come with `FieldKind::List`, a list editor in Properties, and a raised level format.
