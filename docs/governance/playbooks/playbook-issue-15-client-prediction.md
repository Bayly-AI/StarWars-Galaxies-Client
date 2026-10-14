# Playbook: Issue 15 — Netcode & Sync: Movement Prediction, Reconciliation & Remote Interpolation (`swg-client-sync`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#15  
> **Branch:** `enhancement/15-client-movement-prediction`  
> **Status:** Implementation Playbook  
> **Related Files:**
> - [prediction.rs](file:///Users/raybayly/Development/BAI/GAMES/SWG/Client/crates/swg-client-sync/src/prediction.rs)
> - [reconciliation.rs](file:///Users/raybayly/Development/BAI/GAMES/SWG/Client/crates/swg-client-sync/src/reconciliation.rs)
> - [remote_interpolation.rs](file:///Users/raybayly/Development/BAI/GAMES/SWG/Client/crates/swg-client-sync/src/remote_interpolation.rs)
> - [sync_pipeline_tests.rs](file:///Users/raybayly/Development/BAI/GAMES/SWG/Client/crates/swg-client-sync/tests/sync_pipeline_tests.rs)

---

## 1. Objectives & Scope

Implement high-performance, low-latency multiplayer synchronization in `crates/swg-client-sync`:

1. **Client-Side Movement Prediction (`prediction.rs`):**
   - High-frequency local input collection (`ClientInput`) with sequence tracking, delta time, directional vectors, yaw/pitch orientation, and sprint states.
   - Immediate deterministic simulation applying player movement to position and velocity without waiting for network round-trip.
   - Ring-buffered input history retaining unacknowledged user commands for rollback and replay.

2. **Authoritative Server Reconciliation & Smooth Error Blending (`reconciliation.rs`):**
   - Verification against authoritative server transform snapshots (`ServerTransformUpdate`).
   - Efficient pruning of acknowledged sequence frames.
   - Desynchronization detection against configurable error thresholds.
   - Replay simulation from server truth through pending inputs.
   - Exponential smoothing error blending (`ErrorBlender`) preventing jarring positional pops and visual rubberbanding.

3. **Remote Entity Hermite Spline Interpolation (`remote_interpolation.rs`):**
   - Time-buffered snapshot history per remote entity.
   - Configurable jitter delay buffer (default 100ms) absorbing network packet jitter and out-of-order delivery.
   - Smooth Cubic Hermite Spline interpolation using position and velocity vectors for seamless, natural movement.
   - Graceful linear velocity extrapolation with clamping for temporary network stalls.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Zero perceptible local movement lag under simulated network latency (WASD responds immediately).
- [x] Server reconciliation detects discrepancies, replays pending inputs, and blends visual positions smoothly without snapping.
- [x] Remote players render smooth, continuous trajectories via Hermite spline interpolation over a 100ms jitter buffer.
- [x] 100% test pass rate across workspace (`cargo test --workspace`).
- [x] Zero warnings across workspace (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes cleanly before PR creation.
