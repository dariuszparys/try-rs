# Repository Guidelines

## Project Structure & Modules
- `src/`: Rust source for a single-binary CLI (`try`). Key modules: `cli.rs` (arg parsing), `tui.rs` (terminal UI), `selector.rs` (filtering), `score.rs` (ranking), `storage.rs` (filesystem ops), `util.rs` (helpers), `error.rs` (errors), `main.rs` (entry + tests).
- `Cargo.toml` / `Cargo.lock`: crate metadata and locked deps.
- `.github/workflows/ci.yml`: CI for check, test, fmt, clippy.
- No separate `tests/` directory; unit tests live alongside code (see `#[cfg(test)]` in `src/main.rs`).

## Build, Run, and Test
- Build debug: `cargo build`
- Build release: `cargo build --release` → `target/release/try`
- Run locally: `cargo run -- [args]` (e.g., `cargo run -- cd foo`)
- Install from workspace: `cargo install --path .`
- Test all: `cargo test --all --locked`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt --all`

## Coding Style & Conventions
- Language: Rust 2024 edition; format with `rustfmt` (CI enforces `cargo fmt --check`).
- Lint clean: fix all `clippy` warnings; treat warnings as errors in CI.
- Naming: modules/files `snake_case`; types/traits `CamelCase`; functions/vars `snake_case`; constants `SCREAMING_SNAKE_CASE`.
- Errors: use `thiserror` types at boundaries; prefer clear `Result<T, E>` returns.
- TUI output should degrade gracefully when not a TTY; avoid printing ANSI when piped.

## Testing Guidelines
- Framework: Rust built-in `#[test]` with helpers in modules; `tempfile` available for fs tests.
- Location: place unit tests in the same file under `#[cfg(test)]` modules.
- Naming: descriptive `test_*` functions that cover happy-path and edge cases (e.g., empty query, non‑TTY).
- Run locally and ensure CI targets pass: `cargo test`, `cargo clippy`, `cargo fmt`.

## Commit & Pull Requests
- Commits: concise, imperative subject line (e.g., "Add fast-create for empty query"); keep related changes together.
- PRs must:
  - Describe the change, rationale, and user impact (CLI flags, env vars like `TRY_PATH`).
  - Include before/after examples: commands (`cargo run -- cd foo`), screenshots/gifs for TUI when relevant.
  - Link issues (e.g., "Fixes #123").
  - Pass CI: check, test, fmt, clippy.

## Security & Configuration Tips
- Scope writes to the configured tries directory (default `~/src/tries`).
- Configure via env `TRY_PATH` or `try init /absolute/path`; document any behavior that touches the filesystem.
- No secrets or network required by the binary; `try clone` only prints a shell pipeline—git runs in the caller’s shell.

