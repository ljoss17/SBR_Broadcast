#![crate_name = "sbr_broadcast"]
mod msg_def;
mod murmur;
mod proc_def;

use crate::murmur::initialise_murmur;
use crate::proc_def::Processor;
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
    let peers_tx = initialise_murmur(system, g);
    let mut sender_proc: Processor = Processor::new(n as u32 + 1);
    for p in peers_tx {
        sender_proc.add_gossip_peer(p);
    }
    sender_proc.broadcast_murmur(String::from("Test message"));

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
