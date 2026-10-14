// crates/swg-client-ui/src/target_status.rs

use serde::{Deserialize, Serialize};

/// Target HUD frame showing selected entity vitals, distance, and posture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetStatus {
    pub target_id: Option<u64>,
    pub name: String,
    pub posture: String,
    pub distance: f32,
    pub health_ratio: f32,
    pub action_ratio: f32,
    pub mind_ratio: f32,
    pub is_hostile: bool,
    pub faction: Option<String>,
}

impl Default for TargetStatus {
    fn default() -> Self {
        Self {
            target_id: None,
            name: String::new(),
            posture: "None".to_string(),
            distance: 0.0,
            health_ratio: 0.0,
            action_ratio: 0.0,
            mind_ratio: 0.0,
            is_hostile: false,
            faction: None,
        }
    }
}

impl TargetStatus {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_target(
        &mut self,
        id: u64,
        name: impl Into<String>,
        is_hostile: bool,
        faction: Option<String>,
    ) {
        self.target_id = Some(id);
        self.name = name.into();
        self.posture = "Standing".to_string();
        self.distance = 0.0;
        self.health_ratio = 1.0;
        self.action_ratio = 1.0;
        self.mind_ratio = 1.0;
        self.is_hostile = is_hostile;
        self.faction = faction;
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    #[must_use]
    pub fn has_target(&self) -> bool {
        self.target_id.is_some()
    }

    pub fn update_vitals(&mut self, h: f32, a: f32, m: f32, dist: f32, posture: impl Into<String>) {
        self.health_ratio = h.clamp(0.0, 1.0);
        self.action_ratio = a.clamp(0.0, 1.0);
        self.mind_ratio = m.clamp(0.0, 1.0);
        self.distance = dist.max(0.0);
        self.posture = posture.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_status_lifecycle() {
        let mut target = TargetStatus::new();
        assert!(!target.has_target());

        target.set_target(
            9001,
            "Stormtrooper Commander",
            true,
            Some("Imperial".to_string()),
        );
        assert!(target.has_target());
        assert_eq!(target.name, "Stormtrooper Commander");
        assert!(target.is_hostile);

        target.update_vitals(0.45, 0.8, 1.0, 32.5, "Incapacitated");
        assert!((target.health_ratio - 0.45).abs() < 1e-4);
        assert!((target.distance - 32.5).abs() < 1e-4);
        assert_eq!(target.posture, "Incapacitated");

        target.clear();
        assert!(!target.has_target());
    }
}
