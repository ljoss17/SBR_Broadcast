use crate::msg_def::Message;
use crate::murmur;

pub struct Processor {
    id: u32,
    gossip: Vec<u32>,
    msg_recv: Message,
}

impl Processor {
    pub fn new(id: u32) -> Processor {
        Processor {
            id: id,
            gossip: Vec::new(),
            msg_recv: Message::new(),
        }
    }

    pub fn init_gossip(&mut self, g: u32, system: Vec<u32>) {
        self.gossip = murmur::init(self.id, g, system);
    }
}
