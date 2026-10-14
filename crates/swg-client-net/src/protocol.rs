// crates/swg-client-net/src/protocol.rs

use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const OP_LOGIN_CLIENT_ID: u32 = 0x4113_1F36;
pub const OP_LOGIN_CLIENT_TOKEN: u32 = 0xAAB2_90E6;
pub const OP_LOGIN_CLUSTER_STATUS: u32 = 0x3436_AE9F;
pub const OP_SELECT_CHARACTER: u32 = 0xB52F_EE8C;
pub const OP_SCENE_CREATE: u32 = 0x3A86_3116;
pub const OP_BASELINE_MESSAGE: u32 = 0x68A7_FF74;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("Message buffer too short for opcode: {0}")]
    BufferTooShort(usize),
    #[error("Unknown application message opcode: {0:#010x}")]
    UnknownOpcode(u32),
    #[error("Malformed string in message: {0}")]
    MalformedString(String),
}

/// Information describing a live Galaxy cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterEntry {
    pub cluster_id: u32,
    pub name: String,
    pub status: u32,
    pub ping_ms: u32,
    pub population: u32,
}

/// High-level SWG application messages.
#[derive(Debug, Clone, PartialEq)]
pub enum GameMessage {
    LoginClientId {
        account_id: u32,
        session_key: String,
    },
    LoginClientToken {
        token: String,
        account_id: u32,
        station_id: u32,
    },
    LoginClusterStatus {
        clusters: Vec<ClusterEntry>,
    },
    SelectCharacter {
        character_id: u64,
    },
    SceneCreate {
        scene_id: String,
        terrain_file: String,
        spawn_pos: [f32; 3],
    },
    BaselineMessage {
        object_id: u64,
        type_id: u32,
        package_type: u8,
        data: Bytes,
    },
}

impl GameMessage {
    /// Encodes the message to raw bytes with 4-byte little-endian opcode.
    #[must_use]
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(128);

        match self {
            GameMessage::LoginClientId {
                account_id,
                session_key,
            } => {
                buf.put_u32_le(OP_LOGIN_CLIENT_ID);
                buf.put_u32_le(*account_id);
                Self::put_string(&mut buf, session_key);
            }
            GameMessage::LoginClientToken {
                token,
                account_id,
                station_id,
            } => {
                buf.put_u32_le(OP_LOGIN_CLIENT_TOKEN);
                Self::put_string(&mut buf, token);
                buf.put_u32_le(*account_id);
                buf.put_u32_le(*station_id);
            }
            GameMessage::LoginClusterStatus { clusters } => {
                buf.put_u32_le(OP_LOGIN_CLUSTER_STATUS);
                buf.put_u32_le(clusters.len() as u32);
                for c in clusters {
                    buf.put_u32_le(c.cluster_id);
                    Self::put_string(&mut buf, &c.name);
                    buf.put_u32_le(c.status);
                    buf.put_u32_le(c.ping_ms);
                    buf.put_u32_le(c.population);
                }
            }
            GameMessage::SelectCharacter { character_id } => {
                buf.put_u32_le(OP_SELECT_CHARACTER);
                buf.put_u64_le(*character_id);
            }
            GameMessage::SceneCreate {
                scene_id,
                terrain_file,
                spawn_pos,
            } => {
                buf.put_u32_le(OP_SCENE_CREATE);
                Self::put_string(&mut buf, scene_id);
                Self::put_string(&mut buf, terrain_file);
                buf.put_f32_le(spawn_pos[0]);
                buf.put_f32_le(spawn_pos[1]);
                buf.put_f32_le(spawn_pos[2]);
            }
            GameMessage::BaselineMessage {
                object_id,
                type_id,
                package_type,
                data,
            } => {
                buf.put_u32_le(OP_BASELINE_MESSAGE);
                buf.put_u64_le(*object_id);
                buf.put_u32_le(*type_id);
                buf.put_u8(*package_type);
                buf.extend_from_slice(data);
            }
        }

        buf.freeze()
    }

    /// Decodes a message from bytes.
    pub fn decode(mut bytes: Bytes) -> Result<Self, ProtocolError> {
        if bytes.len() < 4 {
            return Err(ProtocolError::BufferTooShort(bytes.len()));
        }

        let opcode = bytes.get_u32_le();
        match opcode {
            OP_LOGIN_CLIENT_ID => {
                if bytes.len() < 4 {
                    return Err(ProtocolError::BufferTooShort(bytes.len()));
                }
                let account_id = bytes.get_u32_le();
                let session_key = Self::get_string(&mut bytes)?;
                Ok(GameMessage::LoginClientId {
                    account_id,
                    session_key,
                })
            }
            OP_LOGIN_CLIENT_TOKEN => {
                let token = Self::get_string(&mut bytes)?;
                if bytes.len() < 8 {
                    return Err(ProtocolError::BufferTooShort(bytes.len()));
                }
                let account_id = bytes.get_u32_le();
                let station_id = bytes.get_u32_le();
                Ok(GameMessage::LoginClientToken {
                    token,
                    account_id,
                    station_id,
                })
            }
            OP_LOGIN_CLUSTER_STATUS => {
                if bytes.len() < 4 {
                    return Err(ProtocolError::BufferTooShort(bytes.len()));
                }
                let count = bytes.get_u32_le() as usize;
                let mut clusters = Vec::with_capacity(count);
                for _ in 0..count {
                    if bytes.len() < 4 {
                        return Err(ProtocolError::BufferTooShort(bytes.len()));
                    }
                    let cluster_id = bytes.get_u32_le();
                    let name = Self::get_string(&mut bytes)?;
                    if bytes.len() < 12 {
                        return Err(ProtocolError::BufferTooShort(bytes.len()));
                    }
                    let status = bytes.get_u32_le();
                    let ping_ms = bytes.get_u32_le();
                    let population = bytes.get_u32_le();
                    clusters.push(ClusterEntry {
                        cluster_id,
                        name,
                        status,
                        ping_ms,
                        population,
                    });
                }
                Ok(GameMessage::LoginClusterStatus { clusters })
            }
            OP_SELECT_CHARACTER => {
                if bytes.len() < 8 {
                    return Err(ProtocolError::BufferTooShort(bytes.len()));
                }
                let character_id = bytes.get_u64_le();
                Ok(GameMessage::SelectCharacter { character_id })
            }
            OP_SCENE_CREATE => {
                let scene_id = Self::get_string(&mut bytes)?;
                let terrain_file = Self::get_string(&mut bytes)?;
                if bytes.len() < 12 {
                    return Err(ProtocolError::BufferTooShort(bytes.len()));
                }
                let spawn_x = bytes.get_f32_le();
                let spawn_y = bytes.get_f32_le();
                let spawn_z = bytes.get_f32_le();
                Ok(GameMessage::SceneCreate {
                    scene_id,
                    terrain_file,
                    spawn_pos: [spawn_x, spawn_y, spawn_z],
                })
            }
            OP_BASELINE_MESSAGE => {
                if bytes.len() < 13 {
                    return Err(ProtocolError::BufferTooShort(bytes.len()));
                }
                let object_id = bytes.get_u64_le();
                let type_id = bytes.get_u32_le();
                let package_type = bytes.get_u8();
                Ok(GameMessage::BaselineMessage {
                    object_id,
                    type_id,
                    package_type,
                    data: bytes,
                })
            }
            other => Err(ProtocolError::UnknownOpcode(other)),
        }
    }

    fn put_string(buf: &mut BytesMut, s: &str) {
        let bytes = s.as_bytes();
        buf.put_u32_le(bytes.len() as u32);
        buf.extend_from_slice(bytes);
    }

    fn get_string(buf: &mut Bytes) -> Result<String, ProtocolError> {
        if buf.len() < 4 {
            return Err(ProtocolError::BufferTooShort(buf.len()));
        }
        let len = buf.get_u32_le() as usize;
        if buf.len() < len {
            return Err(ProtocolError::BufferTooShort(buf.len()));
        }
        let bytes = buf.copy_to_bytes(len);
        String::from_utf8(bytes.to_vec()).map_err(|e| ProtocolError::MalformedString(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_client_id_roundtrip() {
        let msg = GameMessage::LoginClientId {
            account_id: 42,
            session_key: "SITH_LORD_SECRET_SESSION_TOKEN_123".to_string(),
        };

        let encoded = msg.encode();
        let decoded = GameMessage::decode(encoded).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_cluster_status_roundtrip() {
        let msg = GameMessage::LoginClusterStatus {
            clusters: vec![
                ClusterEntry {
                    cluster_id: 1,
                    name: "Bloodfin".to_string(),
                    status: 1,
                    ping_ms: 25,
                    population: 450,
                },
                ClusterEntry {
                    cluster_id: 2,
                    name: "Bria".to_string(),
                    status: 1,
                    ping_ms: 30,
                    population: 800,
                },
            ],
        };

        let encoded = msg.encode();
        let decoded = GameMessage::decode(encoded).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_scene_create_roundtrip() {
        let msg = GameMessage::SceneCreate {
            scene_id: "tatooine".to_string(),
            terrain_file: "terrain/tatooine.trn".to_string(),
            spawn_pos: [100.0, 50.0, -250.0],
        };

        let encoded = msg.encode();
        let decoded = GameMessage::decode(encoded).unwrap();
        assert_eq!(msg, decoded);
    }
}
