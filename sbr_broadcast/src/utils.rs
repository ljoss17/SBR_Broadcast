use crate::message::Message;
use rand::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use talk::crypto::{Identity, KeyCard};
use tokio::sync::Mutex;

/// Sample randomly a number of peers from the given system and initialise the replies HashMap for these peers.
/// Send a message to the selected peers.
///
/// # Arguments
///
/// * `msg` - The message to send to the peers, e.g. a Subscription message.
/// * `node_sender` - The Node's Sender used to send messages.
/// * `size` - The number of peers to select randomly.
/// * `system` - The system in which the peers are selected.
/// * `replies` - The Atomic Reference Counter to the replies to initialise.
///
pub async fn sample(
    size: usize,
    system: Vec<KeyCard>,
    replies: &Arc<Mutex<HashMap<Identity, Option<Message>>>>,
) {
    let num_proc: usize = system.len();
    let mut selected: HashMap<Identity, Option<Message>> = HashMap::new();
    for _ in 1..=size {
        let mut rng = rand::thread_rng();
        let n = rng.gen_range(0..num_proc);
        drop(rng);
        selected.insert(system[n].identity().clone(), None);
    }
    let mut locked_psi = replies.lock().await;
    locked_psi.extend(selected);
    drop(locked_psi);
    //let msg = Message::new(msg_type, String::from("Subscription"));
    //let cloned_psi: HashMap<Identity, Option<Message>> = replies.lock().await.clone();
    /*for i in cloned_psi.into_iter() {
        node_sender.send(i.0.clone(), msg.clone()).await;
    }*/

    /*let settings: BestEffortSettings = BestEffortSettings {
        push_settings: PushSettings::default(),
    };
    println!("SAMPLE BROADCAST SETUP");
    let best_effort = BestEffort::new(node_sender.clone(), peers, msg.clone(), settings);
    println!("WILL WAIT FOR BROADCAST");
    best_effort.complete().await;
    println!("SAMPLE BROADCAST DONE");*/
}

/// Extract the type and content of a message.
/// * `Gossip message`: 0
/// * `Echo message`: 1
/// * `Ready message`: 2
/// * `GossipSubscription message`: 3
/// * `EchoSubscription message`: 4
/// * `ReadySubscription message`: 5
/// * Everything else: u32::MAX
///
/// # Arguments
///
/// * `msg` - The message to parse.
///
/*pub fn get_message_type(msg: Message) -> (u32, String) {
    (msg.message_type, msg.content)
}*/

/// Build a message of a given type.
///
/// # Arguments
///
/// * `msg_type` - The message type.
/// * `content` - The content of the message.
///
/*pub fn build_message_with_type(msg_type: u32, content: String) -> Message {
    Message::new(msg_type, content)
}*/

/// Extract the occurences of messages in a HashMap<Identity, Option<String>>.
///
/// # Arguments
///
/// * `messages` - The HashMap to parsed.
///
pub fn check_message_occurrences(
    messages: HashMap<Identity, Option<Message>>,
) -> HashMap<String, usize> {
    let vec_messages = messages.values().flatten();

    let occurences = vec_messages
        .into_iter()
        .fold(HashMap::<String, usize>::new(), |mut m, x| {
            *m.entry(x.content.to_string()).or_default() += 1;
            m
        });
    occurences
}
