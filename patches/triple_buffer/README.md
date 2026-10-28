# triple_buffer

**Patch · stands in for a library**

Hands the newest value from one thread (the writer) to another (the reader) without either one waiting. It is the part of the `triple_buffer` crate's interface that the sound library Kira uses to pass commands to its audio thread.

## Purpose

Stand in for `triple_buffer` (MPL-2.0), so every dependency stays permissive (decision 0056; [`docs/dependencies.md`](../../docs/dependencies.md), "Patches"). The root `Cargo.toml`'s `[patch.crates-io]` points Kira here.

| Folder | What it holds |
| --- | --- |
| [`src/`](src/README.md) | The whole stand-in and its tests, in `lib.rs`. |

## Must never

- Copy code from the original crate: it is written from the interface alone
- Make the writer or the reader wait: the audio thread must never stall
- Use `unsafe` code (the workspace denies it)

## Depends on

`crossbeam-utils` (MIT or Apache-2.0), already in the build through other libraries.

## Status

In use since Phase 7, S1. Removed if Kira drops `triple_buffer` or changes what it uses; `cargo deny check` fails if the original ever comes back.
