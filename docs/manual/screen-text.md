# Text on screens: wrapping, alignment, fonts and fallback

> Words on a game screen wrap inside their box, are shaped as their font says, line up left, in the middle or right, can be cut short with "…", and can be in your project's own fonts, with fallbacks for letters a font doesn't have.

## When to use it

Every `text` and `button` on a [screen](screens.md) is laid out this way, with nothing to switch on: a long line of dialogue or an item's description wraps to fit its box by itself. Read this page when you want more than the standard look:

- **Your own font**, such as a pixel font for a pixel-art game or a handwritten one for a diary. Any TrueType (`.ttf`) or OpenType (`.otf`) file whose licence lets you ship it in a game will do.
- **Other writing systems.** Latin, Greek and Cyrillic letters work with the game's own font, Inter. For Chinese, Japanese or Korean, add a font that has those letters (Noto Sans JP, say) as a **fallback**.
- **Names that might be too long** for their box (an item's name in a slot): cut them with "…" rather than letting them spill out.
- **Spacing:** wide titles with room between the letters, or lines further apart.

The editor's own menus and panels aren't game screens: their text isn't changed by anything here.

## How to use it

**Wrapping.** A text wraps when it's wider than the room it's given: put it in a `column` with a `width` (or anywhere narrower than it), and its words move onto new lines where [Unicode's line-break rules](concepts.md#line-breaking) allow: between words, after a hyphen or a slash, between Chinese or Japanese letters. A word too long for a line on its own (a very long name) breaks between its letters. `\n` in the text always starts a new line, and `nowrap` keeps a text on one line.

```lust
assembly Quest {
    view [width 1920, height 1080] {
        column [width 600, pad 24, gap 8, background "#1e2228", round 8] {
            text "The old well" [size 32, bold, white];
            text "Someone has been dropping coins into the old well at night. Find out who, and why they look so worried." [white];
        }
    }
}

pass Quest;
end
```

```rust
// A screen is written in Lust, whatever language the game is in: the
// files in ui/ are Lust only. A Rust component shows it through its host:
//
//     cx.host().show_screen(cx.me(), "Quest", true);
```

```lua
-- A screen is written in Lust, whatever language the game is in: the
-- files in ui/ are Lust only. A Lua component shows it:
--
--     solis.entity.showScreen(self.me, "Quest")
```

**Alignment.** `left` (the standard), `center` and `right` on a text place each of its lines: a wrapped paragraph in the middle has every line in the middle, not just the first.

**Cutting with "…".** A text asks to be cut by having its own size limited:

- a wrapping text with a `height` or `maxHeight` shows the lines that fit, and its last line ends in "…": `text item.description [maxHeight 60];`
- a `nowrap` text with a `width` or `maxWidth` is cut on its one line: `text item.name [nowrap, maxWidth 180];`

A text without those keeps all its words, spilling past its box if it must, so nothing you write is hidden without you asking.

**Your project's own fonts.**

1. Put the font file in the Library, in a folder such as `fonts`. It's listed as a **Font**, with its own icon.
2. Name it in the screen with `font` and its place in the Library: `text "Inventory" [font "fonts/Pixel.ttf", size 24];`. On the view's own `[ ]` it's the font of every piece that doesn't name one: `view [width 1920, height 1080, font "fonts/Pixel.ttf"] { … }`.
3. Press Play, or look at the screen on the [UI editor's canvas](ui-editor.md): the font is read on its own, without the editor waiting, and the screen is drawn in it.

A font file is one weight: `bold` doesn't make a font of your own bold (it still makes the game's Inter bold). Use the bold file of your font instead.

**Fallback fonts.** List more fonts in the same `font`, between commas, first choice first:

```lust
assembly Dialogue {
    view [width 1920, height 1080, font "fonts/Lora.ttf, fonts/NotoSansJP.otf"] {
        column [width 900, pad 24] {
            text "Merchant: いらっしゃいませ! Welcome!" [size 28, white];
        }
    }
}

pass Dialogue;
end
```

```rust
// A screen is written in Lust, whatever language the game is in: the
// files in ui/ are Lust only. A Rust component shows it through its host:
//
//     cx.host().show_screen(cx.me(), "Dialogue", true);
```

```lua
-- A screen is written in Lust, whatever language the game is in: the
-- files in ui/ are Lust only. A Lua component shows it:
--
--     solis.entity.showScreen(self.me, "Dialogue")
```

Each letter comes from the first font in the list that has it: here the English from Lora, the Japanese from Noto Sans JP, which Lora lacks. After the list comes the game's Inter, so a letter none of your fonts has is still drawn if Inter has it; a letter no font has shows as the first font's empty box. Spaces, and accents written as a letter of their own, stay with the letter before them.

> **Good to know:** a font with Chinese, Japanese or Korean letters is big (often 5 to 20 MB). Only the fonts a screen names are read, so list one only on the screens that need it. A font may be at most 32 MB, and screens can use at most 32 of a project's fonts.

**In game code,** a field can hold one of the project's fonts, as it can a sound or a picture: its type is `Font`, and Properties offers a list of the project's fonts to pick from, which follows a font file you rename. The fonts are `Font.Fonts.Lora` for `fonts/Lora.ttf` (see [Game code](game-code.md)).

```lust
-- sign.lst: a sign whose words and font are picked in Properties.
fetch "solis/project":[Font];

component Sign {
    has {
        Font font = Font.None;
        string words = "Welcome";
    }
}

pass Sign;
end
```

```rust
use reflect::{ComponentSchema, FieldInfo, FieldKind, Hint, NameKind, Offer, Value};

// A Rust component's field picks one of the project's fonts with an offer
// of Font names; it holds the font's name ("Fonts.Lora"). (Its update,
// values, set and RustKind are as in Game code, so they're left out.)
pub fn sign_schema(fonts: Vec<String>) -> ComponentSchema {
    let font = FieldInfo::new("font", "Font", FieldKind::Choice(fonts), Value::Text("None".into()))
        .hint(Hint {
            offers: Offer::Names(NameKind::Font),
            ..Hint::default()
        });
    ComponentSchema {
        name: "Sign".into(),
        help: "A sign whose words and font are picked in Properties.".into(),
        fields: vec![font],
    }
}
```

```lua
-- sign.lua: a field whose kind is Font picks one of the project's fonts.
local Sign = component("Sign", {
    { "font", "Font", "None" },
    { "words", "string", "Welcome" },
})
```

## Settings

The **Screen** component's two settings for its text:

| Setting | What it does |
|---|---|
| Line height | The room from one line to the next, as a share of the font's own (100%). 120 to 150% makes long paragraphs easier to read; under 100% packs a list tighter. 50 to 300%. |
| Letter spacing | Room added after every letter, in pixels of the screen's design size. 2 to 4 suits titles in capitals; a little less than 0 tightens a label. −10 to 50. |

The player's **text size** ([Game screens](screens.md#how-to-use-it)) still grows every text, and wrapping makes room for it.

## Tips and common problems

- **Problem:** Problems says "there's no font fonts/Lora.ttf in the project". **Why:** `font` names a font by its place in the Library, with `/` between folders and its ending. **Fix:** check the path in the Library; capitals don't matter.
- **Problem:** Problems says a font "isn't a font" or "can't be used: it isn't a TrueType or OpenType font Solis can read". **Why:** the file is something else with a font's ending, or damaged. WOFF and WOFF2 web fonts aren't read. **Fix:** use the `.ttf` or `.otf` file from the font's download.
- **Problem:** some letters show as empty boxes. **Why:** none of the fonts listed has them, and neither does Inter. **Fix:** add a font that has them to the end of the `font` list.
- **Problem:** a line of Arabic or Hebrew reads backwards. **Why:** right-to-left writing isn't laid out yet; it comes in a later milestone. **Fix:** for now, keep screens in left-to-right writing systems.
- **Problem:** a long name spills out of its slot. **Why:** text is only cut when it asks to be. **Fix:** give it `nowrap` and a `maxWidth` (or a `maxHeight` for wrapped text).
- **Problem:** a font named on the view isn't used by one piece. **Why:** a piece that names its own `font` uses that instead (only that list, then Inter). **Fix:** list the view's font after the piece's own, between commas.
- **Problem:** Lust's own window (`lust run`) shows a screen with a list of fonts in Inter. **Why:** Lust's windows take one font file, and read the list as a file they can't find. **Fix:** nothing to do: the game draws it right. Lust is asked for lists of fonts.

## Related

- [Game screens](screens.md): writing screens, and the Screen component.
- [The UI editor](ui-editor.md): the canvas draws text as the game does.
- [Concepts](concepts.md): [shaping](concepts.md#shaping), [line breaking](concepts.md#line-breaking), [fallback font](concepts.md#fallback-font).
