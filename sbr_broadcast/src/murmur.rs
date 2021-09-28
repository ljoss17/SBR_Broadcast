use crate::msg_def::Message;
use crate::proc_def::Processor;
use crossbeam::channel::Sender;
use rand::prelude::*;
use std::collections::HashMap;
use std::thread;

/// Initialise the system of processors given a list of processor IDs
/// and a size of Gossip group.
/// # Arguments
///
/// * `system` - A vector of IDs for the processors to initialise.
/// * `g` - A integer which determines the size of the Gossip group for each processor.
///
pub fn initialise_murmur(system: Vec<u32>, g: u32) -> Vec<Sender<Message>> {
    // Initialise Processors ids and channels.
    let mut processors: Vec<Processor> = Vec::new();
    let mut senders: HashMap<u32, Sender<Message>> = HashMap::new();
    for &i in &system {
        let p = Processor::new(i);
        let sender: &Sender<Message> = &p.get_sender();
        processors.push(p);
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

    // Create gossip peers for the sender Processor.
    let mut sender_peers: Vec<Sender<Message>> = Vec::new();
    for _ in 0..g {
        let n = rng.gen_range(0..num_proc);
        let random_id: u32 = system[n];
        sender_peers.push(senders[&random_id].clone());
    }

    // Create a thread per Processor to listen for incoming messages.
    let n_p: usize = processors.len();
    for p_id in 0..n_p {
        let mut proc: Processor = processors[p_id].clone();
        thread::spawn(move || {
            proc.listen();
        });
    }
    sender_peers
}
