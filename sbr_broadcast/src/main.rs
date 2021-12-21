#![crate_name = "sbr_broadcast"]

#[macro_use]
mod my_macros;
mod contagion;
mod message;
mod message_headers;
mod murmur;
mod node;
mod sieve;
mod utils;

use crate::message::{Message, SignedMessage};
use crate::message_headers::{Gossip, InitEcho, InitGossip, InitReady};
use crate::node::Node;
use rand::prelude::*;
use std::collections::HashMap;
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
    println!("Begin Setup");
    // Retrieve parameters from command line and parse them.
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

    // Start rendez-vous server
    let _server = Server::new(
        ("127.0.0.1", 4446),
        ServerSettings {
            shard_sizes: vec![n + 1],
        },
    )
    .await
    .unwrap();

    // Setup N nodes.
    let mut nodes = vec![];
    for i in 0..n {
        nodes.push(tokio::spawn(setup_node(i, g, e, e_thr, r, r_thr, d, d_thr)));
    }

    // Spawn a sender node to test the broadcasst
    tokio::spawn(run_sender(g));

    futures::future::join_all(nodes).await;

    println!("Main ended");
}

/// Setup a sender node which will wait 5 minutes before starting to broadcast.
/// Initialisation is due to the setup time for nodes.
///
/// # Arguments
///
/// * `g` - The Gossip set size.
///
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

    let sender: Sender<SignedMessage> = Sender::new(connector, Default::default());

    // Delay required to retrieve the keycards of the other nodes.
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    let keycards = client.get_shard(0).await.unwrap();

    // Delay required for the initialisation of all nodes.
    tokio::time::sleep(std::time::Duration::from_secs(300)).await;

    // Randomly choose G nodes.
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
    let signature = sender_keychain.sign(&Gossip(msg.clone())).unwrap();
    let signed_msg: SignedMessage = SignedMessage::new(msg, signature);
    println!("SENDER broadcast {:?}", sender_keychain.keycard().clone());
    let best_effort = BestEffort::new(sender.clone(), peers.clone(), signed_msg.clone(), settings);
    best_effort.complete().await;
}

/// Setup and initialise a node with given parameters.
///
/// # Arguments
///
/// * `i` - The ID of the node.
/// * `g` - The Gossip set size.
/// * `e` - The Echo set size.
/// * `e_thr` - The Echo threshold.
/// * `r` - The Ready set size.
/// * `r_thr` - The Ready threshold.
/// * `d` - The Delivery set size.
/// * `d_thr` - The Delivery threshold.
///
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
        .clone()
        .into_iter()
        .filter(|keycard| *keycard != node_keychain.keycard())
        .collect::<Vec<_>>();

    let map_keycards = keycards
        .clone()
        .into_iter()
        .filter(|keycard| *keycard != node_keychain.keycard())
        .map(|keycard| keycard.identity())
        .zip(
            keycards
                .clone()
                .into_iter()
                .filter(|keycard| *keycard != node_keychain.keycard()),
        )
        .collect::<Vec<_>>();

    let map_keycards: HashMap<Identity, KeyCard> = map_keycards
        .into_iter()
        .collect::<HashMap<Identity, KeyCard>>();

    let connector = Connector::new(
        ("127.0.0.1", 4446),
        node_keychain.clone(),
        Default::default(),
    );

    let sender = Sender::new(connector, Default::default());
    let mut receiver = Receiver::new(listener, Default::default());
    let node: Node = Node::new(node_keychain.clone(), map_keycards, i, e_thr, r_thr, d_thr);
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
    let kc: KeyChain = node_keychain.clone();
    tokio::spawn(send_initialisation_signals(kc.keycard(), i));
    node.listen(sender, &mut receiver).await;
}

/// Send signals to initialise the sets which require subscriptions.
///
/// # Arguments
///
/// * `kc` - The KeyCard of the node to initialise.
/// * `id` - The ID of the node.
///
async fn send_initialisation_signals(kc: KeyCard, id: usize) {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let init_keychain = KeyChain::random();

    let connector = Connector::new(
        ("127.0.0.1", 4446),
        init_keychain.clone(),
        Default::default(),
    );

    let sender = Sender::new(connector, Default::default());

    let t_sender = sender.clone();
    let t_kc = kc.clone();
    let t_init_keychain = init_keychain.clone();
    tokio::spawn(async move {
        loop {
            let gossip_init: Message = Message::new(6, String::from("Init Gossip Subscription"));
            let signature = t_init_keychain
                .sign(&InitGossip(gossip_init.clone()))
                .unwrap();
            let signed_msg: SignedMessage = SignedMessage::new(gossip_init, signature);
            let r = t_sender.send(t_kc.identity(), signed_msg).await;
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
    });
    loop {
        let echo_init: Message = Message::new(7, String::from("Init Echo Subscription"));
        let signature = init_keychain.sign(&InitEcho(echo_init.clone())).unwrap();
        let signed_msg: SignedMessage = SignedMessage::new(echo_init, signature);
        let r = sender.send(kc.identity(), signed_msg).await;
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
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    loop {
        let ready_init: Message = Message::new(8, String::from("Init Ready Subscription"));
        let signature = init_keychain.sign(&InitReady(ready_init.clone())).unwrap();
        let signed_msg: SignedMessage = SignedMessage::new(ready_init, signature);
        let r = sender.send(kc.identity(), signed_msg).await;
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
