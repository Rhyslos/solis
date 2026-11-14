# user-guide/src

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Reads everything the guide is made from (`read`, with size limits) and builds it (`build`), listing every problem; the pages missing from the contents (`unlisted`); the name read from `branding.rs` (`constant`). Start here. | `Manual`, `Font`, `read`, `build`, `KEPT_COPY` |
| `main.rs` | The command: builds the guide and writes it to `docs/user-guide/index.html`, or to `--out <file>`. | |
| `contents.rs` | Reads `docs/manual/SUMMARY.md`: the welcome's words, then each `##` group and its pages. | `Contents`, `Group` |
| `blocks.rs` | The Markdown reader's first half: a page cut into headings, paragraphs, lists, tables, quotes and code. Only what the manual uses; anything else stays words. | `Block`, `parse` |
| `inline.rs` | The second half: bold, italics, code, links, keys (the convention in `docs/manual/_template.md`) and the little HTML the manual may use; everything else is made safe. | `render`, `Places` |
| `html.rs` | Blocks as the guide's HTML: headings one level down with their `#` links, boxes, steps, tables of keys; GitHub's heading slugs, so the manual's links work in both. | `Writer`, `slug`, `anchors` |
| `links.rs` | Where each link goes in the guide, and the problem when it goes nowhere. | `Reach`, `PageLinks`, `id` |
| `guide.rs` | The whole page: the welcome, the chapters with their sections and the way on from each, the contents list, filled into `assets/page.html`. | `write` |

Each file's tests are beside it (`*_tests.rs`); `lib_tests.rs` also feeds thousands of damaged pages to the build, none of which may crash it.

**Read next:** [`../tests`](../tests/README.md), the checks on the real manual.
