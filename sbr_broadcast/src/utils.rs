use rand::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use talk::crypto::Identity;
use talk::unicast::Sender;

pub async fn sample(
    msg: String,
    sender: Sender<String>,
    size: u32,
    system: Vec<Identity>,
    psi: &Arc<Mutex<Vec<Identity>>>,
) {
    let mut rng = rand::thread_rng();
    let num_proc: usize = system.len();
    for _ in 1..=size {
        let n = rng.gen_range(0..num_proc);
        let mut locked_psi = psi.lock().unwrap();
        locked_psi.push(system[n].clone());
        drop(locked_psi);
    }
    let cloned_psi: Vec<Identity> = psi.lock().unwrap().clone();
    for i in cloned_psi.into_iter() {
        sender.send(i.clone(), msg.clone()).await;
    }
}

pub fn get_message_type(msg: String) -> (u32, String) {
    let mut split_msg = msg.split_whitespace();
    let msg_type = split_msg.next();
    let msg_type = match msg_type {
        Some(t) => t,
        None => "None",
    };
    let type_value = match msg_type {
        "Gossip" => 0,
        "Echo" => 1,
        "Ready" => 2,
        "GossipSubscription" => 3,
        "EchoSubscription" => 4,
        "ReadySubscription" => 5,
        _ => u32::MAX,
    };
    let content = msg.replacen(msg_type, "", 1);
    (type_value, content)
}

pub fn build_message_with_type(msg_type: String, content: String) -> String {
    let mut msg: String = String::new();
    msg.push_str(&msg_type);
    msg.push_str(" ");
    msg.push_str(&content);
    msg
}

pub fn check_message_occurrences(
    messages: HashMap<Identity, Option<String>>,
) -> HashMap<String, usize> {
    let vec_messages = messages.values().flatten();

    let occurences = vec_messages
        .into_iter()
        .fold(HashMap::<String, usize>::new(), |mut m, x| {
            *m.entry(x.to_string()).or_default() += 1;
            m
        });
    occurences
}
