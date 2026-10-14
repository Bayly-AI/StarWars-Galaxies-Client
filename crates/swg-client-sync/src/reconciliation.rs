// crates/swg-client-sync/src/reconciliation.rs

use crate::prediction::{ClientInput, MovementPredictor, PlayerPhysicsState};
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Authoritative snapshot received from the server.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ServerTransformUpdate {
    pub sequence: u32,
    pub timestamp: f64,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub is_grounded: bool,
}

/// The result of an authoritative reconciliation pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReconciliationOutcome {
    /// Prediction was accurate within threshold; historical inputs up to sequence pruned.
    Accurate { sequence: u32, error_distance: f32 },
    /// Desynchronization detected; state was re-simulated and error blending initialized.
    Corrected {
        sequence: u32,
        error_offset: Vec3,
        replayed_inputs: usize,
    },
    /// Update sequence was outdated (<= last processed server sequence).
    StaleSequence { sequence: u32 },
}

/// Smooth error blender that prevents visual snapping/rubberbanding
/// when authoritative corrections occur.
#[derive(Debug, Clone)]
pub struct ErrorBlender {
    error_offset: Vec3,
    decay_rate: f32, // exponential decay rate in Hz (e.g. 15.0)
}

impl ErrorBlender {
    #[must_use]
    pub fn new(decay_rate: f32) -> Self {
        Self {
            error_offset: Vec3::ZERO,
            decay_rate,
        }
    }

    /// Adds a positional correction offset to the blend pool.
    pub fn add_error(&mut self, error: Vec3) {
        self.error_offset += error;
    }

    /// Advances the smooth error decay over delta time `dt`.
    pub fn update(&mut self, dt: f32) {
        if self.error_offset.length_squared() < 0.0001 {
            self.error_offset = Vec3::ZERO;
            return;
        }

        let decay = (-self.decay_rate * dt).exp();
        self.error_offset *= decay;
    }

    #[must_use]
    pub fn current_offset(&self) -> Vec3 {
        self.error_offset
    }

    pub fn reset(&mut self) {
        self.error_offset = Vec3::ZERO;
    }
}

impl Default for ErrorBlender {
    fn default() -> Self {
        Self::new(15.0)
    }
}

/// Server reconciler ensuring local client simulation remains strictly in sync
/// with authoritative server state while eliminating visual pops.
#[derive(Debug, Clone)]
pub struct ServerReconciler {
    error_threshold: f32,
    last_acknowledged_sequence: u32,
    error_blender: ErrorBlender,
}

impl ServerReconciler {
    #[must_use]
    pub fn new(error_threshold: f32, blend_decay_rate: f32) -> Self {
        Self {
            error_threshold,
            last_acknowledged_sequence: 0,
            error_blender: ErrorBlender::new(blend_decay_rate),
        }
    }

    #[must_use]
    pub fn error_threshold(&self) -> f32 {
        self.error_threshold
    }

    #[must_use]
    pub fn last_acknowledged_sequence(&self) -> u32 {
        self.last_acknowledged_sequence
    }

    #[must_use]
    pub fn error_blender(&self) -> &ErrorBlender {
        &self.error_blender
    }

    /// Returns the smoothed visual render position incorporating predicted position
    /// and decaying reconciliation error offset.
    #[must_use]
    pub fn visual_position(&self, predictor: &MovementPredictor) -> Vec3 {
        predictor.state().position + self.error_blender.current_offset()
    }

    /// Updates error decay on each frame.
    pub fn update_blend(&mut self, dt: f32) {
        self.error_blender.update(dt);
    }

    /// Reconciles the local predictor state with the authoritative server transform update.
    pub fn reconcile(
        &mut self,
        predictor: &mut MovementPredictor,
        server_update: &ServerTransformUpdate,
    ) -> ReconciliationOutcome {
        if server_update.sequence <= self.last_acknowledged_sequence {
            return ReconciliationOutcome::StaleSequence {
                sequence: server_update.sequence,
            };
        }

        self.last_acknowledged_sequence = server_update.sequence;

        // Discard all inputs up to and including the server sequence
        let history = predictor.history_mut();
        while let Some(front) = history.front() {
            if front.sequence <= server_update.sequence {
                history.pop_front();
            } else {
                break;
            }
        }

        // Copy remaining unacknowledged inputs
        let remaining_inputs: Vec<ClientInput> = history.iter().cloned().collect();

        // Check predicted state vs authoritative state
        // If there are no pending inputs, compare directly with current predicted state
        let current_pred_pos = predictor.state().position;

        // Re-simulate from server state through all remaining inputs
        let mut sim_state = PlayerPhysicsState {
            sequence: server_update.sequence,
            position: server_update.position,
            velocity: server_update.velocity,
            yaw: server_update.yaw,
            pitch: server_update.pitch,
            is_grounded: server_update.is_grounded,
        };

        for input in &remaining_inputs {
            MovementPredictor::step_simulation(&mut sim_state, input, predictor.config());
        }

        let error_offset = current_pred_pos - sim_state.position;
        let error_distance = error_offset.length();

        if error_distance > self.error_threshold {
            // Apply corrected state
            predictor.set_state(sim_state);

            // Add error offset to blender so the camera/character doesn't pop visually
            self.error_blender.add_error(error_offset);

            ReconciliationOutcome::Corrected {
                sequence: server_update.sequence,
                error_offset,
                replayed_inputs: remaining_inputs.len(),
            }
        } else {
            // Within tolerance, accept minor drift or adopt authoritative ground truth
            predictor.set_state(sim_state);
            ReconciliationOutcome::Accurate {
                sequence: server_update.sequence,
                error_distance,
            }
        }
    }
}

impl Default for ServerReconciler {
    fn default() -> Self {
        Self::new(0.05, 15.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prediction::{InputCommand, PlayerPhysicsConfig};

    #[test]
    fn test_authoritative_reconciliation_in_tolerance() {
        let config = PlayerPhysicsConfig::default();
        let mut predictor = MovementPredictor::new(PlayerPhysicsState::new(Vec3::ZERO), config);
        let mut reconciler = ServerReconciler::default();

        let (seq1, state1) = predictor.push_input(InputCommand {
            delta_time: 0.05,
            forward: 1.0,
            timestamp: 100.0,
            ..Default::default()
        });
        let (_seq2, _state2) = predictor.push_input(InputCommand {
            delta_time: 0.05,
            forward: 1.0,
            timestamp: 100.05,
            ..Default::default()
        });

        // Server confirms sequence 1 with exact state
        let server_update = ServerTransformUpdate {
            sequence: seq1,
            timestamp: 100.0,
            position: state1.position,
            velocity: state1.velocity,
            yaw: state1.yaw,
            pitch: state1.pitch,
            is_grounded: true,
        };

        let outcome = reconciler.reconcile(&mut predictor, &server_update);
        assert!(
            matches!(outcome, ReconciliationOutcome::Accurate { .. }),
            "Accurate prediction must match"
        );
        assert_eq!(reconciler.last_acknowledged_sequence(), seq1);
        assert_eq!(predictor.history().len(), 1); // Only seq 2 remains
    }

    #[test]
    fn test_reconciliation_correction_and_smooth_blending() {
        let config = PlayerPhysicsConfig::default();
        let mut predictor = MovementPredictor::new(PlayerPhysicsState::new(Vec3::ZERO), config);
        let mut reconciler = ServerReconciler::new(0.05, 15.0);

        // Client pushed 5 frames
        for i in 1..=5 {
            predictor.push_input(InputCommand {
                delta_time: 0.05,
                forward: 1.0,
                timestamp: 100.0 + f64::from(i) * 0.05,
                ..Default::default()
            });
        }

        let client_predicted_pos = predictor.state().position;

        // Server says at seq 3, player was hit by an explosion or stopped by a rock,
        // so position was actually (2.0, 0.0, 0.0) instead of predicted
        let server_update = ServerTransformUpdate {
            sequence: 3,
            timestamp: 100.15,
            position: Vec3::new(2.0, 0.0, 0.0),
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            is_grounded: true,
        };

        let outcome = reconciler.reconcile(&mut predictor, &server_update);
        match outcome {
            ReconciliationOutcome::Corrected {
                error_offset,
                replayed_inputs,
                ..
            } => {
                assert_eq!(replayed_inputs, 2); // seq 4 and seq 5 replayed
                assert!(error_offset.length() > 0.05);
            }
            _ => panic!("Expected reconciliation correction"),
        }

        // Visual position immediately after correction must equal original predicted pos (no pop!)
        let visual_pos_initial = reconciler.visual_position(&predictor);
        assert!(
            (visual_pos_initial - client_predicted_pos).length() < 0.01,
            "Visual position must not pop upon correction"
        );

        // Over 0.3s (several frames), error blends away
        for _ in 0..10 {
            reconciler.update_blend(0.03);
        }

        let visual_pos_blended = reconciler.visual_position(&predictor);
        assert!(
            (visual_pos_blended - predictor.state().position).length() < 0.05,
            "Visual position must smoothly converge to authoritative ground truth"
        );
    }
}
