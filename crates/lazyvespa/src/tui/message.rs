use crossterm::event::KeyEvent;

use super::palette::PaletteMsg;
use crate::core::connection::ActiveTarget;

/// Something that happened and the application reacts to.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key was pressed, translated by the key bindings active in the current state.
    Key(KeyEvent),
    /// Open or close the help.
    ToggleHelp,
    /// Open the command palette.
    OpenPalette,
    /// Message of the command palette.
    Palette(PaletteMsg),
    /// Connect to the configured target with this name.
    SwitchTarget(String),
    /// Result of connecting to a target. Boxed as it is much bigger than other messages.
    TargetOpened(Result<Box<ActiveTarget>, String>),
    /// Close the open overlay.
    CloseOverlay,
    /// Close the application.
    Quit,
}
