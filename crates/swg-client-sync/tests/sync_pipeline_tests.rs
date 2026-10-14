// crates/swg-client-sync/tests/sync_pipeline_tests.rs

use glam::Vec3;
use swg_client_sync::{
    ClientSyncManager, InputCommand, PlayerPhysicsConfig, ReconciliationOutcome, RemoteSnapshot,
    ServerTransformUpdate,
};

#[test]
fn test_client_prediction_and_high_ping_responsiveness() {
    let mut sync_manager = ClientSyncManager::new(
        Vec3::new(100.0, 0.0, 100.0),
        PlayerPhysicsConfig::default(),
        0.100,
    );

    let start_time = 1000.0;
    let mut current_time = start_time;
    let dt = 0.05; // 20 ticks per sec

    let mut sent_packets = Vec::new();

    // Player presses Forward (W) and Sprints across 10 ticks (0.5s)
    for _ in 0..10 {
        current_time += dt as f64;
        let (seq, state) = sync_manager.handle_local_input(InputCommand {
            delta_time: dt,
            forward: 1.0,
            sprint: true,
            timestamp: current_time,
            ..Default::default()
        });
        sent_packets.push((seq, current_time, state));
    }

    // Verify immediate zero-latency movement
    let initial_predicted_pos = sync_manager.local_visual_position();
    assert!(
        initial_predicted_pos.z > 102.0,
        "Player must immediately advance locally despite simulated high ping"
    );

    // Simulated 200ms ping: server responds to seq 5 (from 250ms ago)
    let (acked_seq, acked_time, acked_state) = sent_packets[4];
    let server_update = ServerTransformUpdate {
        sequence: acked_seq,
        timestamp: acked_time,
        position: acked_state.position,
        velocity: acked_state.velocity,
        yaw: acked_state.yaw,
        pitch: acked_state.pitch,
        is_grounded: acked_state.is_grounded,
    };

    let outcome = sync_manager.reconcile_local_player(&server_update);
    assert!(
        matches!(outcome, ReconciliationOutcome::Accurate { .. }),
        "Server confirmation of correct movement must be marked accurate"
    );

    // Current predicted position should remain advanced (no rewinding or pausing)
    let after_ack_pos = sync_manager.local_visual_position();
    assert!(
        (after_ack_pos.z - initial_predicted_pos.z).abs() < 0.01,
        "Acknowledgment of predicted inputs must not alter current visual position"
    );
}

#[test]
fn test_server_reconciliation_desync_recovery() {
    let mut sync_manager =
        ClientSyncManager::new(Vec3::ZERO, PlayerPhysicsConfig::default(), 0.100);

    let mut current_time = 100.0;
    let dt = 0.05;

    // Client predicts 6 ticks moving forward (+Z)
    for _ in 0..6 {
        current_time += dt as f64;
        sync_manager.handle_local_input(InputCommand {
            delta_time: dt,
            forward: 1.0,
            timestamp: current_time,
            ..Default::default()
        });
    }

    let pre_reconcile_pos = sync_manager.local_visual_position();
    assert!(pre_reconcile_pos.z > 0.5);

    // Server says at seq 3 the player bumped into an unseen Cantina table at Z = 0.20
    let server_update = ServerTransformUpdate {
        sequence: 3,
        timestamp: 100.15,
        position: Vec3::new(0.0, 0.0, 0.20),
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        is_grounded: true,
    };

    let outcome = sync_manager.reconcile_local_player(&server_update);
    assert!(
        matches!(outcome, ReconciliationOutcome::Corrected { .. }),
        "Desynchronization must trigger a correction"
    );

    // Visual position immediately after reconciliation must be smooth (no tele-pop)
    let immediate_visual = sync_manager.local_visual_position();
    assert!(
        (immediate_visual.z - pre_reconcile_pos.z).abs() < 0.02,
        "Visual position must remain continuous and smooth immediately following correction"
    );

    // As render frames advance, error decay pulls visual position smoothly to true physics position
    for _ in 0..15 {
        sync_manager.update(0.02, current_time);
    }

    let final_visual = sync_manager.local_visual_position();
    let final_physics = sync_manager.predictor().state().position;
    assert!(
        (final_visual - final_physics).length() < 0.05,
        "Visual position must converge smoothly to physics position"
    );
}

#[test]
fn test_remote_player_hermite_interpolation_100ms_buffer() {
    let mut sync_manager =
        ClientSyncManager::new(Vec3::ZERO, PlayerPhysicsConfig::default(), 0.100);

    let remote_id = 999;
    let base_time = 500.0;

    // Simulate remote player running in a circle on Tatooine:
    // Radius = 20m, Angular Speed = 0.5 rad/s -> Tangential Speed = 10 m/s
    for i in 0..20 {
        let t = base_time + f64::from(i) * 0.05; // 20Hz snapshots
        let angle = (t - base_time) as f32 * 0.5;
        let pos = Vec3::new(20.0 * angle.cos(), 0.0, 20.0 * angle.sin());
        let vel = Vec3::new(-10.0 * angle.sin(), 0.0, 10.0 * angle.cos());

        sync_manager.handle_remote_snapshot(
            remote_id,
            RemoteSnapshot {
                timestamp: t,
                position: pos,
                velocity: vel,
                yaw: angle,
            },
        );
    }

    // Sample at 60Hz rendering frequency inside the jitter buffer window (current_time = base_time + 0.6)
    // Render time = 500.6 - 0.1 = 500.5
    let mut prev_pos: Option<Vec3> = None;
    for step in 0..15 {
        let render_clock = base_time + 0.5 + f64::from(step) * 0.016;
        let transform = sync_manager
            .sample_remote_entity(remote_id, render_clock)
            .expect("Entity should be validly sampled in jitter buffer");

        assert!(
            !transform.is_extrapolated,
            "Sampling within jitter buffer should be interpolated, not extrapolated"
        );

        if let Some(prev) = prev_pos {
            let step_dist = (transform.position - prev).length();
            // At 10 m/s and 16ms per frame, distance should be approx 0.16m
            assert!(
                step_dist > 0.05 && step_dist < 0.30,
                "Trajectory must be smooth without jumps or freezes: dist={step_dist}"
            );
        }
        prev_pos = Some(transform.position);
    }
}

#[test]
fn test_remote_player_extrapolation_during_packet_drop() {
    let mut sync_manager =
        ClientSyncManager::new(Vec3::ZERO, PlayerPhysicsConfig::default(), 0.100);

    let remote_id = 888;
    let base_time = 200.0;

    sync_manager.handle_remote_snapshot(
        remote_id,
        RemoteSnapshot {
            timestamp: base_time,
            position: Vec3::new(0.0, 0.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 10.0),
            yaw: 0.0,
        },
    );
    sync_manager.handle_remote_snapshot(
        remote_id,
        RemoteSnapshot {
            timestamp: base_time + 0.05,
            position: Vec3::new(0.0, 0.0, 0.5),
            velocity: Vec3::new(0.0, 0.0, 10.0),
            yaw: 0.0,
        },
    );

    // Client clock moves forward to base_time + 0.25 (packet drop / starvation)
    // Render time = 200.25 - 0.100 = 200.15 (> 200.05)
    let sample = sync_manager
        .sample_remote_entity(remote_id, base_time + 0.25)
        .expect("Should extrapolate during packet loss");

    assert!(
        sample.is_extrapolated,
        "Must be flagged as extrapolated when starved of packets"
    );
    // Extrapolated dt = 200.15 - 200.05 = 0.10s. Pos = 0.5 + 10.0 * 0.10 = 1.50
    assert!(
        (sample.position.z - 1.50).abs() < 0.05,
        "Extrapolation must project along velocity vector during drop"
    );
}
