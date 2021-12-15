#![crate_name = "sbr_broadcast"]

#[macro_use]
mod my_macros;
mod contagion;
mod message;
mod murmur;
mod node;
mod peer;
mod sieve;
mod utils;

use crate::message::Message;
use crate::node::Node;
use rand::prelude::*;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use talk::broadcast::{BestEffort, BestEffortSettings};
use talk::crypto::{Identity, KeyCard, KeyChain};
use talk::link::rendezvous::{Client, Connector, Listener, Server, ServerSettings};
use talk::time::sleep_schedules::CappedExponential;
use talk::unicast::{Acknowledgement, PushSettings, Receiver, Sender};

extern crate chrono;
extern crate rand;

#[tokio::main]
async fn main() {
    let n_str = env::args().nth(1).expect("Size of system N.");
    let n: usize = match n_str.parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Expected number for first argument, got : {}", n_str);
            return;
        }
    };
    let g_str = env::args().nth(2).expect("Size of Gossip group G.");
    let g: usize = match g_str.parse() {
        Ok(g) => g,
        Err(_) => {
            println!("Expected number for second argument, got : {}", g_str);
            return;
        }
    };
    let e_str = env::args().nth(3).expect("Size of Echo group E.");
    let e: usize = match e_str.parse() {
        Ok(e) => e,
        Err(_) => {
            println!("Expected number for third argument, got : {}", e_str);
            return;
        }
    };
    let e_thr_str = env::args().nth(4).expect("Echo threshold E_thr.");
    let e_thr: usize = match e_thr_str.parse() {
        Ok(e_thr) => e_thr,
        Err(_) => {
            println!("Expected number for fourth argument, got : {}", e_thr_str);
            return;
        }
    };
    let r_str = env::args().nth(5).expect("Size of Echo group R.");
    let r: usize = match r_str.parse() {
        Ok(r) => r,
        Err(_) => {
            println!("Expected number for fifth argument, got : {}", r_str);
            return;
        }
    };
    let r_thr_str = env::args().nth(6).expect("Echo threshold R_thr.");
    let r_thr: usize = match r_thr_str.parse() {
        Ok(r_thr) => r_thr,
        Err(_) => {
            println!("Expected number for sixth argument, got : {}", r_thr_str);
            return;
        }
    };
    let d_str = env::args().nth(7).expect("Size of Echo group D.");
    let d: usize = match d_str.parse() {
        Ok(d) => d,
        Err(_) => {
            println!("Expected number for seventh argument, got : {}", d_str);
            return;
        }
    };
    let d_thr_str = env::args().nth(8).expect("Echo threshold D_thr.");
    let d_thr: usize = match d_thr_str.parse() {
        Ok(d_thr) => d_thr,
        Err(_) => {
            println!("Expected number for eighth argument, got : {}", d_thr_str);
            return;
        }
    };

    let _server = Server::new(
        ("127.0.0.1", 4446),
        ServerSettings {
            shard_sizes: vec![n + 1],
        },
    )
    .await
    .unwrap();

    let mut nodes = vec![];
    for i in 0..n {
        nodes.push(tokio::spawn(setup_node(i, g, e, e_thr, r, r_thr, d, d_thr)));
    }

    tokio::spawn(run_sender(g));

    futures::future::join_all(nodes).await;

    println!("Main ended");
}

async fn run_sender(g: usize) {
    let client = Client::new(("127.0.0.1", 4446), Default::default());

    let sender_keychain = KeyChain::random();
    client
        .publish_card(sender_keychain.keycard(), Some(0))
        .await
        .unwrap();

    let connector = Connector::new(
        ("127.0.0.1", 4446),
        sender_keychain.clone(),
        Default::default(),
    );

    let sender: Sender<Message> = Sender::new(connector, Default::default());

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    let keycards = client.get_shard(0).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(120)).await;

    let mut peers: Vec<Identity> = vec![];
    let num_proc = keycards.clone().len();
    for _ in 1..=g {
        let mut rng = rand::thread_rng();
        let i = rng.gen_range(0..num_proc);
        drop(rng);
        peers.push(keycards[i].identity().clone());
    }
    let push_settings = PushSettings {
        stop_condition: Acknowledgement::Weak,
        retry_schedule: Arc::new(CappedExponential::new(
            Duration::from_secs(1),
            2.,
            Duration::from_secs(180),
        )),
    };
    let settings: BestEffortSettings = BestEffortSettings { push_settings };
    let msg = Message::new(0, String::from("Test message"));
    println!("SENDER broadcast");
    let best_effort = BestEffort::new(sender.clone(), peers.clone(), msg.clone(), settings);
    best_effort.complete().await;
}

async fn setup_node(
    i: usize,
    g: usize,
    e: usize,
    e_thr: usize,
    r: usize,
    r_thr: usize,
    d: usize,
    d_thr: usize,
) {
    let node_keychain = KeyChain::random();

    let listener = Listener::new(
        ("127.0.0.1", 4446),
        node_keychain.clone(),
        Default::default(),
    )
    .await;

    let client = Client::new(("127.0.0.1", 4446), Default::default());

    client
        .publish_card(node_keychain.keycard(), Some(0))
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    let keycards = client.get_shard(0).await.unwrap();

    let other_keycards = keycards
        .into_iter()
        .filter(|keycard| *keycard != node_keychain.keycard())
        .collect::<Vec<_>>();

    let connector = Connector::new(
        ("127.0.0.1", 4446),
        node_keychain.clone(),
        Default::default(),
    );

    let sender = Sender::new(connector, Default::default());
    let mut receiver = Receiver::new(listener, Default::default());
    let node: Node = Node::new(i, e_thr, r_thr, d_thr);
    murmur::init(g, other_keycards.clone(), &node.gossip_peers).await;
    sieve::init(e, other_keycards.clone(), &node.echo_replies).await;
    contagion::init(
        r,
        d,
        other_keycards.clone(),
        &node.ready_replies,
        &node.delivery_replies,
    )
    .await;
    tokio::spawn(test(node_keychain.keycard(), i));
    node.listen(sender, &mut receiver).await;
}

async fn test(kc: KeyCard, id: usize) {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let init_keychain = KeyChain::random();

    let connector = Connector::new(
        ("127.0.0.1", 4446),
        init_keychain.clone(),
        Default::default(),
    );

    let sender = Sender::new(connector, Default::default());
    loop {
        let gossip_init: Message = Message::new(6, String::from("Init Gossip Subscription"));
        let r = sender.send(kc.identity(), gossip_init).await;
        match r {
            Ok(_) => {
                break;
            }
            Err(e) => {
                println!("ERROR : <{}> init gossip send : {}", id, e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
    loop {
        let echo_init: Message = Message::new(7, String::from("Init Echo Subscription"));
        let r = sender.send(kc.identity(), echo_init).await;
        match r {
            Ok(_) => {
                break;
            }
            Err(e) => {
                println!("ERROR : <{}> init sieve send : {}", id, e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
    loop {
        let ready_init: Message = Message::new(8, String::from("Init Ready Subscription"));
        let r = sender.send(kc.identity(), ready_init).await;
        match r {
            Ok(_) => {
                break;
            }
            Err(e) => {
                println!("ERROR : <{}> init contagion send : {}", id, e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}
