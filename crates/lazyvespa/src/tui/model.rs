use crate::core::connection::ActiveTarget;

/// State of the whole application.
#[derive(Debug, Default)]
pub struct Model {
    running: RunningState,
    target: Option<ActiveTarget>,
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

    /// Stops the application.
    pub(super) fn quit(&mut self) {
        self.running = RunningState::Done;
    }
}
