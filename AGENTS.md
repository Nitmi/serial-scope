# AGENTS

## Project Snapshot

- App type: cross-platform serial debugging assistant with plotting
- Actual UI stack: `Rust 2021 + eframe/egui + egui_plot`
- Note: this repository is not using GPUI
- Primary targets: Windows and Linux desktop

## Module Responsibilities

- `src/main.rs`: app bootstrap, window setup, theme, font loading
- `src/app.rs`: top-level app state, event handling, serial-to-UI coordination, plot state
- `src/config.rs`: `config.toml` load/save and persisted settings schema
- `src/parser/`: line accumulation and text-to-plot parsing
- `src/serial/`: serial worker thread, port configuration, protocol helpers, message types
- `src/ui/top_bar.rs`: top toolbar and global controls
- `src/ui/receive_panel.rs`: receive log panel
- `src/ui/send_panel.rs`: send panel, quick commands, auto-send controls
- `src/ui/plot_panel.rs`: plot view, series controls, plot-side interactions

## Common Commands

- Run app: `cargo run`
- Run tests: `cargo test`
- Format check: `cargo fmt --check`
- Lint: `cargo clippy --all-targets --all-features`
- Build release: `cargo build --release`

## Working Rules

- Keep changes incremental and scoped to one subsystem per commit.
- Do not mix parser, layout, and unrelated cleanup in the same commit.
- Preserve Chinese UI copy unless the task explicitly asks for copy changes.
- Keep configuration changes backward compatible with existing `config.toml`.
- Prefer extending current modules over introducing new dependencies.
- Add tests for parser or state-machine changes whenever behavior changes.

## UI And UX Constraints

- Preserve the existing dark theme direction unless a task explicitly changes it.
- High-frequency serial controls should receive stronger visual priority than advanced settings.
- Plotting should favor resilience against mixed debug output without hiding raw receive logs.

## Validation Expectations

- For documentation-only changes, verify file accuracy against the current repo.
- For code changes, run at least `cargo test` before committing.
- Run `cargo fmt --check` before each commit.
- Run `cargo clippy --all-targets --all-features` when the change does not introduce unrelated pre-existing lint noise.
