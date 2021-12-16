use serde::{Deserialize, Serialize};
use talk::crypto::primitives::sign::Signature as SignSignature;

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

    pub fn get_type(self) -> u32 {
        self.message_type.clone()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedMessage {
    message: Message,
    signature: SignSignature,
}

impl SignedMessage {
    pub fn new(message: Message, signature: SignSignature) -> Self {
        SignedMessage { message, signature }
    }

    pub fn get_type(self) -> u32 {
        self.message.get_type()
    }

    pub fn get_message(self) -> Message {
        self.message
    }

    pub fn get_signature(self) -> SignSignature {
        self.signature
    }
}
