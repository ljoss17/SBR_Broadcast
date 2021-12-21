use crate::message::Message;
use rand::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use talk::crypto::{Identity, KeyCard};
use tokio::sync::Mutex;

/// Sample randomly a number of peers from the given system and initialise the replies HashMap for these peers.
///
/// # Arguments
///
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
}

/// Extract the occurences of Messages in a HashMap<Identity, Option<Message>>.
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
