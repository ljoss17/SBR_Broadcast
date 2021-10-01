use crossbeam::channel::Sender;

#[derive(Debug, Clone)]
pub enum MessageType {
    Text,
    Gossip,
    Echo,
    EchoSubscription,
}

#[derive(Debug, Clone)]
/// Structure of a Message
pub struct Message {
    content: String,
    signature: String,
    pub author: u32,
    pub from: u32,
    pub from_tx: Sender<Message>,
    timestamp: DateTime<Utc>,
    pub message_type: MessageType,
}

use chrono::{DateTime, Utc};

impl Message {
    /// Function to initialise a new Message.
    /// # Arguments
    ///
    /// * `content` - String content of the message.
    /// * `signature` - Signature of the message.
    /// * `from` - Sender ID.
    /// * `timestamp` - Timestamp of the message.
    ///
    pub fn new(
        content: String,
        signature: String,
        author: u32,
        from: u32,
        from_tx: Sender<Message>,
        timestamp: DateTime<Utc>,
        message_type: MessageType,
    ) -> Message {
        Message {
            content,
            signature,
            author,
            from,
            from_tx,
            timestamp,
            message_type,
        }
    }

    /// Function used to verify the signature of a message.
    ///
    pub fn verify_murmur(&self) -> bool {
        self.signature
            .eq(format!("{}{}", self.from, self.content).as_str())
    }

    pub fn create_forward_message(self, id: u32) -> Message {
        let mut new_message: Message = self.clone();
        new_message.from = id;
        new_message
    }

    pub fn create_forward_echo(self, id: u32) -> Message {
        let mut new_echo: Message = self.clone();
        new_echo.message_type = MessageType::Echo;
        new_echo.from = id;
        new_echo
    }
}
