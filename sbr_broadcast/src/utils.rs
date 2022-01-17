use crate::message::Message;
use itertools::Itertools;
use rand::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use talk::crypto::{Identity, KeyCard};
use tokio::sync::Mutex;

/// Sample randomly a number of peers from the given system and initialise the replies HashMap for these peers.
/// Specific for Sieve because only the pb.delivered messsage is checked for Echos.
///
/// # Arguments
///
/// * `size` - The number of peers to select randomly.
/// * `system` - The system in which the peers are selected.
/// * `replies` - The Atomic Reference Counter to the replies, which will be initialised.
/// * `duplicates` - The reference to the HashMap containing peers sampled multiple times, which will be initialised.
///
pub async fn sample_sieve(
    size: usize,
    system: Vec<KeyCard>,
    replies: &Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    duplicates: &mut HashMap<Identity, usize>,
) {
    let num_proc: usize = system.len();
    let mut selected: HashMap<Identity, Option<Message>> = HashMap::new();
    let mut selected_duplicates: HashMap<Identity, usize> = HashMap::new();
    for _ in 1..=size {
        let mut rng = rand::thread_rng();
        let n = rng.gen_range(0..num_proc);
        drop(rng);
        selected.insert(system[n].identity().clone(), None);
        *selected_duplicates
            .entry(system[n].identity().clone())
            .or_insert(1) += 1;
    }
    let mut locked_psi = replies.lock().await;
    locked_psi.extend(selected);
    drop(locked_psi);
    duplicates.extend(selected_duplicates);
}

/// Sample randomly a number of peers from the given system and initialise the replies HashMap for these peers.
/// Specific for Contagion because a process can be Ready for multiple messages.
///
/// # Arguments
///
/// * `size` - The number of peers to select randomly.
/// * `system` - The system in which the peers are selected.
/// * `replies` - The Atomic Reference Counter to the replies to initialise.
/// * `duplicates` - The reference to the HashMap containing peers sampled multiple times, which will be initialised.
///
pub async fn sample_contagion(
    size: usize,
    system: Vec<KeyCard>,
    replies: &Arc<Mutex<HashMap<Identity, Vec<Message>>>>,
    duplicates: &mut HashMap<Identity, usize>,
) {
    let num_proc: usize = system.len();
    let mut selected: HashMap<Identity, Vec<Message>> = HashMap::new();
    let mut selected_duplicates: HashMap<Identity, usize> = HashMap::new();
    for _ in 1..=size {
        let mut rng = rand::thread_rng();
        let n = rng.gen_range(0..num_proc);
        drop(rng);
        selected.insert(system[n].identity().clone(), Vec::new());
        *selected_duplicates
            .entry(system[n].identity().clone())
            .or_insert(1) += 1;
    }
    let mut locked_psi = replies.lock().await;
    locked_psi.extend(selected);
    drop(locked_psi);
    duplicates.extend(selected_duplicates);
}

/// Extract the occurences of Messages in a HashMap<Identity, Option<Message>>.
/// Specific for Sieve because only the pb.delivered messsage is checked for Echos.
///
/// # Arguments
///
/// * `messages` - The HashMap to parsed.
/// * `duplicates` - The HashMap containing peers sampled multiple times.
/// * `echo` - The Atomic Reference Counter to the message pb.delivered.
///
pub async fn check_message_occurrences_sieve(
    messages: HashMap<Identity, Option<Message>>,
    duplicates: HashMap<Identity, usize>,
    echo: Arc<Mutex<Option<Message>>>,
) -> usize {
    let mut ids: Vec<Identity> = Vec::new();
    let echo: Message = echo.lock().await.as_ref().unwrap().clone();
    for v in messages.into_iter() {
        if v.1.is_some() {
            if v.1.unwrap().content == echo.content {
                ids.push(v.0.clone());
            }
        }
    }
    let mut occ: usize = 0;

    for id in ids {
        if duplicates.contains_key(&id) {
            occ += duplicates.get(&id).unwrap();
        } else {
            occ += 1;
        }
    }
    occ
}

/// Extract the occurences of Messages in a HashMap<Identity, Vec<Message>>.
/// Specific for Contagion because a process can be Ready for multiple messages.
///
/// # Arguments
///
/// * `messages` - The HashMap to parsed.
/// * `duplicates` - The HashMap containing peers sampled multiple times.
///
pub fn check_message_occurrences_contagion(
    messages: HashMap<Identity, Vec<Message>>,
    duplicates: HashMap<Identity, usize>,
) -> HashMap<String, usize> {
    let values: Vec<String> = messages
        .values()
        .flatten()
        .map(|x| x.content.clone())
        .collect();
    let values: Vec<String> = values.into_iter().unique().collect::<Vec<_>>();
    let mut occ: HashMap<String, usize> = HashMap::new();
    for v in values.clone().into_iter() {
        for m in messages.clone().into_iter() {
            for m2 in m.1 {
                if m2.content == v.clone() {
                    if duplicates.contains_key(&m.0) {
                        *occ.entry(v.clone()).or_insert(0) += duplicates.get(&m.0).unwrap();
                    } else {
                        *occ.entry(v.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    occ
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;
    use std::sync::Arc;
    use talk::crypto::Identity;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn sieve_occurrences() {
        let echo: Arc<Mutex<Option<Message>>> =
            Arc::new(Mutex::new(Some(Message::new(2, String::from("Test1")))));
        let msg1: Option<Message> = Some(Message::new(2, String::from("Test1")));
        let msg2: Option<Message> = Some(Message::new(2, String::from("Test2")));
        let mut messages: HashMap<Identity, Option<Message>> = HashMap::new();
        let mut duplicates: HashMap<Identity, usize> = HashMap::new();
        let id1: [u8; 32] = [1; 32];
        let id1: Identity = Identity::from_bytes(id1);
        let id2: [u8; 32] = [2; 32];
        let id2: Identity = Identity::from_bytes(id2);
        let id3: [u8; 32] = [3; 32];
        let id3: Identity = Identity::from_bytes(id3);
        let id4: [u8; 32] = [4; 32];
        let id4: Identity = Identity::from_bytes(id4);
        messages.insert(id1, msg1.clone());
        messages.insert(id2, msg1.clone());
        messages.insert(id3, msg2.clone());
        messages.insert(id4, msg2.clone());
        duplicates.insert(id2, 2);
        duplicates.insert(id3, 3);
        duplicates.insert(id4, 4);
        let res = check_message_occurrences_sieve(messages, duplicates, echo).await;
        assert_eq!(res, 3);
    }

    #[test]
    fn contagion_occurrences() {
        let msg1: Option<Message> = Some(Message::new(2, String::from("Test1")));
        let msg2: Option<Message> = Some(Message::new(2, String::from("Test2")));
        let mut messages: HashMap<Identity, Vec<Message>> = HashMap::new();
        let mut duplicates: HashMap<Identity, usize> = HashMap::new();
        let id1: [u8; 32] = [1; 32];
        let id1: Identity = Identity::from_bytes(id1);
        let id2: [u8; 32] = [2; 32];
        let id2: Identity = Identity::from_bytes(id2);
        let id3: [u8; 32] = [3; 32];
        let id3: Identity = Identity::from_bytes(id3);
        let id4: [u8; 32] = [4; 32];
        let id4: Identity = Identity::from_bytes(id4);
        messages.insert(id1, vec![msg1.clone().unwrap()]);
        messages.insert(id2, vec![msg1.clone().unwrap()]);
        messages.insert(id3, vec![msg1.clone().unwrap(), msg2.clone().unwrap()]);
        messages.insert(id4, vec![msg2.clone().unwrap()]);
        duplicates.insert(id2, 2);
        duplicates.insert(id3, 3);
        duplicates.insert(id4, 4);
        let res = check_message_occurrences_contagion(messages, duplicates);
        let expected: HashMap<String, usize> =
            HashMap::from([(String::from("Test1"), 6), (String::from("Test2"), 7)]);
        assert_eq!(res, expected);
    }
}
