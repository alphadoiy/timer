# Project Rules

## Scope
- Keep this project as a local-first terminal application.
- Prioritize a pure TUI experience over GUI/web alternatives.

## Product Direction
- Analog clock rendering must look realistic, not cartoon-like.
- No full-screen background color fills in the main dial view.
- Animations should be subtle, smooth, and functional.
- Keep mode switching animation under 700ms.

## CLI and UX
- Preserve these commands: `timer`, `timer clock`, `timer pomodoro`.
- Keep keybindings stable unless explicitly changed:
  - `Tab` / arrow keys: switch mode
  - `Space`: start/pause pomodoro (or continue after completed phase)
  - `r`: reset current phase
  - `q` / `Esc`: quit

## Code Quality
- Keep modules focused: `app`, `animation`, `render`, `modes`, `cli`, `theme`.
- Avoid hard-coding behavior in render code when it belongs to state/animation.
- Prefer deterministic logic for timer state transitions.
- Keep functions small and explicit; avoid hidden global state.

## Rust Skills Workflow
- Prefer using project-local Rust skills from `.codex/skills` when implementing code changes.
- Select the smallest relevant skill set before coding, and follow each selected skill's `SKILL.md`.
- Reuse skill guidance/templates/scripts instead of re-inventing patterns by hand.
- If multiple skills overlap, choose one primary skill and one secondary skill to avoid conflicting instructions.
- When a task does not match any installed skill, fall back to the existing project rules and note the gap.

## Testing
- Run `cargo fmt` and `cargo test` before finalizing changes.
- Add/maintain tests for:
  - pomodoro state transitions
  - animation boundary behavior
  - render text/snapshot sanity

## Performance
- Keep render loop responsive and flicker-free.
- Avoid unnecessary allocations in per-frame drawing paths.
- Preserve frame pacing around 15-24 FPS target.

## Change Management
- Keep this file updated when project constraints change.
- If a new rule conflicts with an existing rule, update this file in the same change.
- Before every `git commit`, append the intended commit summary to `.codex/commit-log.md`.
