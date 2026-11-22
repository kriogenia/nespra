//! Terminal UI following The Elm Architecture.

mod effect;
mod event;
mod help;
mod keymap;
mod message;
mod model;
mod palette;
mod popup;
mod status_bar;
mod target_picker;
mod update;
mod view;

use std::io;

use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

pub use effect::Effect;
pub use message::Message;
pub use model::Model;
pub use update::update;
pub use view::view;

/// Runs the application until it is asked to quit.
///
/// # Errors
///
/// Fails if the terminal cannot be drawn or its events cannot be read.
pub async fn run(terminal: &mut DefaultTerminal, mut model: Model) -> io::Result<()> {
    let (sender, receiver) = mpsc::unbounded_channel();
    let effects = effect::EffectRunner::new(sender);
    let mut events = event::Events::new(receiver);
    while model.is_running() {
        terminal.draw(|frame| view(&model, frame))?;
        if let Some(message) = events.next().await? {
            update(&mut model, message)
                .into_iter()
                .for_each(|effect| effects.run(effect));
        }
    }
    Ok(())
}
