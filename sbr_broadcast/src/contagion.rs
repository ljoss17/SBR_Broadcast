use crate::utils::{build_message_with_type, check_message_occurrences, sample};

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use talk::crypto::Identity;
use talk::unicast::Sender;

pub async fn init(
    r: u32,
    d: u32,
    system: Vec<Identity>,
    sender: Sender<String>,
    ready_peers: &Arc<Mutex<Vec<Identity>>>,
    delivery_peers: &Arc<Mutex<Vec<Identity>>>,
) {
    sample(
        String::from("EchoSubscription"),
        sender.clone(),
        r,
        system.clone(),
        ready_peers,
    )
    .await;
    sample(
        String::from("EchoSubscription"),
        sender.clone(),
        d,
        system.clone(),
        delivery_peers,
    )
    .await;
}

pub async fn ready_subscription(
    ready_messages: Vec<String>,
    from: Identity,
    sender: Sender<String>,
    ready_peers: &Arc<Mutex<Vec<Identity>>>,
) {
    for msg in ready_messages.into_iter() {
        sender.send(from, msg).await;
    }
    let mut locked_ready_peers = ready_peers.lock().unwrap();
    locked_ready_peers.push(from);
    drop(locked_ready_peers);
}

pub async fn deliver(
    content: String,
    sender: Sender<String>,
    ready_peers: &Arc<Mutex<Vec<Identity>>>,
) {
    // crypto.verify(msg)
    let copy_ready_peers: Vec<Identity> = ready_peers.lock().unwrap().clone();
    for r in copy_ready_peers.into_iter() {
        let msg: String = build_message_with_type(String::from("Ready"), content.clone());
        sender.send(r.clone(), msg).await;
    }
}

pub fn deliver_ready(
    content: String,
    from: Identity,
    ready_peers: &Arc<Mutex<Vec<Identity>>>,
    delivery_peers: &Arc<Mutex<Vec<Identity>>>,
    ready_replies: &Arc<Mutex<HashMap<Identity, Option<String>>>>,
    delivery_replies: &Arc<Mutex<HashMap<Identity, Option<String>>>>,
) {
    let new_reply: Option<String> = Some(content.clone());
    // crypto.verify(msg)
    if ready_peers.lock().unwrap().contains(&from) {
        let locked_ready_replies = ready_replies.lock().unwrap();
        locked_ready_replies.insert(from, new_reply.clone());
        drop(locked_ready_replies);
    }
    if delivery_peers.lock().unwrap().contains(&from) {
        let locked_delivery_replies = delivery_replies.lock().unwrap();
        locked_delivery_replies.insert(from, new_reply.clone());
        drop(locked_delivery_replies);
    }
}

async fn check_ready(
    sender: Sender<String>,
    ready_replies: Arc<Mutex<HashMap<Identity, Option<String>>>>,
    ready_messages: Arc<Mutex<Vec<String>>>,
    r_thr: usize,
    ready_peers: Arc<Mutex<Vec<Identity>>>,
) {
    let cloned_ready_replies: HashMap<Identity, Option<String>> =
        ready_replies.lock().unwrap().clone();
    let occ = check_message_occurrences(cloned_ready_replies);
    for m in occ {
        if m.1 >= r_thr {
            let mut locked_ready_replies = ready_messages.lock().unwrap();
            locked_ready_replies.push(m.0.clone());
            drop(locked_ready_replies);
            let cloned_ready_peers: Vec<Identity> = ready_peers.lock().unwrap().clone();
            for r in cloned_ready_peers.into_iter() {
                let msg: String = build_message_with_type(String::from("Ready"), m.0.clone());
                sender.send(r.clone(), msg).await;
            }
        }
    }
}

fn check_delivery(
    delivery_replies: Arc<Mutex<HashMap<Identity, Option<String>>>>,
    d_thr: usize,
    delivered: Arc<Mutex<Option<String>>>,
) {
    if delivered.lock().unwrap().is_none() {
        let cloned_delivery_replies: HashMap<Identity, Option<String>> =
            delivery_replies.lock().unwrap().clone();
        let occ = check_message_occurrences(cloned_delivery_replies);
        for m in occ {
            if m.1 >= d_thr {
                let msg: Option<String> = Some(m.0.clone());
                let locked_delivered = delivered.lock().unwrap();
                *locked_delivered = msg.clone();
                drop(locked_delivered);
                prb_deliver(m.0.clone());
            }
        }
    }
}

pub fn prb_deliver(content: String) {
    // *** Optional lines used to verify delivery of messages. ***
    loop {
        let file = File::create(format!("check/tmp.txt"));
        match file {
            Ok(mut f) => {
                loop {
                    write!(f, "DELIVERED : {}", content);
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
