# The Mixer

> The project's buses side by side, each with a fader, a meter, mute and solo, and ducking that turns music down while someone speaks.

## When to use it
When you want to balance a game's sound as a whole rather than one [sound event](sound-events.md) at a time: the music a little quieter, every effect a little louder, voices always clear. Each sound event plays through a [bus](concepts.md#sound), and a bus's fader turns all of its sounds up or down together, like a channel on a mixing desk. To change how one sound is heard (its volume, how far it carries), change its event instead.

## How to use it
1. Click the **Mixer** tab, beside the Console, Problems and the Profiler at the bottom. (If you moved it, look for its tab, or use **Window > Reset layout**.)
2. Each bus has a strip: its name, a **meter**, a **fader**, its volume in [dB](concepts.md#sound), and **M** (mute) and **S** (solo). **Master** comes first: every other bus is inside it, so its fader turns everything up or down.
3. Drag a fader, or drag or type the number under it (double-click a fader to put it back to 0 dB). The change is heard at once and saved in the project when you let go (in `mixer.solmix`, beside `project.solis`).
4. Press **Play**: the meters show how loud each bus is as the game plays, green, then yellow when loud, red near the top. If Master's meter keeps reaching red, turn something down.
5. Click a bus's name: [Properties](interface-tour.md) shows it, with its name, which bus it's inside, its volume and its **ducking** rules.
6. **Add bus** makes a new bus inside the one Properties shows (or inside Master): "Footsteps" inside Effects, say, so footsteps can be balanced on their own. Then pick it as the **Bus** of the sound events that belong to it.

**Ducking** turns one bus down while another plays, so speech is always heard over the music without turning the music down by hand. A new project already has one rule: while **Voice** plays, **Music** goes down 8 dB, taking 0.2 s to go down and 1 s to come back after the last voice ends. While it's working, the ducked bus's strip says so in amber ("ducked -8"), and so does the line at the top of the Mixer.

**Heard at once**, at the top of the Mixer, is how many sounds play at the same time: 64 to start with, from 16 to 256. When more want to play (a big battle), the most important are heard: by their event's **Priority**, then the loudest where you are. Sounds too far away aren't counted at all. More voices sound fuller and cost a little more time each frame (the [Profiler](profiler.md)'s Sound line shows it).

**Mute and solo** are for listening while you work, and aren't saved: **M** silences a bus, **S** plays only the soloed buses (and the buses they're inside), so you can hear one part of the game on its own. The speaker in the **Game** panel's top right corner mutes everything.

## Settings
| Setting | What it does |
|---|---|
| Name | What sound events call the bus. Renaming it renames it everywhere in the Mixer (buses inside it, ducking rules); sound events that named the old name play through Master until you pick the new one. Master keeps its name. |
| Inside | The bus it's inside: its sounds go through that bus's fader too. A bus can't be inside itself or a bus inside it, and buses nest up to 8 deep. |
| Volume | The fader, from -60 dB (silent) to +12 dB. 0 dB plays the sounds as they are; -6 dB is about half as loud. |
| Quieter while | The bus whose sounds turn this one down (a bus inside it counts too). |
| By | How much quieter, down to -60 dB. -6 to -10 dB is usual for music under speech. |
| Fade down | How long going quieter takes, in seconds. Short (0.1 to 0.3 s) so the first word is clear. |
| Fade back | How long coming back takes after the last of those sounds ends. Longer (0.5 to 2 s), so the music doesn't jump back between sentences. |

A bus can have up to 8 ducking rules (**Add a ducking rule**, and **×** to remove one), and a project up to 64 buses. **Remove bus** moves the buses inside it up to its parent.

**From game code**, `setBusVolume(bus, volume)` sets a bus for the rest of the play to a share of its fader, from 0 (silent) to 1 (as set in the Mixer): what an options menu's sliders call. See [Game code: Sounds](game-code.md#sounds).

## Tips and common problems
- **Problem:** a sound event isn't affected by its bus's fader. **Why:** its **Bus** names a bus that isn't there any more (renamed or removed), so it plays through Master. **Fix:** pick the bus again in the event's Properties.
- **Problem:** some sounds don't play in a busy moment. **Why:** more sounds wanted to play than **Heard at once** allows, and they ranked lowest. **Fix:** raise their event's Priority, or raise Heard at once.
- **Problem:** the meters don't move. **Why:** they show the game's sounds while it plays; the Library's ▶ and Properties' Play don't go through the buses. **Fix:** press Play.
- **Problem:** everything is silent. **Why:** a bus is soloed (only soloed buses are heard), Master is muted, or the Game panel's speaker is crossed out. **Fix:** click the lit **S** or **M**, or the Game panel's speaker.
- **Problem:** the Console says the mixer file had a mistake. **Why:** `mixer.solmix` was edited by hand and can't be read, so the standard buses are used. **Fix:** fix the line it names, or change anything in the Mixer: that writes the file again and keeps the old one as `mixer.solmix.bak`. Changes made to the file by hand are read when the project is opened.

## Related
- [Sound events](sound-events.md): each event's Bus.
- [Game code: Sounds](game-code.md#sounds): `setBusVolume`.
- [Concepts](concepts.md#sound): bus, ducking, decibel.
