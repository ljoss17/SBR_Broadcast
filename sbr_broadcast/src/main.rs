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
use std::{fs, io};
use talk::crypto::{Identity, KeyCard, KeyChain};
use talk::link::rendezvous::{Client, Connector, Listener};
use talk::unicast::{Receiver, Sender};

extern crate chrono;
extern crate rand;

#[tokio::main]
async fn main() {
    my_print!("Start");
    // Read config files
    let content = fs::read_to_string("broadcast.config").expect("Error reading config file");
    let lines = content.split("\n");
    // Default values, if not specified in config file.
    let mut addr: String = String::from("127.0.0.1");
    let mut port: u16 = 4446;
    let mut spawn: usize = 1;
    let mut g: usize = 10;
    let mut e: usize = 40;
    let mut e_thr: usize = 10;
    let mut r: usize = 30;
    let mut r_thr: usize = 10;
    let mut d: usize = 25;
    let mut d_thr: usize = 14;
    for line in lines {
        let mut elems = line.split("=");
        match elems.next().unwrap() {
            "addr" => {
                addr = elems.next().unwrap().to_string();
            }
            "port" => {
                port = elems.next().unwrap().parse().unwrap();
            }
            "spawn" => {
                spawn = elems.next().unwrap().parse().unwrap();
            }
            "N" => {}
            "G" => {
                g = elems.next().unwrap().parse().unwrap();
            }
            "E" => {
                e = elems.next().unwrap().parse().unwrap();
            }
            "E_thr" => {
                e_thr = elems.next().unwrap().parse().unwrap();
            }
            "R" => {
                r = elems.next().unwrap().parse().unwrap();
            }
            "R_thr" => {
                r_thr = elems.next().unwrap().parse().unwrap();
            }
            "D" => {
                d = elems.next().unwrap().parse().unwrap();
            }
            "D_thr" => {
                d_thr = elems.next().unwrap().parse().unwrap();
            }
            "" => {}
            _ => {
                println!("Unknown configuration : {}", line);
            }
        }
    }

    // Setup N nodes.
    let mut identities = vec![];
    for i in 0..spawn {
        let node_keychain = KeyChain::random();
        /*println!(
            "<{}> : KC : {:?}",
            i,
            node_keychain.keycard().identity().clone()
        );*/
        identities.push(node_keychain.keycard().identity().clone());
        tokio::spawn(setup_node(
            node_keychain.clone(),
            addr.clone(),
            port,
            i,
            g,
            e,
            e_thr,
            r,
            r_thr,
            d,
            d_thr,
        ));
    }

    loop {
        let mut input: String = String::new();
        io::stdin().read_line(&mut input).unwrap();
        match input.as_str() {
            "send\n" => {
                let mut rng = rand::thread_rng();
                let n = rng.gen_range(0..spawn);
                trigger_send(addr.clone(), port, identities[n]).await;
            }
            "exit\n" => {
                break;
            }
            _ => {}
        }
    }
}

/// Setup and initialise a node with given parameters.
///
/// # Arguments
///
/// * `node_keychain` - The KeyChain of the node to setup.
/// * `addr` - The adresse of the Rendez-Vous server.
/// * `port` - The port of the Rendez-Vous server.
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
    node_keychain: KeyChain,
    addr: String,
    port: u16,
    i: usize,
    g: usize,
    e: usize,
    e_thr: usize,
    r: usize,
    r_thr: usize,
    d: usize,
    d_thr: usize,
) {
    let client = Client::new((addr.clone(), port), Default::default());

    client
        .publish_card(node_keychain.keycard(), Some(0))
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    let keycards: Vec<KeyCard> = loop {
        let kc = client.get_shard(0).await;
        match kc {
            Ok(kcs) => {
                break kcs;
            }
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    };

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
        (addr.clone(), port),
        node_keychain.clone(),
        Default::default(),
    );

    let sender = Sender::new(connector, Default::default());

    let listener = Listener::new(
        (addr.clone(), port),
        node_keychain.clone(),
        Default::default(),
    )
    .await;
    let mut receiver = Receiver::new(listener, Default::default());

    let mut node: Node = Node::new(node_keychain.clone(), map_keycards, i, e_thr, r_thr, d_thr);
    murmur::init(g, other_keycards.clone(), &node.gossip_peers).await;
    sieve::init(
        e,
        other_keycards.clone(),
        &node.echo_replies,
        &mut node.duplicate_echo,
    )
    .await;
    contagion::init(
        r,
        d,
        other_keycards.clone(),
        &node.ready_replies,
        &node.delivery_replies,
        &mut node.duplicate_ready,
        &mut node.duplicate_delivery,
    )
    .await;
    let kc: KeyChain = node_keychain.clone();
    tokio::spawn(send_initialisation_signals(
        addr.clone(),
        port,
        kc.keycard(),
        i,
    ));

    node.listen(sender, &mut receiver, kc.keycard().clone())
        .await;
}

/// Send signals to initialise the sets which require subscriptions.
///
/// # Arguments
///
/// * `addr` - The adresse of the Rendez-Vous server.
/// * `port` - The port of the Rendez-Vous server.
/// * `kc` - The KeyCard of the node to initialise.
/// * `id` - The ID of the node.
///
async fn send_initialisation_signals(addr: String, port: u16, kc: KeyCard, id: usize) {
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    let init_keychain = KeyChain::random();

    let connector = Connector::new((addr, port), init_keychain.clone(), Default::default());

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
    let t_sender = sender.clone();
    let t_kc = kc.clone();
    let t_init_keychain = init_keychain.clone();
    tokio::spawn(async move {
        loop {
            let echo_init: Message = Message::new(7, String::from("Init Echo Subscription"));
            let signature = t_init_keychain.sign(&InitEcho(echo_init.clone())).unwrap();
            let signed_msg: SignedMessage = SignedMessage::new(echo_init, signature);
            let r = t_sender.send(t_kc.identity(), signed_msg).await;
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
    });
    let t_sender = sender.clone();
    let t_kc = kc.clone();
    let t_init_keychain = init_keychain.clone();
    tokio::spawn(async move {
        loop {
            let ready_init: Message = Message::new(8, String::from("Init Ready Subscription"));
            let signature = t_init_keychain
                .sign(&InitReady(ready_init.clone()))
                .unwrap();
            let signed_msg: SignedMessage = SignedMessage::new(ready_init, signature);
            let r = t_sender.send(t_kc.identity(), signed_msg).await;
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
    });
}

/// Wait for system to finish setup and then send the signal to trigger the Broadcast.
///
/// # Arguments
///
/// * `addr` - The adresse of the Rendez-Vous server.
/// * `port` - The port of the Rendez-Vous server.
/// * `id` - The Identity of the Node which will receive the signal.
///
async fn trigger_send(addr: String, port: u16, id: Identity) {
    my_print!("Trigger send");
    let sender_keychain = KeyChain::random();

    let connector = Connector::new((addr, port), sender_keychain.clone(), Default::default());

    let tmp_sender: Sender<SignedMessage> = Sender::new(connector, Default::default());
    let msg = Message::new(9, String::from("Trigger send"));
    let signature = sender_keychain.sign(&Gossip(msg.clone())).unwrap();
    let signed_msg: SignedMessage = SignedMessage::new(msg, signature);
    loop {
        let r = tmp_sender.send(id, signed_msg.clone()).await;
        match r {
            Ok(_) => {
                break;
            }
            Err(e) => {
                println!("ERROR : echo_subscription send : {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}
