use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
    pub message_type: u32,
    pub content: String,
}

impl Message {
    pub fn new(message_type: u32, content: String) -> Self {
        Message {
            message_type,
            content,
        }
    }
}
