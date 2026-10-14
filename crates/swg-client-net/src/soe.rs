// crates/swg-client-net/src/soe.rs

use bytes::{Buf, BufMut, Bytes, BytesMut};
use crc32fast::Hasher;
use std::collections::BTreeMap;
use thiserror::Error;

pub const SOE_OP_SESSION_REQUEST: u16 = 0x0001;
pub const SOE_OP_SESSION_RESPONSE: u16 = 0x0002;
pub const SOE_OP_MULTI_PACKET: u16 = 0x0003;
pub const SOE_OP_DISCONNECT: u16 = 0x0005;
pub const SOE_OP_PING: u16 = 0x0006;
pub const SOE_OP_DATA_CHANNEL: u16 = 0x0009;
pub const SOE_OP_FRAGMENTED: u16 = 0x000D;
pub const SOE_OP_ACK: u16 = 0x0015;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum SoeError {
    #[error("Packet too short: length {0} is less than minimum {1}")]
    PacketTooShort(usize, usize),
    #[error("CRC32 validation failed: expected {0:#x}, computed {1:#x}")]
    CrcMismatch(u32, u32),
    #[error("Unknown or invalid SOE opcode: {0:#06x}")]
    InvalidOpcode(u16),
    #[error("Fragment error: {0}")]
    FragmentError(String),
}

/// Computes the 32-bit CRC with session seed.
#[must_use]
pub fn compute_soe_crc32(seed: u32, data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(&seed.to_le_bytes());
    hasher.update(data);
    hasher.finalize()
}

/// Applies SOE symmetric XOR keystream encryption.
pub fn apply_soe_crypto(seed: u32, data: &mut [u8]) {
    let mut key = seed;
    for byte in data.iter_mut() {
        key = key.wrapping_mul(1103515245).wrapping_add(12345);
        let mask = ((key >> 16) & 0xFF) as u8;
        *byte ^= mask;
    }
}

/// High-level SOE packet envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoePacket {
    SessionRequest {
        crc_length: u32,
        connection_id: u32,
        client_udp_buffer_size: u32,
    },
    SessionResponse {
        connection_id: u32,
        seed: u32,
        server_udp_buffer_size: u32,
        crc_length: u8,
        encryption_enabled: bool,
    },
    DataChannel {
        sequence: u16,
        payload: Bytes,
    },
    Fragmented {
        sequence: u16,
        total_size: u32,
        payload: Bytes,
    },
    Ack {
        sequence: u16,
    },
    Ping,
    Disconnect {
        connection_id: u32,
        reason: u16,
    },
}

impl SoePacket {
    /// Serializes an SOE packet into raw UDP payload bytes (big-endian opcodes).
    #[must_use]
    pub fn encode(&self, seed: u32, enable_encryption: bool) -> Bytes {
        let mut buf = BytesMut::with_capacity(64);

        match self {
            SoePacket::SessionRequest {
                crc_length,
                connection_id,
                client_udp_buffer_size,
            } => {
                buf.put_u16(SOE_OP_SESSION_REQUEST);
                buf.put_u32(*crc_length);
                buf.put_u32(*connection_id);
                buf.put_u32(*client_udp_buffer_size);
            }
            SoePacket::SessionResponse {
                connection_id,
                seed,
                server_udp_buffer_size,
                crc_length,
                encryption_enabled,
            } => {
                buf.put_u16(SOE_OP_SESSION_RESPONSE);
                buf.put_u32(*connection_id);
                buf.put_u32(*seed);
                buf.put_u32(*server_udp_buffer_size);
                buf.put_u8(*crc_length);
                buf.put_u8(if *encryption_enabled { 1 } else { 0 });
            }
            SoePacket::DataChannel { sequence, payload } => {
                buf.put_u16(SOE_OP_DATA_CHANNEL);
                buf.put_u16(*sequence);
                buf.extend_from_slice(payload);
            }
            SoePacket::Fragmented {
                sequence,
                total_size,
                payload,
            } => {
                buf.put_u16(SOE_OP_FRAGMENTED);
                buf.put_u16(*sequence);
                buf.put_u32(*total_size);
                buf.extend_from_slice(payload);
            }
            SoePacket::Ack { sequence } => {
                buf.put_u16(SOE_OP_ACK);
                buf.put_u16(*sequence);
            }
            SoePacket::Ping => {
                buf.put_u16(SOE_OP_PING);
            }
            SoePacket::Disconnect {
                connection_id,
                reason,
            } => {
                buf.put_u16(SOE_OP_DISCONNECT);
                buf.put_u32(*connection_id);
                buf.put_u16(*reason);
            }
        }

        // Apply encryption if enabled (skipping opcode)
        if enable_encryption && buf.len() > 2 {
            apply_soe_crypto(seed, &mut buf[2..]);
        }

        // Append 4-byte CRC32
        let crc = compute_soe_crc32(seed, &buf);
        buf.put_u32(crc);

        buf.freeze()
    }

    /// Deserializes a raw UDP payload into an SOE packet.
    pub fn decode(raw: Bytes, seed: u32, encryption_enabled: bool) -> Result<Self, SoeError> {
        if raw.len() < 6 {
            return Err(SoeError::PacketTooShort(raw.len(), 6));
        }

        // Verify CRC32 at the end
        let payload_len = raw.len() - 4;
        let expected_crc = u32::from_be_bytes([
            raw[payload_len],
            raw[payload_len + 1],
            raw[payload_len + 2],
            raw[payload_len + 3],
        ]);

        let computed_crc = compute_soe_crc32(seed, &raw[..payload_len]);
        if expected_crc != computed_crc {
            return Err(SoeError::CrcMismatch(expected_crc, computed_crc));
        }

        let mut data = raw.slice(..payload_len);
        let opcode = data.get_u16();

        // Decrypt if encryption enabled (skipping opcode)
        if encryption_enabled && !data.is_empty() {
            let mut mutable = data.to_vec();
            apply_soe_crypto(seed, &mut mutable);
            data = Bytes::from(mutable);
        }

        match opcode {
            SOE_OP_SESSION_REQUEST => {
                if data.len() < 12 {
                    return Err(SoeError::PacketTooShort(data.len(), 12));
                }
                let crc_length = data.get_u32();
                let connection_id = data.get_u32();
                let client_udp_buffer_size = data.get_u32();
                Ok(SoePacket::SessionRequest {
                    crc_length,
                    connection_id,
                    client_udp_buffer_size,
                })
            }
            SOE_OP_SESSION_RESPONSE => {
                if data.len() < 14 {
                    return Err(SoeError::PacketTooShort(data.len(), 14));
                }
                let connection_id = data.get_u32();
                let seed = data.get_u32();
                let server_udp_buffer_size = data.get_u32();
                let crc_length = data.get_u8();
                let encryption_enabled = data.get_u8() != 0;
                Ok(SoePacket::SessionResponse {
                    connection_id,
                    seed,
                    server_udp_buffer_size,
                    crc_length,
                    encryption_enabled,
                })
            }
            SOE_OP_DATA_CHANNEL => {
                if data.len() < 2 {
                    return Err(SoeError::PacketTooShort(data.len(), 2));
                }
                let sequence = data.get_u16();
                Ok(SoePacket::DataChannel {
                    sequence,
                    payload: data,
                })
            }
            SOE_OP_FRAGMENTED => {
                if data.len() < 6 {
                    return Err(SoeError::PacketTooShort(data.len(), 6));
                }
                let sequence = data.get_u16();
                let total_size = data.get_u32();
                Ok(SoePacket::Fragmented {
                    sequence,
                    total_size,
                    payload: data,
                })
            }
            SOE_OP_ACK => {
                if data.len() < 2 {
                    return Err(SoeError::PacketTooShort(data.len(), 2));
                }
                let sequence = data.get_u16();
                Ok(SoePacket::Ack { sequence })
            }
            SOE_OP_PING => Ok(SoePacket::Ping),
            SOE_OP_DISCONNECT => {
                if data.len() < 6 {
                    return Err(SoeError::PacketTooShort(data.len(), 6));
                }
                let connection_id = data.get_u32();
                let reason = data.get_u16();
                Ok(SoePacket::Disconnect {
                    connection_id,
                    reason,
                })
            }
            other => Err(SoeError::InvalidOpcode(other)),
        }
    }
}

/// Packet fragment reassembler reconstructing large game payloads from SOE fragments.
#[derive(Default)]
pub struct FragmentReassembler {
    expected_total_size: usize,
    accumulated: Vec<u8>,
}

impl FragmentReassembler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            expected_total_size: 0,
            accumulated: Vec::new(),
        }
    }

    /// Adds a fragment. If complete, returns `Some(Bytes)`.
    pub fn add_fragment(&mut self, total_size: u32, fragment: &[u8]) -> Option<Bytes> {
        if self.accumulated.is_empty() {
            self.expected_total_size = total_size as usize;
            self.accumulated.reserve(self.expected_total_size);
        }

        self.accumulated.extend_from_slice(fragment);

        if self.accumulated.len() >= self.expected_total_size {
            let complete = Bytes::from(std::mem::take(&mut self.accumulated));
            self.expected_total_size = 0;
            Some(complete)
        } else {
            None
        }
    }
}

/// Sliding-window sequencer tracking reliable packet reception and ACK generation.
#[derive(Default)]
pub struct PacketSequencer {
    expected_sequence: u16,
    pending_acks: Vec<u16>,
    reorder_buffer: BTreeMap<u16, Bytes>,
}

impl PacketSequencer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            expected_sequence: 0,
            pending_acks: Vec::new(),
            reorder_buffer: BTreeMap::new(),
        }
    }

    /// Processes an incoming sequenced packet. Returns ordered packets ready for handling.
    pub fn process_incoming(&mut self, sequence: u16, payload: Bytes) -> Vec<Bytes> {
        self.pending_acks.push(sequence);

        if sequence == self.expected_sequence {
            let mut ready = vec![payload];
            self.expected_sequence = self.expected_sequence.wrapping_add(1);

            while let Some(next_payload) = self.reorder_buffer.remove(&self.expected_sequence) {
                ready.push(next_payload);
                self.expected_sequence = self.expected_sequence.wrapping_add(1);
            }

            ready
        } else if sequence > self.expected_sequence {
            self.reorder_buffer.insert(sequence, payload);
            Vec::new()
        } else {
            // Duplicate or old packet
            Vec::new()
        }
    }

    /// Generates cumulative ACK packet if unacknowledged packets exist.
    #[must_use]
    pub fn generate_ack(&mut self) -> Option<SoePacket> {
        if self.pending_acks.is_empty() {
            None
        } else {
            let last_ack = self.expected_sequence.wrapping_sub(1);
            self.pending_acks.clear();
            Some(SoePacket::Ack { sequence: last_ack })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soe_session_request_response_roundtrip() {
        let seed = 0x1234_5678;

        let req = SoePacket::SessionRequest {
            crc_length: 4,
            connection_id: 1001,
            client_udp_buffer_size: 496,
        };

        let encoded = req.encode(seed, false);
        let decoded = SoePacket::decode(encoded, seed, false).unwrap();
        assert_eq!(req, decoded);

        let resp = SoePacket::SessionResponse {
            connection_id: 1001,
            seed: 0x9988_7766,
            server_udp_buffer_size: 496,
            crc_length: 4,
            encryption_enabled: true,
        };

        let encoded_resp = resp.encode(seed, false);
        let decoded_resp = SoePacket::decode(encoded_resp, seed, false).unwrap();
        assert_eq!(resp, decoded_resp);
    }

    #[test]
    fn test_soe_crypto_data_channel() {
        let seed = 0xCAFE_BABE;
        let packet = SoePacket::DataChannel {
            sequence: 42,
            payload: Bytes::from_static(b"HELLO_SWG_GALAXY_SERVER"),
        };

        let encoded = packet.encode(seed, true);
        let decoded = SoePacket::decode(encoded, seed, true).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_fragment_reassembly() {
        let mut reassembler = FragmentReassembler::new();

        let part1 = b"PART_1_";
        let part2 = b"PART_2_";
        let part3 = b"COMPLETE!";
        let total_size = (part1.len() + part2.len() + part3.len()) as u32;

        assert_eq!(reassembler.add_fragment(total_size, part1), None);
        assert_eq!(reassembler.add_fragment(total_size, part2), None);
        let full = reassembler.add_fragment(total_size, part3).unwrap();
        assert_eq!(full, Bytes::from_static(b"PART_1_PART_2_COMPLETE!"));
    }

    #[test]
    fn test_packet_sequencer_reordering() {
        let mut seq = PacketSequencer::new();

        // Receive seq 1 first (out of order)
        let ready1 = seq.process_incoming(1, Bytes::from_static(b"PACKET_1"));
        assert!(ready1.is_empty());

        // Receive seq 0
        let ready0 = seq.process_incoming(0, Bytes::from_static(b"PACKET_0"));
        assert_eq!(ready0.len(), 2);
        assert_eq!(ready0[0], Bytes::from_static(b"PACKET_0"));
        assert_eq!(ready0[1], Bytes::from_static(b"PACKET_1"));

        // Receive seq 2
        let ready2 = seq.process_incoming(2, Bytes::from_static(b"PACKET_2"));
        assert_eq!(ready2.len(), 1);
        assert_eq!(ready2[0], Bytes::from_static(b"PACKET_2"));
    }
}
