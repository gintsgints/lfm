use std::{io, path::PathBuf};

use crate::file_find::FileFindResult;
use crate::presets::Preset;
use crate::search::SearchResult;
use crate::state::PersistedState;
use crate::ui::{file_panel, input_box, pinned_panel};

/// State of a query popup backed by a background search engine: the content
/// search (`ContentSearch`) and the file find (`FileFind`) differ only in what
/// a result is.
pub struct ResultPanel<T> {
    pub root: PathBuf,
    pub query: input_box::Model,
    pub results: Vec<T>,
    pub selection: usize,
    pub done: bool,
    /// The index for `root` is still being built; a query typed now runs as
    /// soon as it is ready.
    pub indexing: bool,
    pub input_focused: bool,
}

impl<T> ResultPanel<T> {
    pub fn new(root: PathBuf) -> Self {
        let mut query = input_box::Model::new();
        query.open();
        Self {
            root,
            query,
            results: Vec::new(),
            selection: 0,
            done: true,
            indexing: false,
            input_focused: true,
        }
    }
}

pub type ContentSearch = ResultPanel<SearchResult>;
pub type FileFind = ResultPanel<FileFindResult>;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TransferOp {
    Copy,
    Move,
    Delete,
}

pub struct TransferProgress {
    pub op: TransferOp,
    pub current: u64,
    pub total: u64,
}

/// A copy or move that has been confirmed but not yet started, described by the
/// exact sources and destination so it can be launched as-is once any overwrite
/// prompt is resolved.
pub enum PendingKind {
    Copy(Vec<PathBuf>, PathBuf),
    Move(Vec<PathBuf>, PathBuf),
    CopyRename(PathBuf, PathBuf),
    MoveRename(PathBuf, PathBuf),
}

impl PendingKind {
    pub fn op(&self) -> TransferOp {
        match self {
            PendingKind::Copy(..) | PendingKind::CopyRename(..) => TransferOp::Copy,
            PendingKind::Move(..) | PendingKind::MoveRename(..) => TransferOp::Move,
        }
    }

    /// Names of destination entries that already exist and would be overwritten.
    /// A source whose destination path is itself (copy/move onto the same
    /// location) is not reported as a conflict.
    pub fn conflicts(&self) -> Vec<String> {
        match self {
            PendingKind::Copy(sources, dst_dir) | PendingKind::Move(sources, dst_dir) => sources
                .iter()
                .filter_map(|src| {
                    let name = src.file_name()?;
                    let target = dst_dir.join(name);
                    (target != *src && target.exists()).then(|| name.to_string_lossy().into_owned())
                })
                .collect(),
            PendingKind::CopyRename(src, dst) | PendingKind::MoveRename(src, dst) => {
                if dst != src && dst.exists() {
                    vec![
                        dst.file_name()
                            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                    ]
                } else {
                    Vec::new()
                }
            }
        }
    }
}

/// A transfer awaiting the user's answer to an overwrite prompt.
pub struct PendingOverwrite {
    pub kind: PendingKind,
    pub conflicts: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum TransferMode {
    #[default]
    None,
    Copy,
    CopyRename,
    Move,
    MoveRename,
    Rename,
}

impl TransferMode {
    pub fn is_copy(self) -> bool {
        matches!(self, Self::Copy | Self::CopyRename)
    }

    pub fn is_move(self) -> bool {
        matches!(self, Self::Move | Self::MoveRename)
    }

    pub fn with_rename(self) -> bool {
        matches!(self, Self::CopyRename | Self::MoveRename)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActivePanel {
    LeftFiles,
    RightFiles,
    Pinned,
}

impl ActivePanel {
    pub fn next(self) -> Self {
        match self {
            ActivePanel::LeftFiles => ActivePanel::RightFiles,
            ActivePanel::RightFiles => ActivePanel::LeftFiles,
            ActivePanel::Pinned => ActivePanel::Pinned,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ActivePanel::LeftFiles => ActivePanel::RightFiles,
            ActivePanel::RightFiles => ActivePanel::LeftFiles,
            ActivePanel::Pinned => ActivePanel::Pinned,
        }
    }
}

/// Open command picker. While `input` is `Some`, the picker has moved on to
/// the second step (collecting `{input}`) for the highlighted preset.
pub struct CommandPicker {
    pub presets: Vec<Preset>,
    pub selection: usize,
    pub input: Option<input_box::Model>,
}

impl CommandPicker {
    pub fn new(presets: Vec<Preset>) -> Self {
        Self {
            presets,
            selection: 0,
            input: None,
        }
    }
}

/// Text contents of a file, displayed in a scrollable read-only viewer.
pub struct FileView {
    pub name: String,
    pub content: String,
    pub scroll: u16,
}

/// Output of a `capture`-mode preset, displayed in a scrollable popup.
pub struct CapturePopup {
    pub label: String,
    /// `None` means the command failed to spawn (binary not found, etc.).
    pub exit_code: Option<i32>,
    pub output: String,
    pub scroll: u16,
}

pub struct Model {
    pub active_panel: ActivePanel,
    /// The file panel that was active when the pinned panel was opened.
    pub origin_panel: ActivePanel,
    pub left_files: file_panel::Model,
    pub right_files: file_panel::Model,
    pub pinned_panel: pinned_panel::Model,
    pub transfer_mode: TransferMode,
    pub rename_input: input_box::Model,
    pub show_help: bool,
    /// Index of the highlighted line in the help panel; drives its scrolling.
    pub help_selection: usize,
    /// Whether Shift is currently held. Tracked only when the terminal reports
    /// key release events (Kitty protocol), so the bottom hint bar can show the
    /// shifted command set while Shift is down. Stays `false` otherwise.
    pub shift_held: bool,
    #[cfg(feature = "debug")]
    pub show_debug: bool,
    pub progress: Option<TransferProgress>,
    /// A confirmed copy/move held back while the user answers an overwrite prompt.
    pub pending_overwrite: Option<PendingOverwrite>,
    /// Name to select in the left panel after the next `progress_done`.
    pub pending_select: Option<String>,
    pub error_message: Option<String>,
    pub content_search: Option<ContentSearch>,
    pub file_find: Option<FileFind>,
    pub command_picker: Option<CommandPicker>,
    pub capture_popup: Option<CapturePopup>,
    pub file_view: Option<FileView>,
}

impl Model {
    pub fn init(persisted: PersistedState) -> io::Result<Self> {
        let cwd = std::env::current_dir()?;
        Ok(Self {
            active_panel: ActivePanel::LeftFiles,
            origin_panel: ActivePanel::LeftFiles,
            left_files: file_panel::Model::init(cwd.clone())?,
            right_files: file_panel::Model::init(cwd)?,
            pinned_panel: pinned_panel::Model::with_pins(persisted.pins),
            transfer_mode: TransferMode::None,
            rename_input: input_box::Model::new(),
            show_help: false,
            help_selection: 0,
            shift_held: false,
            #[cfg(feature = "debug")]
            show_debug: true,
            progress: None,
            pending_overwrite: None,
            pending_select: None,
            error_message: None,
            content_search: None,
            file_find: None,
            command_picker: None,
            capture_popup: None,
            file_view: None,
        })
    }

    pub fn to_persisted(&self) -> PersistedState {
        PersistedState {
            pins: self.pinned_panel.pins.clone(),
        }
    }
}
