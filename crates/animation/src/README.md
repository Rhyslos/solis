# animation/src

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | The crate's map. Start here. | |
| `clip.rs` | A clip of frames: the frame at a time, the events between two times. Tests in `clip_tests.rs`. | `Clip`, `ClipFrame`, `Moment` |
| `frames_file.rs` | A sprite's clips file (`.solframes`): read within limits and written. Tests in `frames_file_tests.rs`. | `FramesFile`, `TagChange`, `OwnClip`, `read`, `write` |
| `animator_file.rs` | An animator (`.solanim`): parameters, states (a clip, or a clip per direction) and transitions with conditions; read within limits and written. Tests in `animator_file_tests.rs`. | `AnimatorFile`, `Parameter`, `State`, `Transition`, `Condition`, `Facing`, `read`, `write` |
| `animator.rs` | An animator made ready to run, and one entity's place in it, stepped once a tick. Tests in `animator_tests.rs`. | `Animator`, `Run`, `ParamValue` |
| `clip_set.rs` | A sheet's Aseprite tags and its clips file put together. Tests in `clip_set_tests.rs`. | `ClipSet` |
