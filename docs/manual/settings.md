# Settings

> The editor's settings, and the open project's, in one window, a page per subject, with a search box.

## When to use it
When you want to change how the editor itself looks or behaves (its accent colour, how large it's drawn, putting the panels back as they started, or hiding the parts that aren't built yet), or a setting that belongs to the whole project, such as its 2D resolution. The list on the left has two parts:
- **Editor:** settings for the editor on this computer, not for your game. They aren't saved in the area or the project.
- **This project:** settings saved in the open project, so they travel with it to other computers and other people. They need a project open.

Neither part is an undo step. Your game's controls are in **Edit > Input actions…** instead ([Input](input.md)).

## How to use it
1. Open **Edit > Settings…**.
2. Pick a page from the list on the left: **Appearance** under *Editor*, **2D** under *This project*. More pages join as features that have settings arrive.
3. Change a setting: it takes effect at once. On the Appearance page each setting is a row, with its name and a line about it on the left and its control on the right: a row of colour swatches, a drop-down list, a row of choices side by side, a button or a switch. They work from the keyboard too: **Tab** moves from one to the next (a thin ring in the accent colour shows where you are), **Space** or **Enter** presses or flips it, and the **arrow keys** move along the swatches and the choices, or step through a drop-down list without opening it. The editor's are remembered next time you start; a project's are saved in the project straight away (once you let go, when you drag a number).
4. Can't find something? Type in **Search settings**: it looks through every page's settings, their names and their descriptions, and shows the matches together.
5. **Reset this page** puts the page's settings back as they were at first.
6. Close the window with its ×. The window keeps its size while you work in it; when a page is longer than the window, scroll it with the mouse wheel, or drag the window's corner to make it larger.

## Settings
| Setting | What it does |
|---|---|
| Appearance > Accent colour | The colour that marks what's chosen and selected: the chosen tool, the front tab, the selected row. Five colours, each with a swatch: **Blue** (the standard), **Grey** (no colour at all, for a calm screen), **Orange**, **Teal** and **Purple**. They are light colours on a dark editor, so words and icons drawn on the accent are dark. Colours that mean something keep their own whatever you pick: **Play** is the one light button on screen, Stop is red while the game runs, and warnings and errors are amber and red. |
| Appearance > Interface scale | How large everything in the editor is drawn: **100%** (the standard), **125%**, **150%**, **175%** or **200%**. Raise it when text is too small to read comfortably, which is common on a laptop or a very sharp (4K) screen: everything grows together, so nothing overlaps, and thin lines stay one crisp pixel instead of blurring. It changes at once, launcher and all, and less fits on screen the higher it goes. It doesn't change your game. |
| Appearance > Density | How tall buttons, fields and drop-down lists are: **Compact** (22 pixels at 100% scale: more on screen), **Default** (24) or **Comfortable** (28: larger things to aim at with the mouse). Unlike the interface scale, text stays the same size; only the room around it changes. |
| Appearance > Hints | The switch turns the Hints panel's hints on or off. Off, the panel says how to turn them back on; **Reset this page** turns them on again, and puts the accent colour, interface scale, density and Hide unfinished parts back to their standards. |
| Appearance > Panel layout | **Reset layout** puts every panel of the workspace you're in back where it started (in Level: the Area panel on the left, the viewport in the middle, Properties on the right, and the Library, Console and the rest along the bottom). The other workspaces keep their layouts. |
| Appearance > Hide unfinished parts | Some places in the editor are for features that come in later updates. They are shown in grey with a small hollow square, and do nothing yet. Switch this on to hide every one of them, so only what works is on screen; switch it off to see them again. Off to start. See [Unfinished parts](unfinished-parts.md). |
| Appearance > Code editor split | Not built yet: it is one of those [unfinished parts](unfinished-parts.md), shown so you can see where the setting will be. It will show game code next to the viewport. Today, game code opens in your own code editor ([Game code](game-code.md)). |
| 2D > Base resolution | The size the project's 2D games are drawn at, in pixels, before they're scaled up: 480 × 270 to start. See [2D](2d-pixel-perfect.md). |
| 2D > Pixels per unit | How many pixels make a metre: 16 to start. |
| 2D > Tile size | The 2D view's grid, in pixels: 16 × 16 to start. |
| 2D > Scaling to the screen | **Whole steps** (crisp, with bars if the screen doesn't divide evenly) or **Fill the screen**. The line under the page says how the base resolution fits 1080p, 1440p and 4K screens. |

## Tips and common problems
- **Problem:** everything in the editor is tiny. **Why:** on a very sharp screen each pixel is small, and the editor is drawn pixel for pixel at 100%. **Fix:** raise **Appearance > Interface scale**.
- **Problem:** my accent colour changed after an update. **Why:** the editor has one look now, with five accents chosen to suit it, where it had ten. The one you had was swapped for the nearest of the five: Petrol blue, Marine green and Olive became Teal; Red, Copper, Orange, Amber and Gold became Orange; Plum became Purple. **Fix:** pick another under **Appearance > Accent colour** if you'd rather.
- **Problem:** I can't find **Theme** or **Rounded panels**. **Why:** Standard and Graphite were replaced by one look, dark neutral greys throughout, and panels are no longer drawn as rounded cards. **Fix:** nothing to do: your other settings are as you left them.
- **Problem:** a setting is grey, has a small square after its name, and its switch won't move. **Why:** it's an [unfinished part](unfinished-parts.md): the design has a place for it, and it isn't built yet. **Fix:** rest the pointer on it to read what it will do; switch on **Hide unfinished parts** if you'd rather not see it.
- **Problem:** the 2D page's settings are greyed out. **Why:** they belong to a project, and none is open. **Fix:** open or make one (**File**).
- **Problem:** the Console says `pixels.solpix` had a mistake. **Why:** the file was edited by hand and can't be read. **Fix:** the standard 2D settings are used meanwhile; change anything on the 2D page and the file is written again, with the damaged one kept beside it as `pixels.solpix.bak`.
- **Problem:** the panels are in such a tangle that you can't find Settings. **Why:** panels can be dragged anywhere, including over each other. **Fix:** **Window > Reset layout** does the same as the button in Settings, and the menus at the top are always there.

## Related
- [Interface tour](interface-tour.md): the panels, and dragging them around ([docking](concepts.md#the-window)).
- [2D: the pixel-perfect view](2d-pixel-perfect.md): what the 2D settings do.
- [Unfinished parts](unfinished-parts.md): what the grey controls with a square mark are, and hiding them.
- [Concepts](concepts.md): [accent colour](concepts.md#the-window), [interface scale](concepts.md#the-window), [density](concepts.md#the-window), [layout](concepts.md#the-window), [unfinished part](concepts.md#the-window).
