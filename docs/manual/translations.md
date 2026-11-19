# Translations: your game's text in several languages

> Every line your game shows has a key, such as `Shop.Welcome`, and its words in each language you add; a player reads the game in their language, and a line not translated yet shows in the language you write in.

## When to use it

Use translations for any words a player reads: menus, signs, item names and descriptions, what characters say, and captions. Even a game in one language is worth writing this way from the start:

- **Nothing is lost later.** Adding a language later means sending a spreadsheet to a translator, not hunting through code for every sentence.
- **A misspelt key is caught before the game runs.** Game code names a line as `Text.Shop.Welcome`, one of the project's names, so `Text.Shop.Welcom` is a mistake in [Problems](problems.md) with its line, never a blank sign.
- **Plurals come out right in every language.** "1 apple, 2 apples" in English; French says "0 pomme"; Polish has three forms. Each translator writes their own language's forms.

Words that aren't the game's text (a name the player typed, a number) aren't translated: show them as they are, or pass them into a line as a value (below).

## How to use it

**Your languages.** Open **Project settings › Languages** (File › Project settings). Your project starts in English, its **main language**: the one you write in. Its text is shown wherever another language has no line yet, so the game works before anything is translated.

1. Press **+ Language** and pick one from the list, or type its short code (`fr`, `nb`, `pt-BR`) and press `Enter`.
2. To write in another language yourself, open its row's ⋯ and choose **Make main**.

Each row shows how much of that language is translated, and how many lines it's missing.

![Project settings › Languages: English as the main language, Norsk nearly done, Deutsch under half done, and the player's subtitle and caption settings beside them](pictures/translations-languages.png)
The Languages page: each language's share translated, + Language, the Translations tab, and the spreadsheet out and back in.

**Lines of text.** A line's **key** is a group and a name joined by a dot, such as `Shop.Welcome` or `Menu.Options.Volume`. Make one in the **Translations** tab (Project settings › Languages › **Open the Translations tab**): press **+ Key**, type the key and press `Enter`, then type its words in the main language's column.

The tab shows every line, the main language beside one other (choose which at the top). A line that language lacks says **missing** in red. Type in a cell to write it: `Enter` or `F2` starts typing, `Enter` keeps it and moves down, `Tab` keeps it and moves across, `Esc` leaves it as it was, and `Ctrl + Z` undoes. **Missing only** shows just the lines still to translate, and the search finds a key or any words.

![The Translations tab: each key with its words in English and Norsk, a missing line marked in red](pictures/translations-tab.png)
The Translations tab, beside the level's view.

> **Good to know:** a key is made once and never changed. Change a line's words as often as you like, but keep its key: translators' work is found by it. To rename a line, make a new key.

**Where the words are kept.** Each language's lines are in small text files, `locale/<code>/<group>.ftl` (`locale/nb/shop.ftl` for Norsk's shop lines), in the Fluent format translators know. You never need to open them, but you can, and Git merges them line by line. In a file, a key's dots are dashes:

```text
# locale/nb/shop.ftl
Shop-Welcome = Velkommen!
Shop-Apples = { $count ->
    [one] { $who } har ett eple
   *[other] { $who } har { $count } epler
}
```

**Values and plurals.** A line can hold values, written `{ $name }`. A value that's a number can pick between forms: `{ $count -> [one] … *[other] … }`, where `*` marks the form used when no other fits. Each language has its own forms (`one`, `few`, `many`, `other`), and its translator writes the ones it needs.

**In game code.** `World:text(Text.Shop.Welcome)` gives a line in the player's language. A line that takes values is a method on its group's object, `ShopText`, in `solis/texts`, taking its values in the order the main language first uses them; a number is a `float`, anything else a `string`:

```lust
-- shopkeeper.lst: greets whoever walks into the shop, in the player's language.
fetch "solis/project":Text;
fetch "solis/world":World;
fetch "solis/texts":ShopText;
fetch "solis/zones":Entered;

component Shopkeeper {
    -- Someone walked in: welcome them, and say how many apples are left.
    on Entered(entity who) {
        World:say(World:text(Text.Shop.Welcome));
        World:say(ShopText:apples(3, "Mira"));
    }
}
pass Shopkeeper;
end
```

```rust
use script::{Context, Event, RustComponent};
use translations::TextValue;

pub struct Shopkeeper;

impl RustComponent for Shopkeeper {
    fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
        if event.name != "Entered" {
            return false;
        }
        // Someone walked in: welcome them, and say how many apples are left.
        let welcome = cx.host().text("Shop.Welcome");
        let apples = cx.host().text_with(
            "Shop.Apples",
            &[("count", TextValue::Number(3.0)), ("who", TextValue::Text("Mira".into()))],
        );
        cx.host().say(&welcome);
        cx.host().say(&apples);
        true
    }

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
local Shopkeeper = component("Shopkeeper", {})

-- Someone walked in: welcome them, and say how many apples are left.
function Shopkeeper:on_Entered(who)
    solis.World:say(solis.World:text("Shop.Welcome"))
    solis.World:say(solis.ShopText:apples(3, "Mira"))
end
```

On a screen, show a line with `text World:text(Text.Shop.Welcome);`. Text effects inside a line (`[wave]`, `[colour=…]`) are read after it's translated, so each language carries its own.

**Data tables.** Mark a [data table](data-tables.md)'s text column **Translated** (right-click its heading). Each of its cells then has a key, `Items.IronSword.description`, listed in the Translations tab; its main-language words stay in the table, and the other languages' go in `locale/<code>/items.ftl`. Game code reading `Items:row(Item.IronSword).description` gets it in the player's language.

**The player's language.** A player reads the game in the language they chose, else the one Windows is set to when your project has it, else the main language. Offer the choice in your own menu: `Screens:setLanguage(Language.Nb)`, `Screens:language()`, and `Screens:languageName(Language.Nb)` for the name to show (`Norsk`). The change shows from the next tick. While you play in the editor, choose it on the Languages page's card.

**Translators' spreadsheet.** **Export spreadsheet** writes `translations.csv` in your project's folder: a column of keys, the main language, and a column for each language. A translator fills in their column in any spreadsheet program; **Import** reads it back and writes each changed line into its file. A column for a language your project hasn't, or a key it hasn't, is passed over, and the Console says so.

> **Careful:** text shown isn't part of the game. Two players in different languages play the same game, so never let a line's words decide what happens (comparing them, or keeping them in a component's field): decide with values and states, and turn them into words only to show them.

## Settings

| Setting | What it does |
|---|---|
| Main language | The language you write in, first in the list. Its words are shown wherever another language lacks a line. |
| + Language | Adds a language by its short code: `fr`, `de`, `nb`, `pt-BR`, `zh-Hans`. |
| Remove (a row's ⋯) | Takes the language out of the project. Its files in `locale/` are kept. |
| Translated (a table column's menu) | Gives each cell of a text column a key, translated like any line. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Type in a cell | `Enter`, `F2`, or start typing |
| Keep it and move down, or across | `Enter`, `Tab` |
| Leave the cell as it was | `Esc` |
| Copy or paste cells | `Ctrl + C`, `Ctrl + V` |
| Undo or redo | `Ctrl + Z`, `Ctrl + Y` |

## Tips and common problems

- **Problem:** Problems says "Norsk is missing 12 of 48 lines". **Why:** those lines have no Norwegian yet, so players see the main language there. **Fix:** open the Translations tab, choose Norsk and Missing only.
- **Problem:** "lines of the main language nothing uses". **Why:** no game code, screen or sound names those keys. **Fix:** use them, or take them out (empty a line's main-language cell).
- **Problem:** "uses $navn, which English doesn't give". **Why:** a translation names a value the line never gets; it would show as written. **Fix:** use the same value names as the main language.
- **Problem:** a cell won't keep what you typed: "That text isn't Fluent". **Why:** `{` and `}` mark values. **Fix:** write `{ $name }` around a value, and `{"{"}` for a brace itself.
- **Problem:** Chinese, Japanese or Korean show as boxes. **Why:** the game's own font has only Latin, Greek and Cyrillic letters. **Fix:** add a font with them to the project and name it after the screen's font, as [Text on screens](screen-text.md) shows. Right-to-left languages (Arabic, Hebrew) aren't laid out yet.

## Related

- [Subtitles and captions](subtitles-and-captions.md): the words shown for voices and sounds.
- [Data tables](data-tables.md), [Text on screens](screen-text.md), [Text effects](text-effects.md).
- [Concepts](concepts.md): [main language](concepts.md#main-language), [text key](concepts.md#text-key), [Fluent](concepts.md#fluent).
