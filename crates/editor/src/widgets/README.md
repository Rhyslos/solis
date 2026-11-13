# editor/src/widgets

The kit: the design's controls, each drawn exactly as the project lead's design has it (`docs/design/g-layout/specification.md`, lines 98 to 125, and the screens that show them), from the tokens and nothing else. The screens take them up one milestone at a time; egui's own controls, styled by `../theme.rs`, stay inside the panels that haven't been rebuilt yet.

Every control follows the accent, the density and the interface scale through the tokens; can be reached with Tab and used with Space or Enter (and the arrow keys, along a row of choices or a slider), with a 1 px accent ring outside it while it holds the keyboard; and has a name a screen reader can say, which is also how the tests find it.

| File | What it draws |
| --- | --- |
| `mod.rs` | What the controls share: the focus ring, a one-pixel border, words clipped to their room, and the arrow keys' step while a control holds the keyboard. |
| `button.rs` | `Button::plain`, `Button::primary` (the accent) and `Button::light` (Play's look), with an icon, at a page's or a header's height, or as wide as their room, and `named` for screen readers when the word alone doesn't say what it does ("Play the sound"); `dashed` ("+ Component"); `icon` (26 x 26, an icon alone, named for hover and screen readers); `link`. |
| `toggle.rs` | The switch: 32 x 18, its knob at the right on the accent when on. |
| `checkbox.rs` | The checkbox: a 12 x 12 rounded square, filled with the accent and ticked when on. |
| `segmented.rs` | The segmented control: choices side by side in a dark well, the chosen one lifted. |
| `sub_tab.rs` | A File page's sub-tab (Preferences' General, Appearance…): its word, brighter with the accent's line under it while in front. |
| `swatch.rs` | A row of colour swatches, the chosen one ringed in its own colour (the accent's choices). |
| `columns.rs` | Columns side by side inside one shelf tab, each under the shelf's 30 px header with its title, note and tools (the 2D editors' shelf tabs). |
| `chip.rs` | Chips, in the three sizes the design draws them (the Library's kinds, the Place tab's filters, the input page's contexts), with a kind's dot. |
| `pill.rs` | The see-through pill that floats over the viewport, and the hint of a tool's keys along a view's foot (`key_hints`). |
| `card.rs` | A card: the lifted box with a title that holds a few rows. |
| `keycap.rs` | A keycap: bound, in conflict (amber), or waiting for a key (the accent, dashed). |
| `slider.rs` | The slider: a thin track filled with the accent up to a round light knob. |
| `tile.rs` | A tile: a square thumbnail over a name (cut off with "...") and a kind with its dot; chosen, it takes the accent. |
| `rows.rs` | `setting_row` (a name and a line about it on the left, the control in a 260 px column at the right, a hairline under), `key_value`, `section_caps` and `column_header`. |
| `field.rs` | The field (24 tall, its border the accent while the keyboard is in it or it is dragged): `text_field`, `number_field` (with an axis letter), `framed` (the box round any of egui's editing, as Properties' fields use it) and `dropdown`, which is meant to be the one drop-down everywhere; `dropdown_showing` is the same with words of its own in the box (a dash for values that differ, a name no longer there) and a quiet `list_note` ending the list (Properties' pickers use it), `dropdown_of_names` the same over a list of names (files, buses, clips), and `dropdown_button` with `list_row` build a list `dropdown` can't (the entity field's, with a box to find one by name). A long list scrolls. The editing inside is egui's own. |
| `menu.rs` | The rows of the design's menus: `item` (with quiet words after it, a key), `choice` and `check` (a check mark while in force), `caps` (small capitals over a group) and `line`. They go inside egui's own menus. |
| `dashed.rs` | A dashed outline round a rounded rectangle, which egui can't draw itself. |
| `dialog.rs` | The shared dialog card (round 1, `10-dialog-*`): a title, what is asked, and the answers in a row, those that set the work aside at the left, the main one in the accent at the far right. Unsaved changes, Changed on disk and Keep or Undo all are this card. |
| `sheet.rs` | Test only: the widget sheet, one page with every control in each of its states. |
| `tests.rs` | Each control measured against the tokens, found by its name, pressed, and driven from the keyboard. |
| `picture_tests.rs` | The sheet drawn for real: its colours held against the design's own pictures (the switch from screen 8, the keycap from 13, a card from 1); the focus ring; borders at 150%; and the sheet as an approved picture. |

**To add a control:** take every colour and size from `../tokens.rs` and `../tokens/` (add the token there if the design has a value they lack), give it `Sense::click()` and a `WidgetInfo` so it can be focused and found, draw `focus_ring`, put it on the sheet in each of its states, and test it there.
