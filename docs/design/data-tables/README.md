# docs/design/data-tables

The mock-up of data tables (the game stages, stage 1; [the plan](../../plans/game-stages.md#the-next-milestone-data-tables), [decision 0128](../../decisions/0128-data-tables.md)). The project lead approved the quick sketch (`00-quick-sketch.png`) on their phone on 2026-11-18, before the overnight run, and asked for the rest to be drawn and built, reviewing it after. The two drawings are HTML laid over a real screenshot of the editor (1440 × 900, the sample harbour level on Home), in the G layout's colours, fonts and sizes.

| File | Shows |
| --- | --- |
| `00-quick-sketch.png` | The approved quick sketch: a new editor tab, `Items.table.lst`, with + Row, + Column ▾ and Search; columns ID, Name (text · translated), Price (whole number), Icon (sprite), Rarity and Description; the row picked outlined; the note saying how game code reads it |
| `01-the-table-tab.png` | The Table tab beside the level's view on Home: the bar (+ Row, + Column ▾, Search, "Each row is a Item", the row count, Import CSV, Export CSV), the headings with what each column holds under the name's weight, the cursor's row and cell outlined in the accent, and the note with this table's own names; the keys and the sort in callouts |
| `02-typing-and-a-new-column.png` | A choice cell being typed in with what fits listed under it, and + Column ▾ open with the kinds a column can hold |

**Differences from the quick sketch:** game code reads `Items:row(Item.IronSword).price` rather than `Items.IronSword.price` (Lust can't give an object a field per row without writing the data into code, so the row comes from a method, as the game API guidelines' rule 4 has it); the first column is the rows' **ID**, which the sketch has; the bar gains **Each row is a** (what game code calls a row's state, `Item`), the row count, and **Import CSV** and **Export CSV**; a picture column shows the picture's icon and name rather than a drawing of it (thumbnails in cells are left for later).
