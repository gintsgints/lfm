# Changelog

## v0.2.0

### New features

- **Nerd Font icons** — directory and file icons in the file list (requires a Nerd Font terminal font)
- **Sort cycling** (`s`) — cycle sort order per panel: name, date modified, extension, size; active sort shown in panel title
- **Zip / extract** (`z` / `u`) — zip selected or active items into a `.zip` archive; extract `.zip` and `.tar.gz` archives into a subdirectory
- **Open with default app** (`o`) — opens the hovered file or directory with the system default application (`open` on macOS, `xdg-open` on Linux, `start` on Windows)

## v0.1.0

### New features

- **Help panel** (`?`) — inline keybinding reference overlay
- **Mode-aware hint line** — context-sensitive key hints at the bottom of the screen
- **Open in `$EDITOR`** (`e`) — launch `$EDITOR` on the selected item
- **Copy mode** (`c`) — open a second panel to pick a copy destination; supports multi-item copy
- **Multi-item selection** (`J` / `K`, `Shift+↓` / `Shift+↑`) — mark multiple items before an operation
- **Live filter** (`/`) — narrow the file list; exit restores cursor position
- **Pinned directories** (`p`) — bookmark and instantly jump to frequently used directories
- **Create files and directories** (`n`) — full path support (e.g. `foo/bar/baz.txt`); append `/` for a directory
- **Delete with confirmation** (`d`) — removes files or directories after an explicit confirmation prompt
- **Session persistence** — active directory and pinned list saved to `~/.config/lfm/state.json` on quit
- **Shell wrapper** — `cd` to the active directory on exit via `LFM_CHOOSEDIR`
- **Catppuccin Mocha colour theme**
- **Vim-style navigation** (`h` / `j` / `k` / `l`)

### Other

- GitHub Actions CI (build + test on every push)
- GitHub Actions release workflow — produces binaries for Linux x86_64, macOS arm64, and Windows x86_64 on version tags
