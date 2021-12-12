use crate::message::Message;
use crate::sieve;
use rand::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use talk::broadcast::{BestEffort, BestEffortSettings};
use talk::crypto::{Identity, KeyCard};
use talk::time::sleep_schedules::CappedExponential;
use talk::unicast::{Acknowledgement, PushSettings, Sender};
use tokio::sync::Mutex;

/// Initialises the Gossip set used in the Murmur algorithm. Chose randomly a number of peers as Gossip peers.
///
/// # Arguments
///
/// * `g` - The number of Gossip peers.
/// * `node_sender` - The Node's Sender used to send messages.
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

pub async fn gossip_subscribe(node_sender: Sender<Message>, gossip_peers: Vec<Identity>) {
    let push_settings = PushSettings {
        stop_condition: Acknowledgement::Weak,
        retry_schedule: Arc::new(CappedExponential::new(
            Duration::from_secs(1),
            2.,
            Duration::from_secs(180),
        )),
    };
    let settings: BestEffortSettings = BestEffortSettings { push_settings };
    let msg = Message::new(3, String::from("GossipSubscription"));
    let best_effort = BestEffort::new(
        node_sender.clone(),
        gossip_peers.clone(),
        msg.clone(),
        settings,
    );
    best_effort.complete().await;
}

/// Deliver a Gossip type message. Dispatch a verified message to its Gossip peers.
///
/// # Arguments
///
/// * `content` - The content of the message (Without the prepending type).
/// * `node_sender` - The Node's Sender used to send messages.
/// * `gossip_peers` - The Atomic Reference Counter to the peers to which the Gossip will be spread.
/// * `delivered_gossip` - The Atomic Reference Counter to the status of the delivered Gossip message.
/// * `echo` - The Atomic Reference Counter to the status of the Echo message (used by Sieve).
/// * `echo_peers` - The Atomic Reference Counter to the echo peers (used by Sieve).
///
pub async fn deliver_gossip(
    content: Message,
    node_sender: Sender<Message>,
    gossip_peers: Arc<Mutex<Vec<Identity>>>,
    delivered_gossip: Arc<Mutex<Option<Message>>>,
    echo: Arc<Mutex<Option<Message>>>,
    echo_peers: Arc<Mutex<Vec<Identity>>>,
) {
    // crypto.verify(msg)
    dispatch(
        content,
        node_sender,
        gossip_peers,
        delivered_gossip,
        echo,
        echo_peers,
    )
    .await;
}

/// Dispatch a message to the Gossip peers. If no Gossip message has yet been delivered, send a Gossip
/// message to the given peers, and then Probabilistic Broadcast deliver the message.
///
/// # Arguments
///
/// * `content` - The content of the message (Without the prepending type).
/// * `node_sender` - The Node's Sender used to send messages.
/// * `peers` - The Atomic Reference Counter to the peers to which the Gossip will be spread.
/// * `delivered_gossip` - The Atomic Reference Counter to the status of the delivered Gossip message.
/// * `echo` - The Atomic Reference Counter to the status of the Echo message (used by Sieve).
/// * `echo_peers` - The Atomic Reference Counter to the echo peers (used by Sieve).
///
pub async fn dispatch(
    content: Message,
    node_sender: Sender<Message>,
    peers: Arc<Mutex<Vec<Identity>>>,
    delivered_gossip: Arc<Mutex<Option<Message>>>,
    echo: Arc<Mutex<Option<Message>>>,
    echo_peers: Arc<Mutex<Vec<Identity>>>,
) {
    if delivered_gossip.lock().await.is_none() {
        let mut locked_delivered = delivered_gossip.lock().await;
        *locked_delivered = Some(content.clone());
        drop(locked_delivered);
        let push_settings = PushSettings {
            stop_condition: Acknowledgement::Weak,
            retry_schedule: Arc::new(CappedExponential::new(
                Duration::from_secs(1),
                2.,
                Duration::from_secs(10),
            )),
        };
        let settings: BestEffortSettings = BestEffortSettings { push_settings };
        let cp_peers: Vec<Identity> = peers.lock().await.clone();
        let best_effort = BestEffort::new(
            node_sender.clone(),
            cp_peers.clone(),
            content.clone(),
            settings,
        );
        best_effort.complete().await;
        sieve::deliver(content, node_sender, &echo, &echo_peers).await;
    }
}

/// Deliver a GossipSubscription type message. If a Gossip message has already been delivered, send it to
/// the subscribing peer. Add the peer to the Gossip peers.
///
/// # Arguments
///
/// * `node_sender` - The Node's Sender used to send messages.
/// * `from` - The Identity of the Node subscribing.
/// * `gossip_peers` - The Atomic Reference Counter to the peers to which the Gossip will be spread.
/// * `delivered_gossip` - The Atomic Reference Counter to the status of the delivered Gossip message.
///
pub async fn gossip_subscription(
    node_sender: Sender<Message>,
    from: Identity,
    gossip_peers: Arc<Mutex<Vec<Identity>>>,
    delivered_gossip: Arc<Mutex<Option<Message>>>,
) {
    let cloned_delivered: Option<Message> = delivered_gossip.lock().await.clone();
    if let Some(delivered_msg) = cloned_delivered {
        let r = node_sender.send(from, delivered_msg.clone()).await;
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
