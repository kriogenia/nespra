//! Terminal UI following The Elm Architecture.

mod event;
mod message;
mod model;
mod update;
mod view;

use std::io;

use ratatui::DefaultTerminal;

pub use message::Message;
pub use model::Model;
pub use update::update;
pub use view::view;

/// Runs the application until it is asked to quit.
///
/// # Errors
///
/// Fails if the terminal cannot be drawn or its events cannot be read.
pub fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut model = Model::default();
    while model.is_running() {
        terminal.draw(|frame| view(&model, frame))?;
        if let Some(message) = event::next()? {
            update(&mut model, message);
        }
    }
    Ok(())
}
