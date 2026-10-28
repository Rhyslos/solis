# 0058. Listeners, and who hears what

- **Status:** Accepted (extends 0056)
- **Date:** 2026-10-28

## Problem

Decision 0056 gave the game one listener: the Game panel's camera, which hears every sound. Looking at the sound event mock-up, the project lead asked for a clear transmitter and receiver: not everything should hear every emitter, and not every emitter should be heard by everything. Some games need it:

- **Several players' ears:** split-screen, or each player in a networked game hearing from their own character or camera.
- **Sounds for some listeners only:** a team's radio, a whisper to one player, the interface clicks of the player who clicked, the footsteps of the player's own character in first person.

The same review asked that sound events say **in the world** or **everywhere** instead of "3D" and "2D", and show whether each file is **mono** or **stereo**. Both are done in Phase 7, S2.

## Options

1. **A `Listener` component, and "Heard by" on each sound event** (recommended). Put a Listener on a camera or character; with none, the game's camera hears, as now. Each event says who hears it: everyone, only the player who caused it, or named groups that listeners belong to.
2. **A `Listener` component only:** several sets of ears, but every listener hears every sound.
3. **Channels on both ends:** emitters send on channels and each listener picks the channels it takes. The most flexible, but the most to set up, even for a game with one player.

## Decision

Option 1, as the project lead chose:

- **`Listener`** is a built-in component. A game with none hears through its camera, so nothing needs setting up for one player.
- **"Heard by"** on a sound event: **Everyone** (the default), **Only who caused it** (the entity that played it, or its player), or **Groups**: names such as "Team Red" or "Radio". A Listener lists the groups it belongs to.
- **Nothing about sound crosses the network** (netcode, Phase 16): each computer plays the sounds of its own player's listener from what happens in the game. The listener is part of the game's state; what's heard stays outside it, so sound still never changes the game (decision 0056).
- It's built as Phase 7, milestone **S8**, after reverb zones. The Listener's and Heard by's places in Properties are mocked up for the project lead first.

## Consequences

- S3 keeps the camera as the only listener; S8 replaces that with "the Listener components, or the camera".
- `.solsound` gains a `heard_by` setting in S8. Files without it are heard by everyone, so S2's files keep working.
- Netcode's plan (Phase 16) uses one listener per player on each computer, and needs no sound messages.
- Phase 7 has 14 milestones instead of 13.
