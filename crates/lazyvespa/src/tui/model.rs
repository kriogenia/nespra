use super::palette::Palette;
use crate::core::{config::Config, connection::ActiveTarget};

/// State of the whole application.
#[derive(Debug, Default)]
pub struct Model {
    running: RunningState,
    config: Config,
    target: Option<ActiveTarget>,
    overlay: Option<Overlay>,
    notice: Option<Notice>,
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

    /// Message for the user about the last action, like an error.
    pub fn notice(&self) -> Option<&Notice> {
        self.notice.as_ref()
    }

    /// Replaces the target the application is connected to.
    pub(super) fn set_target(&mut self, target: ActiveTarget) {
        self.target = Some(target);
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
