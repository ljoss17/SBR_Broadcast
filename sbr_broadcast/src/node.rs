use crate::contagion::{deliver_ready, ready_subscribe, ready_subscription};
use crate::message::Message;
use crate::murmur::{deliver_gossip, gossip_subscribe, gossip_subscription};
use crate::sieve::{deliver_echo, echo_subscribe, echo_subscription};
use std::collections::HashMap;
use std::sync::Arc;
use talk::crypto::Identity;
use talk::unicast::{Receiver, Sender};
use tokio::sync::Mutex;

pub struct Node {
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
        id: usize,
        echo_threshold: usize,
        ready_threshold: usize,
        delivery_threshold: usize,
    ) -> Self {
        Node {
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

    pub async fn listen(self, sender: Sender<Message>, receiver: &mut Receiver<Message>) {
        loop {
            let (identity, raw_message, _) = receiver.receive().await;
            let message: Message = raw_message.into();

            let msg_type = message.message_type.clone();
            match msg_type {
                // Gossip
                0 => {
                    let gp = self.gossip_peers.lock().await.clone();
                    let dg = self.delivered_gossip.clone();
                    let ec = self.echo.clone();
                    let ep = self.echo_peers.lock().await.clone();
                    let m = message.clone();
                    let s = sender.clone();
                    tokio::spawn(async move { deliver_gossip(m, s, gp, dg, ec, ep).await });
                }
                // Echo
                1 => {
                    let rp = self.ready_peers.lock().await.clone();
                    let de = self.delivered_echo.clone();
                    let er = self.echo_replies.clone();
                    let ethr = self.echo_threshold.clone();
                    let m = message.clone();
                    let s = sender.clone();
                    tokio::spawn(
                        async move { deliver_echo(m, identity, er, s, de, ethr, rp).await },
                    );
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
                    tokio::spawn(async move {
                        deliver_ready(id, m, identity, rp, rr, dr, s, rm, rthr, dthr, dm).await
                    });
                }
                // GossipSubscription
                3 => {
                    let gp = self.gossip_peers.clone();
                    let dm = self.delivered_msg.lock().await.clone();
                    let s = sender.clone();
                    tokio::spawn(async move { gossip_subscription(s, identity, gp, dm).await });
                }
                // EchoSubscription
                4 => {
                    let s = sender.clone();
                    let ec = self.echo.lock().await.clone();
                    let ep = self.echo_peers.clone();

                    tokio::spawn(async move { echo_subscription(s, identity, ec, ep).await });
                }
                // ReadySubscription
                5 => {
                    let s = sender.clone();
                    let rm = self.ready_messages.lock().await.clone();
                    let rp = self.ready_peers.clone();
                    let id = self.id.clone();

                    tokio::spawn(async move { ready_subscription(id, s, identity, rm, rp).await });
                }
                // Send gossip Subscriptions
                6 => {
                    let tokio_sender = sender.clone();
                    let peers = self.gossip_peers.lock().await.clone();
                    tokio::spawn(async move {
                        gossip_subscribe(tokio_sender, peers).await;
                    });
                }
                // Send gossip Subscriptions
                7 => {
                    let tokio_sender = sender.clone();
                    let replies = self.echo_replies.lock().await.clone();
                    tokio::spawn(async move {
                        echo_subscribe(tokio_sender, replies).await;
                    });
                }
                // Send gossip Subscriptions
                8 => {
                    let tokio_sender = sender.clone();
                    let r_replies = self.ready_replies.lock().await.clone();
                    let d_replies = self.delivery_replies.lock().await.clone();
                    let id = self.id.clone();
                    tokio::spawn(async move {
                        ready_subscribe(id, tokio_sender, r_replies, d_replies).await;
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
