# Subtitles and captions: words for what's said and heard

> A sound with a caption shows its words low on the screen when it's heard, and a voice line shows its speaker and what they say, each in the player's language, as big and on as dark a box as the player chose.

## When to use it

Captions and subtitles let everyone follow your game: players who can't hear well, players with the sound down, and players reading a language they know better than the one spoken.

- **A subtitle** is what a character says: "**Gardener:** Oh! A new face in the village." Players usually leave subtitles on.
- **A caption** is a sound worth knowing about: "[The door creaks]", "[Footsteps behind you]". Many players leave these off, so they start off.

Solis draws them itself, over every screen your game shows, so you don't make a screen for them. You give each sound its words, and each player chooses how they see them.

## How to use it

1. Open a [sound event](sound-events.md) (click it in the Library). Under **Caption**, type the [key](concepts.md#text-key) of its words in **Words**: `Captions.DoorCreaks`.
2. Write the key's words in the [Translations](translations.md) tab, in each language: `[The door creaks]`, `[Døren knirker]`.
3. For a voice line, also type who says it in **Speaker**, as a key too (`Speakers.Gardener`, whose words are the character's name), so the name is translated with everything else.
4. Play the game and turn **Captions for sounds** on in Project settings › Languages' card. When the sound plays where the player can hear it, its words show for as long as it plays (at least two seconds, at most eight, so they can be read).


**What counts as heard.** A caption shows only for a sound the player hears: one in range of the [listener](listeners.md) (nearer than its **Silent past**), one the listener is meant to hear (its **Heard by**), and one whose bus game code hasn't turned down to nothing. A sound too far away, or one never started because too many played at once, shows nothing, just as it sounds like nothing. A looping sound shows its caption as it starts.

**The player's choices.** Each player chooses, in your game's own options menu:

| Choice | Game code |
|---|---|
| Subtitles on or off | `Screens.subtitles = true`, `Screens.subtitles` |
| Captions for sounds on or off | `Screens.captions = true`, `Screens.captions` |
| Size (1 as designed; 0.75 small, 1.5 large) | `Screens.captionSize = 1.5`, `Screens.captionSize` |
| How dark the box behind them is (0 none, 1 solid) | `Screens.captionBackground = 0.6`, `Screens.captionBackground` |

They're the player's [settings](concepts.md#player-setting), kept for the next time the game starts, never in save games. Here's an options menu's code for them:

```lust
-- options.lst: an options menu's choices for captions.
fetch "solis/screens":Screens;

component Options {
    -- Turn captions for sounds on or off.
    method switchCaptions() {
        Screens.captions = not Screens.captions;
    }

    -- Step the size through small, medium and large.
    method nextCaptionSize() {
        float size = Screens.captionSize + 0.25;
        is size > 1.5? then {
            size = 0.75;
        }
        Screens.captionSize = size;
    }
}
pass Options;
end
```

```rust
use script::{Context, RustComponent};

pub struct Options;

impl Options {
    /// Turn captions for sounds on or off.
    pub fn switch_captions(cx: &mut Context<'_>) {
        let on = cx.host().settings().captions;
        cx.host().set_setting(script::settings::Setting::Captions(!on));
    }

    /// Step the size through small, medium and large.
    pub fn next_caption_size(cx: &mut Context<'_>) {
        let mut size = cx.host().settings().caption_size + 0.25;
        if size > 1.5 {
            size = 0.75;
        }
        cx.host().set_setting(script::settings::Setting::CaptionSize(size));
    }
}

impl RustComponent for Options {
    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
local Options = component("Options", {})

-- Turn captions for sounds on or off.
function Options:switchCaptions()
    solis.Screens:setCaptions(not solis.Screens:captions())
end

-- Step the size through small, medium and large.
function Options:nextCaptionSize()
    local size = solis.Screens:captionSize() + 0.25
    if size > 1.5 then
        size = 0.75
    end
    solis.Screens:setCaptionSize(size)
end
```

While you play in the editor, set them on the card beside the languages in Project settings › Languages; the editor keeps them from one play to the next.

> **Good to know:** the words follow the player's [language](translations.md). Change it while the game plays and the next caption is in the new language.

## Settings

| Setting | What it does |
|---|---|
| Words (a sound event) | The key of the words shown when it's heard. |
| Speaker (a sound event) | The key of who says it: a voice line, shown as a subtitle with their name. |
| Subtitles | The player's: voices' words shown. On to start with. |
| Captions for sounds | The player's: sounds' words shown. Off to start with. |
| Size | The player's: how big the words are, 0.5 to 2. |
| Background | The player's: how dark the box behind the words is, 0 to 1 (0.6 to start with). |

## Tips and common problems

- **Problem:** a sound plays but no caption shows. **Why:** captions for sounds are off (they start off), the sound has no Words, or it was too far away to hear. **Fix:** turn Captions for sounds on, give it Words, or check its Silent past.
- **Problem:** the caption shows the key itself, `Captions.DoorCreaks`. **Why:** no language has a line for that key. **Fix:** make it in the Translations tab (+ Key).
- **Problem:** a voice line shows no name. **Why:** its Speaker is empty. **Fix:** type the speaker's key.

## Related

- [Translations](translations.md), [Sound events](sound-events.md), [Listeners and who hears what](listeners.md).
- [Concepts](concepts.md): [caption and subtitle](concepts.md#caption), [player setting](concepts.md#player-setting).
