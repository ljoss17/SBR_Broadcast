use crate::message::Message;
use crate::utils::{check_message_occurrences, sample};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use talk::broadcast::{BestEffort, BestEffortSettings};
use talk::crypto::{Identity, KeyCard};
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
    node_sender: Sender<Message>,
    ready_replies: HashMap<Identity, Option<Message>>,
    delivery_replies: HashMap<Identity, Option<Message>>,
) {
    let push_settings = PushSettings {
        stop_condition: Acknowledgement::Weak,
        retry_schedule: Arc::new(CappedExponential::new(
            Duration::from_secs(1),
            2.,
            Duration::from_secs(180),
        )),
    };
    let settings: BestEffortSettings = BestEffortSettings { push_settings };
    let peers_ready: Vec<Identity> = ready_replies.into_keys().collect();
    let peers_delivery: Vec<Identity> = delivery_replies.into_keys().collect();
    let msg = Message::new(5, String::from("ReadySubscription"));
    let best_effort = BestEffort::new(
        node_sender.clone(),
        peers_ready,
        msg.clone(),
        settings.clone(),
    );
    best_effort.complete().await;
    let msg = Message::new(5, String::from("DeliverySubscription"));
    let best_effort = BestEffort::new(
        node_sender.clone(),
        peers_delivery,
        msg.clone(),
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
    node_sender: Sender<Message>,
    from: Identity,
    ready_messages: Arc<Mutex<Vec<Message>>>,
    ready_peers: Arc<Mutex<Vec<Identity>>>,
) {
    for msg in ready_messages.lock().await.clone().into_iter() {
        let r = node_sender.send(from, msg.clone()).await;
        match r {
            Ok(_) => {}
            Err(e) => {
                println!("ERROR : ready_subscription send : {}", e);
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
    content: Message,
    node_sender: Sender<Message>,
    ready_peers: Arc<Mutex<Vec<Identity>>>,
) {
    // crypto.verify(msg)
    for r in ready_peers.lock().await.clone().into_iter() {
        let msg: Message = Message::new(2, content.content.clone());
        let r = node_sender.send(r.clone(), msg).await;
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
    id: usize,
    content: Message,
    from: Identity,
    ready_peers: Arc<Mutex<Vec<Identity>>>,
    ready_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    delivery_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    node_sender: Sender<Message>,
    ready_messages: Arc<Mutex<Vec<Message>>>,
    r_thr: usize,
    d_thr: usize,
    delivered: Arc<Mutex<Option<Message>>>,
) {
    let new_reply: Option<Message> = Some(content.clone());
    // crypto.verify(msg)
    let rp: Vec<Identity> = ready_replies.lock().await.clone().into_keys().collect();
    if rp.contains(&from) {
        drop(rp);
        let mut should_check: bool = false;
        if ready_replies.lock().await.get(&from).unwrap().is_none() {
            should_check = true;
        }
        let mut locked_ready_replies = ready_replies.lock().await;
        locked_ready_replies.insert(from, new_reply.clone());
        drop(locked_ready_replies);
        if should_check {
            check_ready(
                node_sender,
                ready_messages,
                r_thr,
                ready_peers,
                ready_replies,
            )
            .await;
        }
    }
    let dp: Vec<Identity> = delivery_replies.lock().await.clone().into_keys().collect();
    if dp.contains(&from) {
        drop(dp);
        let mut should_check: bool = false;
        if delivery_replies.lock().await.get(&from).unwrap().is_none() {
            should_check = true;
        }
        let mut locked_delivery_replies = delivery_replies.lock().await;
        locked_delivery_replies.insert(from, new_reply.clone());
        drop(locked_delivery_replies);
        if should_check {
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
    node_sender: Sender<Message>,
    ready_messages: Arc<Mutex<Vec<Message>>>,
    r_thr: usize,
    ready_peers: Arc<Mutex<Vec<Identity>>>,
    ready_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
) {
    let cloned_ready_replies: HashMap<Identity, Option<Message>> =
        ready_replies.lock().await.clone();
    let occ = check_message_occurrences(cloned_ready_replies);
    for m in occ {
        if m.1 >= r_thr {
            let msg = Message::new(2, m.0.clone());
            let mut locked_ready_replies = ready_messages.lock().await;
            locked_ready_replies.push(msg.clone());
            drop(locked_ready_replies);
            let cloned_ready_peers: Vec<Identity> = ready_peers.lock().await.clone();
            for r in cloned_ready_peers.into_iter() {
                let r = node_sender.send(r.clone(), msg.clone()).await;
                match r {
                    Ok(_) => {}
                    Err(e) => {
                        println!("ERROR : check_ready send : {}", e);
                    }
                }
            }
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
    delivery_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
) {
    if delivered.lock().await.is_none() {
        let cloned_delivery_replies: HashMap<Identity, Option<Message>> =
            delivery_replies.lock().await.clone();
        let occ = check_message_occurrences(cloned_delivery_replies);
        for m in occ {
            if m.1 >= d_thr {
                println!("{} delivered : {}", id, m.0.clone());
                let msg = Message::new(2, m.0.clone());
                let msg: Option<Message> = Some(msg);
                let mut locked_delivered = delivered.lock().await;
                *locked_delivered = msg.clone();
                drop(locked_delivered);
                prb_deliver(m.0.clone(), id.to_string());
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
pub fn prb_deliver(content: String, uid: String) {
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
                    /*
                    let wf = f.write(b"DELIVERED");
                    match wf {
                        Ok(_) => {
                            break;
                        }
                        Err(_) => thread::sleep(Duration::from_secs(1)),
                    }
                    */
                }
                //break;
            }
            Err(_) => thread::sleep(Duration::from_secs(1)),
        }
    }
}
