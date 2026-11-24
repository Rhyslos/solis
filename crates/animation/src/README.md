# animation/src

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | The crate's map. Start here. | |
| `clip.rs` | A clip of frames: the frame at a time, the events between two times. Tests in `clip_tests.rs`. | `Clip`, `ClipFrame`, `Moment` |
| `clips_file.rs` | A picture's clips file (`.clips.lst`, Lust, Fixes 3): the grid that cuts the picture into numbered cells, cells drawn by hand, looks, clips with one, four or eight directions and their rows. Read within limits and written. Tests in `clips_file_tests.rs`. | `ClipsFile`, `Clip`, `Way`, `Facing`, `read`, `write` |
| `frames_file.rs` | The clips file before version 2 (`.solframes`): read within limits (`clips_file::read` turns it into this version's) and written, for its tests. Tests in `frames_file_tests.rs`. | `FramesFile`, `TagChange`, `OwnClip`, `read`, `write` |
| `animator_file.rs` | An animator (`.solanim`): parameters, states (a clip, or a clip per direction) and transitions with conditions; read within limits and written. Tests in `animator_file_tests.rs`. | `AnimatorFile`, `Parameter`, `State`, `Transition`, `Condition`, `Facing`, `read`, `write` |
| `animator.rs` | An animator made ready to run, and one entity's place in it, stepped once a tick. Tests in `animator_tests.rs`. | `Animator`, `Run`, `ParamValue` |
| `clip_set.rs` | A sheet's Aseprite tags and its clips file put together, with each clip's rows by facing (`ClipSet::facing` gives the row a sprite facing a way plays, and whether it's flipped). Tests in `clip_set_tests.rs`. | `ClipSet` |
| `atlas.rs` | Atlas crops (decision 0089): named boxes cut out of a picture, kept in a Lust data file beside it (`tools.png` gives `tools.atlas.lst`; decision 0081), never changing the picture; read within limits and written. Tests in `atlas_tests.rs`. | `Atlas`, `Crop`, `read`, `write`, `file_for` |
