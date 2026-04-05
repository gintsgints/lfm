# lfm — Lazy File Manager

A fast, keyboard-driven TUI file manager built in Rust, inspired by two-panel file managers like Midnight Commander.

## Features

- Single-panel file browser with vim-style navigation
- Copy mode — open a second panel to pick a copy destination
- Multi-item selection with shift-select
- Live filter (`/`) to narrow directory listings
- Pinned directories for quick access
- Create files and directories with full path support (`test/a/b.txt`)
- Delete files and directories with confirmation
- Open items in `$EDITOR`
- Catppuccin Mocha colour theme
- Persists current directory and pinned list across sessions
- Exits to the active directory via a shell wrapper

## Requirements

lfm uses [Nerd Font](https://www.nerdfonts.com/) icons in the file list. Your terminal must use a Nerd Font patched typeface, otherwise icons render as placeholder boxes.

**Install a Nerd Font:**

- **macOS (Homebrew):**
  ```bash
  brew install --cask font-jetbrains-mono-nerd-font
  ```
  Then set your terminal font to *JetBrainsMono Nerd Font* (or whichever you installed).

- **Linux:**  
  Download a font from [nerdfonts.com/font-downloads](https://www.nerdfonts.com/font-downloads), unzip into `~/.local/share/fonts/`, then run `fc-cache -fv`.

- **Windows:**  
  Download and install from [nerdfonts.com/font-downloads](https://www.nerdfonts.com/font-downloads), then select the font in your terminal emulator settings.

## Installation

```bash
cargo build --release
# copy target/release/lfm somewhere on your $PATH
```

## Shell integration (cd on exit)

Add to `~/.zshrc` (or `~/.bashrc`):

```zsh
lfm() {
  local tmp
  tmp=$(mktemp)
  LFM_CHOOSEDIR="$tmp" command lfm "$@"
  local dir
  dir=$(cat "$tmp")
  rm -f "$tmp"
  [[ -n "$dir" && -d "$dir" ]] && cd "$dir"
}
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` | Go to parent directory |
| `l` / `→` | Enter directory |
| `Tab` | Next panel |
| `Shift+Tab` | Previous panel |

### Selection

| Key | Action |
|-----|--------|
| `J` / `Shift+↓` | Mark item and move down |
| `K` / `Shift+↑` | Mark item and move up |
| `Esc` | Clear selection |

### File operations

| Key | Action |
|-----|--------|
| `n` | Create file or directory (end path with `/` for directory) |
| `d` | Delete selected or current item (with confirmation) |
| `c` | Copy selected or current item — opens destination panel |
| `e` | Open selected item in `$EDITOR` |

### Filter

| Key | Action |
|-----|--------|
| `/` | Enter filter mode |
| `Enter` / `Esc` | Exit filter, restore path and selection |

### Pinned directories

| Key | Action |
|-----|--------|
| `p` | Open pinned panel |
| `p` (in panel) | Pin current or selected directory |
| `Enter` / `Space` | Navigate to pinned directory |
| `d` (in panel) | Delete pinned directory |
| `Esc` | Close pinned panel |

### Copy mode

| Key | Action |
|-----|--------|
| `c` | Start copy — right panel opens at current directory |
| `h/l/j/k` | Navigate destination panel |
| `Enter` | Confirm copy into selected directory (or current dir) |
| `Esc` | Cancel copy |

### Other

| Key | Action |
|-----|--------|
| `?` | Show keybinding help |
| `q` | Quit and cd to active directory |

## Session persistence

On quit, lfm saves the active directory and pinned list to `~/.config/lfm/state.json`.

## Development

```bash
cargo build          # compile
cargo run            # compile and run
cargo test           # run tests
cargo fmt            # format code
cargo clippy -- -D warnings -W clippy::pedantic   # lint (hard mode)
```

Built with [ratatui](https://github.com/ratatui/ratatui).
