// crates/swg-client-sync/src/lib.rs

pub mod prediction;
pub mod reconciliation;
pub mod remote_interpolation;

use glam::Vec3;
pub use prediction::{
    ClientInput, InputCommand, MovementPredictor, PlayerPhysicsConfig, PlayerPhysicsState,
};
pub use reconciliation::{
    ErrorBlender, ReconciliationOutcome, ServerReconciler, ServerTransformUpdate,
};
pub use remote_interpolation::{
    HermiteSpline, InterpolatedTransform, RemoteEntityInterpolator, RemoteSnapshot,
};

/// High-level client synchronization manager orchestrating client prediction,
/// authoritative reconciliation, and remote entity interpolation.
#[derive(Debug, Clone)]
pub struct ClientSyncManager {
    predictor: MovementPredictor,
    reconciler: ServerReconciler,
    remote_interpolator: RemoteEntityInterpolator,
}

impl ClientSyncManager {
    #[must_use]
    pub fn new(
        initial_pos: Vec3,
        physics_config: PlayerPhysicsConfig,
        jitter_buffer_secs: f64,
    ) -> Self {
        let initial_state = PlayerPhysicsState::new(initial_pos);
        Self {
            predictor: MovementPredictor::new(initial_state, physics_config),
            reconciler: ServerReconciler::default(),
            remote_interpolator: RemoteEntityInterpolator::new(jitter_buffer_secs),
        }
    }

    #[must_use]
    pub fn predictor(&self) -> &MovementPredictor {
        &self.predictor
    }

    pub fn predictor_mut(&mut self) -> &mut MovementPredictor {
        &mut self.predictor
    }

    #[must_use]
    pub fn reconciler(&self) -> &ServerReconciler {
        &self.reconciler
    }

    pub fn reconciler_mut(&mut self) -> &mut ServerReconciler {
        &mut self.reconciler
    }

    #[must_use]
    pub fn remote_interpolator(&self) -> &RemoteEntityInterpolator {
        &self.remote_interpolator
    }

    pub fn remote_interpolator_mut(&mut self) -> &mut RemoteEntityInterpolator {
        &mut self.remote_interpolator
    }

    /// Handles immediate local WASD input and returns sequence and predicted state.
    pub fn handle_local_input(&mut self, cmd: InputCommand) -> (u32, PlayerPhysicsState) {
        self.predictor.push_input(cmd)
    }

    /// Reconciles local player state against an incoming authoritative server snapshot.
    pub fn reconcile_local_player(
        &mut self,
        server_update: &ServerTransformUpdate,
    ) -> ReconciliationOutcome {
        self.reconciler
            .reconcile(&mut self.predictor, server_update)
    }

    /// Ingests a network update for a remote entity.
    pub fn handle_remote_snapshot(&mut self, entity_id: u64, snapshot: RemoteSnapshot) {
        self.remote_interpolator.push_snapshot(entity_id, snapshot);
    }

    /// Samples a remote entity's interpolated transform at `client_time`.
    #[must_use]
    pub fn sample_remote_entity(
        &self,
        entity_id: u64,
        client_time: f64,
    ) -> Option<InterpolatedTransform> {
        self.remote_interpolator.sample(entity_id, client_time)
    }

    /// Visual position of the local player with error smoothing.
    #[must_use]
    pub fn local_visual_position(&self) -> Vec3 {
        self.reconciler.visual_position(&self.predictor)
    }

    /// Per-frame maintenance updating blend decay and buffer pruning.
    pub fn update(&mut self, dt: f32, client_time: f64) {
        self.reconciler.update_blend(dt);
        self.remote_interpolator.prune_old_snapshots(client_time);
    }
}
