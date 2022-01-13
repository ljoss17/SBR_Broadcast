use crate::contagion;
use crate::message::{Message, SignedMessage};
use crate::message_headers::{Echo, EchoSubscription};
use crate::utils::{check_message_occurrences, sample};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use talk::broadcast::{BestEffort, BestEffortSettings};
use talk::crypto::{Identity, KeyCard, KeyChain};
use talk::time::sleep_schedules::Constant;
use talk::unicast::{Acknowledgement, PushSettings, Sender};
use tokio::sync::Mutex;

/// Initialises the Echo set used in the Sieve algorithm. Sample randomly a number of peers from the system
/// and send them an EchoSubscription.
///
/// # Arguments
///
/// * `e` - The number of Echo peers.
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

/// Send EchoSubscription to Echo peers.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `node_sender` - The Node's Sender used to send Messages.
/// * `echo_replies` - The Echo replies used to get the Echo peers.
///
pub async fn echo_subscribe(
    keychain: KeyChain,
    node_sender: Sender<SignedMessage>,
    echo_replies: HashMap<Identity, Option<Message>>,
) {
    let push_settings = PushSettings {
        stop_condition: Acknowledgement::Strong,
        retry_schedule: Arc::new(Constant::new(Duration::from_millis(100))),
    };
    let settings: BestEffortSettings = BestEffortSettings { push_settings };
    // Collect Identities to which a Subscription is sent.
    let peers: Vec<Identity> = echo_replies.into_keys().collect();
    let msg = Message::new(4, String::from("EchoSubscription"));
    let signature = keychain.sign(&EchoSubscription(msg.clone())).unwrap();
    let signed_msg = SignedMessage::new(msg, signature);
    let best_effort = BestEffort::new(node_sender.clone(), peers, signed_msg, settings);
    best_effort.complete().await;
    my_print!("Finished Sieve Subscriptions");
}

/// Deliver an EchoSubscription type Message. If an Echo Message has already been delivered, send
/// it to the subscribing Node.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `node_sender` - The Node's Sender used to send Messages.
/// * `from` - The Identity of the Node subscribing.
/// * `delivered_echo` - The status of the delivered Echo Message.
/// * `echo_peers` - The Atomic Reference Counter to the Echo peers to update.
///
pub async fn echo_subscription(
    keychain: KeyChain,
    node_sender: Sender<SignedMessage>,
    from: Identity,
    delivered_echo: Option<Message>,
    echo_subscribers: Arc<Mutex<Vec<Identity>>>,
) {
    if delivered_echo.is_some() {
        let echo_message: Message = delivered_echo.unwrap();
        let msg: Message = Message::new(1, echo_message.content.clone());
        let signature = keychain.sign(&Echo(msg.clone())).unwrap();
        let signed_msg = SignedMessage::new(msg, signature);
        loop {
            let r = node_sender.send(from, signed_msg.clone()).await;
            match r {
                Ok(_) => {
                    break;
                }
                Err(e) => {
                    println!("ERROR : echo_subscription send : {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
    let mut locked_echo_subscribers = echo_subscribers.lock().await;
    locked_echo_subscribers.push(from);
    drop(locked_echo_subscribers);
}

/// Probabilistic Broadcast Deliver. If the Message is verified, send an Echo of the Message to the
/// Echo peers.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `signed_msg` - The signed Message to deliver.
/// * `node_sender` - The Node's Sender used to send the Gossip Subscription to the peers.
/// * `delivered_echo` - The Atomic Reference Counter to the status of the delivered Echo Message.
/// * `echo_peers` - The Echo peers to which the Echo is sent.
///
pub async fn deliver(
    keychain: KeyChain,
    signed_msg: SignedMessage,
    node_sender: Sender<SignedMessage>,
    delivered_echo: Arc<Mutex<Option<Message>>>,
    echo_subscribers: Vec<Identity>,
) {
    let recv_msg = Some(signed_msg.clone().get_message());
    let mut locked_echo = delivered_echo.lock().await;
    *locked_echo = recv_msg;
    drop(locked_echo);
    let msg: Message = Message::new(1, signed_msg.clone().get_message().content);
    let signature = keychain.sign(&Echo(msg.clone())).unwrap();
    let signed_echo: SignedMessage = SignedMessage::new(msg, signature);
    let push_settings = PushSettings {
        stop_condition: Acknowledgement::Strong,
        retry_schedule: Arc::new(Constant::new(Duration::from_millis(100))),
    };
    let settings: BestEffortSettings = BestEffortSettings { push_settings };
    let best_effort = BestEffort::new(
        node_sender.clone(),
        echo_subscribers,
        signed_echo.clone(),
        settings,
    );
    best_effort.complete().await;
}

/// Deliver an Echo type Message. Save the Echo Message in the Echo replies, used to track when a Message
/// is ready to be delivered.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `message` - The Message of the Message (Without the prepending type).
/// * `from` - The Identity of the Node sending the Echo.
/// * `echo_replies` - The Atomic Reference Counter to the Echo replies received.
/// * `node_sender` - The Node's Sender used to send the Gossip Subscription to the peers.
/// * `delivered_echo` - The Atomic Reference Counter to the status of the delivered Echo Message.
/// * `e_thr` - The threshold defining if enough Echo replies have been received.
/// * `ready_peers` - The Ready peers (used by Contagion).
///
pub async fn deliver_echo(
    keychain: KeyChain,
    message: Message,
    from: Identity,
    echo_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    node_sender: Sender<SignedMessage>,
    delivered_echo: Arc<Mutex<Option<Message>>>,
    e_thr: usize,
    ready_peers: Vec<Identity>,
    ready_messages: Arc<Mutex<Vec<Message>>>,
) {
    if echo_replies.lock().await.contains_key(&from) {
        if echo_replies.lock().await.get(&from).unwrap().is_none() {
            let new_reply: Option<Message> = Some(message.clone());
            let mut locked_replies = echo_replies.lock().await;
            locked_replies.insert(from, new_reply);
            drop(locked_replies);
        }

        check_echoes(
            keychain.clone(),
            node_sender.clone(),
            delivered_echo.clone(),
            e_thr,
            ready_peers.clone(),
            echo_replies,
            ready_messages.clone(),
        )
        .await;
    }
}

/// Check the status of the Echo replies received. If more than the threshold have been
/// received, Probabilistic Consistent Broadcast deliver the Message.
///
/// # Arguments
///
/// * `keychain` - KeyChain used to sign the Message.
/// * `node_sender` - The Node's Sender used to send the Gossip Subscription to the peers.
/// * `delivered_echo` - The Atomic Reference Counter to the status of the delivered Echo Message.
/// * `e_thr` - The threshold defining if enough Echo replies have been received.
/// * `ready_peers` - The Ready peers (used by Contagion).
/// * `echo_replies` - The Echo replies received.
///
pub async fn check_echoes(
    keychain: KeyChain,
    node_sender: Sender<SignedMessage>,
    delivered_echo: Arc<Mutex<Option<Message>>>,
    e_thr: usize,
    ready_peers: Vec<Identity>,
    echo_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    ready_messages: Arc<Mutex<Vec<Message>>>,
) {
    if delivered_echo.lock().await.is_none() {
        let echo_replies: HashMap<Identity, Option<Message>> = echo_replies.lock().await.clone();
        let occ = check_message_occurrences(echo_replies);
        for m in occ {
            if m.1 >= e_thr {
                let msg = Message::new(1, m.0.clone());
                let echo = Some(msg.clone());
                let mut locked_delivered_echo = delivered_echo.lock().await;
                *locked_delivered_echo = echo.clone();
                drop(locked_delivered_echo);
                contagion::deliver(
                    keychain,
                    msg.clone(),
                    node_sender.clone(),
                    ready_peers,
                    ready_messages,
                )
                .await;
                return;
            }
        }
    }
}
