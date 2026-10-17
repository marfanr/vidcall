// will be moved to sfu
use std::{
    fmt,
    str::FromStr,
    sync::{Arc, RwLock},
};

use arc_swap::ArcSwap;
use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;
use webrtc::peer_connection::PeerConnection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidRoomId;

impl fmt::Display for InvalidRoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid room id")
    }
}

impl std::error::Error for InvalidRoomId {}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RoomId(Arc<str>);

impl RoomId {
    pub const LEN: usize = 12;

    pub fn generate() -> Self {
        Self(nanoid::nanoid!(Self::LEN).into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for RoomId {
    type Err = InvalidRoomId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let valid = s.len() == Self::LEN
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
        if valid {
            Ok(Self(s.into()))
        } else {
            Err(InvalidRoomId)
        }
    }
}

impl TryFrom<String> for RoomId {
    type Error = InvalidRoomId;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<RoomId> for String {
    fn from(id: RoomId) -> Self {
        id.0.to_string()
    }
}

impl fmt::Display for RoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::borrow::Borrow<str> for RoomId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

pub struct Peer {
    pub id: Uuid,
    pub ws_tx: mpsc::Sender<Message>,
    pub pc: Box<dyn PeerConnection>,
}

pub struct Room {
    pub id: RoomId,
    pub name: RwLock<Arc<str>>,
    pub peers: ArcSwap<Vec<Peer>>,
}

impl Room {
    pub fn new(id: RoomId, name: RwLock<Arc<str>>) -> Self {
        Self {
            id,
            name,
            peers: ArcSwap::from_pointee(Vec::new()),
        }
    }

    pub fn add_peer(&self, _peer: Arc<Peer>) {
        // self.peers.insert(peer.id, peer);
    }

    // /// Hapus peer dan tutup PeerConnection-nya. Satu tempat untuk cleanup.
    // pub async fn remove_peer(&self, peer_id: &Uuid) {
    //     // `remove` melepas guard sebelum .await
    //     if let Some((_, peer)) = self.peers.remove(peer_id) {
    //         if let Err(e) = peer.pc.close().await {
    //             warn!(%peer_id, error = %e, "failed to close peer connection");
    //         }
    //     }
    // }

    // pub async fn close(&self) {
    //     let ids: Vec<Uuid> = self.peers.iter().map(|p| *p.key()).collect();
    //     for id in ids {
    //         self.remove_peer(&id).await;
    //     }
    // }
}
