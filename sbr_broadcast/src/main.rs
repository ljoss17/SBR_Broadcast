#![crate_name = "sbr_broadcast"]
mod msg_def;
mod murmur;
mod proc_def;

use crate::murmur::initialise_murmur;

extern crate chrono;
extern crate rand;

fn main() {
    println!("Hello, world!");
    let system: Vec<u32> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let g: u32 = 3;
    initialise_murmur(system, g)
}
