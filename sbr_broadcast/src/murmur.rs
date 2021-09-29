use crate::definition_message::Message;
use crate::definition_processor::Processor;
use crossbeam::channel::Sender;
use rand::prelude::*;
use std::collections::HashMap;

/// Update a vector of Processor to link Gossip groups.
/// # Arguments
///
/// * `processors` - A reference to the vector of Processor.
/// * `system` - A reference to the vector of IDs.
/// * `g` - A integer which determines the size of the Gossip group for each processor.
///
pub fn initialise_murmur(processors: &mut Vec<Processor>, system: &Vec<u32>, g: u32) {
    // Get Sender channels.
    let mut senders: HashMap<u32, Sender<Message>> = HashMap::new();
    for &i in system {
        let sender: &Sender<Message> = &processors[i as usize].get_sender();
        senders.insert(i, sender.clone());
    }

    // Setup gossip connexions.
    let mut rng = rand::thread_rng();
    let num_proc = processors.len();
    for p in processors.iter_mut() {
        let mut group: Vec<u32> = Vec::new();
        loop {
            let n = rng.gen_range(0..num_proc);
            let random_id: u32 = system[n];

            // Only add if the random processor is new and not self.
            if (random_id != p.id) && (!group.contains(&random_id)) {
                group.push(random_id);
                p.add_gossip_peer(senders[&random_id].clone());
            }

            // Stop random selection when the correct amount of processors are in the gossip group.
            if group.len() == g as usize {
                break;
            }
        }
    }
}

/// Get a Gossip group from the list of Processor.
/// # Arguments
///
/// * `processors` - A reference to the vector of Processor.
/// * `system` - A reference to the vector of IDs.
/// * `g` - A integer which determines the size of the Gossip group for each processor.
///
pub fn get_sender_gossip(
    processors: &mut Vec<Processor>,
    system: &Vec<u32>,
    g: u32,
) -> Vec<Sender<Message>> {
    // Get Sender channels.
    let mut senders: HashMap<u32, Sender<Message>> = HashMap::new();
    for &i in system {
        let sender: &Sender<Message> = &processors[i as usize].get_sender();
        senders.insert(i, sender.clone());
    }
    // Create gossip peers for the sender Processor.
    let mut sender_peers: Vec<Sender<Message>> = Vec::new();
    let mut rng = rand::thread_rng();
    let num_proc = processors.len();
    for _ in 0..g {
        let n = rng.gen_range(0..num_proc);
        let random_id: u32 = system[n];
        sender_peers.push(senders[&random_id].clone());
    }
    sender_peers
}
