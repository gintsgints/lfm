# AGENTS.md

## Developer commands

```bash
cargo build          # compile
cargo run            # compile and run
cargo test           # run all tests
cargo fmt            # format code
cargo clippy -- -D warnings -W clippy::pedantic   # lint (hard mode)
```

**Required order before commit:** `cargo fmt` → `cargo clippy` → `cargo test`

## Architecture

Elm-style **MVU** (Model-View-Update):
- All state in immutable `Model`
- `update()` is a pure function returning `(Model, Effect)`
- Side effects (I/O, threads, editor spawns) executed in `main`
- Background ops communicate via `mpsc` channels (`ProgressMsg`, `SearchMsg`)

Entry point: `src/main.rs`

## Requirements

- **Nerd Font required** — icons render as placeholder boxes without it
- Terminal must use a Nerd Font patched typeface (JetBrainsMono Nerd Font, etc.)

## State persistence

Pinned directories saved to `~/.config/lfm/state.json`

## Reference

- Full keybindings, features, and architecture in `README.md`
- Dev workflow documented in `CLAUDE.md`