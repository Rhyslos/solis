# sound/tests

Test files for the `sound` crate. The tests themselves are in [`../src/tests.rs`](../src/README.md).

| Folder | What it holds |
| --- | --- |
| `data/` | Small sound files the tests read: good Ogg Vorbis files (`silence_22k.ogg`, `tone_mono_44k.ogg`, `tone_stereo_48k.ogg`, `tone_six_channels.ogg`) and two damaged ones fuzzing found (`huge_codebook.ogg`, `huge_comment_count.ogg`), whose headers claim far more than they hold. `data/` has no README of its own, because the tests read its files as data. |
