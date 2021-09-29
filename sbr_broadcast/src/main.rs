#![crate_name = "sbr_broadcast"]
mod definition_message;
mod definition_processor;
mod murmur;

use crate::definition_message::Message;
use crate::definition_processor::Processor;
use crate::murmur::{get_sender_gossip, initialise_murmur};
use crossbeam::channel::Sender;
use std::thread;
use std::time::Duration;

extern crate chrono;
extern crate rand;

fn main() {
    let n: usize = 100;
    let mut system: Vec<u32> = vec![0; n];
    for i in 0..n {
        system[i] = i as u32;
    }
    let g: u32 = 4;

    let mut processors: Vec<Processor> = Vec::new();
    add_processors(&mut processors, &system);

    initialise_murmur(&mut processors, &system, g);

    // Create a sender Processor.
    let peers_tx: Vec<Sender<Message>> = get_sender_gossip(&mut processors, &system, g);
    let mut sender_proc: Processor = Processor::new(n as u32 + 1);
    for p in peers_tx {
        sender_proc.add_gossip_peer(p);
    }

    spawn_processors(&mut processors);

    sender_proc.broadcast_murmur(String::from("Test message"));

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

/// Update the system of processors given a list of processor IDs.
/// # Arguments
///
/// * `processors` - A vector of Processor to update with new processors.
/// * `system` - A vector of IDs for the processors to initialise.
///
fn add_processors(processors: &mut Vec<Processor>, system: &Vec<u32>) {
    // Initialise Processors ids and channels.
    for &i in system {
        let p = Processor::new(i);
        processors.push(p);
    }
}

/// Spawn a thread for each Processor in which they listen for incoming messages.
/// # Arguments
///
/// * `processors` - A vector of Processor to update with new processors.
///
fn spawn_processors(processors: &mut Vec<Processor>) {
    // Create a thread per Processor to listen for incoming messages.
    let n_p: usize = processors.len();
    for p_id in 0..n_p {
        let mut proc: Processor = processors[p_id].clone();
        thread::spawn(move || {
            proc.listen();
        });
    }
}
