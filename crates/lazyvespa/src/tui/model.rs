use std::{path::PathBuf, sync::Arc};

use nespra::admin::Admin;

use super::{
    history_picker::HistoryPicker,
    palette::Palette,
    query_editor::QueryEditor,
    results::{ResultsMsg, ResultsView},
    saved_picker::SavedPicker,
    target_picker::TargetPicker,
};
use crate::core::{
    config::Config,
    connection::ActiveTarget,
    health::HealthCheck,
    history::{Histories, History},
    query::QueryDraft,
    saved::SavedQueries,
    search::SearchOutcome,
    store::StoreError,
};

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
    results: ResultsView,
    pane: Pane,
    histories: Stored<Histories>,
    saved: Stored<SavedQueries>,
    health: HealthMonitor,
}

/// Health checks of the target.
#[derive(Debug, Default)]
struct HealthMonitor {
    /// Number of checks started, used as the identifier of the last one.
    checks: u64,
    /// Identifier of the check waiting for an answer, if any.
    pending: Option<u64>,
    /// Result of the last check of the target, `None` before the first answer.
    last: Option<Result<HealthCheck, String>>,
}

/// Data kept in a file between runs.
#[derive(Debug, Default)]
struct Stored<T> {
    value: T,
    /// Where the data is saved, `None` when it is not saved.
    path: Option<PathBuf>,
}

impl<T: Clone> Stored<T> {
    /// The data and where to save it, if it is saved.
    fn to_save(&self) -> Option<(PathBuf, T)> {
        Some((self.path.clone()?, self.value.clone()))
    }
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
    /// List of the queries run against the target.
    HistoryPicker(HistoryPicker),
    /// List of the saved queries.
    SavedPicker(SavedPicker),
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

    /// Uses the histories loaded from the file, saving the new queries in it.
    ///
    /// When they could not be loaded, the history starts empty and is not saved, so the
    /// file is not overwritten, and the error is shown to the user.
    pub fn with_history(mut self, path: PathBuf, loaded: Result<Histories, StoreError>) -> Self {
        self.histories = self.stored("history", path, loaded);
        self
    }

    /// Uses the saved queries loaded from the file, saving the changes in it.
    ///
    /// When they could not be loaded, there are no saved queries and the changes are not
    /// saved, so the file is not overwritten, and the error is shown to the user.
    pub fn with_saved_queries(
        mut self,
        path: PathBuf,
        loaded: Result<SavedQueries, StoreError>,
    ) -> Self {
        self.saved = self.stored("saved queries", path, loaded);
        self
    }

    /// Data loaded from the file, saved in it. When it could not be loaded, it is empty
    /// and not saved, and the error is shown to the user.
    fn stored<T: Default>(
        &mut self,
        name: &str,
        path: PathBuf,
        loaded: Result<T, StoreError>,
    ) -> Stored<T> {
        match loaded {
            Ok(value) => Stored {
                value,
                path: Some(path),
            },
            Err(error) => {
                self.notice = Some(Notice::Error(format!(
                    "{error}. The {name} will not be saved."
                )));
                Stored::default()
            }
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
        self.results = ResultsView::default();
        // The answer of a pending check is about the previous target, so it is ignored
        self.health.pending = None;
        self.health.last = None;
    }

    /// Result of the last health check of the target, `None` before the first answer.
    pub fn health(&self) -> Option<&Result<HealthCheck, String>> {
        self.health.last.as_ref()
    }

    /// Starts a health check of the target, returning its identifier and the client to
    /// run it. Nothing is started without a target or while a check is waiting for its
    /// answer, so a slow target is not flooded with checks.
    pub(super) fn start_health_check(&mut self) -> Option<(u64, Admin)> {
        let admin = self.target.as_ref()?.connection.admin.clone();
        if self.health.pending.is_some() {
            return None;
        }
        self.health.checks += 1;
        self.health.pending = Some(self.health.checks);
        Some((self.health.checks, admin))
    }

    /// Stores the result of the health check if it is the pending one.
    pub(super) fn finish_health_check(&mut self, id: u64, result: Result<HealthCheck, String>) {
        if self.health.pending == Some(id) {
            self.health.pending = None;
            self.health.last = Some(result);
        }
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

    /// View of the results.
    pub fn results(&self) -> &ResultsView {
        &self.results
    }

    /// Applies a message to the view of the results of the last search.
    pub(super) fn update_results(&mut self, message: ResultsMsg) {
        let outcome = match &self.search {
            SearchState::Done(outcome) => Some(outcome.as_ref()),
            _ => None,
        };
        self.results.update(message, outcome);
    }

    /// Starts a new search, returning its identifier.
    pub(super) fn start_search(&mut self) -> u64 {
        self.searches += 1;
        self.search = SearchState::Running(self.searches);
        self.results = ResultsView::default();
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

    /// History of the queries run against the current target.
    pub fn history(&self) -> &History {
        let name = self
            .target
            .as_ref()
            .map_or("", |active| active.name.as_str());
        self.histories.value.target(name)
    }

    /// Records a query run against the current target, returning the histories to save
    /// and where, if they are saved.
    pub(super) fn record_query(&mut self, query: QueryDraft) -> Option<(PathBuf, Histories)> {
        let active = self.target.as_ref()?;
        self.histories.value.record(&active.name, query);
        self.histories.to_save()
    }

    /// Queries saved by name.
    pub fn saved_queries(&self) -> &SavedQueries {
        &self.saved.value
    }

    /// Saves a query under the name, returning whether it replaced another one, and the
    /// saved queries to save and where, if they are saved.
    pub(super) fn save_query(
        &mut self,
        name: String,
        query: QueryDraft,
    ) -> (bool, Option<(PathBuf, SavedQueries)>) {
        let replaced = self.saved.value.insert(name, query).is_some();
        (replaced, self.saved.to_save())
    }

    /// Deletes the saved query with the name, returning the saved queries to save and
    /// where, if they are saved.
    pub(super) fn delete_saved_query(&mut self, name: &str) -> Option<(PathBuf, SavedQueries)> {
        self.saved.value.remove(name)?;
        self.saved.to_save()
    }

    /// Opens the help, or closes it if it is already open.
    pub(super) fn toggle_help(&mut self) {
        self.overlay = match self.overlay {
            Some(Overlay::Help) => None,
            _ => Some(Overlay::Help),
        };
    }

    /// Opens the command palette with the text already typed.
    pub(super) fn open_palette(&mut self, input: &str) {
        self.overlay = Some(Overlay::Palette(Palette::with_input(input)));
    }

    /// Opens the list of the configured targets.
    pub(super) fn open_target_picker(&mut self) {
        let names = self.config.targets.keys().cloned().collect();
        let current = self.target.as_ref().map(|active| active.name.as_str());
        self.overlay = Some(Overlay::TargetPicker(TargetPicker::new(names, current)));
    }

    /// Opens the list of the queries run against the target.
    pub(super) fn open_history_picker(&mut self) {
        self.overlay = Some(Overlay::HistoryPicker(HistoryPicker::new(self.history())));
    }

    /// History picker, if it is open.
    pub(super) fn history_picker_mut(&mut self) -> Option<&mut HistoryPicker> {
        match &mut self.overlay {
            Some(Overlay::HistoryPicker(picker)) => Some(picker),
            _ => None,
        }
    }

    /// Opens the list of the saved queries.
    pub(super) fn open_saved_picker(&mut self) {
        self.overlay = Some(Overlay::SavedPicker(SavedPicker::new(&self.saved.value)));
    }

    /// Saved queries picker, if it is open.
    pub(super) fn saved_picker_mut(&mut self) -> Option<&mut SavedPicker> {
        match &mut self.overlay {
            Some(Overlay::SavedPicker(picker)) => Some(picker),
            _ => None,
        }
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
