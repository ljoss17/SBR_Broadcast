pub struct Message {
    content: String,
    from: u32,
    to: u32,
    timestamp: String,
}

impl Message {
    pub fn new() -> Message {
        Message {
            content: String::from(""),
            from: 0,
            to: 0,
            timestamp: String::from(""),
        }
    }
}
