# Animating sprites

> Make sprites walk, swing and wave: play the clips you tagged in Aseprite, make clips of your own in Solis, and give frames events that play footsteps and tell your game code.

## When to use it
For anything in a 2D game that moves frame by frame: a farmer walking, a hoe swinging, crops swaying, a door opening. A [clip](concepts.md#2d) is a run of a [sprite sheet](concepts.md#2d)'s frames, each shown for its own time, played forward, backward or there and back (ping-pong), over and over or a number of times.

The quickest way is Aseprite's own: **tag** runs of frames there (*walk*, *idle*, *hoe*) and every tag is a clip in Solis, with Aseprite's frame times. To change a clip's speed, give frames events (a footstep on the frame a foot lands), or make clips Aseprite doesn't have, Solis keeps a small **clips file** (`.solframes`) beside the Aseprite file. A sprite can play either.

Choosing clips by what's happening (walking when moving, idle when not) is game code for now; the state machine that does it for you comes next.

## How to use it
1. **Tag your frames in Aseprite** and save. In Solis, click the file in the Library: Properties lists its tags.
2. **Play a clip on a sprite:** select a sprite and pick a **Clip** in its Sprite component. Press **Play**: it plays at its frame times. To see it move while you edit, open the viewport's **≡** menu and tick **Preview animation**.
3. **Open the clips:** in the Aseprite file's Properties, press **Edit clips…**. Solis makes `walker.solframes` beside `walker.aseprite` (or opens the one there) and opens two tabs:
   - **Clips**, beside Properties: every clip, with how long it is and where it comes from (*Aseprite* or *made here*). Click one to choose it. Below the list: its **Speed**, **Direction** and how many times it **Plays**, and **Plays first**.
   - **Frames**, at the bottom: the chosen clip's frames in order along a time ruler, each as wide as it lasts, so you see its timing at a glance. A ping-pong's way back is shown faded after them. The blue line is where it's playing; the lane below shows its events.
4. **Watch it:** press **▶** (or **Space** over the Frames tab). **⏮** goes back to the start, **■** stops, **⏭** goes to the last frame. **−** and **+** zoom the ruler.
5. **Give a frame an event:** click the frame, then type a name in **event**, or pick a sound event from **◆ ▾**. When the clip reaches that frame:
   - a sound event of the same name (`footstep`, or its path, `sfx/footstep`) plays on the sprite by itself, with no code;
   - game code on the entity hears `AnimationCue`, with the event's name.
   Events typed in Aseprite (a cel's user data) work the same way.
6. **Make a clip of your own:** **＋ Clip** makes an empty one; **⧉ Copy** copies the chosen clip (a tag's copy can then be changed here). In the Frames tab, **＋ Add frames ▾** shows the sheet's frames: click one to add it after the picked frame. Double-click a clip of your own to rename it.
7. **Change frames of your own clips:** click a frame to pick it (Ctrl+click or Shift+click for several), then set its **time**, drag it to a new place, or press **Delete** to take it out.
8. **Close the file:** the **×** on either tab closes both.

Every change is saved at once and shows at once on every sprite playing the file. **Ctrl+Z** and **Ctrl+Y** over either tab undo and redo changes to the file.

### In game code
Bring in what you need from `solis/animation` at the top of a file:

```lust
get "solis/animation":[playClip, stopClip];
fetch "solis/animation":AnimationCue;
fetch "solis/animation":ClipFinished;

component Farmer {
    has {
        int steps = 0;
    }
    on AnimationCue(string name) {
        is name == "footstep"? then {
            steps = steps + 1;
        }
    }
    on ClipFinished(string clip) {
        is clip == "hoe"? then {
            playClip(me, "idle");
        }
    }
}
pass Farmer;
end
```

- `playClip(entity, "walk")` plays a clip. If it's already playing that clip it carries on, so it's safe to call every tick; another clip, or one that has finished, starts from its beginning.
- `stopClip(entity)` stops it where it is; `playClip` with the same clip carries on from there.
- In Lua: `solis.playClip(entity, "walk")` and `solis.stopClip(entity)`; a function called `on_AnimationCue(name)` or `on_ClipFinished(clip)` reacts to the events, as with any event. In Rust: `cx.host().play_clip(entity, "walk")` and `stop_clip`, and the events arrive in `handle` named `AnimationCue` and `ClipFinished`.

A clip's place is part of the game: the same inputs give the same frames every time, and a saved game comes back on the same frame.

## Settings
The Sprite component's clip settings:

| Setting | What it does |
|---|---|
| Sheet | An Aseprite file (its tags are its clips) or a clips file (`.solframes`). |
| Clip | The clip it plays. Empty: the clips file's clip marked **Plays first**, or with none, the still **Frame**. |
| Playing | Whether the clip moves on while the game runs. Game code's `playClip` and `stopClip` turn it on and off. |
| Speed | 1 as drawn, 2 twice as fast, 0 stopped. Multiplies the clip's own speed. |
| Clip time | How far into the clip it is, in seconds. Give a crowd of copies different times so they don't all step in step. |
| Frame | The frame shown when no clip plays. |

A clip's settings in the Clips tab:

| Setting | What it does |
|---|---|
| Speed | How fast this clip plays for every sprite, 1 as drawn. A tag's speed is kept in the clips file. |
| Direction | Forward, reverse, ping-pong (there and back) or ping-pong reversed. A tag's is Aseprite's. |
| Plays | Over and over, once, or a number of times; it then stays on its last frame and game code hears `ClipFinished`. A tag's is Aseprite's (its Repeat field). |
| Plays first | The clip a sprite plays when its Clip is empty. |

## Shortcuts
| Action | Shortcut |
|---|---|
| Pick a frame | Click it |
| Pick several frames | `Ctrl` + click, or `Shift` + click for a run |
| Move picked frames (your own clips) | Drag them |
| Take picked frames out (your own clips) | `Delete` over the Frames tab |
| Play or stop the clip | `Space` over the Frames tab |
| Undo or redo a change to the clips file | `Ctrl + Z`, `Ctrl + Y` over either tab |

## Tips and common problems
- **Problem:** sprites don't move in the viewport. **Why:** the editor rests unless something changes. **Fix:** tick **Preview animation** in the viewport's **≡** menu, or press **Play**.
- **Problem:** a tag's frames can't be moved, timed or taken out. **Why:** a tag's frames are Aseprite's, so saving there keeps both in step. **Fix:** change them in Aseprite, or **⧉ Copy** the clip to change the copy here.
- **Problem:** an event doesn't play a sound. **Why:** no sound event has its name. **Fix:** name it after the sound event (`footstep` or `sfx/footstep`), or pick one from **◆ ▾**.
- **Problem:** a whole field of crops sways in step. **Fix:** give each a slightly different **Clip time**.
- **Problem:** two footsteps in a ping-pong walk. **Why:** a ping-pong shows its middle frames twice a pass, there and back. **Fix:** put the event on the first or last frame, or use **Forward** for walks.

## Related
- [Sprites from Aseprite](sprites.md): sheets, placing sprites, pivots.
- [Sound events](sound-events.md): the sounds an event plays.
- [Game code](game-code.md): components and events.
- [Concepts](concepts.md#2d): clip, clips file, frame event.
