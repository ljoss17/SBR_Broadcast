use crate::message::Message;
use serde::{Deserialize, Serialize};
use talk::crypto::Statement;

#[derive(Serialize)]
pub enum Header {
    Gossip,
    Echo,
    Ready,
    GossipSubscription,
    EchoSubscription,
    ReadySubscription,
    InitGossip,
    InitEcho,
    InitReady,
}

#[derive(Serialize, Deserialize)]
pub struct Gossip(pub Message);

#[derive(Serialize, Deserialize)]
pub struct Echo(pub Message);

#[derive(Serialize, Deserialize)]
pub struct Ready(pub Message);

#[derive(Serialize, Deserialize)]
pub struct GossipSubscription(pub Message);

#[derive(Serialize, Deserialize)]
pub struct EchoSubscription(pub Message);

#[derive(Serialize, Deserialize)]
pub struct ReadySubscription(pub Message);

#[derive(Serialize, Deserialize)]
pub struct InitGossip(pub Message);

#[derive(Serialize, Deserialize)]
pub struct InitEcho(pub Message);

#[derive(Serialize, Deserialize)]
pub struct InitReady(pub Message);

impl Statement for Gossip {
    type Header = Header;
    const HEADER: Header = Header::Gossip;
}

impl Statement for Echo {
    type Header = Header;
    const HEADER: Header = Header::Echo;
}

impl Statement for Ready {
    type Header = Header;
    const HEADER: Header = Header::Ready;
}

impl Statement for GossipSubscription {
    type Header = Header;
    const HEADER: Header = Header::GossipSubscription;
}

impl Statement for EchoSubscription {
    type Header = Header;
    const HEADER: Header = Header::EchoSubscription;
}

impl Statement for ReadySubscription {
    type Header = Header;
    const HEADER: Header = Header::ReadySubscription;
}

impl Statement for InitGossip {
    type Header = Header;
    const HEADER: Header = Header::InitGossip;
}

impl Statement for InitEcho {
    type Header = Header;
    const HEADER: Header = Header::InitEcho;
}

impl Statement for InitReady {
    type Header = Header;
    const HEADER: Header = Header::InitReady;
}
