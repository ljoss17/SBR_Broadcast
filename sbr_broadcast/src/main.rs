#![crate_name = "sbr_broadcast"]
mod definition_message;
mod definition_processor;
mod murmur;
mod sieve;

use crate::definition_processor::Processor;
use crate::murmur::{get_sender_gossip, initialise_murmur};
use crate::sieve::initialise_sieve;
use std::thread;
use std::time::Duration;

extern crate chrono;
extern crate rand;

fn main() {
    let n: usize = 128;
    let mut system: Vec<u32> = vec![0; n];
    for i in 0..n {
        system[i] = i as u32;
    }
    let g: u32 = 8;
    let e: u32 = 126;
    let e_thr: u32 = 101;

    let mut processors: Vec<Processor> = Vec::new();
    add_processors(&mut processors, &system);

    spawn_processors(&mut processors);
    println!("Init Murmur");
    initialise_murmur(&mut processors, &system, g);
    println!("Init Sieve");
    initialise_sieve(&mut processors, &system, e, e_thr);

    let senders = get_sender_gossip(&mut processors, &system, g);
    let mut sender_proc: Processor = Processor::new((n + 1) as u32);
    sender_proc.gossip = senders;
    sender_proc.broadcast_murmur(String::from("Test Message"));

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
