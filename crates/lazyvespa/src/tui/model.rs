/// State of the whole application.
#[derive(Debug, Default)]
pub struct Model {
    running: RunningState,
}

/// Whether the application keeps running.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum RunningState {
    #[default]
    Running,
    Done,
}

impl Model {
    /// Whether the application keeps running.
    pub fn is_running(&self) -> bool {
        self.running == RunningState::Running
    }

    /// Stops the application.
    pub(super) fn quit(&mut self) {
        self.running = RunningState::Done;
    }
}
