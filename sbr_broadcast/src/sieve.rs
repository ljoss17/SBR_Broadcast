use crate::contagion;
use crate::utils::{build_message_with_type, check_message_occurrences, sample};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use talk::broadcast::{BestEffort, BestEffortSettings};
use talk::crypto::Identity;
use talk::unicast::{PushSettings, Sender};

pub async fn init(
    e: u32,
    system: Vec<Identity>,
    sender: Sender<String>,
    echo_peers: &Arc<Mutex<Vec<Identity>>>,
) {
    sample(
        String::from("EchoSubscription"),
        sender,
        e,
        system,
        echo_peers,
    )
    .await;
}

pub async fn echo_subscription(
    from: Identity,
    sender: Sender<String>,
    echo: &Arc<Mutex<Option<String>>>,
    echo_peers: &Arc<Mutex<Vec<Identity>>>,
) {
    if echo.lock().unwrap().is_some() {
        let echo_content: String = echo.lock().unwrap().clone().unwrap();
        let msg: String = build_message_with_type(String::from("Echo"), echo_content);
        sender.send(from, msg).await;
    }
    let mut locked_echo_peers = echo_peers.lock().unwrap();
    locked_echo_peers.push(from);
    drop(locked_echo_peers);
}

pub async fn deliver(
    content: String,
    sender: Sender<String>,
    echo: &Arc<Mutex<Option<String>>>,
    echo_peers: &Arc<Mutex<Vec<Identity>>>,
) {
    // crypto.verify(msg)
    let recv_msg = Some(content.clone());
    let mut locked_echo = echo.lock().unwrap();
    *locked_echo = recv_msg;
    drop(locked_echo);
    let msg: String = build_message_with_type(String::from("Echo"), content);
    let settings: BestEffortSettings = BestEffortSettings {
        push_settings: PushSettings::default(),
    };
    let echo_peers: Vec<Identity> = echo_peers.lock().unwrap().clone();
    let best_effort = BestEffort::new(sender, echo_peers, msg.clone(), settings);
    best_effort.complete().await;
}

pub fn deliver_echo(
    content: String,
    from: Identity,
    replies: &Arc<Mutex<HashMap<Identity, Option<String>>>>,
) {
    if replies.lock().unwrap().contains_key(&from) {
        if replies.lock().unwrap().get(&from).is_none() {
            let new_reply: Option<String> = Some(content.clone());
            let mut locked_replies = replies.lock().unwrap();
            locked_replies.insert(from, new_reply);
            drop(locked_replies);
        }
    }
}

pub async fn check_echoes(
    sender: Sender<String>,
    echo_replies: &Arc<Mutex<HashMap<Identity, Option<String>>>>,
    delivered_echo: &Arc<Mutex<Option<String>>>,
    e_thr: usize,
    ready_peers: &Arc<Mutex<Vec<Identity>>>,
) {
    if delivered_echo.lock().unwrap().is_none() {
        let copy_echo_replies: HashMap<Identity, Option<String>> =
            echo_replies.lock().unwrap().clone();
        let occ = check_message_occurrences(copy_echo_replies);
        for m in occ {
            if m.1 >= e_thr {
                let echo = Some(m.0.clone());
                let mut locked_delivered_echo = delivered_echo.lock().unwrap();
                *locked_delivered_echo = echo.clone();
                drop(locked_delivered_echo);
                contagion::deliver(m.0.clone(), sender, ready_peers).await;
                return;
            }
        }
    }
}
