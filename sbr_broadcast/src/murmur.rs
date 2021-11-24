use crate::sieve;
use rand::prelude::*;
use std::sync::{Arc, Mutex};
use talk::broadcast::{BestEffort, BestEffortSettings};
use talk::crypto::Identity;
use talk::unicast::{PushSettings, Sender};

pub async fn init(g: u32, sender: Sender<String>, system: Vec<Identity>) -> Vec<Identity> {
    let mut rng = rand::thread_rng();
    let num_proc = system.len();
    let mut gossip_peers: Vec<Identity> = Vec::new();
    for _ in 1..=g {
        let n = rng.gen_range(0..num_proc);
        gossip_peers.push(system[n].clone());
        sender
            .send(system[n].clone(), String::from("GossipSubscription"))
            .await;
    }
    gossip_peers
}

pub async fn deliver_gossip(
    msg: String,
    sender: Sender<String>,
    peers: &Arc<Mutex<Vec<Identity>>>,
    delivered: &Arc<Mutex<Option<String>>>,
    echo: &Arc<Mutex<Option<String>>>,
    echo_peers: &Arc<Mutex<Vec<Identity>>>,
) {
    // crypto.verify(msg)
    dispatch(msg, sender, peers, delivered, echo, echo_peers).await;
}

pub async fn dispatch(
    content: String,
    sender: Sender<String>,
    peers: &Arc<Mutex<Vec<Identity>>>,
    delivered: &Arc<Mutex<Option<String>>>,
    echo: &Arc<Mutex<Option<String>>>,
    echo_peers: &Arc<Mutex<Vec<Identity>>>,
) {
    if delivered.lock().unwrap().is_none() {
        let mut locked_delivered = delivered.lock().unwrap();
        *locked_delivered = Some(content.clone());
        drop(locked_delivered);
        let settings: BestEffortSettings = BestEffortSettings {
            push_settings: PushSettings::default(),
        };
        let peers: Vec<Identity> = peers.lock().unwrap().clone();
        let best_effort = BestEffort::new(sender.clone(), peers, content.clone(), settings);
        best_effort.complete().await;
        sieve::deliver(content, sender.clone(), echo, echo_peers).await;
    }
}

pub async fn gossip_subscription(
    node: Sender<String>,
    from: Identity,
    gossip_peers: &Arc<Mutex<Vec<Identity>>>,
    delivered: &Arc<Mutex<Option<String>>>,
) {
    let cloned_delivered: Option<String> = delivered.lock().unwrap().clone();
    if let Some(delivered_msg) = cloned_delivered {
        node.send(from, delivered_msg.clone()).await;
    }
    let mut locked_gossip_peers = gossip_peers.lock().unwrap();
    locked_gossip_peers.push(from);
    drop(locked_gossip_peers);
}
