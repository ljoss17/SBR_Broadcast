#![crate_name = "sbr_broadcast"]
mod contagion;
mod definition_message;
mod definition_processor;
mod murmur;
mod sieve;

use crate::contagion::initialise_contagion;
use crate::definition_processor::Processor;
use crate::murmur::{get_sender_gossip, initialise_murmur};
use crate::sieve::initialise_sieve;
use std::env;
use std::thread;
use std::time::Duration;

extern crate chrono;
extern crate rand;

fn main() {
    let n_str = env::args().nth(1).expect("Size of system N.");
    let n: usize = match n_str.parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Expected number for first argument, got : {}", n_str);
            return;
        }
    };
    let g_str = env::args().nth(2).expect("Size of Gossip group G.");
    let g: u32 = match g_str.parse() {
        Ok(g) => g,
        Err(_) => {
            println!("Expected number for second argument, got : {}", g_str);
            return;
        }
    };
    let e_str = env::args().nth(3).expect("Size of Echo group E.");
    let e: u32 = match e_str.parse() {
        Ok(e) => e,
        Err(_) => {
            println!("Expected number for third argument, got : {}", e_str);
            return;
        }
    };
    let e_thr_str = env::args().nth(4).expect("Echo threshold E_thr.");
    let e_thr: u32 = match e_thr_str.parse() {
        Ok(e_thr) => e_thr,
        Err(_) => {
            println!("Expected number for fourth argument, got : {}", e_thr_str);
            return;
        }
    };
    let r_str = env::args().nth(5).expect("Size of Echo group R.");
    let r: u32 = match r_str.parse() {
        Ok(r) => r,
        Err(_) => {
            println!("Expected number for fifth argument, got : {}", r_str);
            return;
        }
    };
    let r_thr_str = env::args().nth(6).expect("Echo threshold R_thr.");
    let r_thr: u32 = match r_thr_str.parse() {
        Ok(r_thr) => r_thr,
        Err(_) => {
            println!("Expected number for sixth argument, got : {}", r_thr_str);
            return;
        }
    };
    let d_str = env::args().nth(7).expect("Size of Echo group D.");
    let d: u32 = match d_str.parse() {
        Ok(d) => d,
        Err(_) => {
            println!("Expected number for seventh argument, got : {}", d_str);
            return;
        }
    };
    let d_thr_str = env::args().nth(8).expect("Echo threshold D_thr.");
    let d_thr: u32 = match d_thr_str.parse() {
        Ok(d_thr) => d_thr,
        Err(_) => {
            println!("Expected number for eighth argument, got : {}", d_thr_str);
            return;
        }
    };

    let mut system: Vec<u32> = vec![0; n];
    for i in 0..n {
        system[i] = i as u32;
    }

    let mut processors: Vec<Processor> = Vec::new();
    add_processors(&mut processors, &system);

    spawn_processors(&mut processors);
    println!("Init Murmur");
    initialise_murmur(&mut processors, &system, g);
    println!("Init Sieve");
    initialise_sieve(&mut processors, &system, e, e_thr);
    println!("Init Contagion");
    initialise_contagion(&mut processors, &system, r, r_thr, d, d_thr);

    let senders = get_sender_gossip(&mut processors, &system, g);
    let mut sender_proc: Processor = Processor::new((n + 1) as u32);
    sender_proc.gossip = senders;
    println!("WILL broadcast");
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
