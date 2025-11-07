# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-11-07

### Added
- Initial release of try-cli
- Interactive fuzzy finder for navigating temporary project directories
- Time-sensitive scoring that boosts recently created or visited directories
- Instant directory creation with date-prefixed naming (YYYY-MM-DD-<name>)
- Git clone integration with `try clone <url>` command
- One-key deletion with Ctrl-D and explicit YES confirmation
- Shell integration for bash, zsh, and fish
- One-line installation script for Linux/macOS
- Support for x86_64, aarch64, and armv7 architectures
- Release automation and GitHub Actions workflow
- Configuration for publishing to crates.io
- Contribution guidelines and issue/PR templates
- Comprehensive documentation in README.md

### Fixed
- Binary path extraction in installation script
- Clippy warnings for format! shorthand and named fields

[Unreleased]: https://github.com/dariuszparys/try-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/dariuszparys/try-rs/releases/tag/v0.1.0
