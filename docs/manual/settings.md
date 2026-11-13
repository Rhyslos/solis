# Settings

> Preferences, the editor's own settings, and Project settings, the open project's: two File pages, each with sub-tabs and one search.

## When to use it
When you want to change how the editor itself looks or behaves (its accent colour, how large it's drawn, what the viewport shows, where Home's lists sit, putting the shelf back as it started, or hiding the parts that aren't built yet), or a setting that belongs to the whole project, such as its 2D resolution, its game's controls or how it is exported.

- **Preferences** are yours, kept on this computer. They aren't saved in the area or the project.
- **Project settings** are saved in the open project's folder, in small text files Git can merge, so they travel with it to other computers and other people. They need a project open.

Neither is an undo step. Every change applies at once: there is no Save or Apply button.

## How to use it
1. Click the **File** tab, then **Preferences** or **Project settings** in its column. Or press **Space** and type *Preferences*.
2. Pick a sub-tab under the heading: Preferences has **General**, **Appearance**, **Viewport** and **Shortcuts**; Project settings has **General**, **2D**, **Input** and **Build**. The faint sub-tabs with a small square (Source control, Physics, Platforms and others) are [unfinished parts](unfinished-parts.md).
3. Each setting is a row, with its name and a line about it on the left and its control on the right: a switch, a row of choices side by side, a drop-down list, colour swatches, a number or a button. They work from the keyboard too: **Tab** moves from one to the next (a thin ring in the accent colour shows where you are), **Space** or **Enter** presses or flips it, and the **arrow keys** move along the swatches and the choices, or step through a drop-down list without opening it.
4. Can't find something? Type in the search at the top right: it looks through every sub-tab of the page, at the settings' names and their lines, and shows the matches together.
5. On Preferences, **Reset to defaults** (in the column at the right) puts the sub-tab's settings back as they were at first. On Project settings, a setting changed from its standard has a dot before its name and **Reset** after it; the heading counts them ("2 changed from defaults"), and a card at the right lists each, from what to what.
6. **Back to editor** (or **Esc**) leaves the page.

## Preferences
| Setting | What it does |
|---|---|
| General › Look for updates when Solis starts | Asks GitHub for a newer build at each start, in the background. Off, nothing is asked until you choose **Check now** on **File › Updates**. |
| General › Show hints | The Hints card, last under Properties, says what's selected and what you can do with it. Its **Turn hints off** does the same as this switch. |
| General › Hide unfinished parts | Some places in the editor are for features that come in later updates. They are shown faint with a small hollow square, and do nothing yet. Switch this on to hide every one of them, so only what works is on screen. The card beside it shows what one looks like. See [Unfinished parts](unfinished-parts.md). |
| Appearance › Accent colour | The colour that marks what's chosen and selected: the chosen tool, the front tab, the selected row. Five colours: **Blue** (the standard), **Grey**, **Orange**, **Teal** and **Purple**. Colours that mean something keep their own: **Play** is the one light button on screen, and warnings and errors are amber and red. The small picture at the right shows the editor in the colour chosen. |
| Appearance › Interface scale | How large everything is drawn: **100%** to **200%**. Raise it when text is too small, common on a laptop or a very sharp (4K) screen. Everything grows together, and thin lines stay crisp. It doesn't change your game. |
| Appearance › Density | How tall buttons, fields and drop-down lists are: **Compact** (22 pixels at 100%), **Default** (24) or **Comfortable** (28). Text stays the same size. |
| Appearance › Shelf opens on | Which [shelf](concepts.md#the-window) tab comes to the front when you select something: **The tab in front** (the standard: the shelf stays as it is) or **Entity** (Properties comes to the front every time you pick something). |
| Appearance › Panel layout | Where Home's three lists sit ([panel layout](concepts.md#the-window)): **Area left, Assets bottom** (the standard: the Area list and Properties as tall columns at the sides, Assets a strip under the area), **Area and Assets on the left**, **Area left, Assets right**, or **The shelf (as designed)** (all three on the shelf's Entity tab). It changes at once, so try each; see [Interface tour](interface-tour.md). With a layout other than the shelf, **Shelf opens on › Entity** does nothing: Properties is always in view. |
| Appearance › Sizes | **Reset layout** puts the tab you came from back as it started: the shelf on its first tab, unfolded, and every border you dragged (the shelf's height, the split between groups, the columns and the Assets strip) at its standard size, in every panel layout. To put back one border only, double-click it. |
| Appearance › Code editor split | An [unfinished part](unfinished-parts.md): it will show game code next to the viewport. Today, game code opens in your own code editor ([Game code](game-code.md)). |
| Viewport › Frame time and frame rate | The pill at the top right of the viewport. |
| Viewport › Axis locks, axes and camera speed | The pills at the bottom right: Lock X Y Z, World or Object, and the camera's speed. |
| Viewport › Toolbox | The column of tools at the viewport's left. Q, W, R, S and T work either way. |
| Viewport › Preview animation | Sprites play their clips while you edit. Off (the standard), each shows its clip's frame and the editor rests. The game always animates. |
| Viewport › Grid step, Angle step | The steps moves and turns land on while snapping is on (the Home ribbon's Snapping turns it on and off). |
| Viewport › In the 2D view, moves land on | **Pixels** (whole pixels, so pixel art stays crisp), **Tiles** or **Off**. |
| Viewport › Camera speed | How fast the camera flies, 1 to 8. Also changed by holding the right mouse button and turning the wheel. |
| Viewport › Views, Look, Bands of light, Outlines | The same as the viewport's own pills: how many views, and how models are shaded while you edit ([Looks](looks.md)). |
| Shortcuts | Every key the editor has, as a table you can filter (**All**, **Keys**, **Mouse**) and search. It is to read: changing a key is an unfinished part. The full list is [Keyboard shortcuts](keyboard-shortcuts.md). |

## Project settings
| Setting | What it does |
|---|---|
| General › Project name | Its name in the window's title and on Recent projects. Type a new one and click elsewhere; the folder keeps its own name. |
| General › Game starts in | The area the exported game opens in (the same setting as on Build). |
| General › Engine version | The build of Solis that last opened the project, with **Check now**. |
| General › Scripting | Game code is written in Lust and opens in your code editor. |
| 2D › Base resolution | The size the project's 2D games are drawn at, in pixels, before they're scaled up: 480 × 270 to start. Pick a common size from the list, or type your own. See [2D](2d-pixel-perfect.md). |
| 2D › Pixels per unit | How many pixels make a metre: 16 to start. |
| 2D › Tile size | The 2D view's grid, and new tile maps', in pixels: 16 × 16 to start. |
| 2D › Scaling to the screen | **Whole steps** (crisp, with bars if the screen doesn't divide evenly) or **Fill the screen**. The card beside it says how the base resolution fits 1920 × 1080, 2560 × 1440 and 3840 × 2160 screens. |
| Input | The game's controls: see [Input](input.md). |
| Build | Export for playtesting: see [Export for playtesting](playtest-export.md). |

A number you drag is saved once, when you let go.

## Tips and common problems
- **Problem:** everything in the editor is tiny. **Why:** on a very sharp screen each pixel is small. **Fix:** raise **Appearance › Interface scale**.
- **Problem:** a setting is faint, has a small square after its name, and its switch won't move. **Why:** it's an [unfinished part](unfinished-parts.md). **Fix:** rest the pointer on it to read what it will do; switch on **General › Hide unfinished parts** if you'd rather not see it.
- **Problem:** Project settings says **Open a project first**. **Why:** its settings belong to a project, and none is open. **Fix:** open or make one (**File › Recent projects**).
- **Problem:** the Console says `pixels.solpix` had a mistake. **Why:** the file was edited by hand and can't be read. **Fix:** the standard 2D settings are used meanwhile; change anything on the 2D tab and the file is written again, with the damaged one kept beside it as `pixels.solpix.bak`.
- **Problem:** the shelf is so tall (or folded) that what you want isn't in sight. **Why:** its height and fold are remembered per tab. **Fix:** **Ctrl + Space** unfolds it; double-click its grip, or **Reset layout**, puts it back.
- **Problem:** after an update, Home's Area list and Properties are columns beside the area, not on the shelf. **Why:** that is the new standard panel layout. **Fix:** choose **The shelf (as designed)** under **Appearance › Panel layout** to have them back on the shelf.
- **Problem:** I can't find the Settings window, or File › Input. **Why:** they became these pages. **Fix:** Preferences and Project settings are in the File column; Input is a sub-tab of Project settings.

## Related
- [Projects](launcher.md): Recent projects, Learn, Updates and About, the File column's other pages.
- [Input](input.md): the game's controls, on Project settings › Input.
- [Export for playtesting](playtest-export.md): on Project settings › Build.
- [2D: the pixel-perfect view](2d-pixel-perfect.md): what the 2D settings do.
- [Unfinished parts](unfinished-parts.md): what the faint controls with a square mark are, and hiding them.
- [Concepts](concepts.md): [accent colour](concepts.md#the-window), [interface scale](concepts.md#the-window), [density](concepts.md#the-window), [layout](concepts.md#the-window), [unfinished part](concepts.md#the-window).
