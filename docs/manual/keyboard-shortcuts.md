# Keyboard shortcuts

> Every key and mouse action in Solis, in one place.

Plain clicks on buttons and menus aren't listed. This page is for the keys, and for what the mouse does beyond clicking a button: drags, double-clicks, right-clicks and the wheel. Each part of the editor has its own table, and the page for that part says more.

## In the viewport

| Action | Shortcut |
|---|---|
| Look around | Hold right mouse, move the mouse |
| Fly forward, left, back, right | Hold right mouse + `W`, `A`, `S`, `D` |
| Fly down, up | Hold right mouse + `Q`, `E` |
| Faster for a moment | Hold `Shift` while flying |
| Change camera speed | Hold right mouse + mouse wheel |
| Move forward or back in steps | Mouse wheel |
| Slide the view | Hold middle mouse, drag |
| Slide a flat view (Top, Front, Right, 2D) | Hold right or middle mouse, drag |
| Zoom a flat view (the 2D view: a whole step a notch) | Mouse wheel |
| Use one of several views | Click in it |

## While the game plays

| Action | Shortcut |
|---|---|
| Give the game the mouse and keys | Click the Game panel |
| Give them back to the editor | `Esc` |
| Switch the game's fullscreen setting | `Alt + Enter` |

While the game has them, every key is the game's and the editor's shortcuts are off. In an [exported game](playtest-export.md), the game has them from the start, Esc included, and Alt + Enter really switches the window.

**The game's own keys.** A game's controls are named [input actions](input.md), and these are the keys the ready-made ones start with. They're only a start: change them, and add your own, in **File › Project settings › Input**.

| Action | Keyboard and mouse | Gamepad |
|---|---|---|
| Move | `W` `A` `S` `D`, or the arrow keys | Left stick |
| Look | Move the mouse | Right stick |
| Jump | `Space` | A |
| Sprint | `Shift` | Left stick click |
| Interact | `F` | X |
| Pan the [RTS camera](characters-and-cameras.md) | Arrow keys | Left stick |
| Turn the RTS camera | `Alt` + left and right arrows | Right stick |
| Zoom the RTS camera | Mouse wheel, `Page Up` and `Page Down` | Right and left triggers |
| Drag the map under the RTS camera | Hold the middle mouse button | |
| Move between a [screen's buttons](screens.md) (MenuMove) | Arrow keys | D-pad, left stick |
| Choose the button with the ring round it (MenuChoose) | `Enter`, `Space` | A |
| Back (MenuBack) | `Backspace` | B |
| Pause | `P` | Start |

The [playground](characters-and-cameras.md#how-to-use-it) adds two of its own: it saves on F6 and loads on F7.

In **File › Project settings › Input**, after clicking **+ Add binding**, the next key, mouse button, wheel notch or gamepad button you press is the one bound; Esc gives up instead. That key is only bound: it doesn't also do what it usually does in the editor, so pressing Delete to bind it deletes nothing.

## Tools (when the right mouse button isn't held)

| Tool | Key |
|---|---|
| Select | `Q` |
| Move | `W` |
| Rotate | `R` |
| Scale | `S` |
| Transform (move, rotate and scale) | `T` |

These are the area's keys: they wait while the pointer is over a tab with keys of its own (see [Editing](#editing)).

## Selecting and moving (in the viewport)

| Action | Shortcut |
|---|---|
| Select | Click |
| Add to or take from the selection | `Ctrl + click` |
| Select with a box | Drag on empty space (`Ctrl + drag` adds) |
| Select nothing | `Esc` |
| Select everything | `Ctrl + A` |
| Snapping on or off for one drag | Hold `Ctrl` while dragging a gizmo |
| Ten times finer | Hold `Shift` while dragging a gizmo |
| Cancel a box or a gizmo drag | `Esc`, or right click |
| Lock or free an axis | `X`, `Y`, `Z` |
| World or object axes | `L` |

See [Selecting and moving things](selecting-and-moving.md).

## The Frames tab (a clips file open)

The keys work while the pointer is over the Frames tab (undo and redo over the Clip tab too); see [Animating sprites](sprite-animation.md).

| Action | Shortcut |
|---|---|
| Rename a clip (clips made here) | Double-click it in the Frames tab's list of clips (`Enter` keeps the name, `Esc` leaves it) |
| Pick a frame; several | Click; `Ctrl + click`, or `Shift + click` for a run |
| Move picked frames (clips made here) | Drag them |
| Take picked frames out (clips made here) | `Delete` |
| Play or stop the clip | `Space` |
| Undo or redo a change to the clips file | `Ctrl + Z`, `Ctrl + Y` |

## The Animator tab (an animator open)

These work while the pointer is over the Animator tab; see [Animators](animators.md). Every graph in Solis will work the same way.

| Action | Shortcut |
|---|---|
| Pick a state or an arrow; add to what's picked | Click; `Ctrl + click` or `Shift + click` |
| Pick states with a box | Drag on empty space |
| Move picked states | Drag one of them |
| Join two states | Drag from a state's edge to another |
| Menus (Add state, Make transition, Start here, Delete) | Right-click empty space, a state or an arrow |
| Give up an arrow started with Make transition | `Esc` |
| Delete a parameter, or start it at the value shown | Right-click its name |
| Take away what's picked | `Delete` |
| Pan, zoom, fit everything | Middle-drag, mouse wheel, `F` |
| Undo or redo a change to the animator | `Ctrl + Z`, `Ctrl + Y` |

## The Tileset tab (a tileset open)

These work while the pointer is over the Tileset tab; see [Tilesets and autotiling](tilesets.md).

| Action | Shortcut |
|---|---|
| Give a tile to a slot | Click the tile in the picture, then the slot; or drag the tile onto the slot |
| Undo or redo a change to the tileset | `Ctrl + Z`, `Ctrl + Y` (over Properties while it shows the tileset, too) |

## The Atlas Crop tab (a picture's crops open)

These work while the pointer is over the Atlas Crop tab; see [Atlas Crop](atlas-crop.md).

| Action | Shortcut |
|---|---|
| Draw a crop | Drag with the left button on the picture |
| Pick a crop | Click it |
| Delete the picked crop | `Delete` |
| Zoom, move the picture | Mouse wheel, middle-button drag |
| Undo or redo a change to the crops | `Ctrl + Z`, `Ctrl + Y` |

## The UI editor (a screen open on the canvas)

These work while the pointer is over the Canvas (or, where it says so, Elements); see [The UI editor](ui-editor.md).

| Action | Shortcut |
|---|---|
| Pick a tool: Select, Box, Ellipse, Line, Pen, Text, Image | `1` to `7` (pointer over the canvas) |
| All the tools in a ring | `E` |
| Back to Select (with the Pen: forget the corners clicked so far) | `Esc` |
| Finish the Pen's line | `Enter` or double-click |
| Pick a piece | Click it (on the canvas or in Elements) |
| Pick more | `Shift + click` (Elements: `Shift` or `Ctrl + click`) |
| Pick everything in a box | Drag on an empty part of the canvas (`Shift + drag` adds) |
| Move a piece | Drag it (between its neighbours, or into a column or row); in Elements, drag its row |
| Size a piece | Drag the square on its right edge, bottom edge or corner |
| Delete what's picked | `Delete` (over the canvas or Elements), or `Backspace` (over the canvas) |
| Copy what's picked, just after itself | `Ctrl + D` (over the canvas or Elements) |
| Pick nothing | `Esc`, or click an empty part of the canvas |
| Undo, redo | `Ctrl + Z`, `Ctrl + Y` (over the canvas or Elements) |
| Zoom, move the view | Mouse wheel, middle mouse drag |
| Fit the canvas in view | Double-click an empty part of it |

## The Map tab (a tile map selected)

These work while the pointer is over the Map tab; see [Tile maps](tile-maps.md).

| Action | Shortcut |
|---|---|
| A terrain's settings (whether it's solid) | Right-click its swatch |
| Undo or redo a change to the map | `Ctrl + Z`, `Ctrl + Y` |

## Painting tiles (a tile map selected)

These work in the viewport while a tile map's entity is selected; the same tools are on the ribbon of **Home tools · Tile map**, and the keys of the tool in use show along the foot of the view. See [Painting tiles](painting-tiles.md).

| Action | Shortcut |
|---|---|
| Brush, Rectangle, Line, Fill, Eyedropper | `B`, `U`, `N`, `G`, `I` |
| Back to a transform tool | `Q`, `W`, `R`, `S`, `T` |
| Paint | Left-drag (a click for the fill) |
| Empty cells | Right-drag |
| Paint with the terrain under the pointer | `Alt + click` |
| Place exact tiles for one stroke | Hold `Ctrl` while painting |
| Flip a copy left and right, top and bottom; turn it a quarter | `X`, `Y`, `Z` (holding a copy) |
| Give up a stroke; put the copy away; leave the painting tool | `Esc` |
| Show the map's art or its meaning | `Tab` |
| Undo or redo a stroke | `Ctrl + Z`, `Ctrl + Y` |
| Slide the view | Middle-drag |

## Editing

| Action | Shortcut |
|---|---|
| Undo | `Ctrl + Z` |
| Redo | `Ctrl + Y`, or `Ctrl + Shift + Z` |
| New area | `Ctrl + N` |
| Open area | `Ctrl + O` |
| Save area | `Ctrl + S` |
| Save area as | `Ctrl + Shift + S` |
| Delete the selection | `Delete` |
| Duplicate the selection | `Ctrl + D` |
| Rename | `F2` |
| Point the camera at the selection | `F` |
| Search for anything to do, any file, entity or panel (with nothing being typed) | `Space` |
| Fold the shelf to its bar, or open it again | `Ctrl + Space` |

All but Ctrl + Space don't work while you're typing in a field: there, Ctrl + Z and Ctrl + Y undo the typing instead. See [Undo and redo](undo.md).

**Where the pointer rests decides what a key acts on**, so one press never does two things. With the pointer over a tab that edits a file of its own (Clips and Frames, Animator, Tileset, Atlas Crop, the UI editor's Canvas and Elements), the keys are that tab's alone: undo and redo, Delete, Ctrl + D and Esc act on what the tab shows, as its table above says, and never on the area as well. The area's other keys (the tools' and the tile painter's keys, F, F2, Ctrl + A, and X, Y, Z and L) wait until the pointer is back over the viewport or another panel. Over the Map tab, undo and redo are the map's, and every other key is still the area's; the same goes for Properties while it shows a tileset's terrains, where undo and redo are that tileset's. On the UI tab, Properties shows the screen's settings, so over it the keys are the screen's, as over Elements: Delete removes the picked piece, never an entity. On a tab that doesn't show the area (Library, Audio, Debug, Build), the area's keys, undo and redo among them, rest too, since what they would change isn't on screen: a Delete over the Mixer never deletes an entity you can't see. They still work over the Area list and Properties wherever those are, and over the Library, F2 and Delete rename and delete the file picked there. New, open and save, the search and Ctrl + Space work wherever the pointer is.

## The search list

Space, or a click on **Search or run a command** in the tab bar, opens it (the box shows the key at its right end); see the [Interface tour](interface-tour.md). These keys are the ones Solis has always had: the design the editor's look follows shows other keys in its pictures, and Solis keeps its own ([decision 0103](../decisions/0103-todays-keys-stay.md)).

| Action | Shortcut |
|---|---|
| Open the search (with nothing being typed) | `Space` |
| Narrow the list | Type part of a name |
| Choose a result | `Up`, `Down` |
| Do what's chosen | `Enter`, or click a result |
| Close the search | `Esc`, or click outside it |

Over the Frames tab, Space plays and stops the clip instead, so point somewhere else first.

The list shows each thing's shortcut beside it, so it is also the quickest place to look a key up. It finds the things without a button on the ribbon too, such as New area, Open area… and Save area as… (on the area's tab: its **+** and a right-click), Reset layout (File › Preferences), the examples (File › Learn), and showing or hiding the viewport's corner controls (the **Show ▾** pill).

## The tab bar and the ribbon

See the [Interface tour](interface-tour.md).

| Action | Mouse or key |
|---|---|
| Bring a tab to the front | Click it |
| Collapse the ribbon to its tabs, or bring it back | Double-click a tab, or click the chevron at the tab bar's right end |
| With the ribbon collapsed: drop it over the editor for one choice | Click a tab; it goes when you choose something or move the pointer below it |
| Scroll a ribbon that doesn't fit the window | Turn the mouse wheel over the ribbon |
| Make the usual kind with a Create button | Click it (**Light** makes a point light) |
| List the other kinds it makes | Click the small arrow on the button, or right-click the button |
| Leave the File page for the tab you came from | `Esc`, or **Back to editor** |

Why no long press on a Create button? Holding the mouse down is slow, and nothing on the button says it can be done; a small arrow can be seen.

## The Library

See [Projects and the Library](projects-and-the-library.md).

| Action | Shortcut |
|---|---|
| Place a model, assembly, sprite sheet or tile map where the camera looks | Double-click it in the Assets list; in the Library tab a double-click only picks it (**Place** on the ribbon places it) |
| Open any other file (an area, a picture in its program) | Double-click it |
| Drop without snapping to the grid, this once | Hold `Ctrl` as you let go |
| Rename or delete the file picked | `F2`, `Delete`, with the pointer over the Library or the Assets list |
| Place a model, assembly, sprite, picture or tile map where you point | Drag it into the viewport |
| Place one under an entity | Drag it onto the entity in the Area panel |
| Move a file to another folder | Drag it onto the folder in the tree |
| Add a sound to a [sound event](sound-events.md) as a take | Drag the sound onto the event |
| Menus (New, Place or Open, Show in Explorer, Rename…, Delete) | Right-click a file, a folder or empty space |
| In Rename…: keep the new name, or leave it as it was | `Enter`, `Esc` |

## In the Area panel

| Action | Shortcut |
|---|---|
| Select | Click |
| Add to or take from the selection | `Ctrl + click` |
| Select a range | `Shift + click` |
| Rename | Double-click (`Enter` keeps the name, `Esc` leaves it) |
| Put under another entity | Drag onto it |
| Make it stand alone | Drag onto empty space |

## Properties

| Action | Shortcut |
|---|---|
| Add the first component found (after typing part of its name in **+ Component**) | `Enter` |

## Audio: the mixer

| Action | Shortcut |
|---|---|
| Move a fader | Drag it up or down |
| Move a fader finely | `Shift` while dragging |
| Put a fader back to 0 dB | Double-click it |
| Drop a meter's peak line to the level now | Click the meter |

See [The Mixer](mixer.md).

## Debug: the Profiler

| Action | Shortcut |
|---|---|
| Pick a frame for the timeline | Click its bar in the frame-time graph |

See [The Profiler](profiler.md).

## The Console's command line

| Action | Shortcut |
|---|---|
| Jump to the command line | ' (the key next to Æ on a Norwegian keyboard) |
| Run the command | `Enter` |
| Earlier and later commands | `Up`, `Down` |
| Finish a name (the choices show at the line's right) | `Tab` |
| Leave the command line | `Esc` |

See [The Console](console.md).

## Recent projects

File › Recent projects is what Solis starts on; see [Projects](launcher.md).

| Action | Shortcut |
|---|---|
| Open the project picked | `Enter`, or double-click its card |
| Walk the cards | `Left`, `Right`, `Up`, `Down` |
| A card's menu (Open, Pin, Show in Explorer, Rename…, Make a copy…, Remove from list) | Right-click it |
| Back to the editor, or carry on without a project | `Esc` |
| In Rename…: keep the new name, or leave it as it was | `Enter`, `Esc` |

## Project settings › Input

See [Input](input.md).

| Action | Shortcut |
|---|---|
| Pick an action; pick one of its bindings | Click its row; click the binding in its card |
| Clear the picked binding | `Backspace` (its × does the same) |
| Bind the next key, mouse button or gamepad button pressed | **+ Add binding**, then press it (`Backspace` too is bound then) |
| Give up binding | `Esc` |

## Editor groups and the shelf

Panels stay where each tab's screen puts them (on Home, where the [panel layout](concepts.md#the-window) chosen in **File › Preferences** puts its three lists); these size the screen, and each tab remembers what you did, for each panel layout.

| Action | Mouse or key |
|---|---|
| Bring a document to the front of its group | Click its tab |
| Close a file's document (an animator, a tileset, a picture's crops, a screen) | Its **×** |
| New area, Open area… | The **+** after the area's tab |
| Save, Save as…, Close others, Show in the Library | Right-click the area's tab |
| Give one of two groups more room | Drag the line between them |
| Make a column, the Assets strip or one of two stacked lists bigger or smaller | Drag the line at its edge (the pointer becomes a two-headed arrow over it) |
| Put a border back to its standard size | Double-click it (the shelf's grip and top edge too) |
| Give a document a group of its own, on one half of the middle | Drag its tab onto that half |
| Join two groups | Drag a tab onto the other group's tabs |
| Put a dragged tab back | `Esc`, while dragging |
| Bring a shelf tab to the front (opening a folded shelf) | Click it |
| Make the shelf taller or shorter | Drag the grip at its bar's left, or the bar's top edge |
| Fold the shelf to its bar, or open it again | `Ctrl + Space` (while typing too) |
| Close a clips file | The **×** after the Frames or Clip shelf tab |

See the [Interface tour](interface-tour.md).

## Related
- [Moving around the viewport](navigating-the-viewport.md)
- [Input](input.md): the game's own keys, and changing them
