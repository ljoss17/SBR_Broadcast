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
    pub fn new(content: String, signature: String, from: u32, timestamp: DateTime<Utc>) -> Message {
        Message {
            content: content,
            signature: signature,
            from: from,
            timestamp: timestamp,
        }
    }

    pub fn verify_murmur(&self) -> bool {
        self.signature
            .eq(format!("{}{}", self.from, self.content).as_str())
    }

    pub fn print_message_info(self) {
        println!(
            "Got message : {}, from : {}, time: {}",
            self.content, self.from, self.timestamp
        );
    }
}
