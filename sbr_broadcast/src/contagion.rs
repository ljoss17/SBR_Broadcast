use crate::definition_message::Message;
use crate::definition_message::MessageType;
use crate::definition_processor::Processor;
use chrono::{DateTime, Utc};
use crossbeam::channel::Sender;

use rand::prelude::*;
use std::collections::HashMap;

pub fn initialise_contagion(
    processors: &mut Vec<Processor>,
    system: &Vec<u32>,
    r: u32,
    ready_thr: u32,
    d: u32,
    deliver_thr: u32,
) {
    // Get Sender channels.
    let mut senders: HashMap<u32, Sender<Message>> = HashMap::new();
    for &i in system {
        let sender: &Sender<Message> = &processors[i as usize].get_sender();
        senders.insert(i, sender.clone());
    }

    // Setup contagion connexions.
    let mut rng = rand::thread_rng();
    let num_proc = processors.len();
    for p in processors.iter_mut() {
        p.ready_thr = ready_thr;
        p.deliver_thr = deliver_thr;
        let mut group_rdy: Vec<u32> = Vec::new();
        // Setup Ready links.
        loop {
            let n = rng.gen_range(0..num_proc);
            let random_id: u32 = system[n];

            // Only subscribe if the random processor is new and not self.
            if (random_id != p.id) && (!group_rdy.contains(&random_id)) {
                group_rdy.push(random_id);
                let timestamp: DateTime<Utc> = Utc::now();
                let gossip_subscription: Message = Message::new(
                    String::from(""),
                    p.id,
                    p.id,
                    p.get_sender().clone(),
                    timestamp,
                    MessageType::ReadySubscription,
                );
                senders[&random_id].send(gossip_subscription).unwrap();
            }

            // Stop random selection when the correct amount of processors are in the echo group.
            if group_rdy.len() == r as usize {
                break;
            }
        }
        let mut group_del: Vec<u32> = Vec::new();
        // Setup Deliver links.
        loop {
            let n = rng.gen_range(0..num_proc);
            let random_id: u32 = system[n];

            // Only subscribe if the random processor is new and not self.
            if (random_id != p.id) && (!group_del.contains(&random_id)) {
                group_del.push(random_id);
                let timestamp: DateTime<Utc> = Utc::now();
                let gossip_subscription: Message = Message::new(
                    String::from(""),
                    p.id,
                    p.id,
                    p.get_sender().clone(),
                    timestamp,
                    MessageType::DeliverySubscription,
                );
                senders[&random_id].send(gossip_subscription).unwrap();
            }

            // Stop random selection when the correct amount of processors are in the echo group.
            if group_del.len() == d as usize {
                break;
            }
        }
    }
}
