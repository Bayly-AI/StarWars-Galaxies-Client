// crates/swg-client-net/src/lib.rs

pub mod protocol;
pub mod session;
pub mod soe;

pub use protocol::{
    ClusterEntry, GameMessage, OP_BASELINE_MESSAGE, OP_LOGIN_CLIENT_ID, OP_LOGIN_CLIENT_TOKEN,
    OP_LOGIN_CLUSTER_STATUS, OP_SCENE_CREATE, OP_SELECT_CHARACTER, ProtocolError,
};
pub use session::{ClientNetworkSession, NetError, SceneInfo, SessionState};
pub use soe::{
    FragmentReassembler, PacketSequencer, SOE_OP_ACK, SOE_OP_DATA_CHANNEL, SOE_OP_DISCONNECT,
    SOE_OP_FRAGMENTED, SOE_OP_MULTI_PACKET, SOE_OP_PING, SOE_OP_SESSION_REQUEST,
    SOE_OP_SESSION_RESPONSE, SoeError, SoePacket, apply_soe_crypto, compute_soe_crc32,
};
