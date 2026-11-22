use crossterm::event::KeyEvent;

/// Something that happened and the application reacts to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// A key was pressed, translated by the key bindings active in the current state.
    Key(KeyEvent),
    /// Open or close the help.
    ToggleHelp,
    /// Close the open overlay.
    CloseOverlay,
    /// Close the application.
    Quit,
}
