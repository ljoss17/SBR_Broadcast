use crate::definition_message::Message;

use chrono::{DateTime, Utc};
use crossbeam::channel::unbounded;
use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;

use std::fs::File;
use std::io::Write;

#[derive(Debug, Clone)]
/// Structure of a Processor
pub struct Processor {
    pub id: u32,
    pub gossip: Vec<Sender<Message>>,
    murmur_message: Option<Message>,
    pub tx: Sender<Message>,
    rx: Receiver<Message>,
    send_counter: u32,
}

impl Processor {
    /// Return a new Processor with a given ID and open a channel for message passing.
    /// # Arguments
    ///
    /// * `id` - A unique ID.
    ///
    pub fn new(id: u32) -> Processor {
        let (tx, rx) = unbounded();
        Processor {
            id,
            gossip: Vec::new(),
            murmur_message: None,
            tx,
            rx,
            send_counter: 0,
        }
    }

    /// Return a reference to the Processor's Sender<Message>.
    ///
    pub fn get_sender(&self) -> &Sender<Message> {
        &self.tx
    }

    /// Add a Sender to the Processor's Gossip group.
    /// # Arguments
    ///
    /// * `proc_tx` - A clone of the Sender<Message> to add.
    ///
    pub fn add_gossip_peer(&mut self, proc_tx: Sender<Message>) {
        self.gossip.push(proc_tx);
    }

    /// Broadcast a message to the Processor's Gossip group.
    /// # Arguments
    ///
    /// * `content` - A string of the message's content.
    ///
    pub fn broadcast_murmur(&mut self, content: String) {
        if self.murmur_message.is_none() {
            let sig: String = format!("{}{}", self.id, content);
            let timestamp: DateTime<Utc> = Utc::now();
            self.murmur_message = Some(Message::new(
                content.clone(),
                sig.clone(),
                self.id,
                timestamp,
            ));
            for tx in &self.gossip.clone() {
                let msg: Message = Message::new(content.clone(), sig.clone(), self.id, timestamp);
                self.forward_murmur(msg, tx.clone())
            }
        }
    }

    /// Forward a message to a Processor.
    /// # Arguments
    ///
    /// * `msg` - The Message to forward.
    /// * `tx` - The target Processor's Sender<Message>.
    ///
    fn forward_murmur(&mut self, msg: Message, tx: Sender<Message>) {
        tx.send(msg).unwrap();
        self.send_counter += 1;
    }

    /// Verify if a Message is correct.
    /// If correct :
    /// * Deliver the Message
    /// * Dispatch the message to the Gossip group
    /// If incorrect :
    /// * Ignore the Message
    /// # Arguments
    ///
    /// * `msg` - The Message to forward.
    ///
    fn deliver_murmur(&mut self, msg: Message) {
        if msg.verify_murmur() {
            // *** Optional lines used to verify delivery of messages. ***
            let mut file: File = File::create(format!("check/{}.txt", self.id)).unwrap();
            let wf = file.write(b"DELIVERED");
            match wf {
                Ok(_) => (),
                Err(err) => println!("ERROR writing delivered file : {}", err),
            }
            // *** End of optional lines ***
            self.murmur_message = Some(msg.clone());
            for tx in &self.gossip.clone() {
                self.forward_murmur(msg.clone(), tx.clone())
            }
        }
    }

    /// Listen for incoming Messages and handle them if it's the first message received.
    ///
    pub fn listen(&mut self) {
        loop {
            println!("Processor {} send counter : {}", self.id, self.send_counter);
            let msg: Message = self.rx.recv().unwrap();
            let f_msg: Message = msg.clone();
            if self.murmur_message.is_none() {
                println!("DELIVER : id {}", self.id);
                msg.print_message_info();
                self.deliver_murmur(f_msg);
            }
        }
    }
}
