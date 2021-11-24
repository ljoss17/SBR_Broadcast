use talk::crypto::Identity;

#[derive(Debug, Clone)]
pub struct Peer {
    pub id: u32,
    pub addr: Identity,
}

impl Peer {}
