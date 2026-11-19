# Data tables

> Your game's data, such as every item with its price and description or every crop with its days to grow, in one table you edit like a spreadsheet, and game code reads by name with every spelling checked.

## When to use it
Whenever a game has **many things of one kind that differ only in their numbers and words**: items in a shop, enemies with their health and speed, crops, spells, levels' par times, a list of quests' titles. Without a table, those numbers end up scattered through code (`is item == Item.Sword? then { price = 120; }`) or copied into many components, and changing a price means hunting for it.

A [data table](concepts.md#game-code) keeps them together: one row a thing, one column a fact about it. You edit it in the **Table** tab, which works like a spreadsheet, and game code asks for a row by its ID: `Items:row(Item.IronSword).price`. Because Solis turns the table's IDs and columns into names game code checks, a misspelt row (`Item.IronSwrod`) or column (`.prise`) is a mistake in [Problems](problems.md) **before the game runs**, never a price of 0 found while playing.

Tables are **read-only while the game plays**: they're your game's constants, its design, not something that changes as it's played. What changes (the player's gold, what's in their bag) belongs in components and game-wide objects, which are saved. That's also why a table isn't in save games: a save keeps what happened, and the table says what things are.

> **Good to know:** an inventory, a shop or a crafting system isn't built into Solis: they're games you make with tables, [tags](tags.md) and game code. A table is the part that says what each item *is*.

## How to use it

### Make a table
1. In the [Library](projects-and-the-library.md), right-click empty space or a folder and choose **New table**. A table called `table.table.lst` is made with one column, **Name**, and one row. Rename it like any file (right-click › **Rename…**) after what it holds, in the plural (`items`, `crops`, `enemies`): game code calls the table by that name.
2. It opens in the **Table** tab, beside the level's view on Home. Later, double-click it in the Library to open it again: tables have their own icon (a little grid) and a sand-coloured mark, and the Library's **Code** filter shows them.

### Columns and rows
- **+ Row** adds a row below the one you're on, with an ID of its own (Item, Item2…). Type over it: an ID is how game code names the row, so it starts with a capital letter and holds only letters, digits and `_` (**IronSword**, not *iron sword*). Solis says why one can't be, and suggests one that can.
- **+ Column ▾** adds a column after the one you're on; choose what it holds:

| Kind | Holds | Game code gets |
|---|---|---|
| text | words, such as a name or a description | `string` |
| whole number | 5, -3, 120 | `int` |
| number | 0.25, 3.5 | `float` |
| yes or no | a tick box: click it, or press `Enter` on it | `bool` |
| choice | one of the column's own options, such as Common and Rare | a state of the table's own (`ItemRarity`) |
| colour | a colour, typed as `#ff8800` | `colour` |
| sound, clip, picture, prefab, screen, level, tag | one of the project's names, picked from a list as you type | that name (`Picture`, `Sound`…) |

- **Right-click a heading** to rename the column (its name is what game code reads, so `daysToGrow` rather than *days to grow*; the heading shows it as "Days to grow"), to type a choice's options (separated by commas; renaming one carries its cells with it), to mark a text column **Translated**, to change what it holds (cells whose text fits keep it), to move it left or right, or to delete it.
- **Right-click a row** to add a row below, move it up or down, copy, clear, or delete it (or every row picked).
- **Each row is a** names what one row is, *Item* for a table of items: game code's names come from it (`Item`, `ItemRow`, `ItemRarity`). A table read without one is named from its file (*items* gives *Item*); a new table's starts as *TableRow*, so set it once you've named the table.

Every change is one step to [undo](undo.md) with `Ctrl + Z` over the tab, and is saved at once.

### Type in cells
Click a cell to put the cursor on it, then use the keys spreadsheets use: `Enter` or `F2` starts typing (or just type: the first key replaces what's there), `Enter` keeps it and moves down, `Tab` keeps it and moves right, and `Esc` leaves the cell as it was. A double-click types too. A value that doesn't fit its column (*lots* in a price) isn't kept, and the [Console](console.md) says why.

In a choice or a name column, what fits what you've typed is listed under the cell: click one, or press `Enter` on a whole option.

### Copy, paste and spreadsheets
Pick a block of cells (`Shift + click`, a drag, or `Shift` with the arrows) and press `Ctrl + C`: it's copied as the tab-separated text spreadsheets use, so it pastes into one as cells. `Ctrl + V` pastes a spreadsheet's cells at the cursor, adding rows if they run past the end. `Delete` empties the block.

**Export CSV** writes the whole table beside it as a CSV file (`items.csv` beside `items.table.lst`), which any spreadsheet opens. **Import CSV** reads that file back: rows are matched by their ID (the first column) and columns by their name or heading, so you can edit prices in a spreadsheet and bring them back; rows and columns the table doesn't have are added. An import is one step to undo.

### Look without changing
**Search** shows only the rows with a cell holding what you type. **Click a heading** to sort the rows by it, lowest first; click again for highest first, and again for the table's own order. Both are only for looking: the table keeps its order, which is the order game code lists the rows in.

### Read a table in game code
Each table gets a [Lust](concepts.md#game-code) file of its own, which Solis writes into `solis/tables/` whenever the table changes: `data/items.table.lst` gives `solis/tables/data/items.lst`. In it are a state of the rows' IDs (`Item`), a class for one row (`ItemRow`, a field a column, read-only) and the table itself (`Items`), which gives a row by its ID or every row in order:

```lust
-- shop.lst: a shopkeeper who knows every price.
fetch "solis/tables/data/items":[Items, Item, ItemRow, ItemRarity];
fetch "solis/world":World;

component Shopkeeper {
    has {
        int gold = 200;
    }

    -- Sells one thing, if the player has the gold.
    method sell(Item which) {
        ItemRow item = Items:row(which);
        is gold >= item.price? then {
            gold -= item.price;
            World:say("Sold: {item.name}");
        }
    }

    method update(float dt) {
        -- Every rare item, in the table's order.
        loop (ItemRow item in Items:rows()) {
            is item.rarity == ItemRarity.Rare? then {
                World:say("{item.name}: {item.price} gold");
            }
        }
    }
}

pass Shopkeeper;
end
```

```rust
// Rust reads the table through its host, by the table's name and each
// row's ID and column's name as text.
use script::{Context, RustComponent};

pub struct Shopkeeper {
    gold: i64,
}

impl RustComponent for Shopkeeper {
    fn update(&mut self, cx: &mut Context<'_>, _dt: f64) {
        let tables = cx.host().tables();
        let Some(items) = tables.get("Items") else {
            return;
        };

        // One row by its ID.
        if let Some(sword) = items.row("IronSword") {
            let price = sword.whole("price").unwrap_or(0);
            if self.gold >= price {
                self.gold -= price;
            }
        }

        // Every rare item, in the table's order.
        let rare: Vec<&str> = items
            .rows()
            .filter(|row| row.text("rarity") == Some("Rare"))
            .map(|row| row.id())
            .collect();
        cx.host().say(&format!("Rare: {}", rare.join(", ")));
    }

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
-- The table's Lust file is Lust's. Lua reaches the table through
-- solis.Items, with each row's ID as text: a misspelt one stops with
-- the nearest ID.
local Shopkeeper = component("Shopkeeper", {
    { "gold", "int", 200 },
})

function Shopkeeper:update(dt)
    -- One row by its ID.
    local sword = solis.Items:row("IronSword")
    if self.gold >= sword.price then
        self.gold = self.gold - sword.price
    end

    -- Every rare item, in the table's order.
    for _, item in ipairs(solis.Items:rows()) do
        if item.rarity == "Rare" then
            solis.World:say(item.name .. ": " .. item.price .. " gold")
        end
    end
end
```

`Items:row(...)` is quick enough to call every tick: a row is made once and kept. `Items:rows()` gives a new list each time, so you may change the list (not the rows). In a very big table, ask for rows by ID where you can: listing 100,000 rows makes all of them at once.

A component can keep a row's ID in a field (`Item stock = Item.Apple;`): Properties then offers the table's IDs as a choice.

A table saved while the game runs is read from the next tick on. A [save game](save-games.md) or a [recording](save-games.md) remembers which tables it was made with: loading or playing one back after a table changed says so once in the Console, since the game may not go the same way.

> **Good to know:** **Translated** on a text column does nothing yet. It marks the text the coming translations milestone will translate, so tables made now are ready for it.

## Settings
| Setting | What it does |
|---|---|
| Each row is a | What one row is (*Item*): game code's names for the table's IDs, its row class and its choices come from it. |
| A column's name | What game code reads (`row.price`): a small letter first, then letters, digits and `_`; never `id`, which is the row's ID. |
| A column's kind | What its cells hold, and what game code gets: see the table above. |
| A choice's options | The names a choice cell can hold, separated by commas. |
| Translated | Marks a text column for translating (from the translations milestone on). |

The limits: 100,000 rows, 64 columns, 2,000,000 cells in all, 1,024 options in a choice and 10,000 characters in a cell. Past one, Solis says which and suggests splitting the table.

## Shortcuts
These work inside the Table tab's grid only, once you've clicked a cell.

| Action | Shortcut |
|---|---|
| Type in the cell; keep what's typed and move down | `Enter` |
| Type in the cell | `F2`, or any key typed |
| Keep what's typed and move right; move right | `Tab` |
| Move left | `Shift + Tab` |
| Move | the arrow keys |
| Pick a block | `Shift` + an arrow key, `Shift + click`, or a drag |
| Leave the cell as it was | `Esc` |
| Empty the cells picked | `Delete` |
| Copy, paste cells | `Ctrl + C`, `Ctrl + V` |
| Undo, redo | `Ctrl + Z`, `Ctrl + Y` |
| Sort by a column | Click its heading |

## Tips and common problems
- **Problem:** Problems says `Item.IronSwrod` isn't an Item. **Why:** the table has no row of that ID: a typo, or a row renamed since. **Fix:** correct the line, or rename the row back. Renaming a row's ID doesn't change your code: Problems points at each line still using the old one.
- **Problem:** a name cell shows *none*. **Why:** nothing's picked, or the file it named was deleted. **Fix:** type in it and pick one; game code gets an empty name from it, which matches nothing.
- **Problem:** the table isn't in game code, and the Console says it can't be reached. **Why:** its file name can't be a Lust name (it starts with a digit), or another table or one of Solis's own objects (`World`, `Input`…) has the same name. **Fix:** rename the file.
- **Problem:** Import CSV says there's no CSV. **Why:** it reads the CSV beside the table with the table's name (`items.csv`). **Fix:** export first, or save your spreadsheet under that name, as CSV in UTF-8.

## Related
- [Game code](game-code.md), [States](states.md) and [Concepts](concepts.md#game-code): states, and the project's names.
- [Tags](tags.md), for kinds of entity rather than kinds of data.
- [Save games](save-games.md).
