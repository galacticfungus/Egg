use std::fmt;

pub enum MessageType {
    Debug,
    User,
    Both,
}

pub struct ErrorContext {
    messages: Vec<(String, MessageType)>,
}

impl ErrorContext {
    pub(crate) fn add_message(&mut self, message: String, message_type: MessageType) {
        self.messages.push((message, message_type));
    }

    pub(crate) fn new() -> ErrorContext {
        ErrorContext {
            messages: Vec::with_capacity(3),
        }
    }
}

impl std::fmt::Display for ErrorContext {
    // Display only prints context messages that are assigned MessageType::User or MessageType::Both
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for line in self.messages.iter().rev().filter(|line| match line.1 {
            MessageType::Debug => false,
            _ => true,
        }) {
            writeln!(f, "{}", line.0).unwrap();
        }
        Ok(())
    }
}

impl std::fmt::Debug for ErrorContext {
    // Debug only prints context messages that are assigned MessageType::Debug or MessageType::Both
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for line in self.messages.iter().rev().filter(|line| match line.1 {
            MessageType::User => false,
            _ => true,
        }) {
            writeln!(f, "{}", line.0).unwrap();
        }
        Ok(())
    }
}
