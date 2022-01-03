use std::{fs, io};
use talk::link::rendezvous::{Server, ServerSettings};

#[tokio::main]
async fn main() {
    // Read config files
    let content = fs::read_to_string("broadcast.config").expect("Error reading config file");
    let lines = content.split("\n");
    // Default values, if not specified in config file.
    let mut addr: String = String::from("127.0.0.1");
    let mut port = 4446;
    let mut n: usize = 1;
    for line in lines {
        let mut elems = line.split("=");
        match elems.next().unwrap() {
            "addr" => {
                addr = elems.next().unwrap().to_string();
            }
            "port" => {
                port = elems.next().unwrap().parse().unwrap();
            }
            "N" => {
                n = elems.next().unwrap().parse().unwrap();
            }
            "spawn" => {}
            "G" => {}
            "E" => {}
            "E_thr" => {}
            "R" => {}
            "R_thr" => {}
            "D" => {}
            "D_thr" => {}
            "" => {}
            _ => {
                println!("Unknown configuration : {}", line);
            }
        }
    }
    println!("Start Rendezvous server...");
    // Start rendez-vous server
    let _server = Server::new(
        (addr.clone(), port),
        ServerSettings {
            shard_sizes: vec![n],
        },
    )
    .await
    .unwrap();

    loop {
        let mut input: String = String::new();
        io::stdin().read_line(&mut input).unwrap();
        match input.as_str() {
            "exit\n" => {
                break;
            }
            _ => {}
        }
    }
    println!("Rendezvous server stopped!");
}
