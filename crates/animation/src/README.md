# animation/src

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | The crate's map. Start here. | |
| `clip.rs` | A clip of frames: the frame at a time, the events between two times. Tests in `clip_tests.rs`. | `Clip`, `ClipFrame`, `Moment` |
| `frames_file.rs` | A sprite's clips file (`.solframes`): read within limits and written. Tests in `frames_file_tests.rs`. | `FramesFile`, `TagChange`, `OwnClip`, `read`, `write` |
| `clip_set.rs` | A sheet's Aseprite tags and its clips file put together. Tests in `clip_set_tests.rs`. | `ClipSet` |
