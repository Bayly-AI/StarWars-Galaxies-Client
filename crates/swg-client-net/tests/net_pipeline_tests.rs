// crates/swg-client-net/tests/net_pipeline_tests.rs

use bytes::Bytes;
use swg_client_net::{ClientNetworkSession, ClusterEntry, GameMessage, SessionState, SoePacket};
use tokio::net::UdpSocket;

#[tokio::test]
async fn test_full_login_and_zone_transition_pipeline() {
    // 1. Spawn mock server on localhost
    let server_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server_sock.local_addr().unwrap();

    let server_handle = tokio::spawn(async move {
        let mut buf = [0u8; 1024];

        // Receive SessionRequest
        let (len, client_addr) = server_sock.recv_from(&mut buf).await.unwrap();
        let raw = Bytes::copy_from_slice(&buf[..len]);
        let req = SoePacket::decode(raw, 0, false).unwrap();

        let conn_id = match req {
            SoePacket::SessionRequest { connection_id, .. } => connection_id,
            other => panic!("Expected SessionRequest, got {other:?}"),
        };

        let seed = 0x55443322;
        // Reply SessionResponse
        let resp = SoePacket::SessionResponse {
            connection_id: conn_id,
            seed,
            server_udp_buffer_size: 496,
            crc_length: 4,
            encryption_enabled: true,
        };
        let encoded_resp = resp.encode(0, false);
        server_sock
            .send_to(&encoded_resp, client_addr)
            .await
            .unwrap();

        // Receive LoginClientId
        let (len, _) = server_sock.recv_from(&mut buf).await.unwrap();
        let raw = Bytes::copy_from_slice(&buf[..len]);
        let data_pkt = SoePacket::decode(raw, seed, true).unwrap();
        let client_msg = match data_pkt {
            SoePacket::DataChannel { payload, .. } => GameMessage::decode(payload).unwrap(),
            other => panic!("Expected DataChannel, got {other:?}"),
        };

        match client_msg {
            GameMessage::LoginClientId { account_id, .. } => {
                assert_eq!(account_id, 101);
            }
            other => panic!("Expected LoginClientId, got {other:?}"),
        }

        // Send LoginClientToken
        let token_msg = GameMessage::LoginClientToken {
            token: "VALID_JWT_STATION_TOKEN".to_string(),
            account_id: 101,
            station_id: 202,
        };
        let token_pkt = SoePacket::DataChannel {
            sequence: 0,
            payload: token_msg.encode(),
        };
        server_sock
            .send_to(&token_pkt.encode(seed, true), client_addr)
            .await
            .unwrap();

        // Send LoginClusterStatus
        let cluster_msg = GameMessage::LoginClusterStatus {
            clusters: vec![ClusterEntry {
                cluster_id: 1,
                name: "Ahazi".to_string(),
                status: 1,
                ping_ms: 15,
                population: 300,
            }],
        };
        let cluster_pkt = SoePacket::DataChannel {
            sequence: 1,
            payload: cluster_msg.encode(),
        };
        server_sock
            .send_to(&cluster_pkt.encode(seed, true), client_addr)
            .await
            .unwrap();

        // Receive SelectCharacter
        let (len, _) = server_sock.recv_from(&mut buf).await.unwrap();
        let raw = Bytes::copy_from_slice(&buf[..len]);
        let sel_pkt = SoePacket::decode(raw, seed, true).unwrap();
        match sel_pkt {
            SoePacket::DataChannel { payload, .. } => {
                let msg = GameMessage::decode(payload).unwrap();
                match msg {
                    GameMessage::SelectCharacter { character_id } => {
                        assert_eq!(character_id, 8888_9999);
                    }
                    other => panic!("Expected SelectCharacter, got {other:?}"),
                }
            }
            SoePacket::Ack { .. } => {
                // If ACK was sent first, read next
                let (len, _) = server_sock.recv_from(&mut buf).await.unwrap();
                let raw = Bytes::copy_from_slice(&buf[..len]);
                let data = SoePacket::decode(raw, seed, true).unwrap();
                if let SoePacket::DataChannel { payload, .. } = data {
                    let msg = GameMessage::decode(payload).unwrap();
                    if let GameMessage::SelectCharacter { character_id } = msg {
                        assert_eq!(character_id, 8888_9999);
                    }
                }
            }
            other => panic!("Expected SelectCharacter packet, got {other:?}"),
        }

        // Send SceneCreate
        let scene_msg = GameMessage::SceneCreate {
            scene_id: "tatooine".to_string(),
            terrain_file: "terrain/tatooine.trn".to_string(),
            spawn_pos: [120.0, 5.0, -340.0],
        };
        let scene_pkt = SoePacket::DataChannel {
            sequence: 2,
            payload: scene_msg.encode(),
        };
        server_sock
            .send_to(&scene_pkt.encode(seed, true), client_addr)
            .await
            .unwrap();
    });

    // 2. Client connect & authenticate
    let mut client = ClientNetworkSession::connect(&server_addr.to_string())
        .await
        .unwrap();
    assert_eq!(client.state(), SessionState::Connected);

    let clusters = client
        .authenticate(101, "SECRET_SESSION_TOKEN")
        .await
        .unwrap();
    assert_eq!(client.state(), SessionState::Authenticated);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].name, "Ahazi");

    // 3. Enter scene
    let scene = client.enter_scene(8888_9999).await.unwrap();
    assert_eq!(client.state(), SessionState::InScene);
    assert_eq!(scene.scene_id, "tatooine");
    assert_eq!(scene.terrain_file, "terrain/tatooine.trn");
    assert_eq!(scene.spawn_pos, [120.0, 5.0, -340.0]);

    server_handle.await.unwrap();
}
