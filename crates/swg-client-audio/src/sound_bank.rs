// crates/swg-client-audio/src/sound_bank.rs

use serde::{Deserialize, Serialize};

/// Ground surface material for terrain-dependent footstep sound resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SurfaceType {
    Sand,
    Rock,
    Metal,
    Grass,
    Water,
}

/// Authentic sound effect identifiers from retail Pre-CU archives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SoundId {
    BlasterE11Fire,
    BlasterDlt19Fire,
    LightsaberIgnite,
    LightsaberSwing,
    Footstep(SurfaceType),
    CantinaBandPerformance,
    TatooineDesertWind,
    NabooWaterfall,
}

pub struct SoundBank;

impl SoundBank {
    /// Resolves asset path for a sound effect.
    #[must_use]
    pub fn resolve_asset_path(id: SoundId) -> &'static str {
        match id {
            SoundId::BlasterE11Fire => "sound/wep_blaster_e11_fire.wav",
            SoundId::BlasterDlt19Fire => "sound/wep_blaster_dlt19_fire.wav",
            SoundId::LightsaberIgnite => "sound/wep_lightsaber_ignite.wav",
            SoundId::LightsaberSwing => "sound/wep_lightsaber_swing.wav",
            SoundId::Footstep(SurfaceType::Sand) => "sound/fs_sand_walk_01.wav",
            SoundId::Footstep(SurfaceType::Rock) => "sound/fs_rock_walk_01.wav",
            SoundId::Footstep(SurfaceType::Metal) => "sound/fs_metal_walk_01.wav",
            SoundId::Footstep(SurfaceType::Grass) => "sound/fs_grass_walk_01.wav",
            SoundId::Footstep(SurfaceType::Water) => "sound/fs_water_splash_01.wav",
            SoundId::CantinaBandPerformance => "sound/mus_cantina_band_01.mp3",
            SoundId::TatooineDesertWind => "sound/amb_tatooine_desert_wind.wav",
            SoundId::NabooWaterfall => "sound/amb_naboo_waterfall.wav",
        }
    }

    /// Resolves default 3D audio attenuation distance parameters (min_distance, max_distance).
    #[must_use]
    pub fn default_distances(id: SoundId) -> (f32, f32) {
        match id {
            SoundId::Footstep(_) => (1.0, 30.0),
            SoundId::BlasterE11Fire | SoundId::BlasterDlt19Fire => (5.0, 150.0),
            SoundId::LightsaberIgnite | SoundId::LightsaberSwing => (2.0, 60.0),
            SoundId::CantinaBandPerformance => (10.0, 80.0),
            SoundId::TatooineDesertWind | SoundId::NabooWaterfall => (20.0, 300.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_bank_resolution() {
        assert_eq!(
            SoundBank::resolve_asset_path(SoundId::BlasterE11Fire),
            "sound/wep_blaster_e11_fire.wav"
        );
        assert_eq!(
            SoundBank::resolve_asset_path(SoundId::Footstep(SurfaceType::Sand)),
            "sound/fs_sand_walk_01.wav"
        );
        let (min_d, max_d) = SoundBank::default_distances(SoundId::BlasterE11Fire);
        assert_eq!(min_d, 5.0);
        assert_eq!(max_d, 150.0);
    }
}
