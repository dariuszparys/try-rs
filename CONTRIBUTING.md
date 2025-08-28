# Contributing

Thank you for helping improve this project! Please read AGENTS.md for the full contributor guide.

- Start here: see `AGENTS.md` for project structure, commands, style, and PR requirements.
- Toolchain: install Rust via https://rustup.rs (stable). CI enforces `rustfmt` and `clippy`.
- Build/run quickly:
  - `cargo build` (or `cargo build --release`)
  - `cargo run -- [args]` (e.g., `cargo run -- cd foo`)
- Tests and linting:
  - `cargo test --all --locked`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --all`
- Branching: use short, descriptive names (e.g., `feat/fast-create`, `fix/tui-overflow`).
- Commits: concise, imperative subject lines (e.g., "Add fast-create for empty query"). Group related changes.
- Opening a PR:
  - Fill out the PR template, link issues, add before/after examples for TUI when relevant.
  - Ensure CI passes and docs (README/AGENTS.md) reflect user-facing changes.

Security & scope:
- The binary should only write under the configured tries directory (default `~/src/tries`).
- Document any filesystem-impacting changes and guard against touching paths outside the configured root.

For anything unclear or larger refactors, open an issue first to align on approach.

