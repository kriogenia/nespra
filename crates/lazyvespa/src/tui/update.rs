use super::{Message, Model};

/// Applies a message to the model. It is the only place where the state changes.
pub fn update(model: &mut Model, message: Message) {
    match message {
        Message::Quit => model.quit(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_quit() {
        let mut model = Model::default();
        assert!(model.is_running());
        update(&mut model, Message::Quit);
        assert!(!model.is_running());
    }
}
