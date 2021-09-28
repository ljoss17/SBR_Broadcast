#![crate_name = "sbr_broadcast"]
mod msg_def;
mod murmur;
mod proc_def;

use crate::murmur::initialise_murmur;
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
    let g: u32 = 8;
    initialise_murmur(system, g);
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
