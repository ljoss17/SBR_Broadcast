use crossbeam::channel::Sender;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub enum MessageType {
    Text,
    Gossip,
    Echo,
    EchoSubscription,
    Ready,
    ReadySubscription,
    DeliverySubscription,
}

#[derive(Debug, Clone)]
/// Structure of a Message
pub struct Message {
    content: String,
    pub signature: u64,
    pub author: u32,
    pub from: u32,
    pub from_tx: Sender<Message>,
    timestamp: DateTime<Utc>,
    pub message_type: MessageType,
}

use chrono::{DateTime, Utc};

impl Hash for Message {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.content.hash(state);
        self.author.hash(state);
        self.timestamp.hash(state);
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

impl Message {
    /// Function to initialise a new Message.
    /// # Arguments
    ///
    /// * `content` - String content of the message.
    /// * `from` - Sender ID.
    /// * `timestamp` - Timestamp of the message.
    ///
    pub fn new(
        content: String,
        author: u32,
        from: u32,
        from_tx: Sender<Message>,
        timestamp: DateTime<Utc>,
        message_type: MessageType,
    ) -> Message {
        let mut msg: Message = Message {
            content,
            signature: 0,
            author,
            from,
            from_tx,
            timestamp,
            message_type,
        };
        msg.signature = calculate_hash(&msg);
        msg
    }

    /// Function used to verify the signature of a message.
    ///
    pub fn verify_murmur(&self) -> bool {
        self.signature.eq(&calculate_hash(self))
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

    pub fn create_forward_ready(self, id: u32) -> Message {
        let mut new_echo: Message = self.clone();
        new_echo.message_type = MessageType::Ready;
        new_echo.from = id;
        new_echo
    }
}
