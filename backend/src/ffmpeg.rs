use std::io::ErrorKind;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::{Child, Command};

pub struct FfmpegBuilder {
    instance: Command,
}

impl FfmpegBuilder {
    pub fn new() -> Self {
        let mut instance = Command::new("ffmpeg");

        instance
            .args(["-hide_banner", "-loglevel", "warning"])
            .args(["-protocol_whitelist", "file,udp,rtp"])
            .args(["-fflags", "+genpts"])
            .args(["-analyzeduration", "1000000"])
            .args(["-probesize", "1000000"]);

        Self { instance }
    }

    pub fn input_sdp(mut self, path: impl AsRef<std::ffi::OsStr>) -> Self {
        self.instance.args(["-f", "sdp", "-i"]).arg(path);

        self
    }

    pub fn video_copy(mut self, map: &str) -> Self {
        self.instance.args(["-map", map]).args(["-c:v", "copy"]);

        self
    }

    pub fn setts(mut self, filter: &str) -> Self {
        self.instance.args(["-bsf:v", filter]);

        self
    }

    pub fn disable_audio(mut self) -> Self {
        self.instance.arg("-an");
        self
    }

    pub fn output(mut self, path: impl AsRef<std::ffi::OsStr>) -> Self {
        self.instance.arg("-y").arg(path);

        self
    }

    pub fn stdin_piped(mut self) -> Self {
        self.instance.stdin(Stdio::piped());
        self
    }

    pub fn stdout_null(mut self) -> Self {
        self.instance.stdout(Stdio::null());
        self
    }

    pub fn stderr_inherit(mut self) -> Self {
        self.instance.stderr(Stdio::inherit());
        self
    }

    pub fn build(self) -> Command {
        self.instance
    }
}

fn udp_port_in_use(port: u16) -> bool {
    match std::net::UdpSocket::bind(("127.0.0.1", port)) {
        Err(e) if e.kind() == ErrorKind::AddrInUse => true,
        _ => false,
    }
}


pub async fn wait_ffmpeg_ready(port: u16, child: &mut Child, timeout: Duration) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(format!("ffmpeg exited before ready (status {status})"));
            }
            Ok(None) => {}
            Err(e) => return Err(format!("poll ffmpeg process: {e}")),
        }

        let rtp_in_use = udp_port_in_use(port);
        let rtcp_in_use = udp_port_in_use(port.saturating_add(1));
        if rtp_in_use && rtcp_in_use {
            return Ok(());
        }

        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "ffmpeg did not bind UDP port {port} within {} ms",
                timeout.as_millis()
            ));
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
