use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::sfu::errors::InvalidRoomId;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RoomId(Arc<str>);

impl RoomId {
    pub const LEN: usize = 21;

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
