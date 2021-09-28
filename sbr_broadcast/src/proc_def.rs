use crate::msg_def::Message;

use chrono::{DateTime, Utc};
use crossbeam::channel::unbounded;
use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;
use std::collections::HashMap;

use std::fs::File;
use std::io::Write;

#[derive(Debug, Clone)]
/// Structure of a Processor
pub struct Processor {
    pub id: u32,
    pub gossip: HashMap<u32, Sender<Message>>,
    msg_recv: Option<Message>,
    pub tx: Sender<Message>,
    rx: Receiver<Message>,
    send_counter: u32,
}

impl Processor {
    /// Initialise a new Processor with a given ID and open a channel for message passing :
    /// # Arguments
    ///
    /// * `id` - A unique ID.
    pub fn new(id: u32) -> Processor {
        let (tx, rx) = unbounded();
        Processor {
            id: id,
            gossip: HashMap::new(),
            msg_recv: None,
            tx: tx,
            rx: rx,
            send_counter: 0,
        }
    }

    pub fn get_sender(&self) -> Sender<Message> {
        self.tx.clone()
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
                timestamp,
            ));
            for (_, tx) in &self.gossip.clone() {
                let msg: Message = Message::new(content.clone(), sig.clone(), self.id, timestamp);
                self.forward_murmur(msg, tx.clone())
            }
        }
    }

    fn forward_murmur(&mut self, msg: Message, tx: Sender<Message>) {
        tx.send(msg).unwrap();
        self.send_counter += 1;
    }

    fn dispatch_murmur(&mut self, msg: Message) {
        if self.msg_recv.is_none() {
            self.msg_recv = Some(msg.clone());
            for (_, tx) in &self.gossip.clone() {
                self.forward_murmur(msg.clone(), tx.clone())
            }
        }
    }

    fn deliver_murmur(&mut self, msg: Message) {
        if msg.verify_murmur() {
            let mut file: File = File::create(format!("check/{}.txt", self.id)).unwrap();
            let wf = file.write(b"DELIVERED");
            match wf {
                Ok(_) => (),
                Err(err) => println!("ERROR writing delivered file : {}", err),
            }
            self.dispatch_murmur(msg);
        }
    }

    pub fn listen(&mut self) {
        loop {
            println!("Processor {} send counter : {}", self.id, self.send_counter);
            let msg: Message = self.rx.recv().unwrap();
            let f_msg: Message = msg.clone();
            if self.msg_recv.is_none() {
                println!("DELIVER : id {}", self.id);
                msg.print_message_info();
                self.deliver_murmur(f_msg);
            }
        }
    }
}
