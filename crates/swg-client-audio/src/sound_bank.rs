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
    BlasterFire,
    BlasterHit,
    LightsaberIgnite,
    LightsaberSwing,
    LightsaberClash,
    LightsaberHit,
    Footstep(SurfaceType),
    RetailFootstep(SurfaceType),
    CantinaBandPerformance,
    TatooineDesertWind,
    NabooWaterfall,
    UiSelectRotate,
    ContainerCreakClose,
    AirTaxiRun,
    SarlaccPit,
    DroidProtocolVoice,
    DroidGonkVoice,
    PlanetaryCorellia,
    CombatSkirmishTheme,
    CombatVictoryTheme,
    TitleTheme,
}

pub struct SoundBank;

impl SoundBank {
    /// Resolves asset path for a sound effect or music track.
    #[must_use]
    pub fn resolve_asset_path(id: SoundId) -> &'static str {
        match id {
            SoundId::BlasterE11Fire => "sound/wep_blaster_e11_fire.wav",
            SoundId::BlasterDlt19Fire => "sound/wep_blaster_dlt19_fire.wav",
            SoundId::BlasterFire => "sample/wep_blaster_fire_2.wav",
            SoundId::BlasterHit => "sample/wep_lt_blaster_hit_terrain.wav",
            SoundId::LightsaberIgnite => "sound/wep_lightsaber_ignite.wav",
            SoundId::LightsaberSwing => "sound/wep_lightsaber_swing.wav",
            SoundId::LightsaberClash => "sample/wep_ltsaber_vs_ltsaber_03.wav",
            SoundId::LightsaberHit => "sample/wep_lightsaber_vs_sml_sft.wav",
            SoundId::Footstep(SurfaceType::Sand) => "sound/fs_sand_walk_01.wav",
            SoundId::Footstep(SurfaceType::Rock) => "sound/fs_rock_walk_01.wav",
            SoundId::Footstep(SurfaceType::Metal) => "sound/fs_metal_walk_01.wav",
            SoundId::Footstep(SurfaceType::Grass) => "sound/fs_grass_walk_01.wav",
            SoundId::Footstep(SurfaceType::Water) => "sound/fs_water_splash_01.wav",
            SoundId::RetailFootstep(SurfaceType::Sand) => "sample/fs_out_dry_dirt_03.wav",
            SoundId::RetailFootstep(SurfaceType::Rock) => "sample/fs_bare_rock_04.wav",
            SoundId::RetailFootstep(SurfaceType::Grass) => "sample/fs_bare_grass_03.wav",
            SoundId::RetailFootstep(SurfaceType::Metal) => "sample/cr_lg_fs_carpet_04.wav",
            SoundId::RetailFootstep(SurfaceType::Water) => "sound/fs_water_splash_01.wav",
            SoundId::CantinaBandPerformance => "music/mus_max_rebo_song_1.mp3",
            SoundId::TatooineDesertWind => "music/mus_desert_amb_a_lp.mp3",
            SoundId::NabooWaterfall => "sound/amb_naboo_waterfall.wav",
            SoundId::UiSelectRotate => "sample/ui_select_rotate.wav",
            SoundId::ContainerCreakClose => "sample/item_creak_close.wav",
            SoundId::AirTaxiRun => "sample/veh_airtaxi_run_lp.wav",
            SoundId::SarlaccPit => "sample/item_sarlaccpit.wav",
            SoundId::DroidProtocolVoice => "voice/sample/pdm011.wav",
            SoundId::DroidGonkVoice => "sample/dro_gonk_voc_a.wav",
            SoundId::PlanetaryCorellia => "music/mus_theme_corellia.mp3",
            SoundId::CombatSkirmishTheme => "music/mus_combat_b_lp.mp3",
            SoundId::CombatVictoryTheme => "music/mus_combat_b_vict.mp3",
            SoundId::TitleTheme => "music/mus_title_2_lp.mp3",
        }
    }

    /// Resolves authentic SOE sound template (.snd) descriptor path if available.
    #[must_use]
    pub fn resolve_template_path(id: SoundId) -> Option<&'static str> {
        match id {
            SoundId::UiSelectRotate => Some("sound/ui_select_rotate.snd"),
            SoundId::ContainerCreakClose => Some("sound/item_creak_close.snd"),
            SoundId::AirTaxiRun => Some("sound/veh_airtaxi_run_lp.snd"),
            SoundId::SarlaccPit => Some("sound/item_sarlaccpit.snd"),
            SoundId::DroidProtocolVoice => {
                Some("voice/sound/voice_dro_protocol_m_impressive.snd")
            }
            _ => None,
        }
    }

    /// Resolves default 3D audio attenuation distance parameters (min_distance, max_distance).
    #[must_use]
    pub fn default_distances(id: SoundId) -> (f32, f32) {
        match id {
            SoundId::Footstep(_) | SoundId::RetailFootstep(_) => (1.0, 30.0),
            SoundId::BlasterE11Fire
            | SoundId::BlasterDlt19Fire
            | SoundId::BlasterFire
            | SoundId::BlasterHit => (5.0, 150.0),
            SoundId::LightsaberIgnite
            | SoundId::LightsaberSwing
            | SoundId::LightsaberClash
            | SoundId::LightsaberHit => (2.0, 60.0),
            SoundId::CantinaBandPerformance => (10.0, 80.0),
            SoundId::TatooineDesertWind | SoundId::NabooWaterfall => (20.0, 300.0),
            SoundId::UiSelectRotate => (1.0, 10.0),
            SoundId::ContainerCreakClose => (1.0, 15.0),
            SoundId::AirTaxiRun => (5.0, 100.0),
            SoundId::SarlaccPit => (15.0, 200.0),
            SoundId::DroidProtocolVoice | SoundId::DroidGonkVoice => (2.0, 40.0),
            SoundId::PlanetaryCorellia
            | SoundId::CombatSkirmishTheme
            | SoundId::CombatVictoryTheme
            | SoundId::TitleTheme => (0.0, 0.0), // Non-spatial background music
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
        assert_eq!(
            SoundBank::resolve_asset_path(SoundId::RetailFootstep(SurfaceType::Sand)),
            "sample/fs_out_dry_dirt_03.wav"
        );
        assert_eq!(
            SoundBank::resolve_asset_path(SoundId::UiSelectRotate),
            "sample/ui_select_rotate.wav"
        );
        assert_eq!(
            SoundBank::resolve_template_path(SoundId::UiSelectRotate),
            Some("sound/ui_select_rotate.snd")
        );
        let (min_d, max_d) = SoundBank::default_distances(SoundId::BlasterFire);
        assert_eq!(min_d, 5.0);
        assert_eq!(max_d, 150.0);
    }
}
