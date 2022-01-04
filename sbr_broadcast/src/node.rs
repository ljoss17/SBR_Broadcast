use crate::contagion::{deliver_ready, ready_subscribe, ready_subscription};
use crate::message::{Message, SignedMessage};
use crate::message_headers::{
    Echo, EchoSubscription, Gossip, GossipSubscription, Ready, ReadySubscription,
};
use crate::murmur::{deliver_gossip, dispatch, gossip_subscribe, gossip_subscription};
use crate::sieve::{deliver_echo, echo_subscribe, echo_subscription};
use std::collections::HashMap;
use std::sync::Arc;
use talk::crypto::{Identity, KeyCard, KeyChain};
use talk::unicast::{Receiver, Sender};
use tokio::sync::Mutex;

pub struct Node {
    kc: KeyChain,
    keycards: HashMap<Identity, KeyCard>,
    pub id: usize,
    pub gossip_peers: Arc<Mutex<Vec<Identity>>>,
    pub echo_subscribers: Arc<Mutex<Vec<Identity>>>,
    ready_subscribers: Arc<Mutex<Vec<Identity>>>,
    echo_threshold: usize,
    ready_threshold: usize,
    delivery_threshold: usize,
    pub echo: Arc<Mutex<Option<Message>>>,
    pub echo_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    pub ready_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    ready_messages: Arc<Mutex<Vec<Message>>>,
    pub delivery_replies: Arc<Mutex<HashMap<Identity, Option<Message>>>>,
    pub delivered_gossip: Arc<Mutex<Option<Message>>>,
    delivered_echo: Arc<Mutex<Option<Message>>>,
    delivered_msg: Arc<Mutex<Option<Message>>>,
}

impl Node {
    pub fn new(
        kc: KeyChain,
        keycards: HashMap<Identity, KeyCard>,
        id: usize,
        echo_threshold: usize,
        ready_threshold: usize,
        delivery_threshold: usize,
    ) -> Self {
        Node {
            kc,
            keycards,
            id,
            gossip_peers: Arc::new(Mutex::new(Vec::new())),
            echo_subscribers: Arc::new(Mutex::new(Vec::new())),
            ready_subscribers: Arc::new(Mutex::new(Vec::new())),
            echo_threshold,
            ready_threshold,
            delivery_threshold,
            echo: Arc::new(Mutex::new(None)),
            echo_replies: Arc::new(Mutex::new(HashMap::new())),
            ready_replies: Arc::new(Mutex::new(HashMap::new())),
            ready_messages: Arc::new(Mutex::new(Vec::new())),
            delivery_replies: Arc::new(Mutex::new(HashMap::new())),
            delivered_gossip: Arc::new(Mutex::new(None)),
            delivered_echo: Arc::new(Mutex::new(None)),
            delivered_msg: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn listen(
        self,
        sender: Sender<SignedMessage>,
        receiver: &mut Receiver<SignedMessage>,
        dummy_kc: KeyCard,
    ) {
        loop {
            let (identity, raw_message, acknowledger) = receiver.receive().await;
            let message: SignedMessage = raw_message;

            let msg_type = message.clone().get_type();
            let mut kc = dummy_kc.clone();
            if msg_type < 6 {
                kc = self.keycards[&identity].clone();
            }
            match msg_type {
                // Gossip
                0 => {
                    let correct = message
                        .clone()
                        .get_signature()
                        .verify(&kc, &Gossip(message.clone().get_message()));
                    if correct.is_ok() {
                        acknowledger.strong();
                        let gp = self.gossip_peers.lock().await.clone();
                        let dg = self.delivered_gossip.clone();
                        let ec = self.echo.clone();
                        let ep: Vec<Identity> =
                            self.echo_replies.lock().await.clone().into_keys().collect();
                        let m = message.clone();
                        let s = sender.clone();
                        let keychain = self.kc.clone();
                        tokio::spawn(async move {
                            deliver_gossip(keychain, m, s, gp, dg, ec, ep).await
                        });
                    } else {
                        my_print!(format!("Problem with Gossip : {:?}", correct));
                    }
                }
                // Echo
                1 => {
                    let kc = self.keycards[&identity].clone();
                    let correct = message
                        .clone()
                        .get_signature()
                        .verify(&kc, &Echo(message.clone().get_message()));
                    if correct.is_ok() {
                        acknowledger.strong();
                        let rp = self.ready_subscribers.lock().await.clone();
                        let de = self.delivered_echo.clone();
                        let er = self.echo_replies.clone();
                        let ethr = self.echo_threshold.clone();
                        let m = message.clone().get_message();
                        let s = sender.clone();
                        let keychain = self.kc.clone();
                        let rm = self.ready_messages.clone();
                        tokio::spawn(async move {
                            deliver_echo(keychain, m, identity, er, s, de, ethr, rp, rm).await
                        });
                    } else {
                        my_print!(format!("Problem with Echo : {:?}", correct));
                    }
                }
                // Ready
                2 => {
                    let correct = message
                        .clone()
                        .get_signature()
                        .verify(&kc, &Ready(message.clone().get_message()));
                    if correct.is_ok() {
                        acknowledger.strong();
                        let rp = self.ready_subscribers.lock().await.clone();
                        let rr = self.ready_replies.clone();
                        let dr = self.delivery_replies.clone();
                        let rm = self.ready_messages.clone();
                        let dm = self.delivered_msg.clone();
                        let rthr = self.ready_threshold.clone();
                        let dthr = self.delivery_threshold.clone();
                        let m = message.clone();
                        let s = sender.clone();
                        let id = self.id.clone();
                        let keychain = self.kc.clone();
                        tokio::spawn(async move {
                            deliver_ready(
                                keychain, id, m, identity, rp, rr, dr, s, rm, rthr, dthr, dm,
                            )
                            .await
                        });
                    } else {
                        my_print!(format!("Problem with Ready : {:?}", correct));
                    }
                }
                // GossipSubscription
                3 => {
                    let correct = message
                        .clone()
                        .get_signature()
                        .verify(&kc, &GossipSubscription(message.clone().get_message()));
                    if correct.is_ok() {
                        acknowledger.strong();
                        let gp = self.gossip_peers.clone();
                        let dm = self.delivered_msg.lock().await.clone();
                        let s = sender.clone();
                        let keychain = self.kc.clone();
                        tokio::spawn(async move {
                            gossip_subscription(keychain, s, identity, gp, dm).await
                        });
                    } else {
                        my_print!(format!("Problem with Gossip Subscription : {:?}", correct));
                    }
                }
                // EchoSubscription
                4 => {
                    let correct = message
                        .clone()
                        .get_signature()
                        .verify(&kc, &EchoSubscription(message.clone().get_message()));
                    if correct.is_ok() {
                        acknowledger.strong();
                        let s = sender.clone();
                        let ec = self.echo.lock().await.clone();
                        let ep = self.echo_subscribers.clone();
                        let keychain = self.kc.clone();

                        tokio::spawn(async move {
                            echo_subscription(keychain, s, identity, ec, ep).await
                        });
                    } else {
                        my_print!(format!("Problem with Echo Subscription : {:?}", correct));
                    }
                }
                // ReadySubscription
                5 => {
                    let correct = message
                        .clone()
                        .get_signature()
                        .verify(&kc, &ReadySubscription(message.clone().get_message()));
                    if correct.is_ok() {
                        acknowledger.strong();
                        let s = sender.clone();
                        let rm = self.ready_messages.lock().await.clone();
                        let rp = self.ready_subscribers.clone();
                        let id = self.id.clone();
                        let keychain = self.kc.clone();
                        tokio::spawn(async move {
                            ready_subscription(keychain, id, s, identity, rm, rp).await
                        });
                    } else {
                        my_print!(format!("Problem with Ready Subscription : {:?}", correct));
                    }
                }
                // Send Gossip Subscriptions
                6 => {
                    let tokio_sender = sender.clone();
                    let peers = self.gossip_peers.lock().await.clone();
                    let keychain = self.kc.clone();
                    tokio::spawn(async move {
                        gossip_subscribe(keychain, tokio_sender, peers).await;
                    });
                }
                // Send Echo Subscriptions
                7 => {
                    let tokio_sender = sender.clone();
                    let replies = self.echo_replies.lock().await.clone();
                    let keychain = self.kc.clone();
                    tokio::spawn(async move {
                        echo_subscribe(keychain, tokio_sender, replies).await;
                    });
                }
                // Send Ready Subscriptions
                8 => {
                    let tokio_sender = sender.clone();
                    let r_replies = self.ready_replies.lock().await.clone();
                    let d_replies = self.delivery_replies.lock().await.clone();
                    let keychain = self.kc.clone();
                    tokio::spawn(async move {
                        ready_subscribe(keychain, tokio_sender, r_replies, d_replies).await;
                    });
                }
                // Trigger sender
                9 => {
                    let tokio_sender = sender.clone();
                    let keychain = self.kc.clone();
                    let msg = Message::new(0, String::from("Test message"));
                    let signature = keychain.sign(&Gossip(msg.clone())).unwrap();
                    let signed_msg: SignedMessage = SignedMessage::new(msg, signature);
                    let peers = self.gossip_peers.lock().await.clone();
                    let dg = self.delivered_gossip.clone();
                    let ec = self.echo.clone();
                    let ep = self.echo_subscribers.lock().await.clone();
                    tokio::spawn(async move {
                        dispatch(keychain, signed_msg, tokio_sender, peers, dg, ec, ep).await
                    });
                }
                // Not valid
                _ => {
                    println!("Not a valid message type!");
                }
            }
        }
    }
}
