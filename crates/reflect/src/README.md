# reflect/src

How components describe themselves (decision 0019): stable IDs, plain values, component schemas, and a registry looked up by component name. Any component, whatever language it is written in, is held and shown through these.

**Start reading at** `value.rs`, then `schema.rs`.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers. | |
| `id.rs` | Stable IDs: random 64-bit numbers written as 13 characters, so two people on two branches never make the same one (decision 0020); also fast maps and sets keyed by them. | `Id`, `IdMap`, `IdSet`, `IdError` |
| `value.rs` | Plain values: what a component's fields hold. | `Value`, `AssetRef` |
| `schema.rs` | A component's name and fields, with their kinds, starting values and hints for the inspector. | `ComponentSchema`, `FieldInfo`, `FieldKind`, `Hint` |
| `component.rs` | A component's values, and typed access to them for components written in Rust. | `ComponentData`, `Component`, `Fields` |
| `registry.rs` | Every component the program knows, by name. | `Registry`, `RegistryError` |
| `checksum.rs` | One number summing up a game's state, the same on every computer, so two runs can be compared tick by tick (not for security). | `Checksum` |
| `upgrade.rs` | The one upgrade path every file format shares (Phase 8, C1): a format's version, how to find it in a file, and its upgraders, run one version at a time; newer files refused. | `Format`, `Step`, `Upgraded`, `UpgradeError`, `Versioned` |
| `ordinary_file.rs` | The one way a file is opened and read, for every crate: only an ordinary file (never a folder, a device or a pipe, which a project can ship a link to), and never more of it than asked. | `open`, `read_limited`, `read_start`, `Refused` |
| `data_files.rs` | The names of the Lust data files Solis makes (decision 0081): `farm.tilemap.lst`'s kind, and its name without it. | `KINDS`, `kind_of`, `stem`, `path_stem` |
| `test_support.rs` | What every crate's tests need, behind the `test-support` feature (turned on only by a dev-dependency): `TempFolder`, the one way a test makes a folder in the system's temporary folder, removed with everything in it when the test ends. | `TempFolder` |

## How the pieces fit

A component is a `ComponentData`: a name and its `Value`s. Its `ComponentSchema`, found in the `Registry` by that name, says what each field is, so the editor can draw it and files can store it without knowing the component. Rust components implement `Component` to turn into values and back.

**Read next:** the crate's [README](../README.md).
