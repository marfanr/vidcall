use arc_swap::{ArcSwap, ArcSwapOption};
use rtc::media_stream::MediaStreamTrack;
use rtc::rtp::Packet;
use rtc::rtp_transceiver::rtp_sender::{
    RTCRtpCodec, RTCRtpCodingParameters, RTCRtpEncodingParameters, RtpCodecKind,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use tracing::debug;
use uuid::Uuid;
use validator::ValidateRequired;
use webrtc::media_stream::track_local::TrackLocal;
use webrtc::media_stream::track_local::static_rtp::TrackLocalStaticRTP;
use webrtc::rtp_transceiver::RtpSender;
use crate::utils::describe_nal;

#[derive(Clone)]
pub struct Output {
    pub sub: Uuid,
    pub ssrc: u32,
    pub track: Arc<dyn TrackLocal>,
    pub sender: Arc<dyn RtpSender>,
}

pub struct RecordingHandle {
    pub tx: mpsc::Sender<Packet>,
}

pub struct PublishedTrack {
    pub publisher: Uuid,
    pub kind: RtpCodecKind,
    pub codec: RTCRtpCodec,
    pub outputs: ArcSwap<Vec<Output>>,
    pub active: AtomicBool,
    recorder: ArcSwapOption<RecordingHandle>,
    has_keyframe: AtomicBool,
}

impl PublishedTrack {
    pub fn new(publisher: Uuid, kind: RtpCodecKind, codec: RTCRtpCodec) -> PublishedTrack {
        Self {
            publisher,
            kind,
            codec,
            outputs: ArcSwap::from_pointee(Vec::new()),
            // A track only reaches this state after its first RTP packet arrived,
            // so the publisher is considered active until proven otherwise.
            active: AtomicBool::new(true),
            recorder: ArcSwapOption::empty(),
            has_keyframe: AtomicBool::new(false),
        }
    }

    /// Whether the publisher is currently sending media for this track.
    /// This is the single source of truth for the mute state of a track.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    pub fn attach_recorder(&self, handle: RecordingHandle) {
        self.has_keyframe.store(false, Ordering::Relaxed);
        self.recorder.store(Some(Arc::new(handle)));
    }

    pub fn stop_recording(&self) {
        self.recorder.store(None);
    }

    pub fn is_recording(&self) -> bool {
        self.recorder.load().is_some()
    }

    pub fn has_keyframe(&self) -> bool {
        self.has_keyframe.load(Ordering::Relaxed)
    }

    fn is_keyframe(payload: &[u8]) -> bool {
        let Some(f) = payload.first().map(|f| f.clone()) else { return false; };
        let h = oxideav_bitstream::h264::nal_header(f);
        h.2 == oxideav_bitstream::h264::NAL_TYPE_IDR
    }

    pub fn create_track_local(&self) -> (u32, Arc<dyn TrackLocal>) {
        let ssrc = Uuid::new_v4().as_u128() as u32;
        let track: Arc<dyn TrackLocal> = Arc::new(TrackLocalStaticRTP::new(MediaStreamTrack::new(
            format!("stream-{}", self.publisher),
            format!("track-{}-{}", self.publisher, self.kind),
            format!("label-{}-{}", self.publisher, self.kind),
            self.kind,
            vec![RTCRtpEncodingParameters {
                rtp_coding_parameters: RTCRtpCodingParameters {
                    ssrc: Some(ssrc),
                    ..Default::default()
                },
                codec: self.codec.clone(),
                ..Default::default()
            }],
        )));
        (ssrc, track)
    }

    /// Hot path: called per RTP packet from publisher.
    pub async fn forward(&self, pkt: &Packet) {
        if let Some(rec) = self.recorder.load_full() {
            match rec.tx.try_send(pkt.clone()) {
                Ok(()) => {}
                Err(_) => debug!("recorder queue full/closed for {}", self.publisher),
            }
        }

        if Self::is_keyframe(&pkt.payload) {
            self.has_keyframe.store(true, Ordering::Relaxed);
        }

        let outs = self.outputs.load_full();

        if let Some(desc) = describe_nal(&pkt.payload) {
            debug!("nal: {desc}");
        }

        for o in outs.iter() {
            let mut p = pkt.clone();
            p.header.ssrc = o.ssrc;
            p.header.extensions.clear();
            p.header.extension = false;

            if let Err(e) = o.track.write_rtp(p).await {
                println!("forward {} -> {}: {e}", self.publisher, o.sub);
            }
        }
    }




    pub async fn drop_output(&self, sub: Uuid) {
        self.outputs.rcu(|cur| {
            cur.iter()
                .filter(|o| o.sub != sub)
                .cloned()
                .collect::<Vec<_>>()
        });
    }

    pub fn is_output_exist(&self, sub: Uuid) -> bool {
        self.outputs.load().iter().any(|o| o.sub == sub)
    }

    pub fn add_output(&self, o: Output) {
        self.outputs.rcu(|out| {
            let mut n = Vec::clone(out);
            n.push(o.clone());
            n
        });
    }

    pub fn renew_outputs(&self) -> Arc<Vec<Output>> {
        self.outputs.swap(Arc::new(Vec::new()))
    }
}
