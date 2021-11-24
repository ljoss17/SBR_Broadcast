use crate::contagion::deliver_ready;
use crate::murmur::{deliver_gossip, gossip_subscription};
use crate::sieve::{deliver_echo, echo_subscription};
use crate::utils::get_message_type;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use talk::crypto::Identity;
use talk::unicast::{Receiver, Sender};

pub struct Node {
    gossip_peers: Arc<Mutex<Vec<Identity>>>,
    echo_peers: Arc<Mutex<Vec<Identity>>>,
    ready_peers: Arc<Mutex<Vec<Identity>>>,
    delivery_peers: Arc<Mutex<Vec<Identity>>>,
    echo_threshold: usize,
    ready_threshold: usize,
    delivery_threshold: usize,
    echo: Arc<Mutex<Option<String>>>,
    echo_replies: Arc<Mutex<HashMap<Identity, Option<String>>>>,
    ready_replies: Arc<Mutex<HashMap<Identity, Option<String>>>>,
    delivery_replies: Arc<Mutex<HashMap<Identity, Option<String>>>>,
    delivered_echo: Arc<Mutex<Option<String>>>,
    delivered_msg: Arc<Mutex<Option<String>>>,
}

impl Node {
    pub fn new(echo_threshold: usize, ready_threshold: usize, delivery_threshold: usize) -> Self {
        Node {
            gossip_peers: Arc::new(Mutex::new(Vec::new())),
            echo_peers: Arc::new(Mutex::new(Vec::new())),
            ready_peers: Arc::new(Mutex::new(Vec::new())),
            delivery_peers: Arc::new(Mutex::new(Vec::new())),
            echo_threshold,
            ready_threshold,
            delivery_threshold,
            echo: Arc::new(Mutex::new(None)),
            echo_replies: Arc::new(Mutex::new(HashMap::new())),
            ready_replies: Arc::new(Mutex::new(HashMap::new())),
            delivery_replies: Arc::new(Mutex::new(HashMap::new())),
            delivered_echo: Arc::new(Mutex::new(None)),
            delivered_msg: Arc::new(Mutex::new(None)),
        }
    }

    async fn listen(&self, sender: Sender<String>, receiver: &mut Receiver<String>) {
        let (identity, message, _) = receiver.receive().await;

        tokio::spawn(async move {
            let (msg_type, content) = get_message_type(message.clone());
            match msg_type {
                // Gossip
                0 => {
                    deliver_gossip(
                        message.clone(),
                        sender.clone(),
                        &self.gossip_peers.clone(),
                        &self.delivered_msg.clone(),
                        &self.echo.clone(),
                        &self.echo_peers.clone(),
                    )
                    .await;
                }
                // Echo
                1 => {
                    deliver_echo(content, identity, &self.echo_replies.clone());
                }
                // Ready
                2 => {}
                // GossipSubscription
                3 => {
                    gossip_subscription(
                        sender.clone(),
                        identity,
                        &self.gossip_peers.clone(),
                        &self.delivered_msg.clone(),
                    )
                    .await;
                }
                // EchoSubscription
                4 => {
                    echo_subscription(
                        identity,
                        sender.clone(),
                        &self.echo.clone(),
                        &self.echo_peers.clone(),
                    )
                    .await;
                }
                // ReadySubscription
                5 => {
                    deliver_ready(
                        content,
                        identity,
                        &self.ready_peers.clone(),
                        &self.delivery_peers.clone(),
                        &self.ready_replies.clone(),
                        &self.delivery_replies.clone(),
                    );
                }
                // Not valid
                _ => {
                    println!("Not a valid message type!");
                }
            }
        });
    }
}
