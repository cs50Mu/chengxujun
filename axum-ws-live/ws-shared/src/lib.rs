use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Msg {
    pub room: String,
    pub username: String,
    pub timestamp: u64,
    pub data: MsgData,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MsgData {
    Join,
    Leave,
    Message(String),
}

impl TryFrom<&str> for Msg {
    type Error = serde_json::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        serde_json::from_str(value)
    }
}

impl TryFrom<&Msg> for String {
    type Error = serde_json::Error;

    fn try_from(msg: &Msg) -> Result<Self, Self::Error> {
        serde_json::to_string(msg)
    }
}

impl Msg {
    pub fn new(room: impl Into<String>, username: impl Into<String>, data: MsgData) -> Self {
        Self {
            room: room.into(),
            username: username.into(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            data,
        }
    }

    pub fn join(room: &str, username: &str) -> Self {
        Msg::new(room, username, MsgData::Join)
    }

    pub fn leave(room: &str, username: &str) -> Self {
        Msg::new(room, username, MsgData::Leave)
    }

    pub fn message(room: &str, username: &str, message: MsgData) -> Self {
        Msg::new(room, username, message)
    }
}
