use crate::message::{Message, SignedMessage};
use crate::message_headers::{Gossip, GossipSubscription};
use crate::sieve;
use rand::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use talk::broadcast::{BestEffort, BestEffortSettings};
use talk::crypto::{Identity, KeyCard, KeyChain};
use talk::time::sleep_schedules::{CappedExponential, Constant};
use talk::unicast::{Acknowledgement, PushSettings, Sender};
use tokio::sync::Mutex;

/// Initialises the Gossip set used in the Murmur algorithm. Randomly chooses peers.
///
/// # Arguments
///
/// * `g` - The number of Gossip peers.
/// * `system` - The system in which the peers are randomly chosen.
/// * `gossip_peers` - The Atomic Reference Counter to the Gossip peers to update.
///
pub async fn init(g: usize, system: Vec<KeyCard>, gossip_peers: &Arc<Mutex<Vec<Identity>>>) {
    let num_proc = system.len();
    let mut peers: Vec<Identity> = vec![];
    for _ in 1..=g {
        let mut rng = rand::thread_rng();
        let n = rng.gen_range(0..num_proc);
        drop(rng);
        peers.push(system[n].identity().clone());
    }
    let mut locked_gossip_peers = gossip_peers.lock().await;
    locked_gossip_peers.append(&mut peers);
    drop(locked_gossip_peers);
}

/// Send GossipSubscription to Gossip peers.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `node_sender` - The Node's Sender used to send Messages.
/// * `gossip_peers` - The Gossip peers.
///
pub async fn gossip_subscribe(
    keychain: KeyChain,
    node_sender: Sender<SignedMessage>,
    gossip_peers: Vec<Identity>,
) {
    let push_settings = PushSettings {
        stop_condition: Acknowledgement::Strong,
        retry_schedule: Arc::new(Constant::new(Duration::from_millis(100))),
    };
    let settings: BestEffortSettings = BestEffortSettings { push_settings };
    let msg = Message::new(3, String::from("GossipSubscription"));
    let signature = keychain.sign(&GossipSubscription(msg.clone())).unwrap();
    let signed_msg = SignedMessage::new(msg, signature);
    let best_effort = BestEffort::new(
        node_sender.clone(),
        gossip_peers.clone(),
        signed_msg.clone(),
        settings,
    );
    best_effort.complete().await;
    my_print!("Finished Murmur Subscriptions");
}

/// Deliver a Gossip type Message. Dispatch a verified Message to its Gossip peers.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `signed_msg` - The signed Message to deliver.
/// * `node_sender` - The Node's Sender used to send Messages.
/// * `gossip_peers` - The peers to which the Gossip will be spread.
/// * `delivered_gossip` - The Atomic Reference Counter to the status of the delivered Gossip Message.
/// * `echo` - The Atomic Reference Counter to the status of the Echo Message (used by Sieve).
/// * `echo_peers` - The echo peers (used by Sieve).
///
pub async fn deliver_gossip(
    keychain: KeyChain,
    signed_msg: SignedMessage,
    node_sender: Sender<SignedMessage>,
    gossip_peers: Vec<Identity>,
    delivered_gossip: Arc<Mutex<Option<Message>>>,
    echo: Arc<Mutex<Option<Message>>>,
    echo_peers: Vec<Identity>,
) {
    dispatch(
        keychain,
        signed_msg,
        node_sender,
        gossip_peers,
        delivered_gossip,
        echo,
        echo_peers,
    )
    .await;
}

/// Dispatch a Message to the Gossip peers. If no Gossip Message has yet been delivered, send a Gossip
/// Message to the given peers, and then Probabilistic Broadcast deliver the Message.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `signed_msg` - The signed Message to deliver.
/// * `node_sender` - The Node's Sender used to send Messages.
/// * `peers` - The peers to which the Gossip will be spread.
/// * `delivered_gossip` - The Atomic Reference Counter to the status of the delivered Gossip Message.
/// * `echo` - The Atomic Reference Counter to the status of the Echo Message (used by Sieve).
/// * `echo_peers` - The echo peers (used by Sieve).
///
pub async fn dispatch(
    keychain: KeyChain,
    signed_msg: SignedMessage,
    node_sender: Sender<SignedMessage>,
    peers: Vec<Identity>,
    delivered_gossip: Arc<Mutex<Option<Message>>>,
    echo: Arc<Mutex<Option<Message>>>,
    echo_peers: Vec<Identity>,
) {
    if delivered_gossip.lock().await.is_none() {
        let mut locked_delivered = delivered_gossip.lock().await;
        *locked_delivered = Some(signed_msg.clone().get_message());
        drop(locked_delivered);
        let push_settings = PushSettings {
            stop_condition: Acknowledgement::Strong,
            retry_schedule: Arc::new(CappedExponential::new(
                Duration::from_secs(1),
                2.,
                Duration::from_secs(10),
            )),
        };
        let signature = keychain
            .sign(&Gossip(signed_msg.clone().get_message()))
            .unwrap();
        let signed_broadcast = SignedMessage::new(signed_msg.clone().get_message(), signature);
        let settings: BestEffortSettings = BestEffortSettings { push_settings };
        let best_effort = BestEffort::new(
            node_sender.clone(),
            peers,
            signed_broadcast.clone(),
            settings,
        );
        best_effort.complete().await;
        sieve::deliver(keychain, signed_msg, node_sender, echo, echo_peers).await;
    }
}

/// Deliver a GossipSubscription type Message. If a Gossip Message has already been delivered, send it to
/// the subscribing peer. Add the peer to the Gossip peers.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `node_sender` - The Node's Sender used to send Messages.
/// * `from` - The Identity of the Node subscribing.
/// * `gossip_peers` - The Atomic Reference Counter to the peers to which the Gossip will be spread.
/// * `delivered_gossip` - The status of the delivered Gossip Message.
///
pub async fn gossip_subscription(
    keychain: KeyChain,
    node_sender: Sender<SignedMessage>,
    from: Identity,
    gossip_peers: Arc<Mutex<Vec<Identity>>>,
    delivered_gossip: Option<Message>,
) {
    if let Some(delivered_msg) = delivered_gossip {
        let signature = keychain.sign(&Gossip(delivered_msg.clone())).unwrap();
        let signed_msg = SignedMessage::new(delivered_msg, signature);
        let r = node_sender.send(from, signed_msg.clone()).await;
        match r {
            Ok(_) => {}
            Err(e) => {
                println!("ERROR : gossip_subscription send : {}", e);
            }
        }
    }
    let mut locked_gossip_peers = gossip_peers.lock().await;
    locked_gossip_peers.push(from);
    drop(locked_gossip_peers);
}
