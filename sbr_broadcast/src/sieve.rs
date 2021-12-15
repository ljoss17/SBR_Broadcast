use crate::contagion;
use crate::message::Message;
use crate::utils::{check_message_occurrences, sample};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use talk::broadcast::{BestEffort, BestEffortSettings};
use talk::crypto::{Identity, KeyCard};
use talk::time::sleep_schedules::CappedExponential;
use talk::unicast::{Acknowledgement, PushSettings, Sender};
use tokio::sync::Mutex;

/// Initialises the Echo set used in the Sieve algorithm. Sample randomly a number of peers from the system
/// and send them an EchoSubscription.
///
/// # Arguments
///
/// * `e` - The number of Echo peers.
/// * `node_sender` - The Node's Sender used to send messages.
/// * `system` - The system in which the peers are randomly chosen.
/// * `echo_replies` - The Atomic Reference Counter to the Echo replies from the chosen peers.
///
pub async fn init(
    e: usize,
    system: Vec<KeyCard>,
    echo_replies: &Arc<Mutex<HashMap<Identity, Option<Message>>>>,
) {
    sample(e, system, echo_replies).await;
}

pub async fn echo_subscribe(
    node_sender: Sender<Message>,
    echo_replies: HashMap<Identity, Option<Message>>,
) {
    let push_settings = PushSettings {
        stop_condition: Acknowledgement::Strong,
        retry_schedule: Arc::new(CappedExponential::new(
            Duration::from_secs(1),
            2.,
            Duration::from_secs(180),
        )),
    };
    let settings: BestEffortSettings = BestEffortSettings { push_settings };
    let peers: Vec<Identity> = echo_replies.into_keys().collect();
    let msg = Message::new(4, String::from("EchoSubscription"));
    let best_effort = BestEffort::new(node_sender.clone(), peers, msg.clone(), settings);
    best_effort.complete().await;
}

/// Deliver an EchoSubscription type message. If an Echo message has already been delivered, send
/// it to the subscribing Node.
///
/// # Arguments
///
/// * `node_sender` - The Node's Sender used to send messages.
/// * `from` - The Identity of the Node subscribing.
/// * `delivered_echo` - The Atomic Reference Counter to the status of the delivered Echo message.
/// * `echo_peers` - The Atomic Reference Counter to the Echo peers to update.
///
pub async fn echo_subscription(
    node_sender: Sender<Message>,
    from: Identity,
    delivered_echo: Option<Message>,
    echo_peers: Arc<Mutex<Vec<Identity>>>,
) {
    if delivered_echo.is_some() {
        let echo_content: Message = delivered_echo.unwrap();
        let msg: Message = Message::new(1, echo_content.content.clone());
        let r = node_sender.send(from, msg).await;
        match r {
            Ok(_) => {}
            Err(e) => {
                println!("ERROR : echo_subscription send : {}", e);
            }
        }
    }
    let mut locked_echo_peers = echo_peers.lock().await;
    locked_echo_peers.push(from);
    drop(locked_echo_peers);
}

/// Probabilistic Broadcast Deliver. If the message is verified, send an Echo of the message to the
/// Echo peers.
///
/// # Arguments
///
/// * `content` - The content of the message (Without the prepending type).
/// * `node_sender` - The Node's Sender used to send the Gossip Subscription to the peers.
/// * `delivered_echo` - The Atomic Reference Counter to the status of the delivered Echo message.
/// * `echo_peers` - The Atomic Reference Counter to the Echo peers to update.
///
pub async fn deliver(
    content: Message,
    node_sender: Sender<Message>,
    delivered_echo: Arc<Mutex<Option<Message>>>,
    echo_peers: Vec<Identity>,
) {
    // crypto.verify(msg)
    let recv_msg = Some(content.clone());
    let mut locked_echo = delivered_echo.lock().await;
    *locked_echo = recv_msg;
    drop(locked_echo);
    let msg: Message = Message::new(1, content.content.clone());
    let push_settings = PushSettings {
        stop_condition: Acknowledgement::Strong,
        retry_schedule: Arc::new(CappedExponential::new(
            Duration::from_secs(1),
            2.,
            Duration::from_secs(180),
        )),
    };
    let settings: BestEffortSettings = BestEffortSettings { push_settings };
    let best_effort = BestEffort::new(node_sender.clone(), echo_peers, msg.clone(), settings);
    best_effort.complete().await;
}

/// Deliver an Echo type message. Save the Echo message in the Echo replies, used to track when a message
/// is ready to be delivered.
///
/// # Arguments
///
/// * `content` - The content of the message (Without the prepending type).
/// * `from` - The Identity of the Node sending the Echo.
/// * `echo_replies` - The Atomic Reference Counter to the Echo replies received.
///
pub async fn deliver_echo(
    content: Message,
    from: Identity,
    echo_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    node_sender: Sender<Message>,
    delivered_echo: Arc<Mutex<Option<Message>>>,
    e_thr: usize,
    ready_peers: Vec<Identity>,
) {
    if echo_replies.lock().await.contains_key(&from) {
        if echo_replies.lock().await.get(&from).unwrap().is_none() {
            let new_reply: Option<Message> = Some(content.clone());
            let mut locked_replies = echo_replies.lock().await;
            locked_replies.insert(from, new_reply);
            drop(locked_replies);
        }
    }
    let echo_replies: HashMap<Identity, Option<Message>> = echo_replies.lock().await.clone();
    check_echoes(
        node_sender,
        delivered_echo,
        e_thr,
        ready_peers,
        echo_replies,
    )
    .await;
}

/// Check the status of the Echo replies received. If more than the threshold have been
/// received, Probabilistic Consistent Broadcast deliver the message.
///
/// # Arguments
///
/// * `node_sender` - The Node's Sender used to send the Gossip Subscription to the peers.
/// * `delivered_echo` - The Atomic Reference Counter to the status of the delivered Echo message.
/// * `e_thr` - The threshold defining if enough Echo replies have been received.
/// * `ready_peers` - The Atomic Reference Counter to the Ready peers (used by Contagion).
/// * `echo_replies` - The Atomic Reference Counter to the Echo replies received.
///
pub async fn check_echoes(
    node_sender: Sender<Message>,
    delivered_echo: Arc<Mutex<Option<Message>>>,
    e_thr: usize,
    ready_peers: Vec<Identity>,
    echo_replies: HashMap<Identity, Option<Message>>,
) {
    if delivered_echo.lock().await.is_none() {
        let occ = check_message_occurrences(echo_replies);
        for m in occ {
            if m.1 >= e_thr {
                let msg = Message::new(1, m.0.clone());
                let echo = Some(msg.clone());
                let mut locked_delivered_echo = delivered_echo.lock().await;
                *locked_delivered_echo = echo.clone();
                drop(locked_delivered_echo);
                contagion::deliver(msg.clone(), node_sender.clone(), ready_peers).await;
                return;
            }
        }
    }
}
