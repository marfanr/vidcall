// will be moved to sfu
use std::
sync::{Arc, RwLock};

use arc_swap::ArcSwap;
use axum::extract::ws::Message;
use tokio::sync::mpsc;
use uuid::Uuid;
use webrtc::peer_connection::PeerConnection;

pub struct Peer {
    pub id: Uuid,
    pub ws_tx: mpsc::Sender<Message>,
    pub pc: Box<dyn PeerConnection>,
}

pub struct RoomSfu {
    pub id: Uuid,
    pub name: RwLock<Arc<str>>,
    pub peers: ArcSwap<Vec<Peer>>,
}

impl RoomSfu {
    pub fn new(id: Uuid, name: RwLock<Arc<str>>) -> Self {
        Self {
            id,
            name,
            peers: ArcSwap::from_pointee(Vec::new()),
        }
    }

    pub fn add_peer(&self, _peer: Arc<Peer>) {
        // self.peers.insert(peer.id, peer);
    }
}
