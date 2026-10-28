# asset_db/src

The project folder and its assets (decision 0021): `project.solis`, the `assets/` folder with a `.solmeta` sidecar beside each asset holding its stable ID, the rebuildable import cache in `.solis/`, watching the folder for changes, and worker threads for imports.

**Start reading at** `project.rs`, then `db.rs`.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers. | |
| `project.rs` | The project folder: the project file naming it, `assets/` holding what the user owns, `.solis/` as a cache that can be deleted any time. Also safe saving (write a temporary file, then swap it in). | `Project`, `ProjectError`, `write_atomically`, `read_limited` |
| `db.rs` | The asset database: every asset in `assets/` by its ID. The folder is the truth: the database is worked out from it on opening and checked again whenever something changes. | `AssetDb`, `AssetRecord`, `AssetKind`, `Change` |
| `sidecar.rs` | Sidecars: small TOML files beside each asset holding its ID, its import settings and the IDs of its parts, committed with the asset so the ID survives Git and renames. Fuzz-tested. | `Sidecar`, `Part`, `PartKey` |
| `backup.rs` | A copy before an older file changes (Phase 8, C1): saving a file not in the current format checks the new text reads back, copies the old file to `.solis/backups/<date and time>/`, then swaps the new one in; old backups trimmed by age and size, never the newest. | `save`, `trim`, `folder`, `SaveError` |
| `files.rs` | Every file and folder in `assets/` as the Library lists them, and renaming, moving, new folders and deleting to the Recycle Bin (a sidecar always goes with its file). | `Listing`, `FileEntry`, `FileKind`, `FileError` |
| `cache.rs` | The import cache: imported models saved as `.solbin` files keyed by a hash of content and settings, so a rename or a branch switch never imports again. Read as hostile; fuzz-tested. | `load`, `encode`, `decode`, `key` |
| `hash.rs` | A fast 64-bit hash of a file's bytes, to spot the same content under another name (not for security). | `ContentHash`, `hash_file` |
| `watch.rs` | Watches the assets folder. Events are only hints: the database looks again and decides. | `Watcher` |
| `jobs.rs` | A small pool of worker threads for imports; a newer job for the same asset drops the older one's result. | `Jobs` |

## How the pieces fit

Opening a project (`project.rs`) builds an `AssetDb` (`db.rs`) by listing `assets/` (`files.rs`) and reading each sidecar (`sidecar.rs`). When `watch.rs` reports a change, the database looks again and reports `Change`s; models to import go to `jobs.rs`, which checks `cache.rs` first (keyed by `hash.rs`).

**Read next:** the crate's [README](../README.md).
