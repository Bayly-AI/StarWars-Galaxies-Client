// crates/swg-client-net/src/session.rs

use crate::protocol::{ClusterEntry, GameMessage, ProtocolError};
use crate::soe::{FragmentReassembler, PacketSequencer, SoeError, SoePacket};
use bytes::Bytes;
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::net::UdpSocket;

#[derive(Error, Debug)]
pub enum NetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SOE protocol error: {0}")]
    Soe(#[from] SoeError),
    #[error("Application protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("Connection timed out waiting for server response")]
    Timeout,
    #[error("Session handshake rejected or disconnected: {0}")]
    HandshakeFailed(String),
}

/// Network session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Disconnected,
    Connecting,
    Connected,
    Authenticated,
    ZoneTransition,
    InScene,
}

/// Information returned upon entering a scene.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneInfo {
    pub scene_id: String,
    pub terrain_file: String,
    pub spawn_pos: [f32; 3],
}

/// Client Network Session managing the UDP socket, SOE handshake, sequencing, and message exchange.
pub struct ClientNetworkSession {
    socket: Arc<UdpSocket>,
    server_addr: SocketAddr,
    state: SessionState,
    connection_id: u32,
    seed: u32,
    encryption_enabled: bool,
    send_sequence: u16,
    sequencer: PacketSequencer,
    reassembler: FragmentReassembler,
}

impl ClientNetworkSession {
    /// Connects to a remote server and performs the full SOE handshake.
    pub async fn connect(server_addr_str: &str) -> Result<Self, NetError> {
        let server_addr: SocketAddr = server_addr_str
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let connection_id = fastrand::u32(1000..99999);

        let mut session = Self {
            socket: Arc::new(socket),
            server_addr,
            state: SessionState::Connecting,
            connection_id,
            seed: 0,
            encryption_enabled: false,
            send_sequence: 0,
            sequencer: PacketSequencer::new(),
            reassembler: FragmentReassembler::new(),
        };

        // Send SessionRequest
        let req = SoePacket::SessionRequest {
            crc_length: 4,
            connection_id,
            client_udp_buffer_size: 496,
        };
        session.send_packet(&req, false).await?;

        // Wait for SessionResponse
        let mut buf = [0u8; 1024];
        let (len, from_addr) = session.socket.recv_from(&mut buf).await?;
        if from_addr != server_addr {
            return Err(NetError::HandshakeFailed("Address mismatch".to_string()));
        }

        let raw = Bytes::copy_from_slice(&buf[..len]);
        let resp = SoePacket::decode(raw, 0, false)?;

        match resp {
            SoePacket::SessionResponse {
                seed,
                encryption_enabled,
                ..
            } => {
                session.seed = seed;
                session.encryption_enabled = encryption_enabled;
                session.state = SessionState::Connected;
                Ok(session)
            }
            SoePacket::Disconnect { reason, .. } => Err(NetError::HandshakeFailed(format!(
                "Disconnected with reason code: {reason}"
            ))),
            _ => Err(NetError::HandshakeFailed(
                "Expected SessionResponse".to_string(),
            )),
        }
    }

    /// Current session state.
    #[must_use]
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Client connection ID assigned for this session.
    #[must_use]
    pub fn connection_id(&self) -> u32 {
        self.connection_id
    }

    /// Sends a low-level SOE packet over UDP.
    pub async fn send_packet(&self, packet: &SoePacket, encrypt: bool) -> Result<(), NetError> {
        let encoded = packet.encode(self.seed, encrypt && self.encryption_enabled);
        self.socket.send_to(&encoded, self.server_addr).await?;
        Ok(())
    }

    /// Sends an application game message on the reliable data channel.
    pub async fn send_game_message(&mut self, msg: &GameMessage) -> Result<(), NetError> {
        let payload = msg.encode();
        let seq = self.send_sequence;
        self.send_sequence = self.send_sequence.wrapping_add(1);

        let packet = SoePacket::DataChannel {
            sequence: seq,
            payload,
        };
        self.send_packet(&packet, true).await
    }

    /// Receives and decodes the next game message, handling acknowledgments and sequencing.
    pub async fn recv_game_message(&mut self) -> Result<GameMessage, NetError> {
        let mut buf = [0u8; 2048];

        loop {
            let (len, from_addr) = self.socket.recv_from(&mut buf).await?;
            if from_addr != self.server_addr {
                continue;
            }

            let raw = Bytes::copy_from_slice(&buf[..len]);
            let packet = match SoePacket::decode(raw, self.seed, self.encryption_enabled) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("Failed to decode SOE packet: {e:?}");
                    continue;
                }
            };

            match packet {
                SoePacket::DataChannel { sequence, payload } => {
                    let ready_payloads = self.sequencer.process_incoming(sequence, payload);

                    // Send ACK
                    if let Some(ack) = self.sequencer.generate_ack() {
                        let _ = self.send_packet(&ack, false).await;
                    }

                    if let Some(first) = ready_payloads.into_iter().next() {
                        let game_msg = GameMessage::decode(first)?;
                        return Ok(game_msg);
                    }
                }
                SoePacket::Fragmented {
                    sequence,
                    total_size,
                    payload,
                } => {
                    let ready_fragments = self.sequencer.process_incoming(sequence, payload);

                    if let Some(ack) = self.sequencer.generate_ack() {
                        let _ = self.send_packet(&ack, false).await;
                    }

                    for frag in ready_fragments {
                        if let Some(full_payload) = self.reassembler.add_fragment(total_size, &frag)
                        {
                            let game_msg = GameMessage::decode(full_payload)?;
                            return Ok(game_msg);
                        }
                    }
                }
                SoePacket::Ack { .. } => {
                    // Handled ACK
                }
                SoePacket::Ping => {
                    // Keepalive ping received
                }
                SoePacket::Disconnect { reason, .. } => {
                    self.state = SessionState::Disconnected;
                    return Err(NetError::HandshakeFailed(format!(
                        "Server disconnected: {reason}"
                    )));
                }
                _ => {}
            }
        }
    }

    /// Authenticates with the login server and awaits the Galaxy cluster list.
    pub async fn authenticate(
        &mut self,
        account_id: u32,
        session_key: &str,
    ) -> Result<Vec<ClusterEntry>, NetError> {
        self.send_game_message(&GameMessage::LoginClientId {
            account_id,
            session_key: session_key.to_string(),
        })
        .await?;

        // Expect token or cluster status
        let mut clusters = Vec::new();
        while clusters.is_empty() {
            match self.recv_game_message().await? {
                GameMessage::LoginClientToken { .. } => {
                    self.state = SessionState::Authenticated;
                }
                GameMessage::LoginClusterStatus { clusters: list } => {
                    clusters = list;
                    self.state = SessionState::Authenticated;
                    break;
                }
                _ => {}
            }
        }

        Ok(clusters)
    }

    /// Transitions to the zone server and retrieves initial scene creation data.
    pub async fn enter_scene(&mut self, character_id: u64) -> Result<SceneInfo, NetError> {
        self.state = SessionState::ZoneTransition;
        self.send_game_message(&GameMessage::SelectCharacter { character_id })
            .await?;

        loop {
            match self.recv_game_message().await? {
                GameMessage::SceneCreate {
                    scene_id,
                    terrain_file,
                    spawn_pos,
                } => {
                    self.state = SessionState::InScene;
                    return Ok(SceneInfo {
                        scene_id,
                        terrain_file,
                        spawn_pos,
                    });
                }
                _ => continue,
            }
        }
    }
}
