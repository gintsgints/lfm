# lfm — Lazy File Manager

A fast, keyboard-driven TUI file manager built in Rust, inspired by two-panel file managers like Midnight Commander.

## Features

- Single-panel file browser with vim-style navigation
- Copy and move files/directories — open a second panel to pick a destination
- Multi-item selection with shift-select
- Live filter (`/`) to narrow directory listings
- Pinned directories for quick access
- Navigate to any path instantly with `g` (supports `~` expansion)
- Create files and directories with full path support (`test/a/b.txt`)
- Delete files and directories with confirmation
- Open items in `$EDITOR` or with the default application
- Sort by name, date modified, extension, or size
- Zip selected items; extract `.zip` and `.tar.gz` archives
- Recursive content search (`S`) with live streaming results
- Fuzzy-find files by name (`F`)
- User-defined preset commands (`x`) — run any shell command on the selection, with optional `{input}` prompt
- Error popup for failed file operations
- Nerd Font icons in the file list
- Catppuccin Mocha colour theme
- Persists pinned directories across sessions
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
| `l` / `→` / `Enter` | Enter directory |
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
| `r` | Rename current item |
| `g` | Go to path (supports `~`) |
| `n` | Create file or directory (end path with `/` for directory) |
| `d` | Delete selected or current item (with confirmation) |
| `c` | Copy selected or current item — opens destination panel, `C` with rename before |
| `m` | Move selected or current item — opens destination panel, `M` with rename before |
| `v` | View file contents in a scrollable read-only popup (text files only) |
| `e` | Open selected item in `$EDITOR` |
| `o` | Open with default application |
| `x` | Run a preset command on the selection (see [Preset commands](#preset-commands)) |
| `s` | Cycle sort order: name → date → ext → size |
| `z` | Zip selected or current item(s) |
| `u` | Extract `.zip` or `.tar.gz` archive |
| `S` | Search file contents recursively in current directory |
| `F` | Fuzzy-find files by name in current directory |

### Filter

| Key | Action |
|-----|--------|
| `/` | Enter filter mode |
| `↓` / `Enter` / `Tab` | Lock filter and move to file list |
| `Esc` | Clear filter |

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

### Move mode

| Key | Action |
|-----|--------|
| `m` | Start move — right panel opens at current directory |
| `h/l/j/k` | Navigate destination panel |
| `Enter` | Confirm move into selected directory (or current dir) |
| `Esc` | Cancel move |

### Content search

| Key | Action |
|-----|--------|
| `S` | Open content search overlay |
| `Tab` | Switch focus between query input and results list |
| `j` / `↓` | Move down in results |
| `k` / `↑` | Move up in results |
| `Enter` | Navigate to the selected file |
| `Esc` | Cancel search |

### Other

| Key | Action |
|-----|--------|
| `?` | Show keybinding help |
| `q` | Quit and cd to active directory |

## Preset commands

Press `x` in a file panel to open the **preset picker** — a list of user-defined commands loaded from `~/.config/lfm/commands.json`. On first run, lfm writes a starter file with a few examples.

### Picker keys

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Run the selected preset (prompts for `{input}` first if the template needs it) |
| `Esc` | Close picker / back out of input step |

### Config format

Each entry in `commands.json` is a JSON object:

```json
{
  "presets": [
    { "label": "file (show type)",   "command": ["file", "{paths}"],          "output": "capture" },
    { "label": "wc (count lines)",   "command": ["wc", "-l", "{paths}"],      "output": "capture" },
    { "label": "grep for {input}",   "command": "grep -rn {input} {files}",   "output": "capture" },
    { "label": "open in VS Code",    "command": ["code", "{paths}"] }
  ]
}
```

- **`label`** — shown in the picker.
- **`command`** — an **array** runs the program directly (no shell, no quoting pitfalls); a **string** runs via `sh -c`, so pipes, redirects, and `&&` work.
- **`output`** *(optional)* — `background` (default), `block`, or `capture`. See below.

### Placeholders

Templates may reference any of the following. Both `{files}` and `{paths}` expand to **N entries** (one per selected file) when used as a whole argv element; embedding them inside a larger string (e.g. `--out={files}.bak`) requires a single selection.

| Placeholder | Expands to |
|-------------|------------|
| `{files}` | File names as shown in the panel (relative to the panel's cwd) |
| `{paths}` | Absolute paths |
| `{cwd}` | The active panel's current directory (absolute) |
| `{input}` | A single value the picker prompts for (only when this placeholder is present) |

In `sh -c` mode every expanded value is single-quoted, so filenames with spaces or quotes stay safe.

### Selection model

Commands act on **marked files if any, otherwise the file under the cursor** — the same rule as copy, move, and delete. A command that references `{files}` or `{paths}` but has nothing selected (e.g. in an empty directory) raises an error and doesn't run.

The command's working directory is the active panel's current directory; both panels are refreshed when the command finishes.

### Output modes

| Mode | Behaviour |
|------|-----------|
| `background` *(default)* | Spawn and forget. Output is discarded. Good for GUI launches (`code`, `mpv`, `xdg-open`). |
| `block` | Suspend the TUI, run the command attached to the terminal, then wait for `Enter` to return. Good for interactive tools (`git log`, `htop`, anything piped to `less`). |
| `capture` | Run hidden, collect `stdout` + `stderr`, show the merged buffer in a scrollable popup with `exit N` in the header. Good for quick info commands (`file`, `wc`, `grep`). |

In the capture popup: `j`/`k` scroll a line, `PgUp`/`PgDn` page, `Esc` or `Enter` close.

## Session persistence

On quit, lfm saves the pinned directory list to `~/.config/lfm/state.json`.

## Development

```bash
cargo build          # compile
cargo run            # compile and run
cargo test           # run tests
cargo fmt            # format code
cargo clippy -- -D warnings -W clippy::pedantic   # lint (hard mode)
```

### Debugging

An in-app debug log is available behind the `debug` cargo feature (off by default):

```bash
cargo run --features debug
```

With the feature enabled, press `` ` `` (backtick) to toggle a debug panel at the bottom of the screen. It shows the most recent internal log messages, including:

- The `Message` dispatched for each keypress
- Directory reads
- Content search timings — each search logs a summary line on completion, e.g. `search "." for "foo": 12 hit(s) in 340 file(s), 18.421ms`

Emit your own log lines from anywhere in the code with the `debug_log!` macro:

```rust
debug_log!("value = {value:?}");
```

Built with [ratatui](https://github.com/ratatui/ratatui).

## Architecture

lfm follows an **Elm-style MVU** (Model-View-Update) pattern. All state lives in an immutable `Model`; user input produces `Message` values; `update` is a pure function that returns a new `Model` plus an optional `Effect`; side effects (I/O, spawning threads) are executed in `main`.

### Data flow

```
keyboard event
      │
      ▼
 to_message()          ← input mode intercept (Filter / NewPath / Copy / …)
      │ Message
      ▼
   update()            ← pure; returns (Model, Effect)
      │
  ┌───┴──────────────────────────────────┐
  │ Model                                │ Effect
  ▼                                      ▼
view()                          spawn thread / open editor /
(ratatui render)                write state / quit
```

Background file transfers run in a dedicated OS thread and send `ProgressMsg` values over an `mpsc` channel. The main loop drains this channel each iteration and fires `Message::ProgressTick` / `Message::ProgressDone` into `update` so the progress bar stays live without blocking input.

Content search works the same way: a background thread walks the directory tree and sends `SearchMsg::Hit` results over a channel. Dropping the receiver cancels the thread. Results stream into the UI on each loop iteration with no keypress required.
