use crate::message::{Message, SignedMessage};
use crate::message_headers::{Ready, ReadySubscription};
use crate::utils::{check_message_occurrences, sample};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use talk::broadcast::{BestEffort, BestEffortSettings};
use talk::crypto::{Identity, KeyCard, KeyChain};
use talk::time::sleep_schedules::CappedExponential;
use talk::unicast::{Acknowledgement, PushSettings, Sender};
use tokio::sync::Mutex;

/// Initialises the Ready set and Delivery set used in the Contagion algorithm. Sample randomly a number
///  of peers from the system and send them an ReadySubscription.
///
/// # Arguments
///
/// * `r` - The number of Ready peers.
/// * `d` - The number of Delivery peers.
/// * `node_sender` - The Node's Sender used to send messages.
/// * `system` - The system in which the peers are randomly chosen.
/// * `ready_replies` - The Atomic Reference Counter to the Ready replies from the chosen peers.
/// * `delivery_replies` - The Atomic Reference Counter to the Delivery replies from the chosen peers.
///
pub async fn init(
    r: usize,
    d: usize,
    system: Vec<KeyCard>,
    ready_replies: &Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    delivery_replies: &Arc<Mutex<HashMap<Identity, Option<Message>>>>,
) {
    sample(r, system.clone(), ready_replies).await;
    sample(d, system.clone(), delivery_replies).await;
}

pub async fn ready_subscribe(
    keychain: KeyChain,
    node_sender: Sender<SignedMessage>,
    ready_replies: HashMap<Identity, Option<Message>>,
    delivery_replies: HashMap<Identity, Option<Message>>,
) {
    let push_settings = PushSettings {
        stop_condition: Acknowledgement::Weak,
        retry_schedule: Arc::new(CappedExponential::new(
            Duration::from_secs(1),
            2.,
            Duration::from_secs(10),
        )),
    };
    let settings: BestEffortSettings = BestEffortSettings { push_settings };
    let mut peers_ready: Vec<Identity> = ready_replies.into_keys().collect();
    let mut peers_delivery: Vec<Identity> = delivery_replies.into_keys().collect();
    peers_ready.append(&mut peers_delivery);
    let msg = Message::new(5, String::from("ReadySubscription"));
    let signature = keychain.sign(&ReadySubscription(msg.clone())).unwrap();
    let signed_msg = SignedMessage::new(msg, signature);
    let best_effort = BestEffort::new(
        node_sender.clone(),
        peers_ready,
        signed_msg,
        settings.clone(),
    );
    best_effort.complete().await;
}

/// Deliver a ReadySubscription type message. Send all the messages which are ready to the subscribing Node.
///
/// # Arguments
///
/// * `node_sender` - The Node's Sender used to send messages.
/// * `from` - The Identity of the Node subscribing.
/// * `ready_messages` - Vector of all messages which are ready.
/// * `ready_peers` - The Atomic Reference Counter the Ready peers to update.
///
pub async fn ready_subscription(
    keychain: KeyChain,
    id: usize,
    node_sender: Sender<SignedMessage>,
    from: Identity,
    ready_messages: Vec<Message>,
    ready_peers: Arc<Mutex<Vec<Identity>>>,
) {
    for msg in ready_messages.into_iter() {
        let signature = keychain.sign(&Ready(msg.clone())).unwrap();
        let signed_msg = SignedMessage::new(msg.clone(), signature);
        let r = node_sender.send(from, signed_msg).await;
        match r {
            Ok(_) => {}
            Err(e) => {
                println!("{} ERROR : ready_subscription send : {}", id, e);
            }
        }
    }
    let mut locked_ready_peers = ready_peers.lock().await;
    locked_ready_peers.push(from);
    drop(locked_ready_peers);
}

/// Probabilistic Consistent Broadcast Deliver. If the message is verified, send a Ready of the message to the
/// Ready peers.
///
/// # Arguments
///
/// * `content` - The content of the message (Without the prepending type).
/// * `node_sender` - The Node's Sender used to send the Gossip Subscription to the peers.
/// * `ready_peers` - The Atomic Reference Counter the Ready peers.
///
pub async fn deliver(
    keychain: KeyChain,
    content: Message,
    node_sender: Sender<SignedMessage>,
    ready_peers: Vec<Identity>,
) {
    // crypto.verify(msg)
    for r in ready_peers.into_iter() {
        let msg: Message = Message::new(2, content.content.clone());
        let signature = keychain.sign(&Ready(msg.clone())).unwrap();
        let signed_msg = SignedMessage::new(msg, signature);
        let r = node_sender.send(r.clone(), signed_msg).await;
        match r {
            Ok(_) => {}
            Err(e) => {
                println!("ERROR : deliver send : {}", e);
            }
        }
    }
}

/// Deliver a Ready type message. Check if the sending Node is in the Ready peers and/or Delivery peers,
/// and update the corresponding replies if it is.
///
/// # Arguments
///
/// * `content` - The content of the message (Without the prepending type).
/// * `from` - The Identity of the Node sending the Ready.
/// * `ready_peers` - The Atomic Reference Counter the Ready peers.
/// * `delivery_peers` - The Atomic Reference Counter the Delivery peers.
/// * `ready_replies` - The Atomic Reference Counter to the Ready replies from the chosen peers.
/// * `delivery_replies` - The Atomic Reference Counter to the Delivery replies from the chosen peers.
///
pub async fn deliver_ready(
    keychain: KeyChain,
    kc: KeyCard,
    id: usize,
    signed_msg: SignedMessage,
    from: Identity,
    ready_peers: Vec<Identity>,
    ready_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    delivery_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    node_sender: Sender<SignedMessage>,
    ready_messages: Arc<Mutex<Vec<Message>>>,
    r_thr: usize,
    d_thr: usize,
    delivered: Arc<Mutex<Option<Message>>>,
) {
    let correct = signed_msg
        .clone()
        .get_signature()
        .verify(&kc, &Ready(signed_msg.clone().get_message()));
    if correct.is_ok() {
        let rp: Vec<Identity> = ready_replies.lock().await.clone().into_keys().collect();
        let new_reply: Option<Message> = Some(signed_msg.clone().get_message());
        if rp.contains(&from) {
            drop(rp);
            let mut locked_ready_replies = ready_replies.lock().await;
            locked_ready_replies.insert(from, new_reply.clone());
            drop(locked_ready_replies);
            let ready_replies: HashMap<Identity, Option<Message>> =
                ready_replies.lock().await.clone();
            tokio::spawn(async move {
                check_ready(
                    keychain,
                    node_sender,
                    ready_messages,
                    r_thr,
                    ready_peers,
                    ready_replies,
                )
                .await;
            });
        }
        let dp: Vec<Identity> = delivery_replies.lock().await.clone().into_keys().collect();
        if dp.contains(&from) {
            drop(dp);
            let mut locked_delivery_replies = delivery_replies.lock().await;
            locked_delivery_replies.insert(from, new_reply.clone());
            drop(locked_delivery_replies);
            let delivery_replies: HashMap<Identity, Option<Message>> =
                delivery_replies.lock().await.clone();
            check_delivery(id, d_thr, delivered, delivery_replies).await;
        }
    }
}

/// Check the status of the Ready replies received. If more than the threshold have been
/// received, add the message to the messages which are Ready. And send it as a Ready message to the Ready peers.
///
/// # Arguments
///
/// * `node_sender` - The Node's Sender used to send the Gossip Subscription to the peers.
/// * `ready_messages` - The Atomic Reference Counter to vector of all messages which are ready.
/// * `e_thr` - The threshold defining if enough Echo replies have been received.
/// * `ready_peers` - The Atomic Reference Counter the Ready peers.
/// * `ready_replies` - The Atomic Reference Counter to the Ready replies from the chosen peers.
///
async fn check_ready(
    keychain: KeyChain,
    node_sender: Sender<SignedMessage>,
    ready_messages: Arc<Mutex<Vec<Message>>>,
    r_thr: usize,
    ready_peers: Vec<Identity>,
    ready_replies: HashMap<Identity, Option<Message>>,
) {
    let occ = check_message_occurrences(ready_replies);
    for m in occ {
        if m.1 >= r_thr {
            let msg = Message::new(2, m.0.clone());
            let signature = keychain.sign(&Ready(msg.clone())).unwrap();
            let mut locked_ready_replies = ready_messages.lock().await;
            locked_ready_replies.push(msg.clone());
            drop(locked_ready_replies);

            let push_settings = PushSettings {
                stop_condition: Acknowledgement::Weak,
                retry_schedule: Arc::new(CappedExponential::new(
                    Duration::from_secs(1),
                    2.,
                    Duration::from_secs(10),
                )),
            };
            let signed_msg = SignedMessage::new(msg, signature);
            let settings: BestEffortSettings = BestEffortSettings { push_settings };
            let best_effort = BestEffort::new(
                node_sender.clone(),
                ready_peers.clone(),
                signed_msg.clone(),
                settings.clone(),
            );
            best_effort.complete().await;
        }
    }
}

/// Check the status of the Delivery replies received. If more than the threshold have been
/// received Probabilistic Reliable Broadcast Deliver the message.
///
/// # Arguments
///
/// * `d_thr` - The threshold defining if enough Delivery replies have been received.
/// * `delivered` - The Atomic Reference Counter the delivered message.
/// * `delivery_replies` - The Atomic Reference Counter to the Delivery replies from the chosen peers.
///
async fn check_delivery(
    id: usize,
    d_thr: usize,
    delivered: Arc<Mutex<Option<Message>>>,
    delivery_replies: HashMap<Identity, Option<Message>>,
) {
    let mut locked_delivered = delivered.lock().await;
    if locked_delivered.is_none() {
        let occ = check_message_occurrences(delivery_replies);

        for m in occ {
            if m.1 >= d_thr {
                my_print!(format!("{} delivered : {}", id, m.0.clone()));
                let msg = Message::new(2, m.0.clone());
                let msg: Option<Message> = Some(msg);
                *locked_delivered = msg.clone();
                drop(locked_delivered);
                prb_deliver(m.0.clone(), id.to_string()).await;
                break;
            }
        }
    }
}

/// Probabilistic Reliable Broadcast Deliver. Save the delivered message in a file with a unique name.
/// This is used to see which Nodes have delivered which message.
///
/// # Arguments
///
/// * `content` - The content of the message (Without the prepending type).
/// * `uid` - Unique ID used to name the file and identify which Node has delivered which message.
///
pub async fn prb_deliver(content: String, uid: String) {
    // *** Optional lines used to verify delivery of messages. ***
    loop {
        let file = File::create(format!("check/tmp_{}.txt", uid));
        match file {
            Ok(mut f) => {
                loop {
                    let r = write!(f, "DELIVERED : {}", content);
                    match r {
                        Ok(_) => {
                            break;
                        }
                        Err(e) => {
                            println!("ERROR : prb_deliver write : {}", e);
                        }
                    }
                }
                break;
            }
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}
