# Reverb zones

> Places where sounds echo: walk into a cave and every footstep rings.

## When to use it
Any time a place should sound like a place: a cave, a hall, a tunnel, a small room, a sewer. Reverb is the echo a space adds to every sound in it, and it's one of the strongest cues of where you are: the same footstep sounds tight in a cupboard and huge in a cathedral. A reverb zone adds it while the game's camera (the listener) is inside, so the sound changes as you walk in and out, without touching any sound event.

## How to use it
1. Add an entity where the space is (Entity > Add entity), and move it to the middle of the space.
2. In Properties, **Add component > ReverbZone**.
3. Pick its **Shape**: a **Box** for rooms, halls and tunnels (set its **Size** along X, Y and Z), or a **Ball** for caves and domes (set its **Radius**). With the entity selected, the zone is outlined in the viewport in teal, so you can see what it covers. It grows, turns and moves with its entity.
4. Pick what it **Sounds like**, and its **Strength**.
5. Press **Play** and move the camera in and out: sounds placed in the world (`playSoundAt`, `playSoundOn`, AudioSources) echo inside, and the echo fades in across the **Soft edge** as you go in.

Only sounds placed **In the world** get the echo; music and the interface, heard the same everywhere, don't. Echoes follow their bus: turn a bus down or mute it in the [Mixer](mixer.md) and its echoes go with it.

## Settings
| Setting | What it does |
|---|---|
| Shape | **Box** or **Ball**. |
| Size | A box's size along X, Y and Z, in metres. A hall might be 30 × 20 × 10. |
| Radius | A ball's radius, in metres. |
| Soft edge | How far in from the edge the echo is full, in metres: walking in, it fades in over this distance, so there's no sudden switch. 2 m suits a doorway; 0 switches at the edge. |
| Sounds like | **Small room**: short and close. **Hall**: long and smooth. **Cave**: long and dark. **Sewer**: hard and ringing. **Outdoors**: a little air, for a valley or a courtyard. |
| Strength | How strong the echo is: 1 full, 0.5 half, 0 none. |

**Overlapping zones:** where zones overlap, the smaller wins: a cupboard inside a hall sounds like the cupboard, and stepping out of it blends into the hall. Up to 4 zones are heard at once.

## Tips and common problems
- **Problem:** no echo inside the zone. **Why:** the sound plays **Everywhere** (only sounds **In the world** echo), its bus is muted or turned down, or the camera isn't in the zone (the listener is the game's camera, not the player). **Fix:** check the sound event's Placement, the Mixer, and where the camera is with the zone selected.
- **Problem:** the echo switches on suddenly. **Why:** the **Soft edge** is 0 or very small. **Fix:** 1 to 3 m.
- **Problem:** everything sounds washed out. **Why:** the zone's too strong for the place, or a big zone covers more than it should. **Fix:** lower its **Strength**, or make it smaller.

## Related
- [Sound events](sound-events.md): Placement, In the world.
- [The Mixer](mixer.md): buses, which echoes follow.
- [Concepts](concepts.md#sound): reverb.
