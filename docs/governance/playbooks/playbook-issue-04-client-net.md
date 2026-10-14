# Playbook: Issue 04 — Networking: Client-Side SOE UDP Network Stack, Connection Session & Handshake (`swg-client-net`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#4  
> **Branch:** `enhancement/4-client-soe-network-stack`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement the client-side network protocol stack in `crates/swg-client-net`:

1. **SOE Protocol Layer & Cryptography (`soe.rs`):**
   - Packet framing, CRC32 appending, XOR/seed encryption, and zlib packet decompression.
   - Core SOE packet types: `SessionRequest` (0x01), `SessionResponse` (0x02), `MultiPacket` (0x03), `DataChannel` (0x09), `Fragmented` (0x0D), `Ack` (0x15), `Ping` (0x05), `Disconnect` (0x0A).
   - Sliding-window sequence tracking, cumulative ACK generation, and fragment reassembler.

2. **Asynchronous UDP Client Connection (`connection.rs`):**
   - Asynchronous UDP transport using `tokio::net::UdpSocket`.
   - Heartbeat tick, retransmission timeout (RTO) queue, and packet sequencer.

3. **Client Network Session State Machine (`session.rs`):**
   - States: `Disconnected`, `Connecting`, `Connected`, `Authenticated`, `ZoneTransitioning`, `InScene`.
   - Login handshake: sending `LoginClientId`, receiving `LoginClientToken`, handling `LoginClusterStatus` & `LoginClusterList`.
   - Zone handover: reconnecting to gateway/zone endpoint with session ticket, handling `SceneCreate` and `BaselineMessage`.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Completes SOE handshake with seed exchange and sequence tracking.
- [x] Sliding-window acknowledgment and multi-packet fragment reassembly.
- [x] Authenticates against simulated login server, receives cluster list.
- [x] Handles zone transition and initial scene baseline decoding.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all gates.
