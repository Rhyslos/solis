# 0050. One way to work, everywhere in Solis

- **Status:** Accepted
- **Date:** 2026-10-27

## Problem

The project lead names **app uniformity** as Solis's first goal for the editor.

A tool where every part works its own way stops people at each new part. The project lead learned one engine's visual scripting, then moved to its effects system, which ran on a graph system of its own, different from the one they knew, and hit an instant wall. Aventail, by contrast, works the same whether modelling, unwrapping or texturing: the keys and menus are where you expect them.

Solis is about to grow several editors at once:
- the animator's graph (Phase 7);
- materials and shaders (Phase 9);
- effects (Phase 10);
- the UI canvas (Phase 11);
- later, behaviour trees.

## Decision

Chosen by the project lead: **everything in Solis works one way.** Learn one editor and you know them all.

- **One graph editor.** Every graph (animation states, materials, shaders, effects, later behaviour trees) is the same widget:
  - the same keys and mouse;
  - the same box search;
  - the same wires, groups and comments;
  - the same previews.

  Effects use the material graph's boxes plus a few of their own, not a second graph language.
- **One set of ideas.** An input, a curve, a colour gradient or a preset behaves the same in materials, effects, light and UI, and looks the same in Properties.
- **One place for settings.** Every selected thing is edited in Properties, laid out the same way. Every preview tab has the same controls.
- **One set of keys.** A key means the same thing in every editor. Shortcuts live in one table in the code, and a test fails if two editors give one key different jobs, or one job different keys. CONTRIBUTING rule 17 (asking about controls) applies to the table as a whole.
- **One set of languages.** Custom code anywhere (components, effects, editor tools) is written in Lust, Rust or Lua, the same three as game code (decisions 0002 and 0003). Shader code boxes, in WGSL, are the one exception: graphics cards only run shader languages.
- **The same menus.** Every editor's menus and right-click menus follow one layout, so "where would this be?" has one answer.

## Consequences

- A new editor starts from the shared pieces. Something it needs that the shared pieces lack is added to them, for every editor, not built on the side.
- The Phase 7, 9, 10 and 11 plans follow this. Their controls questions become one question, answered once, in Phase 7.
- CONTRIBUTING gains rule 21, which says the same in short.
