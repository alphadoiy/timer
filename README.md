# timer

`timer` is a Rust terminal dashboard that combines a clock, pomodoro timer, live weather scene, and multi-source music playback in one keyboard-driven TUI.

## Features

- Multi-mode dashboard with `Clock`, `Pomodoro`, and `Music` views
- Pixel-style weather scene rendered with Unicode half blocks
- Pomodoro workflow with start, pause, resume, and reset behavior
- Music playback from local files, folders, HTTP audio URLs, podcasts, playlists, and radio streams
- Vim-style command mode for queue and station management
- Live-stream aware playback UI that avoids fake duration and seek behavior

## Quick Start

```bash
cargo run
```

Start directly in music mode:

```bash
cargo run -- music <PATH_OR_URL>
```

Useful subcommands:

```bash
cargo run -- clock
cargo run -- pomodoro
cargo run -- music ~/Music
```

## Modes

### Clock

- Analog clock display
- System time readout inside the shared dashboard shell

### Pomodoro

- Startable and resettable focus timer
- Weather-driven animated road scene with day/night and precipitation changes

### Music

- Queue-based playback UI
- Volume, mute, shuffle, repeat, and fullscreen visualization
- Source statistics and queue overlays
- Command-line style control inside the TUI via `:`

## Supported Music Sources

- Local audio files
- Directory scans
- HTTP audio URLs
- Podcast RSS or Atom feeds
- `yt-dlp` compatible links
- `m3u` and `pls` radio playlists
- Saved custom stations from `~/.config/timer/radios.toml`

Running `:radio` also loads the built-in Code Radio station for quick live-stream testing.

## Controls

- `Tab` / left / right: switch modes
- `Space`: play or pause music, or start or pause pomodoro
- `n` / `p`: next or previous track
- `v` / `V`: toggle visualizer or fullscreen visualizer
- `Q` / `S`: open queue or source statistics
- `:`: enter command mode
- `q`: quit

## Music Commands

- `:add <url>`: append a source to the current queue
- `:load <url-or-path>`: replace the queue from a source
- `:radio`: load saved stations and the default Code Radio entry
- `:clear`: clear the queue
- `:vol <0-100>`: set volume
- `:seek <+secs/-secs>`: seek within seekable audio
- `:station add <name> <url>`: save a custom station
- `:station rm <name>`: remove a saved station
- `:station list`: list saved stations
- `:help`: show command help

## Open Source

This project is licensed under [MIT](/Users/alphadoiy/Coding/timer/LICENSE).

Third-party Rust crate attributions and license notes are listed in [THIRD_PARTY_NOTICES.md](/Users/alphadoiy/Coding/timer/THIRD_PARTY_NOTICES.md).
