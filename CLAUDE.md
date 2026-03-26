# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build          # Compile
cargo run            # Compile and run
cargo test           # Run all tests
cargo test <name>    # Run a single test by name
cargo clippy         # Lint
cargo fmt            # Format code
cargo check          # Fast type/syntax check without producing a binary
```

## Project Intent

**lfm** is a TUI (Terminal User Interface) file manager — "Lazy File Manager" — built in Rust. Inspired by two-panel file managers (e.g. Midnight Commander), it aims to be fast, keyboard-driven, and extensible via plugins.

### Core UI Panels

- **Path list** — breadcrumb/directory tree navigation
- **File list** — active directory contents (supports virtual filesystems via plugins)
- **Command prompt** — inline command input at the bottom of the screen

### Core Features

- Copy and move files/directories with intuitive keybindings
- Create files and directories
- Plugin system — plugins can provide virtual filesystem views, commands, or UI extensions

### Built-in Plugins

- **zip** — browse ZIP archive contents directly in the file list panel (treat the archive as a directory)

## Architecture

This is a Rust binary crate (`lfm`). Currently early-stage with a single entry point at [src/main.rs](src/main.rs).
