/// A text field somewhere in the UI. Every one of them is an
/// `input_box::Model`, so they all answer the same `EditOp`s; the field only
/// says which one the edit lands in.
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// The filter bar above a file panel.
    Filter,
    /// The "create file or directory" dialog.
    NewPath,
    /// The "jump to path" dialog.
    GotoPath,
    /// The rename dialog, shared by rename-in-place and copy/move-with-rename.
    Rename,
    /// The `{input}` step of a preset command.
    CommandInput,
    /// The content-search popup's query row, which holds both the query and
    /// the file mask; the panel tracks which of the two has the cursor.
    SearchQuery,
    /// The file-find popup's query row, same two fields.
    FindQuery,
}

/// One editing keystroke, independent of which field receives it.
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditOp {
    Char(char),
    Backspace,
    CursorLeft,
    CursorRight,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Clone, Copy)]
pub enum Message {
    /// One editing keystroke in one text field. `keys.rs` picks the field from
    /// the current input mode, so every field shares this single variant.
    Edit(Field, EditOp),
    Quit,
    NextPanel,
    PrevPanel,
    SelectUp,
    SelectDown,
    DirUp,
    DirEnter,
    MarkSelectUp,
    MarkSelectDown,
    ClearSelection,
    TogglePinnedPanel,
    PinCurrentDir,
    SelectPinnedDir,
    DeletePinnedDir,
    EnterFilter,
    ConfirmFilter,
    ExitFilter,
    FilterBarDown,
    NewPath,
    NewPathConfirm,
    NewPathCancel,
    DeleteFiles,
    DeleteConfirm,
    DeleteCancel,
    StartCopy,
    ConfirmCopy,
    CancelCopy,
    StartMove,
    ConfirmMove,
    CancelMove,
    StartCopyRename,
    StartMoveRename,
    RenameInPlace,
    ConfirmRename,
    CancelRename,
    ToggleHelp,
    HelpScrollUp,
    HelpScrollDown,
    OpenEditor,
    OpenDefault,
    CycleSort,
    ZipFiles,
    UnzipFile,
    GotoPath,
    GotoPathConfirm,
    GotoPathCancel,
    ProgressTick {
        current: u64,
        total: u64,
    },
    ProgressDone,
    OverwriteConfirm,
    OverwriteCancel,
    DismissError,
    ContentSearch,
    ContentSearchToggleFocus,
    ContentSearchCancel,
    ContentSearchUp,
    ContentSearchDown,
    ContentSearchConfirm,
    FileFind,
    FileFindToggleFocus,
    FileFindCancel,
    FileFindUp,
    FileFindDown,
    FileFindConfirm,
    /// Shift was pressed (`true`) or released (`false`); updates the hint bar.
    SetShiftHeld(bool),
    OpenCommandPicker,
    CommandPickerUp,
    CommandPickerDown,
    CommandPickerConfirm,
    CommandPickerCancel,
    CommandInputConfirm,
    CommandInputCancel,
    CaptureViewScrollUp,
    CaptureViewScrollDown,
    CaptureViewPageUp,
    CaptureViewPageDown,
    CaptureViewClose,
    ViewFile,
    FileViewScrollUp,
    FileViewScrollDown,
    FileViewPageUp,
    FileViewPageDown,
    FileViewClose,
    #[cfg(feature = "debug")]
    ToggleDebug,
}

impl Message {
    /// Whether handling this message writes to the filesystem, which makes a
    /// built search index stale. Copies, moves, deletes and renames run in the
    /// background instead and are noticed when their progress finishes.
    pub fn mutates_filesystem(self) -> bool {
        matches!(
            self,
            Self::NewPathConfirm | Self::ZipFiles | Self::UnzipFile
        )
    }
}
