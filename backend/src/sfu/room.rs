// will be moved to sfu
use std::sync::{Arc, RwLock};

use crate::models;
use crate::models::room::Room;
use arc_swap::ArcSwap;
use axum::extract::ws::Message;
use tokio::sync::mpsc;
use uuid::Uuid;
use webrtc::peer_connection::PeerConnection;

#[derive(Debug)]
pub struct RoomPeer {
    pub conn_id: Uuid,
    pub user_id: Option<Uuid>,
    pub ws_tx: mpsc::Sender<Message>,
    // pub pc: Box<dyn PeerConnection>,
}

pub struct RoomSfu {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub alias: Arc<str>,
    pub peers: ArcSwap<Vec<Arc<RoomPeer>>>,
}

impl RoomSfu {
    pub fn add_peer(&self, peer: Arc<RoomPeer>) {
        self.peers.rcu(|cur| {
            if cur.iter().any(|p| p.conn_id == peer.conn_id) {
                Arc::clone(cur)
            } else {
                let mut next = Vec::with_capacity(cur.len() + 1);
                next.extend(cur.iter().cloned());
                next.push(peer.clone());
                Arc::new(next)
            }
        });
    }

    pub fn drop_peer(&self, conn_id: Uuid) {
        self.peers.rcu(|cur| {
            let next: Vec<_> = cur
                .iter()
                .filter(|p| p.conn_id != conn_id)
                .cloned()
                .collect();

            if next.len() == cur.len() {
                Arc::clone(cur)
            } else {
                Arc::new(next)
            }
        });
    }

    pub fn broadcast(&self, conn_id: Uuid, msg: Message) -> Vec<Uuid> {
        let peers = self.peers.load();
        let mut failed = Vec::new();

        for peer in peers.iter() {
            if peer.conn_id == conn_id {
                continue;
            }

            if peer.ws_tx.try_send(msg.clone()).is_err() {
                failed.push(peer.conn_id);
            }
        }

        failed
    }

    pub fn send_to_owner(&self, from: Uuid, msg: Message) -> Vec<Uuid> {
        let peers = self.peers.load();
        let mut failed = Vec::new();

        for peer in peers.iter() {
            if peer.conn_id == from || peer.user_id != Some(self.owner_id) {
                continue;
            }

            if peer.ws_tx.try_send(msg.clone()).is_err() {
                failed.push(peer.conn_id);
            }
        }

        failed
    }
}

impl From<&models::room::Room> for RoomSfu {
    fn from(v: &Room) -> Self {
        Self {
            id: v
                .id
                .expect("Room must have an id to be converted into RoomSfu"),
            peers: ArcSwap::from_pointee(Vec::new()),
            alias: v.alias.to_owned(),
            owner_id: v.owner_id,
        }
    }
}
