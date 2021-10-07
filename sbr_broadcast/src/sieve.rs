use crate::definition_message::Message;
use crate::definition_message::MessageType;
use crate::definition_processor::Processor;
use chrono::{DateTime, Utc};
use crossbeam::channel::Sender;

use rand::prelude::*;
use std::collections::HashMap;

pub fn initialise_sieve(processors: &mut Vec<Processor>, system: &Vec<u32>, e: u32, echo_thr: u32) {
    // Get Sender channels.
    let mut senders: HashMap<u32, Sender<Message>> = HashMap::new();
    for &i in system {
        let sender: &Sender<Message> = &processors[i as usize].get_sender();
        senders.insert(i, sender.clone());
    }

    // Setup echo connexions.
    let mut rng = rand::thread_rng();
    let num_proc = processors.len();
    for p in processors.iter_mut() {
        p.echo_thr = echo_thr;
        let mut group: Vec<u32> = Vec::new();
        loop {
            let n = rng.gen_range(0..num_proc);
            let random_id: u32 = system[n];

            // Only subscribe if the random processor is new and not self.
            if (random_id != p.id) && (!group.contains(&random_id)) {
                group.push(random_id);
                let timestamp: DateTime<Utc> = Utc::now();
                let gossip_subscription: Message = Message::new(
                    String::from(""),
                    p.id,
                    p.id,
                    p.get_sender().clone(),
                    timestamp,
                    MessageType::EchoSubscription,
                );
                senders[&random_id].send(gossip_subscription).unwrap();
            }

            // Stop random selection when the correct amount of processors are in the echo group.
            if group.len() == e as usize {
                break;
            }
        }
    }
}
