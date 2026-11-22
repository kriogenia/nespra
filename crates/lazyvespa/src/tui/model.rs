use std::sync::Arc;

use super::{palette::Palette, query_editor::QueryEditor, target_picker::TargetPicker};
use crate::core::{config::Config, connection::ActiveTarget, search::SearchOutcome};

/// State of the whole application.
#[derive(Debug, Default)]
pub struct Model {
    running: RunningState,
    config: Config,
    target: Option<ActiveTarget>,
    overlay: Option<Overlay>,
    notice: Option<Notice>,
    editor: QueryEditor,
    search: SearchState,
    searches: u64,
    pane: Pane,
}

/// Part of the screen receiving the keys.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Pane {
    /// The query editor.
    #[default]
    Editor,
    /// The results of the search.
    Results,
}

/// State of the last search.
#[derive(Debug, Clone, Default)]
pub enum SearchState {
    /// No search was run.
    #[default]
    Idle,
    /// Waiting for the answer of the search with this identifier.
    Running(u64),
    /// The search succeeded.
    Done(Arc<SearchOutcome>),
    /// The search failed.
    Failed(String),
}

/// Message for the user about the last action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// Something went as expected.
    Info(String),
    /// Something failed.
    Error(String),
}

/// Window shown on top of the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    /// Key bindings.
    Help,
    /// Command palette.
    Palette(Palette),
    /// List of targets to connect to.
    TargetPicker(TargetPicker),
}

/// Whether the application keeps running.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum RunningState {
    #[default]
    Running,
    Done,
}

impl Model {
    /// Creates the model of an application with its configuration, connected to the
    /// target, if any.
    pub fn new(config: Config, target: Option<ActiveTarget>) -> Self {
        Model {
            config,
            target,
            ..Model::default()
        }
    }

    /// Configuration of the application.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Target the application is connected to.
    pub fn target(&self) -> Option<&ActiveTarget> {
        self.target.as_ref()
    }

    /// Whether the application keeps running.
    pub fn is_running(&self) -> bool {
        self.running == RunningState::Running
    }

    /// Overlay currently open.
    pub fn overlay(&self) -> Option<&Overlay> {
        self.overlay.as_ref()
    }

    /// Editor of the query.
    pub fn editor(&self) -> &QueryEditor {
        &self.editor
    }

    /// Editor of the query, to update it.
    pub(super) fn editor_mut(&mut self) -> &mut QueryEditor {
        &mut self.editor
    }

    /// Message for the user about the last action, like an error.
    pub fn notice(&self) -> Option<&Notice> {
        self.notice.as_ref()
    }

    /// Replaces the target the application is connected to, forgetting the last search.
    pub(super) fn set_target(&mut self, target: ActiveTarget) {
        self.target = Some(target);
        self.search = SearchState::Idle;
    }

    /// Part of the screen receiving the keys.
    pub fn pane(&self) -> Pane {
        self.pane
    }

    /// Moves the focus to a part of the screen.
    pub(super) fn focus_pane(&mut self, pane: Pane) {
        self.pane = pane;
    }

    /// State of the last search.
    pub fn search(&self) -> &SearchState {
        &self.search
    }

    /// Starts a new search, returning its identifier.
    pub(super) fn start_search(&mut self) -> u64 {
        self.searches += 1;
        self.search = SearchState::Running(self.searches);
        self.searches
    }

    /// Stores the result of the search if it is the one running, returning whether it was.
    pub(super) fn finish_search(
        &mut self,
        id: u64,
        result: Result<Arc<SearchOutcome>, String>,
    ) -> bool {
        if !matches!(self.search, SearchState::Running(running) if running == id) {
            return false;
        }
        self.search = match result {
            Ok(outcome) => SearchState::Done(outcome),
            Err(error) => SearchState::Failed(error),
        };
        true
    }

    /// Opens the help, or closes it if it is already open.
    pub(super) fn toggle_help(&mut self) {
        self.overlay = match self.overlay {
            Some(Overlay::Help) => None,
            _ => Some(Overlay::Help),
        };
    }

    /// Opens an empty command palette.
    pub(super) fn open_palette(&mut self) {
        self.overlay = Some(Overlay::Palette(Palette::default()));
    }

    /// Opens the list of the configured targets.
    pub(super) fn open_target_picker(&mut self) {
        let names = self.config.targets.keys().cloned().collect();
        let current = self.target.as_ref().map(|active| active.name.as_str());
        self.overlay = Some(Overlay::TargetPicker(TargetPicker::new(names, current)));
    }

    /// Target picker, if it is open.
    pub(super) fn target_picker_mut(&mut self) -> Option<&mut TargetPicker> {
        match &mut self.overlay {
            Some(Overlay::TargetPicker(picker)) => Some(picker),
            _ => None,
        }
    }

    /// Command palette, if it is open.
    pub(super) fn palette_mut(&mut self) -> Option<&mut Palette> {
        match &mut self.overlay {
            Some(Overlay::Palette(palette)) => Some(palette),
            _ => None,
        }
    }

    /// Closes the open overlay.
    pub(super) fn close_overlay(&mut self) {
        self.overlay = None;
    }

    /// Shows a message to the user, replacing the previous one.
    pub(super) fn set_notice(&mut self, notice: Notice) {
        self.notice = Some(notice);
    }

    /// Hides the message shown to the user.
    pub(super) fn clear_notice(&mut self) {
        self.notice = None;
    }

    /// Stops the application.
    pub(super) fn quit(&mut self) {
        self.running = RunningState::Done;
    }
}
