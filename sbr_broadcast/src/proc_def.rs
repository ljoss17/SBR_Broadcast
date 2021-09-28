use crate::msg_def::Message;

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;

/// Structure of a Processor
pub struct Processor {
    pub id: u32,
    pub gossip: HashMap<u32, Sender<Message>>,
    msg_recv: Option<Message>,
    pub tx: Arc<Mutex<Sender<Message>>>,
    rx: Arc<Mutex<Receiver<Message>>>,
}

impl Processor {
    /// Initialise a new Processor with a given ID and open a channel for message passing :
    /// # Arguments
    ///
    /// * `id` - A unique ID.
    pub fn new(id: u32) -> Processor {
        let (tx, rx) = channel();
        Processor {
            id: id,
            gossip: HashMap::new(),
            msg_recv: None,
            tx: Arc::new(Mutex::new(tx)),
            rx: Arc::new(Mutex::new(rx)),
        }
    }

    pub fn get_sender(&self) -> Sender<Message> {
        self.tx.lock().unwrap().clone()
    }

    pub fn add_gossip_peer(&mut self, id: u32, proc_tx: Sender<Message>) {
        self.gossip.insert(id, proc_tx);
    }

    pub fn broadcast_murmur(&mut self, content: String) {
        if self.msg_recv.is_none() {
            let sig: String = format!("{}{}", self.id, content);
            let timestamp: DateTime<Utc> = Utc::now();
            self.msg_recv = Some(Message::new(
                content.clone(),
                sig.clone(),
                self.id,
                self.id,
                timestamp,
            ));
            for (id, tx) in &self.gossip.clone() {
                let msg: Message =
                    Message::new(content.clone(), sig.clone(), self.id, *id, timestamp);
                self.forward_murmur(msg, tx.clone(), *id)
            }
        }
    }

    fn forward_murmur(&mut self, msg: Message, tx: Sender<Message>, id: u32) {
        println!("Send from {} to {}", self.id, id);
        tx.send(msg).unwrap();
    }

    fn dispatch_murmur(&mut self, msg: Message) {
        if self.msg_recv.is_none() {
            self.msg_recv = Some(msg.clone());
            for (id, tx) in &self.gossip.clone() {
                self.forward_murmur(msg.clone(), tx.clone(), *id)
            }
        }
    }

    fn deliver_murmur(&mut self, msg: Message) {
        if msg.verify_murmur() {
            self.dispatch_murmur(msg);
        }
    }

    pub fn listen(self) {
        loop {
            let msg: Message = self.rx.lock().unwrap().recv().unwrap();
            println!("Processor {} received :", self.id);
            msg.print_message_info();
        }
    }
}
