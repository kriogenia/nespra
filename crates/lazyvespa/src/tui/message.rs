use std::sync::Arc;

use crossterm::event::KeyEvent;

use super::{
    history_picker::HistoryMsg, palette::PaletteMsg, query_editor::EditorMsg, results::ResultsMsg,
    target_picker::PickerMsg,
};
use crate::core::{connection::ActiveTarget, search::SearchOutcome};

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
    /// Message of the query editor.
    Editor(EditorMsg),
    /// Message of the results view.
    Results(ResultsMsg),
    /// Move the focus to the next part of the screen: YQL, parameters, results.
    FocusNext,
    /// Open the list of targets.
    OpenTargetPicker,
    /// Message of the target picker.
    Picker(PickerMsg),
    /// Open the list of the queries run against the target.
    OpenHistory,
    /// Message of the history picker.
    History(HistoryMsg),
    /// Connect to the configured target with this name.
    SwitchTarget(String),
    /// Result of connecting to a target. Boxed as it is much bigger than other messages.
    TargetOpened(Result<Box<ActiveTarget>, String>),
    /// Run the query of the editor on the target.
    RunQuery,
    /// Result of a search.
    SearchDone {
        /// Identifier of the search.
        id: u64,
        /// Response, shared as it is not cloneable, or the error.
        result: Result<Arc<SearchOutcome>, String>,
    },
    /// The history could not be saved, with the reason.
    HistoryNotSaved(String),
    /// Close the open overlay.
    CloseOverlay,
    /// Close the application.
    Quit,
}
