# updater/src

Updates (decision 0043): finds the newest build of the program on GitHub, downloads its installer, checks it against its published SHA-256 checksum, and starts it. Only the program uses this crate; games never do.

**Start reading at** `check.rs`.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers. | |
| `check.rs` | Checking for updates and downloading one, each on a thread of its own: a check only asks; the download waits until the person says Update now (decision 0045). | `Status`, `Remote`, `start`, `start_download`, `should_offer` |
| `fetch.rs` | Talking to GitHub over HTTPS, checked by the system's own certificates: asking for the latest release and downloading its files. | `Source` |
| `release.rs` | What GitHub says about the newest build, read within limits, and the checksum file published beside it. | `Release`, `Asset`, `read_release`, `read_checksum`, `sha256` |
| `install.rs` | This copy of the program: its build number, whether the installer put it here, where updates are downloaded, and starting an installer. | `this_build`, `is_installed`, `start_installer` |
| `tests.rs` | Tests, including damaged release answers that must be read or refused, never crash. | |

**Read next:** the crate's [README](../README.md), and [`installer/`](../../../installer/README.md) for the installer it starts.
