# Architectural Refactoring Plan for lfm

*Generated from codebase review — target: Elm-style MVU with clean component boundaries*

---

## Overview

**lfm** is a terminal file manager built in Rust using:
- **ratatui** (TUI framework)
- **fff-search** (fast file indexing & search)
- **tui-view** (multi-format file viewer with syntax highlighting)
- **ratatui-image** (terminal graphics protocol for image previews)

Architecture: **Elm-style MVU (Model-View-Update)** — immutable `Model`, pure `update()`, side effects in `main` via `Effect` enum and `mpsc` channels.

---

## Current Strengths

| Area | What's Good |
|------|-------------|
| **MVU purity** | `update()` is a pure function `(Model, Message) → (Model, Effect)` — no I/O, no mutation leakage |
| **Effect system** | All side effects (spawn, editor, commands, transfers) return as `Effect` variants, executed in `main` |
| **Background work** | Transfers & search run on threads with `mpsc` progress/result channels — UI never blocks |
| **Shared search index** | One `FilePicker` index serves both content search & file find; pre-warmed on directory debounce |
| **Terminal graphics** | Queries terminal capabilities at startup; falls back gracefully when unsupported |
| **Test coverage** | Unit tests for presets, file masks, transfer logic, search, key handling, etc. |
| **Config persistence** | Pinned dirs saved to `~/.config/lfm/state.json` |
| **Preset commands** | Flexible template system (`{files}`, `{paths}`, `{input}`, `{cwd}`) with argv/shell modes |

---

## Key Architectural Concerns & Suggestions

### 1. God Model (`src/model.rs` — ~300 lines, 28 fields)

The `Model` struct holds *everything*: two file panels, pinned panel, viewer, search panels, command picker, capture view, transfer state, help, debug, error, progress, rename input, shift tracking, view registry, image picker.

**Problem:** Tight coupling — any change touches `Model`. Hard to test sub-components in isolation.

**Suggestion:** Decompose into **sub-models** with their own `update`/`view`:

```rust
// Each with its own Model, Message, update(), view()
struct FilePanelModel { ... }
struct SearchPanelModel { ... }  // generic over Result type
struct TransferModel { ... }
struct ViewerModel { ... }
struct CommandPickerModel { ... }
```

Then `Model` composes them:

```rust
struct Model {
    left: FilePanelModel,
    right: FilePanelModel,
    pinned: PinnedPanelModel,
    content_search: Option<SearchPanelModel<SearchResult>>,
    file_find: Option<SearchPanelModel<FileFindResult>>,
    viewer: Option<ViewerModel>,
    transfer: TransferModel,
    command_picker: Option<CommandPickerModel>,
    capture: Option<CaptureModel>,
    // cross-cutting: active_panel, help, error, shift_held, view_registry, picker
}
```

This enables **isolated unit tests** and clearer ownership.

---

### 2. Message Enum Explosion (`src/message.rs` — 80+ variants)

Every UI interaction is a distinct `Message` variant. This scales poorly:
- Adding a panel = 10+ new variants
- `update_message` is a 300-line `match` with `dispatch_to_panel` fallback

**Suggestion:** **Hierarchical messages** using enums per component:

```rust
enum Message {
    FilePanel(PanelId, FilePanelMsg),
    SearchPanel(PanelId, SearchPanelMsg),
    Transfer(TransferMsg),
    Viewer(ViewerMsg),
    Global(GlobalMsg),  // Quit, ToggleHelp, Tab, ShiftHeld, etc.
}
```

Each sub-model handles its own message type. `update` becomes:

```rust
fn update(model: Model, msg: Message) -> (Model, Effect) {
    match msg {
        Message::FilePanel(id, msg) => {
            let (panel, effect) = file_panel::update(model.panel(id), msg);
            (model.with_panel(id, panel), effect)
        }
        // ...
    }
}
```

This mirrors the sub-model decomposition and keeps `update` small.

---

### 3. Main Loop Complexity (`src/main.rs` — `run()` is ~200 lines)

The event loop mixes:
- Rendering
- Progress draining
- Search index scheduling/debouncing
- Image worker polling
- Keyboard input with timeout logic
- Effect execution

**Suggestion:** Extract **`EventLoop`** struct:

```rust
struct EventLoop {
    terminal: DefaultTerminal,
    model: Model,
    progress_rx: Option<Receiver<ProgressMsg>>,
    index: SearchIndexManager,
    image_worker: ImageWorkerHandle,
}

impl EventLoop {
    fn run(mut self) -> io::Result<PathBuf> { ... }
    fn drain_background_work(&mut self) -> BackgroundWorkResult { ... }
    fn handle_event(&mut self, event: Event) { ... }
    fn compute_timeout(&self) -> Option<Duration> { ... }
}
```

Benefits: `main.rs` becomes thin; logic is testable; easier to add async runtime later.

---

### 4. Search Index Manager in `main.rs` (`Index` struct)

The `Index` struct (debounce scheduling, engine reuse, staleness tracking) lives in `main.rs` but is **search infrastructure**, not app logic.

**Suggestion:** Move to `src/engine.rs` as `SearchIndexManager`:

```rust
pub struct SearchIndexManager {
    engine: Option<SearchEngine>,
    stale: bool,
    pending: Option<(PathBuf, Instant)>,
}

impl SearchIndexManager {
    pub fn schedule(&mut self, dir: &Path) { ... }
    pub fn tick(&mut self) { ... }
    pub fn get_or_build(&mut self, root: PathBuf) -> &mut SearchEngine { ... }
    pub fn mark_stale(&mut self) { self.stale = true; }
    pub fn debounce_remaining(&self) -> Option<Duration> { ... }
}
```

Then `main` just calls `index.tick()` and `index.get_or_build(root)`.

---

### 5. Transfer Logic in `main.rs` (thread spawning)

Each `Effect::StartCopy/Move/Delete` spawns a thread inline in the `match`. This couples the effect handling to `std::thread`.

**Suggestion:** **TransferService** abstraction:

```rust
struct TransferService {
    tx: Sender<ProgressMsg>,
    handle: JoinHandle<()>,
}

impl TransferService {
    fn start_copy(sources: Vec<PathBuf>, dst: PathBuf) -> Self { ... }
    fn start_move(...) -> Self { ... }
    fn try_recv_progress(&mut self) -> Option<ProgressMsg> { ... }
    fn is_finished(&self) -> bool { ... }
}
```

Model holds `Option<TransferService>`. Main loop calls `service.try_recv_progress()`. Enables:
- Async/await migration later
- Mock in tests
- Cancellation via `Drop`

---

### 6. Viewer & Image Handling

`FileView` holds `ViewContent::Text(ViewState)` or `ViewContent::Image(Box<ImageView>)`. `sync_file_view` in `update.rs` reloads on navigation.

**Concerns:**
- `ViewState` from `tui-view` is opaque — scroll state tied to view instance
- Image worker runs per-file; no caching when navigating back

**Suggestions:**
- **ViewerModel** with `current: Option<ViewEntry>`, `history: VecDeque<ViewEntry>` for back/forward
- **Image cache**: `LruCache<PathBuf, ImageView>` keyed by path + panel size
- Extract `FileViewer` component with its own `update/view`

---

### 7. Error Handling Pattern

Errors are stored as `Option<String>` in `Model` (`error_message`, `pending_overwrite.conflicts`). Rendered via `error_box`/`confirm_box`.

**Issues:**
- Single error slot — concurrent errors overwrite
- No error history / log
- Stringly-typed — no structured error types

**Suggestion:** `ErrorReport` enum with severity, context, recoverable actions:

```rust
enum ErrorReport {
    Io { path: PathBuf, op: IoOp, source: io::Error },
    SearchIndex { root: PathBuf, source: String },
    Command { label: String, source: String },
    Transfer { op: TransferOp, source: String, conflicts: Vec<PathBuf> },
}
```

UI can show inline toasts, history panel, or modal based on severity.

---

### 8. Key Handling & Input Modes

`keys.rs` normalizes events, tracks Shift via Kitty protocol, maps to `Message`. `input_mode(&model)` returns context for hint bar.

**Suggestion:** **InputMode** as explicit state in Model:

```rust
enum InputMode {
    Normal,
    Filter { panel: PanelId },
    Rename { mode: TransferMode },
    Goto { panel: PanelId },
    NewPath { panel: PanelId },
    SearchContent { focused_field: InputField },
    SearchFiles { focused_field: InputField },
    CommandPicker { stage: CommandStage },
    CaptureView,
    FileView { focused: bool },
    Help,
    PinnedPanel,
}
```

Then `update` routes by mode first — eliminates `active_panel` + `transfer_mode` + `rename_input.active` + `file_view_focused` combinatorial complexity.

---

### 9. Testing Gaps

- **Integration tests**: None for full MVU loop (model → effect → main → model)
- **Snapshot tests**: No `insta` tests for view rendering
- **Property tests**: File mask globbing, preset expansion, transfer counting
- **Headless TUI tests**: `ratatui` supports `TestBackend` — use it

**Suggestion:** Add `tests/integration.rs` with:

```rust
#[test]
fn copy_file_then_verify_destination() {
    let mut model = Model::init(tempdir);
    let (model, effect) = update(model, Message::SelectDown); // select file
    let (model, effect) = update(model, Message::StartCopy);
    // simulate right panel navigation, confirm
    // assert transfer effect spawned
}
```

---

### 10. Configuration & Extensibility

- Theme: hardcoded in `theme.rs` (colors only)
- Presets: `commands.json` — good
- No plugin/hook system

**Suggestions:**
- **Theme**: Load from `~/.config/lfm/theme.toml` (TOML → `Theme` struct)
- **Hooks**: `on_enter_dir`, `on_file_select`, `on_transfer_complete` → run presets/scripts
- **Keybindings**: Load from `keys.toml` → map to `Message` (currently hardcoded in `keys.rs`)

---

### 11. Performance Considerations

| Area | Current | Risk | Fix |
|------|---------|------|-----|
| Directory read | `read_dir` synchronous in `navigate_to` | Blocks UI on slow fs (network, large dirs) | Spawn `read_entries` on thread pool; show skeleton |
| Search index | Built on worker thread | Good | Keep; consider incremental update (fff-search `watch: true`) |
| File viewer | Loads entire file into memory (5MB cap) | Large files OOM | Stream for plain text; keep cap for syntax highlighting |
| Image decode | Per-view worker | Re-decodes on revisit | Cache decoded frames keyed by (path, panel_size) |

---

### 12. Dependency on `fff-search`

`fff-search` is the core search engine. It's a single-crate dependency from the same author.

**Risk:** If `fff-search` breaks or is unmaintained, search breaks.

**Mitigation:** Define a `SearchBackend` trait in `lfm`; implement for `fff-search`. Allows swap (e.g., `ripgrep` CLI fallback, `fd` for file find).

---

### 13. No Async Runtime

All background work uses `std::thread` + `mpsc`. This works but:
- Thread per transfer (OK for few, not for many)
- No cancellation beyond `abort` flag
- Can't easily compose multiple futures

**Future-proofing:** Consider `tokio` or `async-std` with `mpsc::unbounded_channel`:

```rust
async fn run_copy(sources: Vec<PathBuf>, dst: PathBuf, tx: Sender<ProgressMsg>) { ... }
```

Then `main` runs on `tokio::main` or a single-threaded executor.

---

## Suggested Refactoring Priority

| Priority | Task | Effort | Impact |
|----------|------|--------|--------|
| **1** | Extract `SearchIndexManager` to `engine.rs` | Low | Clean separation |
| **2** | Decompose `Model` → sub-models | Medium | Testability, maintainability |
| **3** | Hierarchical `Message` enum | Medium | Scales message handling |
| **4** | `EventLoop` struct in `main.rs` | Low | Readability, testability |
| **5** | `TransferService` abstraction | Medium | Async-ready, mockable |
| **6** | `ViewerModel` with history/cache | Medium | UX, performance |
| **7** | Structured error system | Low | Better UX, debugging |
| **8** | Explicit `InputMode` state | Medium | Simplifies update logic |
| **9** | Configurable theme/keys/hooks | Medium | Extensibility |
| **10** | Async runtime migration | High | Future scalability |

---

## Code Quality Notes

- **Clippy pedantic clean** — good discipline
- **Consistent naming** — `snake_case`, descriptive
- **Documentation** — inline comments explain *why*, not just *what*
- **No `unsafe`** — memory safe throughout
- **Feature-gated debug** — `#[cfg(feature = "debug")]` for logging/debug panel

---

## Summary

**lfm** is a well-architected MVU app with solid foundations. The main architectural debt is **centralization in `Model` and `Message`** — a natural consequence of organic growth. Decomposing into **component models with hierarchical messages** would pay dividends in testability, onboarding, and feature velocity.

The search/transfer/viewer pipelines are well-designed for background work. The effect system cleanly separates pure logic from I/O.