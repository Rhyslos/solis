# Listeners and who hears what

> Where the game hears from, and which sounds each listener hears.

## When to use it
Most games need none of this: with no Listener, the game hears through its camera, and every sound is heard. Use it when that isn't enough:
- **The ears should be somewhere else than the camera:** a top-down game that should hear from the character on the ground, not the camera high above. Put a Listener on the character.
- **Some sounds are only for one player:** your own footsteps in first person, a click in your own menu, your own team's radio. A sound event's **Heard by** says who hears it.

Who hears what never changes the game: it only decides what's played on this computer, so a recording replays the same, and a multiplayer game (later, with netcode) sends no sound over the network.

## How to use it
1. Select the camera or the character the game should hear from, and **Add component > Listener** in Properties.
2. Leave **Hears now** on. If a level has several listeners (a player and a spectator camera), this computer hears through the first with **Hears now** on.
3. Give it **Groups** if some sounds are for groups: names with commas between, such as `Team Red, Radio`.
4. In a [sound event](sound-events.md)'s Properties, set **Heard by**:
   - **Everyone** (the start): every listener hears it.
   - **Only who caused it**: only the listener on the entity whose code played it, or on what that entity is part of (a character and the camera inside it count as one). An AudioSource is caused by its own entity.
   - **Groups**: only a listener in one of the event's **Groups**.

## Settings
| Setting | What it does |
|---|---|
| Listener > Hears now | Whether it's the one hearing. The first with this on hears; with none on, the camera does. |
| Listener > Groups | The groups it's in, with commas between. Sounds heard by Groups reach it if they share one. |
| Sound event > Heard by | Everyone, Only who caused it, or Groups, as above. |
| Sound event > Groups | The groups that hear it, with commas between (when Heard by is Groups). |

## Tips and common problems
- **Problem:** a sound heard by Groups is never heard. **Why:** no listener hears now that's in one of its groups; the camera on its own is in no group. **Fix:** add a Listener with that group to the camera or character.
- **Problem:** a sound heard by Only who caused it is never heard. **Why:** it's played by code on an entity that isn't the listener's, nor inside it, nor what it's inside. **Fix:** play it from the player's own entity, or a child of it.
- **Problem:** sounds come from the wrong place. **Why:** the listener is on a different entity than you expect (the first with Hears now on). **Fix:** turn Hears now off on the others.

## Related
- [Sound events](sound-events.md)
- [Game code: Sounds](game-code.md#sounds)
- [Concepts](concepts.md#sound): listener.
