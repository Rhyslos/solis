# sound/tests

Test files for the `sound` crate. The tests themselves are in [`../src/tests.rs`](../src/README.md).

| Folder | What it holds |
| --- | --- |
| `data/` | Small sound files the tests read: good Ogg Vorbis files (`silence_22k.ogg`, `tone_mono_44k.ogg`, `tone_stereo_48k.ogg`, `tone_six_channels.ogg`), two damaged ones fuzzing found (`huge_codebook.ogg`, `huge_comment_count.ogg`), whose headers claim far more than they hold, `chained_huge_codebook.ogg`, a good sound followed by the damaged one (a chained Ogg, refused), and `chained_after_stray_byte.ogg`, a good sound, one stray byte, then a second stream whose setup header holds a codebook with no dimensions (refused for the stray byte; rebuilt from the weekly fuzzer's crash, issue #156, where a second stream got past the check that way). `data/` has no README of its own, because the tests read its files as data. |
