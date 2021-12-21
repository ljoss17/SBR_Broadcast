use crate::contagion::{deliver_ready, ready_subscribe, ready_subscription};
use crate::message::{Message, SignedMessage};
use crate::murmur::{deliver_gossip, gossip_subscribe, gossip_subscription};
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
    pub echo_peers: Arc<Mutex<Vec<Identity>>>,
    ready_peers: Arc<Mutex<Vec<Identity>>>,
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
            echo_peers: Arc::new(Mutex::new(Vec::new())),
            ready_peers: Arc::new(Mutex::new(Vec::new())),
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
    ) {
        loop {
            let (identity, raw_message, _) = receiver.receive().await;
            let message: SignedMessage = raw_message;

            let msg_type = message.clone().get_type();
            match msg_type {
                // Gossip
                0 => {
                    let gp = self.gossip_peers.lock().await.clone();
                    let dg = self.delivered_gossip.clone();
                    let ec = self.echo.clone();
                    let ep = self.echo_peers.lock().await.clone();
                    let m = message.clone();
                    let s = sender.clone();
                    let kc = self.keycards[&identity].clone();
                    let keychain = self.kc.clone();
                    println!("Listen Gossip");
                    tokio::spawn(async move {
                        deliver_gossip(keychain, kc, m, s, gp, dg, ec, ep).await
                    });
                }
                // Echo
                1 => {
                    let rp = self.ready_peers.lock().await.clone();
                    let de = self.delivered_echo.clone();
                    let er = self.echo_replies.clone();
                    let ethr = self.echo_threshold.clone();
                    let m = message.clone().get_message();
                    let s = sender.clone();
                    let keychain = self.kc.clone();
                    tokio::spawn(async move {
                        deliver_echo(keychain, m, identity, er, s, de, ethr, rp).await
                    });
                }
                // Ready
                2 => {
                    let rp = self.ready_peers.lock().await.clone();
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
                    let kc = self.keycards[&identity].clone();
                    tokio::spawn(async move {
                        deliver_ready(
                            keychain, kc, id, m, identity, rp, rr, dr, s, rm, rthr, dthr, dm,
                        )
                        .await
                    });
                }
                // GossipSubscription
                3 => {
                    let gp = self.gossip_peers.clone();
                    let dm = self.delivered_msg.lock().await.clone();
                    let s = sender.clone();
                    let keychain = self.kc.clone();
                    println!("Listen GossipSubscription");
                    tokio::spawn(async move {
                        gossip_subscription(keychain, s, identity, gp, dm).await
                    });
                }
                // EchoSubscription
                4 => {
                    let s = sender.clone();
                    let ec = self.echo.lock().await.clone();
                    let ep = self.echo_peers.clone();
                    let keychain = self.kc.clone();

                    tokio::spawn(
                        async move { echo_subscription(keychain, s, identity, ec, ep).await },
                    );
                }
                // ReadySubscription
                5 => {
                    let s = sender.clone();
                    let rm = self.ready_messages.lock().await.clone();
                    let rp = self.ready_peers.clone();
                    let id = self.id.clone();
                    let keychain = self.kc.clone();
                    tokio::spawn(async move {
                        ready_subscription(keychain, id, s, identity, rm, rp).await
                    });
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
                // Not valid
                _ => {
                    println!("Not a valid message type!");
                }
            }
        }
    }
}
