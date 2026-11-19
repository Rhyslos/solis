# Text effects: colours, waves, icons and typing out

> Words inside a screen's text can be coloured, bold, slanted, wavy or shaking, can hold a small picture such as a coin, and a whole text can type itself out letter by letter with a sound, telling your game code when it's done.

## When to use it

Use text effects to give a screen's words some life without building anything new:

- **Dialogue** that types itself out, a letter at a time, with a soft blip for each letter, pausing after "Well..." and rushing through an excited line. The player can skip ahead, and your game code hears when the line has finished, to show the choices or the next line.
- **A word that matters** in another colour (the item a character asks for), in bold, or slanted for a thought.
- **A wavy word** for something magical or silly, or **shaking** words for fear or anger.
- **A small picture inside a line:** "Costs 5 (a coin) each" with your own coin picture drawn in the line, or an item's icon in a shop's description.

The effects are written inside the text itself, as **markup**: a few words in square brackets, such as `[wave]hello[/wave]`. That keeps them with the words they belong to, so a line translated into another language carries its own effects.

Typing out is for text that's read once, like dialogue. A HUD's numbers or a menu should show at once, so leave typing off on their screens.

## How to use it

**Markup.** Write the effect's word in square brackets before the words it changes, and the same word with a `/` after them:

| Write | What it does |
|---|---|
| `[wave]magic[/wave]` | The letters rise and fall in a wave. |
| `[shake]run![/shake]` | The letters shake. |
| `[colour=red5]roses[/colour]` | The letters in a colour: one of the screen's colour words (`white`, `red5`, `yellow7`, `teal`, the same ones as a piece's colour) or `#hex` (`[colour=#e8955a]`). |
| `[b]now[/b]` | Bold. |
| `[i]maybe[/i]` | Slanted (italic). |
| `[icon=Picture.Ui.Coin]` | A picture inside the text, as tall as the text. It's named as game code names pictures: `Picture.Ui.Coin` is `ui/coin.png`, or the crop called Coin cut from `ui.png` with [Atlas Crop](atlas-crop.md). |
| `[pause 0.5]` | While typing out: wait half a second here. |
| `[speed 2]` | While typing out: from here, twice as fast (`[speed 0.5]` half as fast, `[speed 1]` back to normal, `[speed 0]` the rest at once). |
| `[[` | A `[` in the text itself: `Press [[E]` shows "Press [E]". |

Effects can sit inside each other: `[colour=red5][wave]hot[/wave] soup[/colour]`. A text can mix as many as it likes:

```lust
-- ui/talk.lst: a dialogue box with a speaker's name and a line.
assembly Talk {
    view [width 1920, height 1080] {
        stack [width fill, height fill, bottom, pad 48] {
            column [width 1600, pad 24, gap 8, background "#121619", round 8] {
                text "Mira" [size 26, bold, yellow5];
                text "Oh! A new face in the village. [pause 0.4]Have you seen my [colour=#e8955a]red roses[/colour]? They [wave]wiggle[/wave] when it rains. Only 5 [icon=Picture.Ui.Coin] a bunch!" [size 30, white];
            }
        }
    }
}

pass Talk;
end
```

```rust
// A screen is written in Lust, whatever language the game is in: the
// files in ui/ are Lust only, and so is their markup. A Rust component
// shows the screen through its host:
//
//     cx.host().show_screen(cx.me(), "Talk", true);
```

```lua
-- A screen is written in Lust, whatever language the game is in: the
-- files in ui/ are Lust only, and so is their markup. A Lua component
-- shows the screen:
--
--     solis.entity.showScreen(self.me, "Talk")
```

The words of a text can come from the game too (`text line;`, with `line` a field holding `"Hello [wave]there[/wave]"`): the markup is read when the text is shown, whatever made it.

> **Good to know:** waves and shakes only move each letter where it's drawn. The words around them never move, and the box doesn't change size as they move, so nothing else on the screen jiggles.

**A mistake in the markup** is never hidden: the text shows the brackets as you wrote them, so you see where it went wrong, and **Problems** lists it with its line and what to write instead ("[glow] isn't a text effect", "[wave] is never closed: end it with [/wave]"). Text worked out as the game runs is checked when it's shown, and the Console says what's wrong, once.

**Typing out.**

1. Pick the entity whose **Screen** component shows the dialogue screen.
2. Set **Typing speed** to how many letters a second it types: 30 to 60 suits dialogue. Every `text` on that screen types out (buttons show at once), each time its words change.
3. To hear it, pick a sound event in **Typing sound** (a short blip works best), and whether it plays on **Each letter** or **Each word** in **Sound on**.
4. Press Play.

A text on a typing screen that should show at once, such as the speaker's name, starts with `[speed 0]`: `text "[speed 0]" + speaker;`.

**Hearing when it's done, and skipping.** When every text on the screen has typed out, the entity showing it hears `TypingFinished`, with which screen it was. To skip, call `finishTyping` with the screen: the rest shows at once, and `TypingFinished` follows. Here, pressing Use shows the whole line, and pressing it again once the line is done shows the next one:

```lust
-- talker.lst: a character's lines, one after another.
fetch "solis/input":Input;
fetch "solis/project":[Action, Screen];
fetch "solis/screens":TypingFinished;

component Talker {
    has {
        int line = 0;
        bool typed = false;
    }

    method update(float dt) {
        is Input:pressed(Action.Use)? then {
            is typed? then {
                line = line + 1;
                typed = false;
            } or {
                finishTyping(Screen.Talk);
            }
        }
    }

    on TypingFinished(Screen which) {
        is which == Screen.Talk? then {
            typed = true;
        }
    }
}

pass Talker;
end
```

```rust
use reflect::Value;
use script::{Context, Event, RustComponent};

pub struct Talker {
    line: i64,
    typed: bool,
}

impl RustComponent for Talker {
    fn update(&mut self, cx: &mut Context<'_>, _dt: f64) {
        if cx.host().action("Use").pressed {
            if self.typed {
                self.line += 1;
                self.typed = false;
            } else {
                let me = cx.me();
                cx.host().finish_typing(me, "Talk");
            }
        }
    }

    fn handle(&mut self, _cx: &mut Context<'_>, event: &Event) -> bool {
        if event.name != "TypingFinished" {
            return false;
        }
        if let Some(Value::Text(which)) = event.values.first()
            && which == "Talk"
        {
            self.typed = true;
        }
        true
    }

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
local Talker = component("Talker", {
    { "line", "int", 0 },
    { "typed", "bool", false },
})

function Talker:update(dt)
    if solis.Input:pressed("Use") then
        if self.typed then
            self.line = self.line + 1
            self.typed = false
        else
            solis.entity.finishTyping(self.me, "Talk")
        end
    end
end

function Talker:on_TypingFinished(which)
    if which == "Talk" then
        self.typed = true
    end
end
```

(The screen shows the line its `line` number picks, read from the entity with a reading method, as [Game screens](screens.md) shows.)

> **Good to know:** how far a text has typed isn't part of the game: it's what the screen shows, like a button's hover. That's why game code can't ask for it, and why it doesn't go in save games. What game code hears, `TypingFinished`, comes with a tick's input like a button's event, so a [recording](play-mode.md) keeps it and plays it back on the same tick. Typing follows the game's ticks, so a line types the same at any frame rate, and stops while the game is paused in the editor.

## Settings

The **Screen** component's settings for typing out:

| Setting | What it does |
|---|---|
| Typing speed | How many letters a second its texts type out at, each time their words change. 0 (the standard) shows them at once. 30 to 60 suits dialogue; 1,000 at most. |
| Typing sound | A sound event played as the text types, for a voice of blips. None for silence. It plays at most once a tick, and never for a space. |
| Sound on | **Each letter**, or **Each word** (only a word's first letter): softer for fast text. |

What each effect looks like:

| Effect | How |
|---|---|
| Wave | Each letter rises and falls by an eighth of the text's size, a little behind the one before, going round about once a second. |
| Shake | Each letter jumps about a twentieth of the text's size, 24 times a second. |
| Bold | The game's own bold (Inter's), a little wider. A font of your own is one weight, so its letters are drawn thicker instead. |
| Italic | Every letter slants, its top leaning right a fifth of its height. No font is swapped. |
| Icon | As tall as the text's size, as wide as the picture's shape; a line can break before or after it. Shown in the picture's own colours. |

## Tips and common problems

- **Problem:** Problems says "[glow] isn't a text effect". **Why:** only the words in the table above are effects. **Fix:** use one of them, or write `[[glow]` to show the brackets as text.
- **Problem:** Problems says "[wave] is never closed". **Why:** every `[wave]`, `[shake]`, `[colour=…]`, `[b]` and `[i]` needs its end (`[/wave]`). Until then it runs to the end of the text. **Fix:** add the end where the effect should stop.
- **Problem:** Problems says a colour "isn't a colour". **Why:** colours are the screen's colour words (`red5`, `yellow7`, `white`…) or `#hex`. Names such as `gold` aren't among them. **Fix:** use `#e2b54a` for gold, or a colour word.
- **Problem:** Problems says "there's no picture Picture.Ui.Coin in the project". **Why:** an icon names a picture as game code does: its place in the Library, folders and name each starting with a capital, without the ending. **Fix:** check the picture's name in the Library, or cut a crop of that name with Atlas Crop.
- **Problem:** `[button=Action.Use]` is a mistake. **Why:** pictures of a controller's buttons inside text come in a later milestone. **Fix:** name the button in words for now.
- **Problem:** an older screen now shows a mistake where it used to show `[E]`. **Why:** a `[` starts markup now. **Fix:** write `[[E]`.
- **Problem:** `TypingFinished` never comes. **Why:** it goes to the entity whose Screen shows the screen, and only once **Typing speed** is above 0. **Fix:** put the component on that entity, and set a speed.
- **Problem:** the speaker's name types out too. **Why:** every `text` on a typing screen types. **Fix:** start it with `[speed 0]`.
- **Problem:** Lust's own window (`lust run`) shows the brackets. **Why:** markup is Solis's: Lust's windows show text as it's written. **Fix:** nothing to do; the game draws it right.

## Related

- [Game screens](screens.md): writing screens, and the Screen component.
- [Text on screens](screen-text.md): wrapping, alignment, fonts and fallbacks, which effects build on.
- [Atlas Crop](atlas-crop.md): cutting the small pictures icons show.
- [Sound events](sound-events.md): making the typing sound.
- [Concepts](concepts.md): [text effects](concepts.md#text-effects), [typing out](concepts.md#typing-out).
