# 0087. Screens are Lust's own views, read through the engine; Lust stays as it is

- **Status:** Accepted (the project lead, 2026-11-05: "I want Lust to stay relatively similar, and instead work with API calls through the engine"; components readable from the start; replaces the game UI plan's choices 3 and 4, and the Lust words its M2 planned)
- **Date:** 2026-11-05

## Problem

The game UI plan's M2 (screens in Lust, with values tied to the game) planned new Lust words: `tied` on a screen's fields (a value follows a component's field), drawing words (`box`, `ellipse`, `line`, `path`), `name`, `anchor`, `at`, `like Screen`, and screen rules in Lust's checker. Lust is the project lead's language, and these words would only ever mean something inside Solis.

## Options

1. **New Lust words**, as planned: screens read like Lust's own windows with new shapes, and ties are written in the language.
2. **Values from the engine, shapes in Lust:** no tie word, but new drawing words.
3. **Everything from the engine** (chosen): a screen is an ordinary Lust `assembly` with a `view`, using only the pieces Lust already has, and it reads the game through calls the engine offers, as game code does.

## Decision

- **A screen is a Lust file in `assets/ui/`** holding an `assembly` with a `view`, passed with its file's name (`ui/hud.lst` passes `Hud`). Only Lust's own pieces and styles are used: `column`, `row`, `stack`, `space`, `text`, `progress`, and `canvas` with `box`, `circle` and `line`; padding, gaps, sizes (`fill` too), alignment words, colours, `round`, `border` and `opacity`. The view's `[width 1920, height 1080]` is the size it's designed at, scaled with the screen's height (choice 10). Lust checks it like any file; Lust's own `lust run` can even show it in a window.
- **Solis runs screens in a Luau of their own,** never the game's, once per tick after the tick lands. Solis compiles the screens into a program of their own (an entry Solis writes, opening each screen), and gives it reading calls only. The view's tree (Lust's `__view()`, the same tree Lust's own windows draw) is laid out with Lust's rules and drawn with the shapes and text of decisions 0083 and 0084.
- **Values are read through the engine:**
  - **Game-wide objects** (`Clock.hour`, `Money.amount`, decision 0080) are copied into the screens' Luau after each tick, exactly as every core gets them.
  - **The entity a screen is for** is `Ui:entity()`: the entity whose **Screen** component shows it.
  - **A component's values** are read with a method on the entity, written by Solis for each of the game's components into a file only screens fetch: `Ui:entity():health().hp`. What it gives is a copy of the component as the tick left it; a missing component stops the screen with a message naming the line (API guideline 11).
  - The engine's reading calls (`time`, `position`…) as game code has them.
- **A screen can't change the game.** Writing calls aren't in the screens' Luau, and anything a screen changes in its copies is thrown away at the next tick. Buttons that tell the game something (sending an event, pressing an action) are M3's, through the next tick's input.
- **The `Screen` component** (built in) shows a screen: the screen file, whether it's shown, and its order among several. Game code shows and hides it by changing `shown`, so which screens are open is game state: saved, replayed and the same on every computer.
- No new Lust word. The editor's canvas (M4) edits the same Lust text.

## Consequences

- Lust needs no change for M2. Two small Lust fixes may help later: pieces of your own inside a screen (Lust's runtime keeps their memory only while a window is open), and a way for an engine to list a file's assemblies (until then, a screen file passes the assembly named after it).
- The plan's ties, `name`, `anchor` and drawing words go. Anchoring is Lust's layout (`bottom`, `right`, `pad`, `width fill`), and elements are found by their place in the tree.
- A component named like an engine call on entities (`Position`) can't be read by a screen; Problems says so and suggests a new name.
- The gate's health bar reads the drone's `Health` through `health()`, and the farm HUD reads the clock and money objects.
