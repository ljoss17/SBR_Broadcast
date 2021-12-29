use crate::message::{Message, SignedMessage};
use crate::message_headers::{Ready, ReadySubscription};
use crate::utils::{check_message_occurrences, sample};
use itertools::Itertools;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use talk::broadcast::{BestEffort, BestEffortSettings};
use talk::crypto::{Identity, KeyCard, KeyChain};
use talk::time::sleep_schedules::Constant;
use talk::unicast::{Acknowledgement, PushSettings, Sender};
use tokio::sync::Mutex;

/// Initialises the Ready set and Delivery set used in the Contagion algorithm. Sample randomly a number
///  of peers from the system and send them an ReadySubscription.
///
/// # Arguments
///
/// * `r` - The number of Ready peers.
/// * `d` - The number of Delivery peers.
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

/// Send ReadySubscription to Ready and Delivery peers.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `node_sender` - The Node's Sender used to send Messages.
/// * `ready_replies` - The Ready replies used to get the Ready peers.
/// * `delivery_replies` - The Delivery replies used to get the Delivery peers.
///
pub async fn ready_subscribe(
    keychain: KeyChain,
    node_sender: Sender<SignedMessage>,
    ready_replies: HashMap<Identity, Option<Message>>,
    delivery_replies: HashMap<Identity, Option<Message>>,
) {
    let push_settings = PushSettings {
        stop_condition: Acknowledgement::Strong,
        retry_schedule: Arc::new(Constant::new(Duration::from_millis(100))),
    };
    let settings: BestEffortSettings = BestEffortSettings { push_settings };
    let mut peers_ready: Vec<Identity> = ready_replies.into_keys().collect();
    let mut peers_delivery: Vec<Identity> = delivery_replies.into_keys().collect();
    peers_ready.append(&mut peers_delivery);
    let peers_ready: Vec<Identity> = peers_ready.into_iter().unique().collect::<Vec<_>>();
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
    my_print!("Finished Contagion Subscriptions");
}

/// Deliver a ReadySubscription type Message. Send all the Messages which are ready to the subscribing Node.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `id` - The id of the running Node, used for debug purpose.
/// * `node_sender` - The Node's Sender used to send Messages.
/// * `from` - The Identity of the Node subscribing.
/// * `ready_messages` - Vector of all Messages which are ready.
/// * `ready_subscribers` - The Atomic Reference Counter the Ready peers subscribed to this Node.
///
pub async fn ready_subscription(
    keychain: KeyChain,
    id: usize,
    node_sender: Sender<SignedMessage>,
    from: Identity,
    ready_messages: Vec<Message>,
    ready_subscribers: Arc<Mutex<Vec<Identity>>>,
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
    let mut locked_ready_subscribers = ready_subscribers.lock().await;
    locked_ready_subscribers.push(from);
    drop(locked_ready_subscribers);
}

/// Probabilistic Consistent Broadcast Deliver. If the Message is verified, send a Ready of the Message to the
/// Ready peers.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `message` - The received Message.
/// * `node_sender` - The Node's Sender used to send the Gossip Subscription to the peers.
/// * `ready_subscribers` - The Atomic Reference Counter the Ready peers subscribed to this Node.
///
pub async fn deliver(
    keychain: KeyChain,
    message: Message,
    node_sender: Sender<SignedMessage>,
    ready_subscribers: Vec<Identity>,
    ready_messages: Arc<Mutex<Vec<Message>>>,
) {
    let mut locked_ready_replies = ready_messages.lock().await;
    locked_ready_replies.push(message.clone());
    drop(locked_ready_replies);
    let msg: Message = Message::new(2, message.content.clone());
    let signature = keychain.sign(&Ready(msg.clone())).unwrap();
    let signed_msg = SignedMessage::new(msg, signature);
    let push_settings = PushSettings {
        stop_condition: Acknowledgement::Strong,
        retry_schedule: Arc::new(Constant::new(Duration::from_millis(100))),
    };
    let settings: BestEffortSettings = BestEffortSettings { push_settings };
    let best_effort = BestEffort::new(
        node_sender.clone(),
        ready_subscribers.clone(),
        signed_msg.clone(),
        settings.clone(),
    );
    best_effort.complete().await;
}

/// Deliver a Ready type Message. Check if the sending Node is in the Ready peers and/or Delivery peers,
/// and update the corresponding replies if it is.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `kc` - The KeyCard of the sender, used to verify the Message signature.
/// * `id` - The id of the running Node, used for debug purpose.
/// * `signed_msg` - The signed Message to deliver.
/// * `from` - The Identity of the Node sending the Ready.
/// * `ready_subscribers` - The Ready peers subscribed to this Node.
/// * `ready_replies` - The Atomic Reference Counter the Ready replies to update.
/// * `delivery_replies` - The Atomic Reference Counter the Delivery replies to update.
/// * `node_sender` - The Node's Sender used to send the Gossip Subscription to the peers.
/// * `ready_messages` - The Atomic Reference Counter to the Messages which are Ready.
/// * `r_thr` - The threshold defining if enough Ready replies have been received.
/// * `d_thr` - The threshold defining if enough Delivery replies have been received.
/// * `delivered` - The Atomic Reference Counter to the delivered Message.
///
pub async fn deliver_ready(
    keychain: KeyChain,
    id: usize,
    signed_msg: SignedMessage,
    from: Identity,
    ready_subscribers: Vec<Identity>,
    ready_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    delivery_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    node_sender: Sender<SignedMessage>,
    ready_messages: Arc<Mutex<Vec<Message>>>,
    r_thr: usize,
    d_thr: usize,
    delivered: Arc<Mutex<Option<Message>>>,
) {
    let rp: Vec<Identity> = ready_replies.lock().await.clone().into_keys().collect();
    let new_reply: Option<Message> = Some(signed_msg.clone().get_message());
    if rp.contains(&from) {
        let mut new_ready: bool = false;
        if ready_replies.lock().await.clone()[&from].is_none() {
            new_ready = true;
        }
        drop(rp);
        let mut locked_ready_replies = ready_replies.lock().await;
        locked_ready_replies.insert(from, new_reply.clone());
        drop(locked_ready_replies);
        let ready_replies: HashMap<Identity, Option<Message>> = ready_replies.lock().await.clone();
        if new_ready {
            tokio::spawn(async move {
                check_ready(
                    keychain,
                    from.clone(),
                    node_sender,
                    ready_messages,
                    r_thr,
                    ready_subscribers,
                    ready_replies,
                )
                .await;
            });
        }
    }
    let dp: Vec<Identity> = delivery_replies.lock().await.clone().into_keys().collect();
    if dp.contains(&from) {
        let mut new_delivery: bool = false;
        if delivery_replies.lock().await.clone()[&from].is_none() {
            new_delivery = true;
        }
        drop(dp);
        let mut locked_delivery_replies = delivery_replies.lock().await;
        locked_delivery_replies.insert(from, new_reply.clone());
        drop(locked_delivery_replies);
        let delivery_replies: HashMap<Identity, Option<Message>> =
            delivery_replies.lock().await.clone();
        if new_delivery {
            check_delivery(id, from, d_thr, delivered, delivery_replies).await;
        }
    }
}

/// Check the status of the Ready replies received. If more than the threshold have been
/// received, add the Message to the Messages which are Ready. And send it as a Ready Message to the Ready peers.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `node_sender` - The Node's Sender used to send the Gossip Subscription to the peers.
/// * `ready_messages` - The Atomic Reference Counter to vector of all Messages which are ready.
/// * `r_thr` - The threshold defining if enough Ready replies have been received.
/// * `ready_subscribers` - The Ready peers subscribed to this Node.
/// * `ready_replies` - The Ready replies from the chosen peers.
///
async fn check_ready(
    keychain: KeyChain,
    from: Identity,
    node_sender: Sender<SignedMessage>,
    ready_messages: Arc<Mutex<Vec<Message>>>,
    r_thr: usize,
    ready_subscribers: Vec<Identity>,
    ready_replies: HashMap<Identity, Option<Message>>,
) {
    if ready_replies
        .clone()
        .into_keys()
        .collect::<Vec<Identity>>()
        .contains(&from)
    {
        let occ = check_message_occurrences(ready_replies);
        for m in occ {
            if m.1 >= r_thr {
                let msg = Message::new(2, m.0.clone());
                let signature = keychain.sign(&Ready(msg.clone())).unwrap();
                let mut locked_ready_replies = ready_messages.lock().await;
                locked_ready_replies.push(msg.clone());
                drop(locked_ready_replies);

                let push_settings = PushSettings {
                    stop_condition: Acknowledgement::Strong,
                    retry_schedule: Arc::new(Constant::new(Duration::from_millis(100))),
                };
                let signed_msg = SignedMessage::new(msg, signature);
                let settings: BestEffortSettings = BestEffortSettings { push_settings };
                let best_effort = BestEffort::new(
                    node_sender.clone(),
                    ready_subscribers.clone(),
                    signed_msg.clone(),
                    settings.clone(),
                );
                best_effort.complete().await;
            }
        }
    }
}

/// Check the status of the Delivery replies received. If more than the threshold have been
/// received Probabilistic Reliable Broadcast Deliver the Message.
///
/// # Arguments
///
/// * `id` - The id of the running Node, used for debug purpose.
/// * `d_thr` - The threshold defining if enough Delivery replies have been received.
/// * `delivered` - The Atomic Reference Counter the delivered Message.
/// * `delivery_replies` - The Delivery replies from the chosen peers.
///
async fn check_delivery(
    id: usize,
    from: Identity,
    d_thr: usize,
    delivered: Arc<Mutex<Option<Message>>>,
    delivery_replies: HashMap<Identity, Option<Message>>,
) {
    if delivery_replies
        .clone()
        .into_keys()
        .collect::<Vec<Identity>>()
        .contains(&from)
    {
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
}

/// Probabilistic Reliable Broadcast Deliver. Save the delivered Message in a file with a unique name.
/// This is used to see which Nodes have delivered which Message.
///
/// # Arguments
///
/// * `message` - The Message to deliver.
/// * `uid` - Unique ID used to name the file and identify which Node has delivered which Message.
///
pub async fn prb_deliver(message: String, uid: String) {
    // *** Optional lines used to verify delivery of messages. ***
    loop {
        let file = File::create(format!("check/tmp_{}.txt", uid));
        match file {
            Ok(mut f) => {
                loop {
                    let r = write!(f, "DELIVERED : {}", message);
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
