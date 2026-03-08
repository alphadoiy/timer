# Commit Log

## 2026-03-08

### Commit (pending)
- Initialized Rust CLI project structure (`timer`, `clock`, `pomodoro`) with `ratatui` + `crossterm`.
- Added full-screen TUI app loop, mode switching, and keyboard controls (`Tab/Arrows`, `Space`, `r`, `q`/`Esc`).
- Implemented pomodoro state machine (25/5/15), phase transitions, pause/resume/reset, and tests.
- Built braille-based analog dial renderer with sub-pixel plotting and animated hands.
- Added FIGlet digital time overlay and DevOps-style status readout (CPU/memory).
- Added animation system for transitions/celebration and frame pacing improvements.
- Added project rules and project-local skill installation under `.codex`.

### Commit (pending)
- Refactored pomodoro/clock rendering into dedicated files and introduced a `weathr`-based animation module under `src/render/weathr`.
- Replaced ad-hoc pomodoro scene logic with reusable weather systems (clouds/rain/snow/fog/stars/moon/birds/airplanes/leaves/fireflies/thunderstorm/chimney/house).
- Added fixed-step simulation (`dt`) plus render pipeline layering (`sky/mid/ground/fx`) for stable animation speed across machines.
- Added viewport-aware quality degradation gates to disable heavy effects in smaller terminal regions.
- Switched from dynamic weather intensity mixing to discrete weather presets (`Sunny/Rainy/Snowy/Foggy/Stormy`) for cleaner scene semantics.
- Added weather readout panel on the right-side `Readout` view to mirror the active pomodoro weather preset.
- Fixed strict lint failures (`clippy -D warnings`) and kept test suite green after integration.
