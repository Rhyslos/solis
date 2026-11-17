# level/tests

Tests that use the `level` crate from outside, as another crate would.

| File | What it does |
| --- | --- |
| `merge.rs` | Checks level files merge in Git (decisions 0006 and 0020): each case makes two changes to one level on two "branches" and merges them with Git's own three-way merge, which must succeed and keep both changes. Skipped where Git isn't installed. |

The crate's other tests sit beside its code in [`../src`](../src/README.md).
