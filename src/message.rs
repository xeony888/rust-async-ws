use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum MessageType {
    Ping = 0,
    Pong = 1,
    State = 2,
    SoccerMove = 3,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage {
    pub msg_type: MessageType,
    pub payload: Vec<u8>,
}

impl WsMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1 + self.payload.len());
        bytes.push(self.msg_type as u8);
        bytes.extend(&self.payload);
        return bytes;
    }
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        let msg_type = match data[0] {
            0 => MessageType::Ping,
            1 => MessageType::Pong,
            2 => MessageType::State,
            3 => MessageType::SoccerMove,
            _ => return None,
        };

        Some(WsMessage {
            msg_type,
            payload: data[1..].to_vec(),
        })
    }
}

impl TryFrom<u8> for MessageType {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MessageType::Ping),
            1 => Ok(MessageType::Pong),
            2 => Ok(MessageType::State),
            3 => Ok(MessageType::SoccerMove),
            _ => Err(()),
        }
    }
}
impl From<MessageType> for u8 {
    fn from(value: MessageType) -> u8 {
        return value as u8;
    }
}

#[derive(Serialize, Deserialize)]
pub struct SoccerMoveMessage {
    pub vx: f32,
    pub vy: f32,
    pub target: u8,
}
