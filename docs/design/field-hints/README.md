# docs/design/field-hints

The mock-up of field hints and more kinds of settings (the game stages, stage 1; [the plan](../../plans/game-stages.md), [decision 0127](../../decisions/0127-field-hints-and-pickers.md)). The project lead approved the quick sketch (`00-quick-mock-up.png`) on their phone on 2026-11-18, before the overnight run, and asked for the rest to be drawn and built, reviewing it after. The three drawings are HTML laid over a real screenshot of the editor (1440 × 900, the turret sample, Properties on the right), in the G layout's colours, fonts and sizes.

| File | Shows |
| --- | --- |
| `00-quick-mock-up.png` | The approved quick sketch: a Guard's settings with a slider, a unit, a tip, pickers for a sound (with ▶), a sprite and a prefab, a list, a state and a colour |
| `01-hints.png` | A Guard component written in Lust, as Properties shows it: Speed as a slider with its number, unit and tip ("Walking speed in metres a second · 0 to 10") under it; Alert sound, Portrait and Drops as pickers with the sound's ▶ and the picture's and prefab's thumbnails; Mood as a state; a **Seeing** group heading; Colour as a swatch with its hex; Says over several lines. The note says what's written in the code, and that lists wait for Lust |
| `02-picker-open.png` | The sound picker open: a search box, the project's sounds under their folders, ▶ on each row, how many are shown |
| `03-drag-from-library.png` | A picture dragged from the Library onto the Portrait field, which lights up with what dropping sets |

**Differences from the quick sketch:** hints are written in a comment after each field (`-- [range 0 10]`), not in a side file, so moving to real field attributes once Lust has them is taking away the `--`; and the list (Patrol points) isn't drawn, because Lust can't keep an array in a component yet.
