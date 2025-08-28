# try

Lightweight, time‑sensitive directory navigation for experiments — a fast way to jump between temporary project folders.

Inspired by and adapted from Tobias Lütke’s original Ruby tool: https://github.com/tobi/try



## Quick Start

- Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Install the CLI locally: `cargo install --path .`
- Enable shell integration (bash/zsh): `echo 'eval "$(try init)"' >> ~/.zshrc && source ~/.zshrc`
  - Use `~/.bashrc` on bash; for fish: `try init | source` in `~/.config/fish/config.fish`.
- Launch: run `try`, type to search, press Enter to jump.

## Why `try`?

When experimenting you often create throwaway directories (e.g., `2025-08-21-test-feature-x`, `tmp-viz-poc`). Finding and jumping back to them slows you down. `try` gives you an interactive, fuzzy selector that favors what you touched most recently, and creates a new directory when your query doesn’t exist — all wired to actually `cd` your shell.

## Features

- Interactive fuzzy finder: fast, incremental filtering of your tries.
- Time‑sensitive scoring: boosts recently created or visited dirs.
- Instant creation: press Enter to create when no exact match.
- Git clone integration: `try clone <url>` or pass a git URL to `try` to clone into a date‑prefixed dir.
- One‑key deletion: Ctrl‑D, with an explicit “YES” confirmation.
- Shell integration: prints `cd` commands your shell evaluates.
- Native speed: single‑binary CLI written in Rust.

## Installation

Prerequisites: a working Rust toolchain from https://rustup.rs

- Install from this repo: `cargo install --path .`
- Or build a release binary: `cargo build --release` → `target/release/try`

## Easy Setup

Wire `try` into your shell so it can change directories.

- bash/zsh (recommended):

  ```sh
  # add to ~/.bashrc or ~/.zshrc
  eval "$(try init)"
  # then reload your shell
  source ~/.bashrc  # or: source ~/.zshrc
  ```

- fish:

  ```fish
  # add to ~/.config/fish/config.fish
  eval "$(try init | string collect)"
  ```

Customize the storage location (default: `~/src/tries`) either by passing an absolute path to `init` or by setting `TRY_PATH`. You can also override per‑invocation with the global `--path` option:

```sh
eval "$(try init /absolute/path/to/tries)"
# or
export TRY_PATH=/absolute/path/to/tries
eval "$(try init)"
# or override at call time
try --path /absolute/path/to/tries
```

## Usage

Basic:

```sh
# Open the selector (with shell function installed)
try              # Open the selector
try my-experiment  # Seed the query (shell function calls `try cd ...`)
try cd my-experiment  # Same as above without the shell function

# Clone a git repo into a date-prefixed directory and cd into it
try clone https://github.com/user/repo.git
try clone git@github.com:user/repo my-fork   # custom name

# Shorthand: passing a git URL to `try` behaves like `try clone`
try https://github.com/user/repo
```

Inside the selector:

- Up/Down or Ctrl‑P/Ctrl‑N: move selection
- Type: filter entries
- Enter: select existing or create `YYYY-MM-DD-<query>` and cd
- Ctrl‑D: delete the selected directory (requires typing `YES` to confirm)
- Esc/Ctrl‑C: cancel and return to the shell

Notes:

- If there’s no matching directory, Enter creates one (prefixed by `YYYY-MM-DD-`) and jumps into it.
- Ranking combines fuzzy score with recency to surface likely targets.
- Query terms that start with a hyphen must be placed after `--` so they aren’t parsed as flags, for example: `try cd -- --foo --bar`. With the shell function installed, use: `try -- --foo`.

### Deletion semantics

- Ctrl‑D prompts for confirmation; type `YES` to permanently delete the selected directory.
- File count and size are displayed before confirmation.
- Operations are restricted to the configured tries root; entries outside are never touched.

## CLI Reference

- `try` (with no args): open the selector.
- `try --help`: show top‑level help (lists subcommands and global options).
- `try init [--path PATH] [PATH]`: print the shell function; add it to your rc file.
- `try cd [QUERY...] [--path PATH]`: launch selector and print the `cd`/mkdir/touch commands (used by the shell function).
- `try clone <git-uri> [name] [--path PATH]`: print a clone pipeline (mkdir -p, git clone, touch, cd) into the tries directory.
- Shorthand: `try <git-uri>` behaves like `try clone <git-uri>`.
- Subcommand help: `try cd --help`, `try init --help`, `try clone --help`.

## Configuration

- Default tries directory: `~/src/tries`
- Override via `TRY_PATH` env var or an absolute path argument to `try init`

## Troubleshooting

- `command not found: try`: ensure `~/.cargo/bin` is on your `PATH` or reference the binary directly, e.g. `eval "$(~/.cargo/bin/try init)"`.
- Selector opens but no `cd` happens: confirm your rc file sources the `init` function and that you restarted/reloaded the shell.
- Wrong tries location: check `echo $TRY_PATH` or the path passed to `init`.

## Development

- Build: `cargo build` (or `cargo build --release`)
- Run: `cargo run -- [args]` (e.g., `cargo run -- cd foo`)
- Test: `cargo test --all --locked`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt --all`

Architecture: single‑binary CLI using `crossterm` for TUI, `dirs` for home paths, and `unicode-width` for display width.

### Colors

- Help and error output from `clap` uses its built‑in color logic (color: auto) and respects standard environment conventions.
- `try`’s own warnings and errors are styled on stderr when appropriate and degrade to plain text when not:
  - Colors enabled only if stderr is a TTY.
  - `NO_COLOR` disables colors.
  - `CLICOLOR=0` disables; `CLICOLOR_FORCE!=0` forces enable.
- When output is piped or redirected, styling is disabled to avoid ANSI sequences in logs.

### Error Handling

- Internally, the app uses a small, typed error (`thiserror`) at high‑level boundaries and returns plain `io::Error` for low‑level file operations.
- The selector and CLI keep a lenient UX: non‑critical issues print a warning and continue; critical issues surface clearly and set a non‑zero exit where appropriate.

## Security & Behavior

- `try` writes only under the configured tries directory (default `~/src/tries`).
- No network access or secrets required.

## License

MIT — see [LICENSE](LICENSE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for workflow and PR expectations, and
[AGENTS.md](AGENTS.md) for structure, commands, style, and testing guidelines.
