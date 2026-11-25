use std::sync::Arc;

use crossterm::event::KeyEvent;

use super::{
    history_picker::HistoryMsg, palette::PaletteMsg, query_editor::EditorMsg, results::ResultsMsg,
    saved_picker::SavedMsg, target_picker::PickerMsg,
};
use crate::core::{
    connection::ActiveTarget,
    health::{Deployment, HealthCheck},
    search::SearchOutcome,
};

/// Something that happened and the application reacts to.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key was pressed, translated by the key bindings active in the current state.
    Key(KeyEvent),
    /// Open or close the help.
    ToggleHelp,
    /// Open the command palette with the text already typed.
    OpenPalette(&'static str),
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
    /// Open the list of the saved queries.
    OpenSaved,
    /// Message of the saved queries picker.
    Saved(SavedMsg),
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
    /// Time to do the periodic work, like checking the health of the target.
    Tick,
    /// Result of a health check of the target.
    HealthChecked {
        /// Identifier of the check.
        id: u64,
        /// Health of the target, or why it could not be checked.
        result: Result<HealthCheck, String>,
    },
    /// Open the panel with information about the target: its health and deployment.
    OpenInfo,
    /// Result of a request of the status of the application of the target.
    DeploymentFetched {
        /// Identifier of the request.
        id: u64,
        /// Status of the application, or why it could not be requested.
        result: Result<Deployment, String>,
    },
    /// Save the query of the editor under the name.
    SaveQuery(String),
    /// Some data could not be saved in its file, with the reason.
    NotSaved(String),
    /// Close the open overlay.
    CloseOverlay,
    /// Close the application.
    Quit,
}
