use crate::core::connection::ActiveTarget;

/// State of the whole application.
#[derive(Debug, Default)]
pub struct Model {
    running: RunningState,
    target: Option<ActiveTarget>,
    overlay: Option<Overlay>,
}

/// Window shown on top of the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// Key bindings.
    Help,
}

/// Whether the application keeps running.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum RunningState {
    #[default]
    Running,
    Done,
}

impl Model {
    /// Creates the model of an application connected to the target, if any.
    pub fn new(target: Option<ActiveTarget>) -> Self {
        Model {
            target,
            ..Model::default()
        }
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
    pub fn overlay(&self) -> Option<Overlay> {
        self.overlay
    }

    /// Opens the overlay, or closes it if it is already open.
    pub(super) fn toggle_overlay(&mut self, overlay: Overlay) {
        self.overlay = (self.overlay != Some(overlay)).then_some(overlay);
    }

    /// Closes the open overlay.
    pub(super) fn close_overlay(&mut self) {
        self.overlay = None;
    }

    /// Stops the application.
    pub(super) fn quit(&mut self) {
        self.running = RunningState::Done;
    }
}
