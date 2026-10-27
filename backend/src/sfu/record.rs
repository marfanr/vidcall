use crate::ffmpeg::{wait_ffmpeg_ready, FfmpegBuilder};
use crate::sfu::room::RoomSfu;
use crate::sfu::track::{PublishedTrack, RecordingHandle};
use rtc::rtp::Packet;
use rtc::rtp_transceiver::rtp_sender::RtpCodecKind;
use rtp::codecs::h264::H264Packet;
use rtp::packetizer::Depacketizer;
use sdp_types::{AddrType, Attribute, Media, MediaType, NetType, Origin, TransportProto};
use std::fmt::Debug;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use rtc::shared::marshal::Marshal;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::net::UdpSocket;
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

const LOOPBACK_PT: u8 = 96;
const MAX_CONSECUTIVE_REFUSED: u32 = 100;

pub async fn start_record_bot(room: Arc<RoomSfu>) {
    if room.recording.swap(true, Ordering::SeqCst) {
        info!("recording already active for room {}", room.id);
        return;
    }

    let dir = record_dir();
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        warn!("create recording dir {}: {e}", dir.display());
        room.recording.store(false, Ordering::SeqCst);
        return;
    }

    let tracks: Vec<Arc<PublishedTrack>> = room
        .tracks
        .iter()
        .flat_map(|entry| entry.value().clone())
        .collect();

    let mut started = 0;

    for track in tracks {
        if start_track_recording(room.id, &track).await {
            started += 1;
        }
    }

    if started == 0 {
        // nothing was recorded (e.g. the publisher negotiated VP8 instead of
        // H264); release the flag so a retry after switching codec can work.
        room.recording.store(false, Ordering::SeqCst);
        warn!(
            "recording room {} not started: no recordable track (need video/H264), output dir {}",
            room.id,
            dir.display()
        );
        return;
    }

    info!(
        "recording room {} started, {started} track(s), output dir {}",
        room.id,
        dir.display()
    );
}

pub async fn start_track_recording(room_id: Uuid, track: &Arc<PublishedTrack>) -> bool {
    if track.is_recording() {
        return false;
    }

    if track.kind != RtpCodecKind::Video
        || !track.codec.mime_type.eq_ignore_ascii_case("video/H264")
    {
        info!(
            "skip recording publisher {}: codec={}, kind={}",
            track.publisher, track.codec.mime_type, track.kind
        );
        return false;
    }

    let dir = record_dir();
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        warn!("create recording dir {}: {e}", dir.display());
        return false;
    }

    let record_id = Uuid::new_v4();
    let path = dir.join(format!(
        "{}-{}-{}-{}.mkv",
        room_id, track.publisher, track.kind, record_id
    ));
    let sdp_path = dir.join(format!("{record_id}.sdp"));

    // Startup retries choose a fresh port each time. The track is attached to
    // the recorder channel after FFmpeg is ready, so an initial keyframe may be
    // missed during startup; the recorder waits for the next SPS/IDR packet.
    let (port, mut child, stdin) =
        match spawn_ffmpeg_with_retry(&sdp_path, &path, track.publisher).await {
            Ok(result) => result,
            Err(e) => {
                warn!("could not start recorder for {}: {e}", track.publisher);
                let _ = tokio::fs::remove_file(&sdp_path).await;
                let _ = tokio::fs::remove_file(&path).await;
                return false;
            }
        };

    let sock = match UdpSocket::bind("127.0.0.1:0").await {
        Ok(sock) => sock,
        Err(e) => {
            warn!("bind recorder UDP socket: {e}");
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = tokio::fs::remove_file(&sdp_path).await;
            let _ = tokio::fs::remove_file(&path).await;
            return false;
        }
    };

    if let Err(e) = sock.connect(("127.0.0.1", port)).await {
        warn!("connect recorder UDP socket to {port}: {e}");
        let _ = child.kill().await;
        let _ = child.wait().await;
        let _ = tokio::fs::remove_file(&sdp_path).await;
        let _ = tokio::fs::remove_file(&path).await;
        return false;
    }

    let tee = match std::env::var_os("RECORD_TEE_DIR") {
        Some(dir) => {
            let name = format!(
                "{}-{}-{}-{}.h264",
                room_id, track.publisher, track.kind, record_id
            );
            match create_tee(PathBuf::from(dir), name).await {
                Ok(file) => Some(file),
                Err(e) => {
                    warn!("tee disabled: {e}");
                    None
                }
            }
        }
        None => None,
    };

    let (tx, rx) = mpsc::channel::<Packet>(2048);
    let publisher = track.publisher;
    let output = path.clone();
    let track_for_task = Arc::clone(track);

    tokio::spawn(async move {
        let res = run_recorder(rx, sock, stdin, child, tee, output.clone()).await;

        // TODO compositing


        // Recorder is gone: detach it so the track can be recorded again and
        // the hot path stops pushing into a dead channel.
        track_for_task.stop_recording();
        let _ = tokio::fs::remove_file(&sdp_path).await;

        if let Err(e) = res {
            warn!("recorder {} ({}): {e}", publisher, output.display());
        }
    });

    track.attach_recorder(RecordingHandle { tx });

    info!(
        "recording publisher {} -> {} (rtp udp 127.0.0.1:{port})",
        track.publisher,
        path.display()
    );

    true
}

fn record_dir() -> PathBuf {
    std::env::var_os("RECORD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("vidcall-recordings"))
}

/// Runtime-configurable startup retry settings.
/// Invalid or out-of-range environment values fall back/clamp to safe bounds.
fn env_u64(key: &str, default: u64, min: u64, max: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn startup_attempts() -> u32 {
    env_u64("RECORD_FFMPEG_START_ATTEMPTS", 5, 1, 10) as u32
}

fn ready_timeout() -> Duration {
    Duration::from_millis(env_u64(
        "RECORD_FFMPEG_READY_TIMEOUT_MS",
        8_000,
        500,
        30_000,
    ))
}

fn retry_backoff() -> Duration {
    Duration::from_millis(env_u64("RECORD_FFMPEG_RETRY_BACKOFF_MS", 250, 50, 5_000))
}

/// Finds an even UDP port whose port+1 is also free (RTP + RTCP).
/// This is a best-effort probe: the sockets are released before FFmpeg binds,
/// so a small race with other processes remains. Startup retries handle it.
fn free_rtp_port() -> std::io::Result<u16> {
    let mut last_error = None;

    for _ in 0..128 {
        let rtp = match std::net::UdpSocket::bind("127.0.0.1:0") {
            Ok(socket) => socket,
            Err(e) => return Err(e),
        };
        let port = rtp.local_addr()?.port();

        if port % 2 != 0 || port == u16::MAX {
            continue;
        }

        match std::net::UdpSocket::bind(("127.0.0.1", port + 1)) {
            Ok(_rtcp) => return Ok(port),
            Err(e) => last_error = Some(e),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(ErrorKind::AddrInUse, "could not find an even UDP port pair")
    }))
}

/// Starts FFmpeg and retries transient startup/readiness failures on fresh ports.
/// Returns only after the selected RTP port appears to be owned by a listener.
async fn spawn_ffmpeg_with_retry(
    sdp_path: &Path,
    output: &Path,
    publisher: Uuid,
) -> Result<(u16, Child, ChildStdin), String> {
    let attempts = startup_attempts();
    let timeout = ready_timeout();
    let base_backoff = retry_backoff();
    let mut last_error = String::from("no startup attempt completed");

    for attempt in 1..=attempts {
        let port = match free_rtp_port() {
            Ok(port) => port,
            Err(e) => {
                last_error = format!("allocate RTP/RTCP port pair: {e}");
                warn!("recorder {publisher}: attempt {attempt}/{attempts}: {last_error}");
                if attempt < attempts {
                    tokio::time::sleep(backoff_for(attempt, base_backoff)).await;
                    continue;
                }
                break;
            }
        };

        let origin = Origin::new("-", 0, NetType::In, AddrType::Ip4, "127.0.0.1");
        let video = Media::builder(MediaType::Video, port, TransportProto::RtpAvp, "96")
            .attribute(Attribute::with_value("rtpmap", "96 H264/90000"))
            .attribute(Attribute::with_value("fmtp", "96 packetization-mode=1"))
            .build();

        let session = sdp_types::Session::builder(origin, "recorder")
            .time(sdp_types::Time::new(0, 0))
            .media(video)
            .build();

        let mut out = Vec::new();
        session.write(&mut out).unwrap_or_default();

        let sdp = String::from_utf8(out)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            .unwrap_or_default();

        tokio::fs::write(sdp_path, sdp)
            .await
            .map_err(|e| format!("write SDP {}: {e}", sdp_path.display()))?;

        const SETTS_FILTER: &str = concat!(
            r"setts=pts=if(eq(PTS\,NOPTS)\,N*3000\,PTS)",
            r":dts=if(eq(DTS\,NOPTS)\,if(eq(PTS\,NOPTS)\,N*3000\,PTS)\,DTS)",
            r":duration=if(eq(DURATION\,0)\,3000\,DURATION)",
            r":time_base=1/90000",
        );

        let mut child = FfmpegBuilder::new()
            .input_sdp(&sdp_path)
            .video_copy("0:v:0")
            .setts(SETTS_FILTER)
            .disable_audio()
            .output(&output)
            .stdin_piped()
            .stdout_null()
            .stderr_inherit()
            .build()
            .spawn()
            .map_err(|e| format!("spawn ffmpeg for {publisher}: {e}"))?;


        match wait_ffmpeg_ready(port, &mut child, timeout).await {
            Ok(()) => {
                let Some(stdin) = child.stdin.take() else {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    return Err("ffmpeg stdin unavailable after startup".to_string());
                };

                info!(
                    "recorder {publisher}: FFmpeg ready on UDP {port} (attempt {attempt}/{attempts})"
                );
                return Ok((port, child, stdin));
            }
            Err(e) => {
                last_error = e;
                warn!(
                    "recorder {publisher}: startup attempt {attempt}/{attempts} on UDP {port} failed: {last_error}"
                );
                let _ = child.kill().await;
                let _ = child.wait().await;
                let _ = tokio::fs::remove_file(output).await;

                if attempt < attempts {
                    tokio::time::sleep(backoff_for(attempt, base_backoff)).await;
                }
            }
        }
    }

    Err(format!(
        "FFmpeg startup failed after {attempts} attempt(s): {last_error}"
    ))
}

fn backoff_for(attempt: u32, base: Duration) -> Duration {
    let multiplier = 1u32 << attempt.saturating_sub(1).min(5);
    base.saturating_mul(multiplier).min(Duration::from_secs(5))
}


async fn create_tee(dir: PathBuf, name: String) -> Result<File, String> {
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("create {}: {e}", dir.display()))?;

    let path = dir.join(name);
    let file = File::create(&path)
        .await
        .map_err(|e| format!("create {}: {e}", path.display()))?;

    info!("tee h264 to {}", path.display());
    Ok(file)
}

/// True if this RTP H264 payload starts an SPS or IDR NAL
/// (single NAL, STAP-A containing one, or the first FU-A fragment of one).
fn rtp_starts_keyframe(p: &[u8]) -> bool {
    let Some(&b0) = p.first() else {
        return false;
    };

    match b0 & 0x1f {
        5 | 7 => true,
        24 => {
            // STAP-A: [hdr][size:u16][nal]...
            let mut i = 1;
            while i + 3 <= p.len() {
                let size = u16::from_be_bytes([p[i], p[i + 1]]) as usize;
                let Some(&h) = p.get(i + 2) else {
                    break;
                };
                if matches!(h & 0x1f, 5 | 7) {
                    return true;
                }
                i += 2 + size;
            }
            false
        }
        28 => {
            // FU-A: [fu_indicator][fu_header], start bit + original NAL type
            match p.get(1) {
                Some(&fu) => fu & 0x80 != 0 && matches!(fu & 0x1f, 5 | 7),
                None => false,
            }
        }
        _ => false,
    }
}

/// Re-serialises the RTP packet for the loopback (no CSRC, no extensions).
/// Sequence number, timestamp and marker are preserved.
fn rtp_bytes(pkt: &Packet) -> Vec<u8> {
    pkt.marshal().expect("marshal").to_vec()
}

async fn run_recorder(
    mut rx: mpsc::Receiver<Packet>,
    sock: UdpSocket,
    mut stdin: ChildStdin,
    mut child: Child,
    mut tee: Option<File>,
    output: PathBuf,
) -> Result<(), String> {
    // FFmpeg readiness is checked before this task is spawned; packets can
    // queue in the channel during startup.
    let mut health_check = tokio::time::interval(Duration::from_millis(500));
    health_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let mut depacketizer = H264Packet::default();
    let mut started = false;
    let mut frames = 0u64;
    let mut packets = 0u64;
    let mut errors = 0u64;
    let mut refused_total = 0u64;
    let mut refused_streak = 0u32;
    let mut write_error: Option<String> = None;

    loop {
        let pkt = tokio::select! {
            biased;
            _ = health_check.tick() => {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        write_error = Some(format!("ffmpeg exited during recording: {status}"));
                        break;
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        write_error = Some(format!("poll ffmpeg during recording: {e}"));
                        break;
                    }
                }
            }
            packet = rx.recv() => match packet {
                Some(packet) => packet,
                None => break,
            }
        };

        if !started {
            // ffmpeg cannot probe a stream that does not begin at SPS/PPS or
            // IDR, so wait for the keyframe requested via PLI.
            if !rtp_starts_keyframe(&pkt.payload) {
                continue;
            }
            started = true;
        }

        if let Some(file) = tee.as_mut() {
            match depacketizer.depacketize(&pkt.payload) {
                Ok(nal) if !nal.is_empty() => {
                    if let Err(e) = file.write_all(&nal).await {
                        write_error = Some(format!("tee write {}: {e}", output.display()));
                        break;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    errors += 1;
                    if errors.is_multiple_of(100) {
                        warn!("depacketize error on {}: {e}", output.display());
                    }
                }
            }
        }

        match sock.send(&rtp_bytes(&pkt)).await {
            Ok(_) => {
                refused_streak = 0;
                packets += 1;

                if pkt.header.marker {
                    frames += 1;
                    if frames.is_multiple_of(100) {
                        info!(
                            "recording {}: {frames} frames, {packets} packets",
                            output.display()
                        );
                    }
                }
            }
            // ICMP port unreachable from an earlier datagram: ffmpeg is not
            // (or no longer) listening. Drop the packet; give up only if it persists.
            Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
                refused_total += 1;
                refused_streak += 1;
                if refused_total == 1 {
                    warn!("rtp send refused on {}, dropping", output.display());
                }
                if refused_streak >= MAX_CONSECUTIVE_REFUSED {
                    write_error = Some(format!(
                        "rtp send {}: refused {refused_streak} times in a row, ffmpeg not listening",
                        output.display()
                    ));
                    break;
                }
            }
            Err(e) => {
                write_error = Some(format!("rtp send {}: {e}", output.display()));
                break;
            }
        }
    }

    if let Some(mut file) = tee {
        let _ = file.flush().await;
    }

    let status = if packets == 0 {
        // ffmpeg never received data, so it is still probing and ignores "q".
        let _ = child.kill().await;
        let _ = child.wait().await;
        let _ = tokio::fs::remove_file(&output).await;
        "killed (no data sent)".to_string()
    } else {
        // Ask ffmpeg to finish and write the Matroska trailer.
        let _ = stdin.write_all(b"q\n").await;
        let _ = stdin.flush().await;
        drop(stdin);

        match tokio::time::timeout(Duration::from_secs(10), child.wait()).await {
            Ok(Ok(status)) => status.to_string(),
            Ok(Err(e)) => return Err(format!("wait ffmpeg: {e}")),
            Err(_) => {
                warn!("ffmpeg did not exit in 10s, killing {}", output.display());
                let _ = child.kill().await;
                "killed".to_string()
            }
        }
    };

    info!(
        "recording finished {}: {status}, {frames} frames, {packets} packets, {refused_total} refused",
        output.display()
    );

    if let Some(e) = write_error {
        return Err(e);
    }

    Ok(())
}
