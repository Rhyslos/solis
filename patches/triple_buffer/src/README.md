# triple_buffer/src

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Everything: `triple_buffer` makes a connected writer and reader; the writer's `write` hands over a value, and the reader's `update` and `read` pick up the newest one, neither waiting on the other. Tests at the bottom. | `triple_buffer`, `Input`, `Output` |

**Read next:** the stand-in's [README](../README.md).
