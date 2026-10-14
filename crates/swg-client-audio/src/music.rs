// crates/swg-client-audio/src/music.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Soundtrack theme categories in Pre-CU Star Wars Galaxies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThemeType {
    PlanetaryAmbient,
    CantinaBand,
    CombatSkirmish,
    BossFight,
    VictoryFanfare,
    DefeatStinger,
}

/// Dynamic soundtrack director managing adaptive cross-fading between ambient and combat themes.
#[derive(Debug, Clone)]
pub struct DynamicMusicDirector {
    pub current_theme: ThemeType,
    pub target_theme: Option<ThemeType>,
    pub crossfade_progress: f32,
    pub crossfade_duration: f32,
    pub volumes: HashMap<ThemeType, f32>,
    pub in_combat: bool,
    pub combat_threat: f32,
}

impl Default for DynamicMusicDirector {
    fn default() -> Self {
        let mut volumes = HashMap::new();
        volumes.insert(ThemeType::PlanetaryAmbient, 1.0);
        volumes.insert(ThemeType::CantinaBand, 0.0);
        volumes.insert(ThemeType::CombatSkirmish, 0.0);
        volumes.insert(ThemeType::BossFight, 0.0);
        volumes.insert(ThemeType::VictoryFanfare, 0.0);
        volumes.insert(ThemeType::DefeatStinger, 0.0);

        Self {
            current_theme: ThemeType::PlanetaryAmbient,
            target_theme: None,
            crossfade_progress: 1.0,
            crossfade_duration: 1.5,
            volumes,
            in_combat: false,
            combat_threat: 0.0,
        }
    }
}

impl DynamicMusicDirector {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Triggers transition to a target music theme with smooth cross-fading.
    pub fn transition_to(&mut self, theme: ThemeType) {
        if self.current_theme == theme && self.target_theme.is_none() {
            return;
        }
        if self.target_theme == Some(theme) {
            return;
        }

        if let Some(target) = self.target_theme {
            self.current_theme = target;
        }
        self.target_theme = Some(theme);
        self.crossfade_progress = 0.0;
    }

    /// Enters combat state, switching from ambient to combat theme based on threat level.
    pub fn enter_combat(&mut self, threat_level: f32) {
        self.in_combat = true;
        self.combat_threat = threat_level.clamp(0.0, 1.0);
        let theme = if self.combat_threat > 0.8 {
            ThemeType::BossFight
        } else {
            ThemeType::CombatSkirmish
        };
        self.transition_to(theme);
    }

    /// Exits combat, transitioning back to planetary ambient score.
    pub fn exit_combat(&mut self) {
        self.in_combat = false;
        self.combat_threat = 0.0;
        self.transition_to(ThemeType::PlanetaryAmbient);
    }

    /// Enters a cantina, cross-fading to Figrin D'an jizz band theme.
    pub fn enter_cantina(&mut self) {
        if !self.in_combat {
            self.transition_to(ThemeType::CantinaBand);
        }
    }

    /// Exits cantina back to planetary ambience.
    pub fn exit_cantina(&mut self) {
        if !self.in_combat {
            self.transition_to(ThemeType::PlanetaryAmbient);
        }
    }

    /// Advances soundtrack clock, updating volume cross-fade envelope.
    pub fn tick(&mut self, dt: f32) {
        if let Some(target) = self.target_theme {
            if self.crossfade_duration > 0.0 {
                self.crossfade_progress += dt / self.crossfade_duration;
            } else {
                self.crossfade_progress = 1.0;
            }

            let alpha = self.crossfade_progress.clamp(0.0, 1.0);

            // Fade out current theme
            self.volumes.insert(self.current_theme, 1.0 - alpha);
            // Fade in target theme
            self.volumes.insert(target, alpha);

            if self.crossfade_progress >= 1.0 {
                self.current_theme = target;
                self.target_theme = None;
                self.crossfade_progress = 1.0;
                // Zero out all inactive themes
                for (th, vol) in &mut self.volumes {
                    if *th == self.current_theme {
                        *vol = 1.0;
                    } else {
                        *vol = 0.0;
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn get_volume(&self, theme: ThemeType) -> f32 {
        self.volumes.get(&theme).copied().unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_music_crossfade_ambient_to_combat_and_back() {
        let mut director = DynamicMusicDirector::new();
        assert_eq!(director.current_theme, ThemeType::PlanetaryAmbient);
        assert_eq!(director.get_volume(ThemeType::PlanetaryAmbient), 1.0);
        assert_eq!(director.get_volume(ThemeType::CombatSkirmish), 0.0);

        // Enter skirmish combat (1.5s crossfade)
        director.enter_combat(0.4);
        assert_eq!(director.target_theme, Some(ThemeType::CombatSkirmish));

        // Advance 0.75s (halfway through crossfade)
        director.tick(0.75);
        assert!((director.get_volume(ThemeType::PlanetaryAmbient) - 0.5).abs() < 1e-3);
        assert!((director.get_volume(ThemeType::CombatSkirmish) - 0.5).abs() < 1e-3);

        // Advance another 0.75s (crossfade complete)
        director.tick(0.75);
        assert_eq!(director.current_theme, ThemeType::CombatSkirmish);
        assert_eq!(director.target_theme, None);
        assert_eq!(director.get_volume(ThemeType::CombatSkirmish), 1.0);
        assert_eq!(director.get_volume(ThemeType::PlanetaryAmbient), 0.0);

        // Exit combat
        director.exit_combat();
        director.tick(1.5);
        assert_eq!(director.current_theme, ThemeType::PlanetaryAmbient);
        assert_eq!(director.get_volume(ThemeType::PlanetaryAmbient), 1.0);
    }
}
