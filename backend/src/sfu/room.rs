use crate::models;
use crate::models::room::Room;
use crate::sfu::track::{Output, PublishedTrack};
use axum::extract::ws::Message;
use dashmap::DashMap;
use rtc::rtp_transceiver::rtp_sender::{RTCRtpCodec, RtpCodecKind};
use sonic_rs::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use tracing::warn;
use uuid::Uuid;
use webrtc::data_channel::DataChannel;
use webrtc::peer_connection::PeerConnection;

pub enum RoomMemberstype {
    Waiting,
}

pub struct RoomPeer {
    pub user_id: Option<Uuid>,
    pub username: Arc<str>,
    pub ws_tx: Option<mpsc::Sender<Message>>,
    pub pc: Arc<dyn PeerConnection>,
    ctrl: Arc<dyn DataChannel>,
}

impl RoomPeer {
    pub async fn new(
        user_id: Option<Uuid>,
        username: Arc<str>,
        pc: Arc<dyn PeerConnection>,
        ws_tx: Option<mpsc::Sender<Message>>,
    ) -> Result<Arc<Self>, webrtc::error::Error> {
        let ctrl = pc.create_data_channel("control", None).await?;

        Ok(Arc::new(RoomPeer {
            user_id,
            username,
            ws_tx,
            pc,
            ctrl,
        }))
    }

    pub async fn send_control<T>(&self, msg: &T) -> Result<(), ()>
    where
        T: sonic_rs::Serialize + ?Sized,
    {
        let json = sonic_rs::to_string(msg).map_err(|_| ())?;
        self.ctrl.send_text(json.as_str()).await.map_err(|_| ())?;
        Ok(())
    }
}

pub struct RoomSfu {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub alias: Arc<str>,
    pub peers: DashMap<Uuid, Arc<RoomPeer>>,
    pub tracks: DashMap<Uuid, Vec<Arc<PublishedTrack>>>,
    pub recording: AtomicBool,
}

#[derive(Debug, Serialize)]
pub struct TrackState {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub from: Uuid,
    pub kind: String,
    pub muted: bool,
}

impl RoomSfu {
    pub fn add_peer(&self, conn_id: Uuid, peer: Arc<RoomPeer>) -> bool {
        if self.peers.iter().any(|p| p.user_id == Some(conn_id)) {
            return false;
        }
        self.peers.insert(conn_id, peer);
        true
    }

    pub async fn drop_peer(&self, conn_id: Uuid) {
        self.peers.remove(&conn_id);

        if let Some((_, tracks)) = self.tracks.remove(&conn_id) {
            for track in tracks {
                track.stop_recording();
            }
        }

        for entry in self.tracks.iter_mut() {
            for track in entry.value().iter() {
                track.drop_output(conn_id).await;
            }
        }
    }

    pub async fn attach_existing_tracks(&self, sub_id: Uuid, peer: &RoomPeer) {
        let mut attached_any = false;
        for entry in self.tracks.iter() {
            if entry.key() == &sub_id {
                continue;
            }
            for track in entry.value().iter() {
                if self.attach(track, sub_id, peer).await {
                    attached_any = true;
                }
            }
        }
        if attached_any {
            renegotiate(peer).await;
            println!("renegotiate existing tracks for {sub_id}");
        }
    }

    async fn send_track_state(&self, sub_id: Uuid, track: &PublishedTrack) {
        let Some(peer) = self.peers.get(&sub_id) else {
            return;
        };

        if let Err(_) = peer
            .send_control(&TrackState {
                event_type: "track_state",
                from: track.publisher,
                kind: track.kind.to_string(),
                muted: !track.active.load(Ordering::Relaxed),
            })
            .await
        {
            warn!("send track state to {sub_id} failed, ws queue full or closed");
        }
    }

    pub async fn set_track_active(&self, track: &PublishedTrack, active: bool) -> bool {
        if track.active.swap(active, Ordering::Relaxed) == active {
            return false;
        }

        self.broadcast_control(
            track.publisher,
            &TrackState {
                event_type: "track_state",
                from: track.publisher,
                kind: track.kind.to_string(),
                muted: !active,
            },
        )
            .await;

        true
    }

    pub async fn publish_track(
        &self,
        publisher: Uuid,
        kind: RtpCodecKind,
        codec: RTCRtpCodec,
    ) -> Arc<PublishedTrack> {
        let track = Arc::new(PublishedTrack::new(publisher, kind, codec));
        self.tracks
            .entry(publisher)
            .or_default()
            .push(track.clone());

        if self.recording.load(Ordering::Relaxed) {
            let _ = crate::sfu::record::start_track_recording(self.id, &track).await;
        }

        let subs: Vec<(Uuid, Arc<RoomPeer>)> = self
            .peers
            .iter()
            .filter(|p| p.key() != &publisher)
            .map(|p| (*p.key(), p.value().clone()))
            .collect();

        for (id, peer) in subs {
            if self.attach(&track, id, &peer).await {
                renegotiate(&peer).await;
                println!("renegotiate");
            }
        }
        track
    }

    async fn attach(&self, t: &Arc<PublishedTrack>, sub_id: Uuid, peer: &RoomPeer) -> bool {
        if t.is_output_exist(sub_id) {
            return false;
        }
        let (ssrc, track) = t.create_track_local();

        let sender = match peer.pc.add_track(track.clone()).await {
            Ok(x) => x,
            Err(e) => {
                println!("add_track to {sub_id}: {e}");
                return false;
            }
        };

        let out = Output {
            sub: sub_id,
            ssrc,
            track,
            sender,
        };

        t.add_output(out);

        self.send_track_state(sub_id, t).await;
        true
    }

    pub async fn unpublish_track(&self, track: &Arc<PublishedTrack>) {
        track.stop_recording();

        if let Some(mut entry) = self.tracks.get_mut(&track.publisher) {
            entry.retain(|t| !Arc::ptr_eq(t, track));
        }

        let outs = track.renew_outputs();

        for o in outs.iter() {
            if let Some(peer) = self.peers.get(&o.sub) {
                if let Err(e) = peer.pc.remove_track(&o.sender).await {
                    warn!("remove_track on {}: {e}", o.sub);
                }
                renegotiate(&peer).await;
            }
        }

        self.broadcast_control(
            track.publisher,
            &TrackState {
                event_type: "track_ended",
                from: track.publisher,
                kind: track.kind.to_string(),
                muted: false,
            },
        )
            .await;
    }

    pub fn broadcast(&self, conn_id: Uuid, msg: Message) -> Vec<Uuid> {
        let mut failed = Vec::new();

        for peer in self.peers.iter() {
            if peer.key() == &conn_id {
                continue;
            }

            let Some(ws_tx) = peer.ws_tx.clone() else {
                continue;
            };

            if ws_tx.try_send(msg.clone()).is_err() {
                warn!(
                    "broadcast to {} failed, ws queue full or closed",
                    peer.key()
                );
                failed.push(peer.key().to_owned());
            }
        }

        failed
    }

    pub async fn broadcast_control<T>(&self, conn_id: Uuid, msg: &T) -> Vec<Uuid>
    where
        T: sonic_rs::Serialize + ?Sized,
    {
        let mut failed = Vec::new();

        for peer in self.peers.iter() {
            if peer.key() == &conn_id {
                continue;
            }
            if let Err(_) = peer.send_control(msg).await {
                warn!(
                    "broadcast to {} failed, ws queue full or closed",
                    peer.key()
                );
                failed.push(peer.key().to_owned());
            }
        }

        failed
    }

    pub fn send_to_owner(&self, from: Uuid, msg: Message) -> Vec<Uuid> {
        let mut failed = Vec::new();

        for peer in self.peers.iter() {
            if peer.key() == &from || peer.user_id != Some(self.owner_id) {
                continue;
            }

            let Some(ws_tx) = peer.ws_tx.clone() else {
                continue;
            };

            if ws_tx.try_send(msg.clone()).is_err() {
                failed.push(peer.key().to_owned());
            }
        }

        failed
    }
}

/// send offer to client
pub async fn renegotiate(peer: &RoomPeer) {
    let offer = match peer.pc.create_offer(None).await {
        Ok(o) => o,
        Err(e) => return warn!("create_offer: {e}"),
    };
    if let Err(e) = peer.pc.set_local_description(offer).await {
        return warn!("set_local_description: {e}");
    }
    let Some(desc) = peer.pc.local_description().await else {
        return;
    };
    // TODO: refactor
    // change from ws to control data channel
    let msg = sonic_rs::json!({ "type": "offer", "sdp": desc.sdp }).to_string();

    let Some(ws_tx) = peer.ws_tx.clone() else {
        return;
    };

    if ws_tx.send(Message::Text(msg.into())).await.is_err() {
        warn!("ws closed, drop renegotiation offer");
    }
}

impl From<&models::room::Room> for RoomSfu {
    fn from(v: &Room) -> Self {
        Self {
            id: v
                .id
                .expect("Room must have an id to be converted into RoomSfu"),
            peers: DashMap::new(),
            alias: v.alias.to_owned(),
            owner_id: v.owner_id,
            tracks: DashMap::new(),
            recording: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room() -> RoomSfu {
        RoomSfu {
            id: Uuid::nil(),
            owner_id: Uuid::nil(),
            alias: Arc::from("test"),
            peers: DashMap::new(),
            tracks: DashMap::new(),
            recording: AtomicBool::new(false),
        }
    }

    fn published(publisher: Uuid) -> PublishedTrack {
        PublishedTrack::new(
            publisher,
            RtpCodecKind::Video,
            RTCRtpCodec {
                mime_type: "video/VP8".to_owned(),
                clock_rate: 90000,
                ..Default::default()
            },
        )
    }

    /// A track that just started publishing is active, and every state change must be
    /// reported exactly once. Regression: the track used to start as inactive while the
    /// publisher loop assumed it was active, so turning the camera off was swallowed.
    #[tokio::test]
    async fn track_state_changes_are_reported_once() {
        let room = room();
        let track = published(Uuid::nil());

        assert!(track.is_active());

        // camera off
        assert!(room.set_track_active(&track, false).await);
        assert!(!track.is_active());

        // repeated off is not a state change
        assert!(!room.set_track_active(&track, false).await);
        assert!(!track.is_active());

        // camera on again
        assert!(room.set_track_active(&track, true).await);
        assert!(track.is_active());

        // repeated on is not a state change
        assert!(!room.set_track_active(&track, true).await);
        assert!(track.is_active());
    }
}
