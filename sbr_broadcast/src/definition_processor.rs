use crate::definition_message::{Message, MessageType};

use chrono::{DateTime, Utc};
use crossbeam::channel::unbounded;
use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use std::fs::File;
use std::io::Write;

#[derive(Debug, Clone)]
/// Structure of a Processor
pub struct Processor {
    // General fields.
    pub id: u32,
    pub tx: Sender<Message>,
    rx: Receiver<Message>,
    delivered: bool,
    // Map signature to Message
    messages: HashMap<u64, Message>,
    // Murmur fields.
    pub gossip: Vec<Sender<Message>>,
    murmur_message: Option<Message>,
    // Sieve fields.
    pub echo_peers: Vec<Sender<Message>>,
    echo: Option<Message>,
    echo_messages: HashMap<u64, Vec<u32>>,
    pub echo_thr: u32,
    // Contagion fields.
    ready_peers: HashMap<u32, Sender<Message>>,
    ready_messages: HashMap<u64, Vec<u32>>,
    pub ready_thr: u32,
    deliver_peers: HashMap<u32, Sender<Message>>,
    pub deliver_thr: u32,
    ready: HashMap<u64, Message>,
    // Debug fields.
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
            tx,
            rx,
            delivered: false,
            messages: HashMap::new(),
            gossip: Vec::new(),
            murmur_message: None,
            echo_peers: Vec::new(),
            echo: None,
            echo_messages: HashMap::new(),
            echo_thr: 0,
            ready_peers: HashMap::new(),
            ready_messages: HashMap::new(),
            ready_thr: 0,
            deliver_peers: HashMap::new(),
            deliver_thr: 0,
            ready: HashMap::new(),
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
    pub fn handle_gossip_subscription(&mut self, proc_tx: Sender<Message>) {
        if self.murmur_message.is_some() {
            self.al_send(self.murmur_message.clone().unwrap(), proc_tx.clone());
        }
        self.gossip.push(proc_tx.clone());
    }

    pub fn handle_echo_subscription(&mut self, proc_tx: Sender<Message>) {
        if self.echo.is_some() {
            self.al_send(self.echo.clone().unwrap(), proc_tx.clone());
        }
        self.echo_peers.push(proc_tx.clone());
    }

    pub fn handle_ready_subscription(&mut self, id: u32, proc_tx: Sender<Message>) {
        for (sig, _) in self.ready_messages.clone() {
            self.al_send(self.messages[&sig].clone(), proc_tx.clone());
        }
        self.ready_peers.insert(id, proc_tx.clone());
    }

    pub fn handle_delivery_subscription(&mut self, id: u32, proc_tx: Sender<Message>) {
        for (sig, _) in self.ready_messages.clone() {
            self.al_send(self.messages[&sig].clone(), proc_tx.clone());
        }
        self.deliver_peers.insert(id, proc_tx.clone());
    }

    /// Broadcast a message to the Processor's Gossip group.
    /// # Arguments
    ///
    /// * `content` - A string of the message's content.
    ///
    pub fn broadcast_murmur(&mut self, content: String) {
        if self.murmur_message.is_none() {
            let timestamp: DateTime<Utc> = Utc::now();
            self.murmur_message = Some(Message::new(
                content.clone(),
                self.id,
                self.id,
                self.tx.clone(),
                timestamp,
                MessageType::Text,
            ));
            for tx in &self.gossip.clone() {
                let msg: Message = Message::new(
                    content.clone(),
                    self.id,
                    self.id,
                    self.tx.clone(),
                    timestamp,
                    MessageType::Text,
                );
                self.al_send(msg, tx.clone())
            }
        }
    }

    /// Forward a message to a Processor.
    /// # Arguments
    ///
    /// * `msg` - The Message to forward.
    /// * `tx` - The target Processor's Sender<Message>.
    ///
    fn al_send(&mut self, msg: Message, tx: Sender<Message>) {
        let new_msg = msg.create_forward_message(self.id);
        tx.send(new_msg).unwrap();
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
    /// * `msg` - The Message to deliver.
    ///
    fn al_deliver(&mut self, msg: Message) {
        if !self.messages.contains_key(&msg.signature) {
            self.messages.insert(msg.signature, msg.clone());
        }
        if msg.verify_murmur() && self.murmur_message.is_none() {
            self.murmur_message = Some(msg.clone());
            for tx in &self.gossip.clone() {
                self.al_send(msg.clone(), tx.clone())
            }
            // pb.Deliver
            self.pb_deliver(msg);
        }
    }

    /// Send an Echo message.
    /// # Arguments
    ///
    /// * `msg` - The Echo Message to send.
    /// * `tx` - The target Processor's Sender<Message>.
    ///
    fn send_echo(&mut self, msg: Message, tx: Sender<Message>) {
        let new_msg = msg.create_forward_echo(self.id);
        tx.send(new_msg).unwrap();
        self.send_counter += 1;
    }

    fn send_ready(&mut self, msg: Message, tx: Sender<Message>) {
        let new_msg = msg.create_forward_ready(self.id);
        tx.send(new_msg).unwrap();
        self.send_counter += 1;
    }

    /// Probabilistic Broadcast Deliver.
    /// # Arguments
    ///
    /// * `msg` - The Message to deliver.
    ///
    fn pb_deliver(&mut self, msg: Message) {
        if msg.verify_murmur() {
            self.echo = Some(msg.clone());
            for tx in self.echo_peers.clone() {
                self.send_echo(msg.clone(), tx);
            }
        }
    }

    fn pcb_deliver(&mut self, msg: Message) {
        if msg.verify_murmur() {
            if !self.ready.contains_key(&msg.signature) {
                self.ready.insert(msg.signature, msg.clone());
            }
            for tx in self.ready_peers.clone() {
                self.send_ready(msg.clone(), tx.1);
            }
        }
    }

    /// Check if enough Echo messages have been received to trigger the Probabilistic Consistent
    /// Broadcast Deliver.
    ///
    fn check_echoes(&mut self) {
        if !self.delivered {
            for (msg_sig, ready_procs) in self.echo_messages.clone().iter_mut() {
                if ready_procs.len() >= self.echo_thr as usize {
                    self.pcb_deliver(self.messages[&msg_sig].clone());
                }
            }
        }
    }

    fn check_ready_messages(&mut self) {
        for (msg_sig, ready_procs) in self.ready_messages.clone().iter_mut() {
            if ready_procs.len() >= self.ready_thr as usize {
                for tx in self.ready_peers.clone() {
                    self.send_ready(self.messages[&msg_sig].clone(), tx.1);
                }
            }
        }
        self.check_deliver();
    }

    fn check_deliver(&mut self) {
        if !self.delivered {
            for (_, ready_procs) in self.ready_messages.clone().iter_mut() {
                if ready_procs.len() >= self.deliver_thr as usize {
                    // *** Optional lines used to verify delivery of messages. ***
                    loop {
                        let file = File::create(format!("check/{}.txt", self.id));
                        match file {
                            Ok(mut f) => {
                                loop {
                                    println!("CREATED : {}", self.id);
                                    let wf = f.write(b"DELIVERED");
                                    match wf {
                                        Ok(_) => {
                                            println!("WROTE : {}", self.id);
                                            break;
                                        }
                                        Err(_) => {
                                            println!("FAILED WRITE : {}", self.id);
                                            thread::sleep(Duration::from_secs(1))
                                        }
                                    }
                                }
                                break;
                            }
                            Err(_) => {
                                println!("FAILED CREATE : {}", self.id);
                                thread::sleep(Duration::from_secs(1))
                            }
                        }
                    }

                    // *** End of optional lines ***
                    self.delivered = true;
                    break;
                }
            }
        }
    }

    /// Verify and deliver an Echo message.
    /// # Arguments
    ///
    /// * `id` - ID of the sending Processor.
    /// * `nsg` - The Echo Message.
    ///
    fn deliver_echo(&mut self, id: u32, msg: Message) {
        if !self.messages.contains_key(&msg.signature) {
            self.messages.insert(msg.signature, msg.clone());
        }
        if msg.verify_murmur() {
            if self.echo_messages.contains_key(&msg.signature) {
                let mut new_ids: Vec<u32> = self.echo_messages[&msg.signature].clone();
                new_ids.push(id);
                self.echo_messages.insert(msg.signature, new_ids.clone());
            } else {
                self.echo_messages.insert(msg.signature, vec![id]);
            }
        }
        self.check_echoes();
    }

    fn deliver_ready(&mut self, id: u32, msg: Message) {
        if !self.messages.contains_key(&msg.signature) {
            self.messages.insert(msg.signature, msg.clone());
        }
        if msg.verify_murmur() {
            // Add new reply.ready for message - processor
            if self.ready_messages.contains_key(&msg.signature) {
                if !self.ready_messages[&msg.signature].contains(&id) {
                    let mut new_ids: Vec<u32> = self.ready_messages[&msg.signature].clone();
                    new_ids.push(id);
                    self.ready_messages.insert(msg.signature, new_ids.clone());
                }
            } else {
                self.ready_messages.insert(msg.signature, vec![id.clone()]);
            }
        }
        self.check_ready_messages();
    }

    /// Handle received message depending on its type.
    /// # Arguments
    ///
    /// * `msg` - Received Message.
    ///
    fn handle_message(&mut self, id: u32, msg: Message) {
        let check_msg: Message = msg.clone();
        match check_msg.message_type {
            MessageType::Text => self.al_deliver(msg),
            MessageType::Gossip => self.handle_gossip_subscription(msg.from_tx.clone()),
            MessageType::Echo => self.deliver_echo(msg.from, msg),
            MessageType::EchoSubscription => self.handle_echo_subscription(msg.from_tx.clone()),
            MessageType::ReadySubscription => {
                self.handle_ready_subscription(msg.from, msg.from_tx.clone())
            }
            MessageType::DeliverySubscription => {
                self.handle_delivery_subscription(id, msg.from_tx.clone())
            }
            MessageType::Ready => self.deliver_ready(msg.from, msg),
        }
    }

    /// Listen for incoming Messages and handle them if it's the first message received.
    ///
    pub fn listen(&mut self) {
        loop {
            let msg: Message = self.rx.recv().unwrap();
            self.handle_message(msg.from, msg);
        }
    }
}
