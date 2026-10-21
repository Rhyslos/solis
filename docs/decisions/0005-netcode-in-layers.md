# 0005. Netcode in layers: the shared parts built in, the model optional

- **Status:** Accepted (replaces "the simulation is deterministic by default" and the netcode pitch in the first design)
- **Date:** 2026-10-21

## Problem

The first design promised one deterministic core giving lockstep, rollback and server-authoritative netcode, as Solis's selling point. No single netcode fits every game: strategy games, shooters, fighting games and big co-op games each need a different model, and the model shapes how the whole game is written. Should Solis provide netcode at all, or leave it to each game?

## Options

1. **Leave it to the games.** What Bevy does. Every team has to solve the same hard, shared problems themselves.
2. **One built-in model for everything.** No model suits every genre, so many games would fight it, as many Unity and Godot projects do.
3. **Layers:** build in everything every online game needs, and offer the model as optional modules.

## Decision

Option 3.

**Built in, for every game:**
- **Transport:** connections, reliable and unreliable messages, encryption.
- **Sessions:** lobbies, invites, getting through home routers (relays), and Steam networking.
- **Foundations:** a fixed simulation timestep, seeded randomness, and compact message encoding.
- **Tools:** a lag and packet-loss simulator, network stats, replays, and desync detection.

**One model, optional: lockstep.** For strategy games, Solis's first focus and the flagship sample. Few engines offer it, which keeps a part of the original pitch.

**Other models through existing Rust crates,** made to plug in cleanly and documented, rather than rebuilt: `renet` or `lightyear` for server-authoritative games, `ggrs` (based on GGPO) for rollback, `matchbox` for peer-to-peer.

**Determinism:** the cheap parts (fixed timestep, seeded randomness) are defaults, since they help every game. Strict cross-platform determinism is opt-in, for lockstep games.

## Consequences

- The pitch becomes: the shared networking layers built in properly, with a strategy-ready model included.
- ROADMAP.md Phase 8 is smaller and more realistic.
- Games and the game logic decide what to sync, who owns what, and how to stop cheating.
