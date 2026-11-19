# 0130. Text effects: markup in a screen's text, each letter its own shape, typing out on the game's ticks

- **Status:** Accepted (builds on 0087, 0088, 0089 and 0129; the project lead asked for the recommended answer to every choice while they slept, 2026-11-18)
- **Date:** 2026-11-19

## Problem

Stage 1's next milestone after text layout (decision 0122; [report 07](../reports/after-the-rethink/07-shared-5.md), M1) is text effects: dialogue that types itself out letter by letter, with a sound per letter or word; coloured, wavy and shaking words; small pictures inside text. The dialogue box of the overnight mock-up (`mock-4.png`) shows a coloured word, a wavy word and a line typing out. Game screens are Lust views, and Lust's view words are fixed (decision 0087), so whatever a text asks for must be said inside its words, on the Screen component, or through a call.

## Options

- **How a text asks for an effect** (the research's question 5): a small fixed set of words in square brackets inside the text, checked like code (recommended); Lust style words (Lust stays as it is); or effects set per piece on the Screen component (too coarse for one word).
- **How letters move:** each letter is already its own shape (decision 0084), so a wave or a shake moves only that shape, in the vertex shader or as offsets worked out as the shapes are made (both cost no layout); the layout laid out again each frame (never: decision 0129 keeps paragraphs).
- **Typing out as game state or not:** the reveal kept in the game (saved, replayed) or as UI (recommended, decision 0087: "the UI reads, never writes"), with "finished typing" reaching game code as an event in a tick's input (decision 0088).
- **Where the typing speed is set:** a Lust word on a text (Lust lacks one), a field on the Screen component, or a call.
- **Bold and italic** when a font file is one weight.

## Decision

- **Markup inside any game screen's text**, read after translation so a translated text carries its own: `[wave]…[/wave]`, `[shake]…[/shake]`, `[colour=yellow5]…[/colour]` (the screen's colour words, as Lust's `color` style has them, or `#hex`), `[b]…[/b]`, `[i]…[/i]`, `[icon=Picture.Ui.Coin]` (a picture by the project's name for it, Atlas Crop's crops included, decision 0089), `[pause 0.5]` and `[speed 2]`. `[[` writes a `[`. `=` or a space separates a word from its value. Effects nest and may overlap. **An unknown word, a word never closed, a bad value or a closing word with nothing open is a mistake** in Problems at its line and column when the screen's code is checked (text written out in `text "…"` or `button "…"`), said once in the Console for text worked out as the game runs, and shown as written, never hidden. `[button=Action.Use]` isn't built: it needs the pictures of each controller's buttons (a later milestone), and says so. `ui::markup` reads and writes it (`parse`, `write`; fuzzed).
- **Letters move as cheap offsets, never the layout:** a text's paragraph is laid out once with its plain words (decision 0129) and kept; each frame, each letter's shape is moved (a wave: a sine of the time and the letter's place; a shake: a hash of the letter and a 24th of a second), coloured, slanted or hidden as its effects say. Waves are 0.12 of the text's size high and go round 1.2 times a second; shakes reach 0.05 of its size. Only bold and icons change the layout, and only as the words are laid out.
- **Bold** uses the game's own bold (Inter SemiBold, wider, as the `bold` style does) for letters from the game's font; letters from a project's own font, one weight, are drawn thicker (the edge of their distance image moved out by 0.035 of the size). **Italic** slants every letter (its top leaning right a fifth of its height above the baseline), since no italic font is in the game and a project's italic file can't be paired with its regular one.
- **Icons** are laid out as a box as tall as the text's size, as wide as the picture's shape, between the words like a letter (a line may break before or after one), and drawn from the screens' pictures; a name with no picture is the grey box with its name, as for `image`.
- **Typing out is UI time, never game state.** The Screen component gains **Typing speed** (letters a second, 0 at once, the standard), **Typing sound** (one of the project's sound events, picked in Properties) and **Sound on** (each letter or each word). Every `text` piece on such a screen types out (buttons show at once), each from the tick its words first show, and again when they change; `[pause]` waits, `[speed 2]` types twice as fast from there (`[speed 0]` shows the rest at once). The clock is the game's ticks plus how far into the next one, so the same letters show at the same moments at 30 or 144 frames a second, and typing stops while play is paused. While a screen types, its view is read after every tick (rather than once a frame), so a line begins on the tick it shows whatever the frame rate.
- **Game code** hears `TypingFinished(Screen which)` on the entity showing the screen once every text on it has typed out since one began, with the next tick's input (decision 0088), so a recording keeps it and a replay hears it on the same tick. `finishTyping(Screen.Talk)` on the entity (Lua `solis.entity.finishTyping(self.me, "Talk")`, Rust `cx.host().finish_typing(cx.me(), "Talk")`) shows it all at the end of the tick (a player pressing a key to skip); `TypingFinished` follows. Game code can't ask how far a text has typed: that isn't the game's to know.
- **The typing sound** plays at most once a tick per screen, never on spaces, as a sound like any other (decision 0056: it never changes the game).
- **Lust stays as it is.** A request, ready to send, asks for a per-text typing speed and a `typed` hook on `text`.

## Consequences

- A `[` in a screen's text now starts markup: older screens that show a `[` write `[[` (a breaking change, said in the changelog), and Problems points at each.
- `ui::ElementKind::Text` carries `effects` (`ui::effects::TextEffects`: the runs, the icons, when each letter shows, and the text's number among its view's texts); `ui::Screen` carries `typing` (when each text began, `ui::typing::Started`); `ui::Shape` carries a letter's bold, slant and baseline, which the renderer's shapes now hold (176 bytes, from 160); `ui::Fonts::set_clock` gives the frame's UI time; `ui::TextStyle` carries `inline` (bold parts and icons' shapes), part of what a paragraph is kept by.
- The editor keeps each shown screen's texts' starts (`editor::screen_typing`), sends `TypingFinished` and plays the typing sound; the Game panel's view carries the UI clock to the renderer.
- Lust's own windows (`lust run`) show the markup as written.
- Scale: 2,000 letters waving, shaking and typing out (`ui`'s `scale_check_2000_letters_waving_shaking_and_typing_out`).
