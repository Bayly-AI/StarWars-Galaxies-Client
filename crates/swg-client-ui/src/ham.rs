// crates/swg-client-ui/src/ham.rs

use serde::{Deserialize, Serialize};

/// A single Health, Action, or Mind status pool with wounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HamPool {
    pub current: i32,
    pub max: i32,
    pub wound: i32,
}

impl HamPool {
    #[must_use]
    pub fn new(max: i32) -> Self {
        let max = max.max(1);
        Self {
            current: max,
            max,
            wound: 0,
        }
    }

    /// Effective maximum pool after subtracting permanent wounds.
    #[must_use]
    pub fn effective_max(&self) -> i32 {
        (self.max - self.wound).max(0)
    }

    /// Fraction of current pool relative to total base max [0.0, 1.0].
    #[must_use]
    pub fn current_ratio(&self) -> f32 {
        if self.max <= 0 {
            0.0
        } else {
            (self.current as f32 / self.max as f32).clamp(0.0, 1.0)
        }
    }

    /// Fraction of wound penalty relative to total base max [0.0, 1.0].
    #[must_use]
    pub fn wound_ratio(&self) -> f32 {
        if self.max <= 0 {
            0.0
        } else {
            (self.wound as f32 / self.max as f32).clamp(0.0, 1.0)
        }
    }

    /// Fraction of effective pool [0.0, 1.0].
    #[must_use]
    pub fn effective_ratio(&self) -> f32 {
        if self.max <= 0 {
            0.0
        } else {
            (self.effective_max() as f32 / self.max as f32).clamp(0.0, 1.0)
        }
    }

    /// Applies immediate damage to this pool.
    pub fn apply_damage(&mut self, amount: i32) {
        self.current = self.current.saturating_sub(amount);
    }

    /// Heals damage up to effective maximum.
    pub fn heal_damage(&mut self, amount: i32) {
        let cap = self.effective_max();
        self.current = (self.current + amount).min(cap);
    }

    /// Adds permanent wound penalty.
    pub fn apply_wound(&mut self, amount: i32) {
        self.wound = (self.wound + amount).min(self.max);
        let cap = self.effective_max();
        if self.current > cap {
            self.current = cap;
        }
    }

    /// Heals permanent wound penalty (e.g. by a Doctor or Combat Medic).
    pub fn heal_wound(&mut self, amount: i32) {
        self.wound = (self.wound - amount).max(0);
    }
}

/// The three primary Pre-CU character pools: Health, Action, and Mind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterHam {
    pub health: HamPool,
    pub action: HamPool,
    pub mind: HamPool,
}

impl CharacterHam {
    #[must_use]
    pub fn new(max_health: i32, max_action: i32, max_mind: i32) -> Self {
        Self {
            health: HamPool::new(max_health),
            action: HamPool::new(max_action),
            mind: HamPool::new(max_mind),
        }
    }

    /// True if any of the three primary pools reaches zero or below.
    #[must_use]
    pub fn is_incapacitated(&self) -> bool {
        self.health.current <= 0 || self.action.current <= 0 || self.mind.current <= 0
    }

    /// True if health drops to negative threshold triggering character death.
    #[must_use]
    pub fn is_dead(&self) -> bool {
        self.health.current <= -self.health.max
    }

    /// Updates HAM stats from incoming server delta updates.
    pub fn update_from_server(
        &mut self,
        cur_h: i32,
        wnd_h: i32,
        cur_a: i32,
        wnd_a: i32,
        cur_m: i32,
        wnd_m: i32,
    ) {
        self.health.current = cur_h;
        self.health.wound = wnd_h;
        self.action.current = cur_a;
        self.action.wound = wnd_a;
        self.mind.current = cur_m;
        self.mind.wound = wnd_m;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ham_pool_damage_and_wound_mechanics() {
        let mut pool = HamPool::new(1000);
        assert_eq!(pool.effective_max(), 1000);
        assert_eq!(pool.current_ratio(), 1.0);

        // Take 300 damage
        pool.apply_damage(300);
        assert_eq!(pool.current, 700);
        assert!((pool.current_ratio() - 0.7).abs() < 1e-4);

        // Receive 200 wound penalty
        pool.apply_wound(200);
        assert_eq!(pool.wound, 200);
        assert_eq!(pool.effective_max(), 800);
        assert!((pool.wound_ratio() - 0.2).abs() < 1e-4);

        // Try to heal damage past wound cap
        pool.heal_damage(500);
        assert_eq!(pool.current, 800, "Cannot heal damage past wound penalty");

        // Doctor heals 100 wound
        pool.heal_wound(100);
        assert_eq!(pool.wound, 100);
        assert_eq!(pool.effective_max(), 900);
    }

    #[test]
    fn test_character_incapacitation() {
        let mut ham = CharacterHam::new(1000, 1000, 1000);
        assert!(!ham.is_incapacitated());

        // Deplete Action to 0 (e.g. Burst Run exhaustion)
        ham.action.apply_damage(1000);
        assert!(ham.is_incapacitated());
    }
}
