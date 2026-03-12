# Third-Party Notices

This project uses the following direct third-party Rust crates.
Exact resolved versions come from `Cargo.lock` / `cargo tree`.

## Direct Dependencies

| Crate | Version | License | Repository |
| --- | --- | --- | --- |
| `anyhow` | `1.0.102` | `MIT OR Apache-2.0` | <https://github.com/dtolnay/anyhow> |
| `chrono` | `0.4.44` | `MIT OR Apache-2.0` | <https://github.com/chronotope/chrono> |
| `clap` | `4.5.60` | `MIT OR Apache-2.0` | <https://github.com/clap-rs/clap> |
| `crossterm` | `0.28.1` | `MIT` | <https://github.com/crossterm-rs/crossterm> |
| `dirs` | `6.0.0` | `MIT OR Apache-2.0` | <https://github.com/soc/dirs-rs> |
| `figlet-rs` | `0.1.5` | `Apache-2.0` | <https://github.com/yuanbohan/rs-figlet> |
| `quick-xml` | `0.37.5` | `MIT` | <https://github.com/tafia/quick-xml> |
| `rand` | `0.10.0` | `MIT OR Apache-2.0` | <https://github.com/rust-random/rand> |
| `ratatui` | `0.29.0` | `MIT` | <https://github.com/ratatui/ratatui> |
| `reqwest` | `0.12.28` | `MIT OR Apache-2.0` | <https://github.com/seanmonstar/reqwest> |
| `rodio` | `0.17.3` | `MIT OR Apache-2.0` | <https://github.com/RustAudio/rodio> |
| `rustfft` | `6.4.1` | `MIT OR Apache-2.0` | <https://github.com/ejmahler/RustFFT> |
| `serde` | `1.0.228` | `MIT OR Apache-2.0` | <https://github.com/serde-rs/serde> |
| `serde_json` | `1.0.149` | `MIT OR Apache-2.0` | <https://github.com/serde-rs/json> |
| `symphonia` | `0.5.5` | `MPL-2.0` | <https://github.com/pdeljanov/Symphonia> |
| `sysinfo` | `0.37.2` | `MIT` | <https://github.com/GuillaumeGomez/sysinfo> |
| `toml` | `0.8.23` | `MIT OR Apache-2.0` | <https://github.com/toml-rs/toml> |
| `unicode-width` | `0.2.0` | `MIT OR Apache-2.0` | <https://github.com/unicode-rs/unicode-width> |
| `walkdir` | `2.5.0` | `Unlicense/MIT` | <https://github.com/BurntSushi/walkdir> |

## Notes

- This file lists direct dependencies only. Transitive dependencies are resolved through `Cargo.lock`.
- License data was taken from each crate's published `Cargo.toml` in the local Cargo registry cache.
- `symphonia` is licensed under `MPL-2.0`, which is a file-level copyleft license. Keep its upstream notices intact when redistributing binaries or source bundles that include it.
- Before making the repository public, choose and add a license for this project itself. Third-party dependency licenses do not automatically license your own code.
