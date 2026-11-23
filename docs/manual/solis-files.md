# Solis's files

> What's inside the files Solis reads and writes for a 2D game, with a short example of each, for anyone who writes them by hand or with a script.

## When to use it
Most people never open these files: the editor writes them. Read this page when you want to make or change them some other way:

- **a level generator** that writes rooms or dungeons as level files;
- **an exporter** from another tool (a map editor, a spreadsheet of enemies) into Solis's tile maps, modules or levels;
- **a script that fixes many files at once**, such as giving every enemy in every level a tag;
- **reading a level** in your own tool, to count things or check them.

Every file here is plain text, safe to edit by hand, and made to merge well in Git. Solis checks each one as it reads it and says what's wrong, with the line, so a mistake never breaks a project quietly.

> **Good to know:** paths on this page are as the Library shows them: from the project's own folder, such as `props/crate.module.lst`. A project made by an older Solis keeps its files in an `assets` folder; there, read paths from inside it.

## How to use it
1. **Make the IDs you need** with `solis --new-id` (one) or `solis --new-id 20` (twenty, one a line). An entity, a module's part, a terrain and a sidecar each need one. See [IDs](#ids).
2. **Write a sidecar beside each file** other files point at (a picture, a sprite sheet, a tileset, a tile map, a module), holding its ID. See [Sidecars](#sidecars).
3. **Write the files** in the newest layout, as on this page. Files point at each other by ID, never by path.
4. **Check the project:** `solis --check-project <folder>` reads every level, module, tile map, tileset and sidecar, and says which can't be read and why ([Checking a project](checking-a-project.md)).
5. **Try it:** open the project in the editor, or play a level from the command line with `solis --play <folder> <level>` ([Playing a game from the command line](playing-from-the-command-line.md)).

> **Good to know:** you don't have to write the newest layout to keep old files working. Solis reads every older version of its files, upgrading them as it reads, and writes the newest layout when it next saves one (keeping a copy of the old file first). Older kinds of file, such as `.soltilemap` and `.soltileset`, are moved to their Lust files when the project opens. See [Backups and upgrades](backups-and-upgrades.md). A script should still write the newest layout, so nothing needs moving.

## IDs
Everything that's pointed at has an ID: 13 letters and digits, such as `A9S346Q3D25VT`. IDs are made at random, so two people (or two scripts) never make the same one, and they never change, so renaming or moving a file breaks nothing.

```text
solis --new-id 3
096Q14DR9GPQY
BRE95B9EE0ZBG
909TQM83XSSSS
```

Make them with `solis --new-id` rather than by hand: an ID uses only `0`–`9` and the letters without `I`, `L`, `O` and `U`, and its first character is `0`–`9` or `A`–`F`. One that breaks these rules is refused where it's read.

Each kind of thing keeps its ID in its own place:

| Thing | Its ID is |
|---|---|
| A file (a picture, a sprite sheet, a tileset, a tile map, a module) | in its sidecar, `<file>.solmeta` |
| An entity in a level | its block's name: `entity 096Q14DR9GPQY { … }` |
| A part of a module, a terrain, a tile, a layer, a cell value | its `id = "…"` in the file it's in |
| A tag | its `id` in the project's `tags.lst` |

## Sidecars
A sidecar is a small text file beside a file, named after it with `.solmeta` added: `knight.aseprite.solmeta` beside `knight.aseprite`. It holds the file's ID. Commit it with the file, and move or rename them together.

```text
# Kept by Solis beside its asset: commit it with the asset, and move or
# rename them together (or let Solis do it).
format = 1
id = "A9S346Q3D25VT"
hash = "e05c70ace0567aca"
part = []

[import]
```

- `format` is the sidecar's version: 1.
- `id` is the file's ID. Levels, tile maps and modules name the file by it.
- `hash` is the file's content hash, 16 hexadecimal digits. Solis uses it for one thing: when a file is renamed outside Solis and its sidecar is left behind, the sidecar finds its file again by this hash. A hash that doesn't match does no harm while the two stay together.
- `part` and `[import]` are for 3D models (the IDs of a model's parts). Leave them empty, or leave them out.

A file without a sidecar gets one from Solis once it has been still for two seconds, with a new ID. A script that needs to know the ID (to point a level at a sprite sheet, say) writes the sidecar itself.

**Working out the hash.** It's a 64-bit hash of the file's bytes, worked out like this, with every sum and product wrapping at 64 bits:

```text
state = 0x5EED50115A55E7DB
for each 8 bytes of the file, the last few padded with zero bytes:
    word  = the 8 bytes as a little-endian number
    state = rotate_left((state XOR word) * 0x9E3779B97F4A7C15, 29)
then once more, with the file's length in bytes as the word
z    = state
z    = (z XOR (z >> 30)) * 0xBF58476D1CE4E5B9
z    = (z XOR (z >> 27)) * 0x94D049BB133111EB
hash = z XOR (z >> 31), written as 16 lower-case hexadecimal digits
```

It isn't a security hash: it only tells one file's content from another's.

## Aseprite files
Solis reads Aseprite's own `.aseprite` and `.ase` files, so there's nothing to export ([Sprites from Aseprite](sprites.md)). This is what it takes from one, shown as notes on a knight's file:

```text
knight.aseprite: 32 × 32 pixels, 8 frames, RGBA

layers  Body, Shield        visible layers drawn together, in order, as each frame
        Sketch (hidden)     not drawn: hidden, and reference layers aren't either
tags    idle   1–4  forward    Repeat ∞   ->  the clip idle, Plays over and over
        swing  5–8  forward    Repeat 1   ->  the clip swing, Plays once
slices  feet   pivot (16, 30)             ->  where the sprite stands
cel     frame 6, Body, user data "whoosh" ->  an event on frame 6 of swing
```

- **Frames:** each frame's visible layers are drawn over each other with normal blending, and each frame keeps its own duration. A layer in a hidden group isn't drawn. Layer and cel opacity count. Other blend modes are drawn as normal, and Properties says so.
- **Cels:** ordinary and compressed cels are read, and a linked cel shows the picture of the frame it's linked to. Tilemap layers are skipped, with a note. RGBA, greyscale and indexed colour (with its palette and transparent colour) all work.
- **Tags become clips** with the tag's name, so `Sprite.clip = "swing";` plays the tag *swing*. A tag's direction (forward, reverse, ping-pong, ping-pong reversed) is the clip's **Direction**, and its **Repeat** field is how many times the clip **Plays**: empty (∞) is over and over, 1 is once, 3 is three times ([Animating sprites](sprite-animation.md)). Two tags with one name: the second is left out.
- **Slices become pivots.** A slice's pivot point (set in the slice's properties in Aseprite) is where a sprite stands; a slice without one stands on its middle. A sprite names its slice in `Sprite.pivot`. When you place a sheet in the editor, Solis fills that in with a slice called **pivot**, or else one called **feet**; in a level you write yourself, write it yourself: an empty `pivot` stands the sprite on the bottom middle of its frame. A slice that moves from frame to frame (its keys) moves the pivot with it.
- **User data** text on a cel is an event on that frame (a footstep, a swing), which game code hears as the clip plays.
- **Limits:** up to 4096 × 4096 pixels, 2048 frames, 256 layers and 64 MB a file.

A plain `.png` or `.jpg` works as a sheet of one frame, standing on its bottom middle.

## Levels
A level is a `.sollevel` file: a line saying its version, then a block for each entity, with a line for each field of each component ([Saving levels](saving-levels.md)).

```text
-- A level. Safe to edit by hand: one block per entity, one line per field.
level version 5;

entity 096Q14DR9GPQY {
    name = "Guard";
    order = "a0";
    Collider2D.radius = 4.0;
    Collider2D.character = true;
    Guard.state = "Calm";
    Guard.facing = "Left";
    Guard.sight = 6.0;
    Guard.post = "3YS3C4DWA7N36";
    Sprite.sheet = "A9S346Q3D25VT";  -- sprites/knight.aseprite
    Sprite.clip = "idle";
    Sprite.pivot = "feet";
    Tags.tags = "37ZXYYK596NGY";
    Transform.position = [6.0, 2.0, 0.0];
    Transform.rotation = [0.0, 0.0, 0.0, 1.0];
    Transform.scale = [1.0, 1.0, 1.0];
}

entity 3YS3C4DWA7N36 {
    name = "Gate";
    order = "a1";
    Transform.position = [10.0, 2.0, 0.0];
    TriggerZone.shape = "Box";
    TriggerZone.size = [2.0, 1.0, 1.0];
    TriggerZone.fires = "Once";
}

entity BRE95B9EE0ZBG {
    name = "Camp";
    order = "a2";
    TileMap.map = "FVMXXQKFBF5KZ";  -- maps/camp.tilemap.lst
    Transform.position = [0.0, 8.0, 0.0];
}
```

- `level version 5;` is the layout's version. Write it as it is.
- Each block is `entity <ID> { … }`. Solis writes them sorted by ID, so a change never moves a block; a file in any order reads the same.
- `name` is what the Level panel shows. `order` is the entity's place among its siblings, as text that sorts: Solis writes `a0` to `az`, then `b10`, and so on. `parent = "<ID>";` puts it under another entity.
- Every other line is `Component.field = value;`. Solis writes every field of a component; a field you leave out (as these examples leave most out) takes its standard value (see [The 2D components](#the-2d-components)). A component with no fields, or none written, is `Name = {};`.
- `--` starts a comment. The comment after a file's ID is only for people: Solis writes the file's path there and never reads it.
- A component Solis doesn't know (game code that isn't there yet) is kept exactly as written.

**Values** are written like this:

| Kind | Written | Example |
|---|---|---|
| Yes or no | `true` or `false` | `Sprite.playing = false;` |
| Whole number | digits | `Sprite.frame = 3;` |
| Decimal | with a `.` (a whole number works too) | `Sprite.speed = 1.5;` |
| Text | in quotes; `\"`, `\\` and `\n` inside | `name = "Guard \"Bob\"";` |
| A place, a size | a list of numbers | `Transform.position = [6.0, 2.0, 0.0];` |
| A rotation | four numbers, a quaternion (x, y, z, w) | `[0.0, 0.0, 0.0, 1.0]` is unturned |
| A colour | three numbers from 0 to 1 (and a fourth for its see-through, in game code's colours) | `Sprite.tint = [1.0, 0.5, 0.5];` |
| A file | its ID in quotes; `""` for none | `Sprite.sheet = "A9S346Q3D25VT";` |
| An entity | its ID in quotes; `""` for none | `Guard.post = "3YS3C4DWA7N36";` |
| A [state](concepts.md#game-code) | its option's name in quotes | `Guard.facing = "Left";` |

### A game code's component, and its states
A component of your game code is written the same way, its fields by the names they have in your code. Say the game's `guard.lst` has `state Mood = [Calm, Alert];` and `state Facing = [Left, Right];`, and its `Guard` component's `has` holds `state = Mood.Calm;`, `Facing facing = Facing.Left;`, `float sight = 6;` and `entity post;`. The level above keeps them as:

- **its own state** ([States that start and stop](states.md)) as `Guard.state = "Calm";`;
- **a field that holds a state** as `Guard.facing = "Left";`: the option's name alone, in quotes, never `Facing.Left`;
- `sight` as a decimal, and `post` as the ID of the entity it names.

A field left out starts as its code starts it, so `Guard = {};` is a guard with every field as written in `has`.

### A placed module and its changes
A copy of a [module](modules.md) placed in a level is one block for the copy, plus a short block for each part the copy changes. Say `props/crate.module.lst` (ID `AWJ47TAN9ZT24`) is a crate whose root has the ID `ANPZX45HY43KW` and whose lid, a part under it, has `9RP1XPA7Z3DJ8`:

```text
entity 909TQM83XSSSS {
    name = "Crate";
    order = "a3";
    module = "AWJ47TAN9ZT24";  -- props/crate.module.lst
    Transform.position = [3.0, -1.0, 0.0];
}

entity 909TQM83XSSSS/9RP1XPA7Z3DJ8 {
    name = "Open lid";
    Sprite.frame = 2;
}
```

- **The copy's block** has an ID of its own (a new one for each copy) and `module = "<the module file's ID>";`. Its fields are only those that differ from the module's root: here, where it is. Everything else comes from the module, so changing the module changes every copy.
- **A part's block** is named `<copy's ID>/<the part's ID in the module>`, and holds only what this copy changes: a `name`, an `order`, fields, `remove = ["Sprite"];` to take components off, `removed = true;` to take the part out of this copy, or `parent`. `parent = "<ID>";` moves the part under that entity; `parent = "";` moves it out to the top level; no `parent` line leaves it where the module has it.
- A module placed inside another module is reached by a longer name: `<copy>/<inner copy>/<part>`, at most 16 deep.

## Modules
A module is a `.module.lst` file, a Lust data file: a list of parts, one for each of its entities, each with its fields as the lines a level would write.

```text
-- A module. Written by Solis; safe to edit by hand: a part per entity, a line per setting.
fetch "solis/data/module":[Part];

lock int version = 1;
lock string root = "ANPZX45HY43KW";
lock Part array parts = [
    new Part {
        id = "ANPZX45HY43KW",
        name = "Crate",
        order = "a0",
        settings = [
            "Transform.position = [0.0, 0.0, 0.0]",
            "Collider2D.shape = \"Box\"",
            "Sprite.sheet = \"C629DWG0NJR44\""
        ]
    },
    new Part {
        id = "9RP1XPA7Z3DJ8",
        name = "Lid",
        parent = "ANPZX45HY43KW",
        order = "a0",
        settings = ["Sprite.sheet = \"C629DWG0NJR44\"", "Sprite.frame = 1"]
    },
];
end
```

- `version` is 1. `root` is the ID of the part everything else hangs from.
- Each `settings` line is a field as a level writes it, without the `;`, inside quotes, so the quotes of text in it are written `\"`. A component with no fields is `"Name = {}"`.
- A part may also have `module = "<ID>"` (a module placed inside this one); then parts named `<that part's ID>/<a part inside>` hold its changes, as in a level, with `removed = true`, `remove = […]` and `parent` as there.
- The module file's own ID, in its sidecar, is what a level's `module = "…";` names.

## Tilesets
A tileset is a `.tileset.lst` file: the terrains a tile map's cells can be, and which tiles of a picture show them ([Tilesets and autotiling](tilesets.md)).

```text
-- A tileset. Written by Solis; safe to edit by hand.
fetch "solis/data/tileset":[Tileset, Terrain, Piece, Rule];

lock int version = 4;
lock Tileset tileset = new Tileset { picture = "9T97KB6CQ643D", tile = [16, 16] };
lock Terrain array terrains = [
    new Terrain { id = "EXWCBYB6WK952", name = "grass", code = ".", colour = [0.31, 0.48, 0.23] },
    new Terrain { id = "CWA0432CF1XMW", name = "wall", code = "#", colour = [0.5, 0.5, 0.5], kind = "edges", solid = true },
    new Terrain { id = "6EJZ05XHT5GME", name = "water", code = "~", colour = [0.18, 0.37, 0.56], solid = true },
];
lock Piece array pieces = [
    new Piece { id = "9146S6JR410DD", terrain = "EXWCBYB6WK952", slot = 0, at = [0, 0] },
    new Piece { id = "1YRST4CHN5KN0", terrain = "EXWCBYB6WK952", slot = 0, at = [1, 0], weight = 0.5 },
];
lock Rule array rules = [];
end
```

- `version` is 4. `picture` is the ID of the PNG, JPEG or Aseprite file the tiles come from (leave it out to draw each terrain in its colour), and `tile` the size of a tile in pixels.
- **A terrain** has an ID, a `name` (letters, digits and `_`, up to 32), a `code`, the one character a tile map writes for it (any visible character but `"`, `\`, `?`, `=`, `!`, `_` and `*`), a `colour`, a `kind` (`plain`, the standard, or `16-tile`, `47-tile`, `edges` or `rules`) and whether it's `solid` (nothing with a body walks into it). The first terrain fills a new map. Up to 64.
- **A piece** is a tile of the picture showing one slot of a terrain: `at` is its column and row in the picture, counting tiles from 0 at the top left. A plain terrain has one slot, 0. An `edges` terrain's slot adds up the sides joined to the same terrain: north 1, east 2, south 4, west 8. A `16-tile` slot adds up the corners that are the terrain: top left 8, top right 4, bottom left 2, bottom right 1. A `47-tile` terrain's slots are easiest filled in the Tileset tab, or from its template.
- More than one piece in a slot are variations, picked by `weight` (standard 1). `animation = "columns"` with `frames` and `ms` plays that many tiles to the right; `animation = "frames"` plays an Aseprite picture's frames.
- **A rule** (for a `rules` terrain) is a `pattern` of rows around a cell and the `slot` it shows, tried in order: `?` anything, `=` the same terrain, `!` not the same, `_` nothing, or a terrain's code.

## Clips files
A clips file is a `.clips.lst` file beside its picture or Aseprite file (`knight.clips.lst` beside `knight.png`): what a sprite can play ([Animating sprites](sprite-animation.md)). Its first look is the sheet it plays; an Aseprite file's frames are its cells (from 0), and its tags are clips already, so the file only adds to them (`tags`). Clips made in Solis are `clips`, each with a row of cells for each of its directions (`rows`).

```text
-- Clips of walker.aseprite. Written by Solis; safe to edit by hand.
fetch "solis/data/clips":[Sheet, Cell, LookPicture, ClipSettings, ClipRow, TagChange, Facing, Playback];

lock int version = 2;
lock Sheet sheet = new Sheet { cell = [16, 16], between = [0, 0], edge = [0, 0], first = "hoe" };
lock Cell array cells = [];
lock LookPicture array looks = [
    new LookPicture { name = "Main", picture = "1BWHNR3RV8PYY" },
];
lock ClipSettings array clips = [
    new ClipSettings { name = "hoe", speed = 1.0, playback = Playback.Forward, plays = 1, directions = 1, sameFrames = true, mirror = false, times = [50, 200], events = ["", "swing"] },
];
lock ClipRow array rows = [
    new ClipRow { clip = "hoe", facing = Facing.Any, cells = [4, 1], times = [], events = [] },
];
lock TagChange array tags = [
    new TagChange { name = "walk", speed = 2.0, frames = [0], events = ["dust"] },
];
end
```

## Tile maps
A tile map is a `.tilemap.lst` file: a grid of cells in layers, each cell a terrain's code, and values every cell has ([Tile maps](tile-maps.md)).

```text
-- A tile map. Written by Solis; safe to edit by hand.
-- Cells are in chunks of 32 by 32: each layer's rows of terrain codes (a space
-- for an empty cell), and the values of cells that aren't at their start,
-- each [x, y, value] within the chunk. Exact tiles are cells that show one
-- piece of the tileset as it is, each [x, y] within the chunk with its piece.
fetch "solis/data/tilemap":[ValueKind, TileMap, Layer, Value, Cells, Exact, Numbers, Switches, Choices];

lock int version = 3;
lock TileMap tiles = new TileMap { tileset = "2F5V37E3S3E28", size = [10, 4] };
lock Layer array layers = [
    new Layer { id = "D78F603XZBCWC", name = "Ground", over = false },
];
lock Value array values = [
    new Value { id = "4TR7STD0HK1DB", name = "loot", kind = ValueKind.Number },
];
lock Cells array cells = [
    new Cells { layer = "D78F603XZBCWC", chunk = [0, 0], rows = ["##########", "#........#", "#...~~...#", "##########"] },
];
lock Exact array exact = [];
lock Numbers array numbers = [
    new Numbers { value = "loot", chunk = [0, 0], cells = [[2, 1, 5]] },
];
lock Switches array switches = [];
lock Choices array choices = [];
end
```

- `version` is 3. `tileset` is the tileset's ID, and `size` the map's width and height in cells.
- **Layers** are listed bottom first; `over = true` draws one over sprites.
- **Cells** come in chunks of 32 × 32. `chunk = [1, 0]` holds cells 32 to 63 across and 0 to 31 down. Cells count from the map's top-left corner: x to the right, y down, as the rows read. Each row is one character a cell, a terrain's code, with a space for an empty cell. A chunk or a row left out, or cut short, is empty there.
- **Values** are `ValueKind.Number` (whole numbers), `ValueKind.Switch` (on or off) or `ValueKind.Choice` (with its `options`). Only cells away from their start (0, off, none) are listed, by the value's name: `Numbers` as `[x, y, number]`, `Switches` as `[x, y]` for each cell that's on, `Choices` as `[x, y]` with the option in `picks`, in the same order. The `x` and `y` are within the chunk.
- **Exact** cells show one piece of the tileset as it is (`pieces`, by the piece's ID) instead of the tile worked out from their neighbours.
- Keep every list, even an empty one, as here.
- In a level, the map's top-left corner is on its entity's place, and each cell is one tile of the project's [2D settings](2d-pixel-perfect.md).

## The 2D components
The fields of the built-in components a 2D game uses, as a level writes them, with each one's standard value: what a field left out is. In a 2D level, x is across (to the right), y is up the screen and z stays 0; places are in metres, and a metre is the 2D settings' **Pixels per unit** (16 pixels as standard).

**Transform**: where the entity is, from its parent.

| Field | Holds | Standard |
|---|---|---|
| `position` | x, y, z in metres | `[0.0, 0.0, 0.0]` |
| `rotation` | a quaternion | `[0.0, 0.0, 0.0, 1.0]` |
| `scale` | x, y, z | `[1.0, 1.0, 1.0]` |

**Sprite**: a frame of a sprite sheet, or a clip of its frames playing ([Sprites from Aseprite](sprites.md)).

| Field | Holds | Standard |
|---|---|---|
| `sheet` | the ID of an Aseprite file or picture, or of its clips file (`.clips.lst`) | `""` |
| `clip` | the clip it plays: a tag's name, or a clip made in its clips file. Empty: the clips file's **Plays first**, or the still `frame` | `""` |
| `playing` | whether the clip moves on | `true` |
| `speed` | 1 as drawn, 2 twice as fast (0 to 16) | `1.0` |
| `time` | how far into the clip, in seconds | `0.0` |
| `frame` | the frame shown when no clip plays, counting from 1 as Aseprite does | `1` |
| `pivot` | the slice it stands on; empty for the bottom middle | `""` |
| `flip_x`, `flip_y` | mirrored across, or up and down | `false` |
| `tint` | a colour multiplied in | `[1.0, 1.0, 1.0]` |
| `flash` | how white it flashes, 0 to 1 | `0.0` |
| `outline`, `outline_color` | a one-pixel outline, and its colour | `false`, `[0.0, 0.0, 0.0]` |
| `see_through` | drawn blended, for glass and smoke | `false` |

**Collider2D**: a shape on the ground for 2D physics, in pixels ([Walls and walking in 2D](walls-and-walking.md)).

| Field | Holds | Standard |
|---|---|---|
| `shape` | `"Circle"` or `"Box"` | `"Circle"` |
| `radius` | a circle's radius | `5.0` |
| `width`, `height` | a box's size | `16.0`, `16.0` |
| `across`, `up` | how far right and up of the entity's place its middle is | `0.0`, `0.0` |
| `character` | walked by game code, sliding along walls; otherwise solid and still | `false` |

**TileMap**: a tile map placed with its top-left corner on the entity ([Tile maps](tile-maps.md)).

| Field | Holds | Standard |
|---|---|---|
| `map` | the tile map's ID | `""` |

**TriggerZone**: a place that tells game code when something walks in or out ([Trigger zones](trigger-zones.md)).

| Field | Holds | Standard |
|---|---|---|
| `shape` | `"Box"`, `"Round"` (a circle in 2D) or `"Freeform"` | `"Box"` |
| `size` | a box's size along x, y and z, in metres | `[2.0, 2.0, 2.0]` |
| `radius` | a circle's radius, in metres | `1.0` |
| `points` | a freeform outline: x and y of each point in turn, in metres from the entity, at least three points, never crossing | `[]` |
| `notices` | what it notices: only `"Characters and bodies"` for now | `"Characters and bodies"` |
| `fires` | `"Every time"` or `"Once"` | `"Every time"` |
| `only_tags` | tags' IDs, a space between: only what carries one of them counts | `""` |

**Tags**: the project's tags the entity carries ([Tags](tags.md)).

| Field | Holds | Standard |
|---|---|---|
| `tags` | the tags' IDs from `tags.lst`, a space between, at most 32 | `""` |

The project's `tags.lst` lists them, each an ID and the name game code uses (`Tag.Enemy`):

```text
-- The project's tags (Project settings › Tags). Written by Solis; safe to edit by hand.
fetch "solis/data/tags":[Tag];

lock int version = 1;
lock Tag array tags = [
    new Tag { id = "37ZXYYK596NGY", name = "Enemy" },
];
end
```

**Animator**: an animator choosing the Sprite's clip ([Animators](animators.md)).

| Field | Holds | Standard |
|---|---|---|
| `animator` | the animator file's ID (`.solanim`) | `""` |
| `state` | the state it's in; empty for the animator's first | `""` |
| `speed` | how fast its states play | `1.0` |

**Parallax**: the Sprite as a layer that moves with the camera at its own speed ([Parallax layers](parallax.md)).

| Field | Holds | Standard |
|---|---|---|
| `scroll_x`, `scroll_y` | how far it moves with the camera, across and up: 0 still, 1 with the world (0 to 4) | `0.5`, `0.5` |
| `repeat_x`, `repeat_y` | drawn again and again side by side, or above and below | `true`, `false` |
| `in_front` | drawn over everything | `false` |

**Camera**: a 2D game's camera is a Camera with `view = "2D";` (the standard is `"Perspective"`), following its entity's place ([2D](2d-pixel-perfect.md)).

## Versions
The newest layout of each file, which is what Solis writes:

| File | Where its version is | Newest |
|---|---|---|
| Level, `.sollevel` | `level version 6;` | 6 |
| Module, `.module.lst` | `lock int version = 2;` | 2 |
| Clips, `.clips.lst` | `lock int version = 2;` | 2 |
| Tileset, `.tileset.lst` | `lock int version = 4;` | 4 |
| Tile map, `.tilemap.lst` | `lock int version = 3;` | 3 |
| Tags, `tags.lst` | `lock int version = 1;` | 1 |
| Sidecar, `.solmeta` | `format = 1` | 1 |
| 2D settings, `pixels.solpix` | `pixels version 1;` | 1 |

Animators (`.solanim`) are text too, with their version on their first line that isn't a comment, and a comment at the top saying what each line holds; they're made in the [Clips and Frames tabs](sprite-animation.md) and the [Animator tab](animators.md).

## Tips and common problems
- **Problem:** a sprite in a level you wrote floats or sinks. **Why:** `Sprite.pivot` is empty, so it stands on the bottom middle of its frame. **Fix:** write `Sprite.pivot = "feet";` (or the slice your sheet has).
- **Problem:** "… isn't an ID (13 letters and digits)". **Why:** an ID is mistyped, or made by hand with a letter IDs don't use. **Fix:** make IDs with `solis --new-id`.
- **Problem:** a sprite, tile map or module copy you wrote doesn't show. **Why:** the ID it names isn't in any sidecar: the sidecar wasn't written, was written with another ID, or isn't named `<file>.solmeta` (a sidecar of `knight.aseprite` is `knight.aseprite.solmeta`). **Fix:** check the sidecar beside the file.
- **Problem:** "the ID … is used twice". **Why:** two blocks of one level have the same ID (a script made one ID and used it twice). **Fix:** give each entity its own ID.
- **Problem:** a file says it "was saved by a newer version". **Why:** its version is higher than this Solis reads. **Fix:** write the version on this page, or update Solis.
- **Tip:** to learn a file's layout, make one in the editor and read it: Solis writes every file the way this page shows.

## Related
- [Projects and the Library](projects-and-the-library.md): sidecars, IDs and `solis --new-id`.
- [Saving levels](saving-levels.md), [Modules](modules.md), [Tile maps](tile-maps.md), [Tilesets and autotiling](tilesets.md), [Sprites from Aseprite](sprites.md).
- [Checking a project](checking-a-project.md) and [Playing a game from the command line](playing-from-the-command-line.md): trying files a script wrote.
- [Backups and upgrades](backups-and-upgrades.md): how older files are upgraded.
- Concepts: [project, asset, sidecar](concepts.md#projects), [level, entity, component](concepts.md#the-level), [state](concepts.md#game-code), [2D terms](concepts.md#2d).
