# 07 · Shared 5: screens, words and sound

ROADMAP.md (ideas 23 to 27 and 17): **text effects** (typing out, wavy and shaking words, small icons inside text); **button pictures that match the player's controller**; **tooltips, and drag and drop**; **translations and subtitles** (several languages, a check for missing lines, captions); **music that follows the game** (layers that fade in, changes on the beat). **Gate:** "a shop where items are dragged into a bag, with tooltips, in two languages; dialogue types itself out; the music swells when an enemy is near."

Five milestones, one per bullet. Order suggestion: **text layout first** (wrapping and shaping are missing and everything here needs them), then text effects, button pictures, tooltips and drag and drop, translations, music.

## What exists

- **Game UI** (`crates/ui`, decisions 0083, 0084, 0087 to 0089, 0107): screens are Lust `view`s (an `assembly … like Screen` in `assets/ui/`) run in their own Luau after each tick, reading the game; text laid out **left to right with kerning, a line per `\n`, no wrapping, no shaping** (decision 0084: "no wrapping yet"; `ui/README.md`: "Wrapping, shaping, a worker for letters and a project's own fonts come later"); letters as single-channel distance images in one atlas (`skrifa` reads fonts, `ab_glyph_rasterizer` draws outlines); Inter only; pictures from Atlas Crop; buttons, focus moved by `MenuMove`/`MenuChoose`/`MenuBack` from each tick's frame (so replays match); screens send events (decision 0088); player's text size and UI size (decision 0091). egui brings `skrifa` and **`harfrust`** (shaping) into the build already (game-ui plan, "Fonts").
- **Input** (`crates/input`): named actions, bindings (keyboard, mouse, gamepad via `gilrs`), recordings; `Input:binding(Action)` returns a text name of the bound input.
- **Sound** (`crates/sound`, Kira 0.12.5): sound events with random takes, buses (Master > Music, Effects, Voice, Ambience, UI), Voice ducking Music, reverb zones, listeners, 64 voices; Kira has **clocks** (ticks per second or per minute; sounds and tweens can start at a clock tick plus a fraction) that make beat-synced changes possible.

## M0 (prerequisite). Text layout: wrapping, shaping, fonts

Everything below needs words that wrap in a box, more than one font, and (for translations) shaping. **Ways:**
1. **Grow Solis's own layout** with line breaking (Unicode line-break rules, `unicode-linebreak`, MIT/Apache) and shaping through `harfrust` (already in the build), keeping the distance-image atlas.
2. **`parley`** (Linebender, Apache-2.0/MIT): layout, line breaking, bidirectional text and shaping in one; the game-UI plan's first choice before decision 0084 went simpler.
3. **`cosmic-text`**: full-featured, but rasterises bitmaps per size (outlines and glow on text need extra passes; the game-UI plan rejected it for that).

**Recommendation:** option 2 (`parley`) if its dependencies pass `deny.toml` and the dependency register (rule 5; it reuses `skrifa` and `harfrust` already in the build), otherwise option 1. Keep the distance-image atlas (sharp at any size, outlines and glow for free). Add a project's own fonts and fallback fonts (a font list per screen style).

**Size:** medium.

## M1. Text effects

**What it gives:** dialogue that types itself out letter by letter (with a sound per letter or word), wavy or shaking words, coloured words, small icons inside text (a coin, a button picture).

**Techniques:** inline markup in the string (`[wave]hello[/wave]`, `[shake]`, `[colour=gold]`, `[icon=Coin]`, `[pause 0.5]`), parsed into runs after translation, laid out with icons as inline boxes the height of a line, then **each letter is its own quad**, so per-letter offsets (wave = sin of time + letter index; shake = noise) are a vertex-shader job with no layout cost. Typing out is a reveal count over time.

**Design choices:**
- **Markup words:** a small fixed set (wave, shake, colour, bold, italic, icon, pause, speed), checked when a screen is checked: an unknown tag is a mistake in Problems, not silent text.
- **Game state or not:** the reveal is UI (real time, never in the game's state, decision 0087's "the UI reads, never writes"); "the line finished typing" reaches game code as a screen event (decision 0088), so dialogue advances deterministically through the next tick's input.
- **Icons:** `[icon=Picture.Ui.Coin]` uses Atlas Crop's names (decision 0089), and `[button=Action.Use]` shows the binding's picture (M2).

**Tests:** markup round trips; a typed-out line reveals the same letters at the same time at 30 and 144 fps; wave and shake never move layout; unknown tags reported. **Size:** medium.

## M2. Button pictures that match the controller

**What it gives:** "Press [A] to open" shows the right picture for the device the player last used (keyboard key, gamepad face button by maker), switching instantly.

- **Which device:** the input crate tracks the last device that produced input (keyboard/mouse or which gamepad), as part of the frame (so it's in recordings, though it never changes game state).
- **Pictures:** a **glyph set** per device family (keyboard and mouse, the common gamepad layouts) drawn by Solis as shapes (the distance-field UI can draw rounded keys with letters; face buttons as circles with symbols), so no third-party images and no licence questions (rule 20 also forbids copying other makers' assets); a project can override any picture with its own Atlas Crop picture.
- **Game code and screens:** `[button=Action.Jump]` in text; `Input:bindingPicture(Action.Jump)` as a screen picture; `Input:device()` as a state (`Device.Keyboard`, `Device.Gamepad`) for code that wants to know.

**Question:** naming gamepad buttons by position (south, east, west, north) is neutral; showing maker-specific symbols means drawing shapes that resemble them. **Recommended:** positions drawn as neutral shapes by default, a project option for maker-style letters/symbols drawn by Solis.

**Tests:** switching devices switches the picture next frame; a rebound action shows the new binding. **Size:** small to medium.

## M3. Tooltips, and drag and drop

**What it gives:** hovering an item shows its name and price; items are dragged from a shop into a bag.

- **Tooltips:** a `tip` on any view piece (Lust's views already have tips, LUST.md §17 "Tips"), shown after a delay at the pointer or beside the focused piece for gamepads (a gamepad has no hover: show the tip of the focused button after a delay).
- **Drag and drop:** Lust's view needs words for a draggable piece (carrying a value, e.g. an item ID or slot number), a drop target that accepts a kind, and the drop's result. **The drop must reach game code as an event** (decision 0088: screens send events that join the next tick's input and recordings), e.g. `on ItemDropped(int fromSlot, int toSlot)`, so inventories change only in game code and replays work. While dragging, the UI shows a ghost (UI-only state). **Gamepads:** pick up with Choose, move focus, put down with Choose (the same events).
- **Lust work:** drag/drop words in `view`, or a Solis convention on top of buttons. Ask the Lust side.

**Tests:** a drag replays identically from a recording; dropping outside cancels; gamepad pick-up/put-down sends the same events. **Size:** medium (plus a Lust change).

## M4. Translations and subtitles

**What it gives:** the game's text in several languages, with a check for missing lines; captions for sounds and speech.

**Ways to store translations:**
1. **Fluent** (`.ftl`, Project Fluent): messages with variables, plurals and gender handled by the translator per language; `fluent`/`fluent-bundle` crates are Apache-2.0 or MIT. Powerful; a format translators know.
2. **gettext** (`.po`): the classic, huge tool support, plural forms per language; source text as the key (fragile when the source changes).
3. **A Solis table per language** (Shared 4's data tables: key, then a column per language): editable in the editor, merges in Git, simple; plurals and variables need Solis's own rules.

**Recommendation:** text keys as **project states** (`Text.Shop.Welcome`, guideline 3: "strings are only for real text"; a typo is a mistake before running), with the strings in **Fluent files per language** (`locale/fr/shop.ftl`) so plurals and variables are right, and a **table view** in the editor that shows all languages side by side (built on Shared 4's table widget) and writes the `.ftl` files. A **check** lists keys missing in each language, unused keys, and variables that don't match (Problems entries). Players choose a language in the player's settings (decision 0091's file); the default follows Windows' language.

**Game code:** `World:text(Text.Shop.Welcome)` returns the string in the current language; with values: a generated method per message with its variables (`ShopText:welcome(playerName, gold)`) following rule 4. Screens use `text Text.Shop.Welcome` directly. Text shown is never game state (two players in different languages play the same game).

**Fonts and scripts:** Latin, Greek and Cyrillic work with shaping; **Chinese, Japanese and Korean** need fallback fonts with many letters (the atlas's least-recently-used eviction exists); **right-to-left** (Arabic, Hebrew) needs bidirectional layout, which `parley` provides. **Ask the lead which scripts matter** (question 2).

**Subtitles and captions:** a sound event (`.sound.lst`) gains an optional caption key (and a speaker), shown by a ready-made caption screen when the sound plays and the player turned captions on; dialogue lines get timing (a line per voice clip, or per cue). Accessibility: caption size and background from the player's settings.

**Tests:** a missing key in one language is listed; plural forms right for English, French and Polish (three different plural rules); fuzz the `.ftl` reader as a project file (rule 9: the crate's parser in a fuzz target); captions appear for a sound heard and not for one out of range. **Size:** medium to large.

## M5. Music that follows the game

**What it gives:** layers fade in when danger is near; changes land on the beat.

**Techniques:**
- **Vertical layering:** several stems of one piece play in sync; their volumes follow game values ("danger 0 to 1": drums at 0.3, brass at 0.7), smoothed.
- **Horizontal re-sequencing:** sections (calm, tense, battle) with transitions that wait for the next bar or beat, optionally through a stinger or transition piece.
- **Beat clock:** a tempo and time signature per piece; changes scheduled on Kira's clock at the next beat/bar (Kira schedules sounds and tweens at a clock tick plus a fraction; Solis converts beats to ticks).

**Recommendation:** a **music file** (`.music.lst`): pieces with tempo and metre, sections with their stems, transitions (when: now, next beat, next bar, end of section; how: crossfade time, stinger), and **parameters** (numbers like `danger`) that drive layer volumes through curves. Game code (rule 4, a generated file per music file): `music:setDanger(0.8)`, `music:goTo(BattleMusic.Battle)`. Like all sound, it never changes the game (decision 0056): a parameter set by game code lands at the tick's end, and the music follows on real time.

**Editor UI (mock-up first):** a Music tab with sections as rows of stems along a bar ruler (it can share the timeline widget with Shared 10's cutscenes and the Frames tab: decision 0050's "one way to work"), parameter curves, and a preview with sliders for the parameters. No new keys.

**Tests:** a change requested mid-bar starts on the next bar within one audio buffer; stems stay sample-locked; a parameter change fades smoothly; fuzz the file reader. **Size:** medium to large.

## Dependencies and order

- M0 (text layout) before M1 and M4.
- M1's `[button=…]` needs M2; M4's table view needs Shared 4's table widget.
- M3 and M1 need Lust view words (drag/drop; markup is Solis's own).
- M5's timeline widget can be shared with Shared 10 (cutscenes) and the Frames tab.

## Risks and questions for the project lead

1. **Text layout library:** **recommended** `parley` (shaping, wrapping, right-to-left) if it passes the dependency checks; alternative: grow Solis's own with line breaking and `harfrust`.
2. **Which writing systems for translations:** **recommended** Latin, Greek and Cyrillic now, Chinese/Japanese/Korean through fallback fonts in the same milestone, right-to-left later; alternatives: all now, or Latin only.
3. **Translation files:** **recommended** Fluent (`.ftl`) under a side-by-side table view; alternatives: Solis tables only; `.po`.
4. **Gamepad pictures:** **recommended** neutral shapes by position, with an option for maker-style symbols drawn by Solis; alternative: a project supplies its own pictures only.
5. **Text effect markup words:** **recommended** `[wave] [shake] [colour=…] [b] [i] [icon=…] [button=…] [pause n] [speed n]`.
6. **Music:** **recommended** layers and sections with beat-quantised changes in one music file; alternative: layers only (simpler).

## Crates

| Crate | Licence | Use |
| --- | --- | --- |
| `parley` | Apache-2.0 or MIT | text layout and shaping (candidate) |
| `unicode-linebreak` | Apache-2.0 or MIT | line breaking if Solis grows its own |
| `fluent` / `fluent-bundle` 0.16 / 0.15 | Apache-2.0 or MIT (with `unic-langid`, `intl-memoizer`) | translations |
| `kira` 0.12 (present) | MIT or Apache-2.0 | clocks for beat-synced music |

## Size

Phase: large (text layout, five milestones, two Lust changes).

## Sources

- Kira clocks: [kira clock source docs](https://docs.rs/crate/kira/latest/source/src/clock.rs), [kira README](https://docs.rs/crate/kira/0.12.0/source/README.md).
- Fluent: [fluent-rs](https://github.com/projectfluent/fluent-rs/), [fluent 0.16.1 licences](https://licenses.dev/cargo/fluent/0.16.1), [Rust i18n overview](https://www.arewewebyet.org/topics/i18n/).
- Game UI's current text: `crates/ui/README.md`, decisions 0084 and 0107, `docs/plans/game-ui.md` (parley, harfrust, skrifa).
- Adaptive music techniques (vertical layering, horizontal re-sequencing): [Winifred Phillips, A composer's guide to game music (overview)](https://mitpress.mit.edu/9780262534499/a-composers-guide-to-game-music/).
