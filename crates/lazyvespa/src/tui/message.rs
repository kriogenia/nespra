/// Something that happened and the application reacts to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Close the application.
    Quit,
}
