#[derive(Debug, Clone)]
/// Structure of a Message
pub struct Message {
    content: String,
    signature: String,
    from: u32,
    timestamp: DateTime<Utc>,
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
    pub fn new(content: String, signature: String, from: u32, timestamp: DateTime<Utc>) -> Message {
        Message {
            content: content,
            signature: signature,
            from: from,
            timestamp: timestamp,
        }
    }

    /// Function used to verify the signature of a message.
    ///
    pub fn verify_murmur(&self) -> bool {
        self.signature
            .eq(format!("{}{}", self.from, self.content).as_str())
    }

    /// Function used to print information of the message.
    ///
    pub fn print_message_info(self) {
        println!(
            "Got message : {}, from : {}, time: {}",
            self.content, self.from, self.timestamp
        );
    }
}
