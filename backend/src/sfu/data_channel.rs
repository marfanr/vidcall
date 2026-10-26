use crate::sfu::room::RoomSfu;
use rtc::data_channel::RTCDataChannelMessage;
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ControlMessage {
    Record,
}

pub async fn handle_data_channel(
    conn_id: Uuid,
    msg: RTCDataChannelMessage,
    label: Arc<str>,
    room: Arc<RoomSfu>,
) {
    let json = sonic_rs::from_slice::<ControlMessage>(&msg.data);
    match json {
        Ok(ControlMessage::Record) => {
            info!("Received data {label}");
        }
        Err(_) => {}
    };
}
