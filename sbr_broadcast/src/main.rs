mod msg_def;
mod murmur;
mod proc_def;

use msg_def::Message;

extern crate rand;

fn main() {
    println!("Hello, world!");
    let processors = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    for i in processors {
        println!("i : {}", i);
    }
    //let p = proc_def::Processor::new(1);
}
