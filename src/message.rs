#[derive(Clone, Copy)]
pub enum Message {
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
}
