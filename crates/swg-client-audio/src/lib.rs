// crates/swg-client-audio/src/lib.rs

pub mod listener;
pub mod music;
pub mod sound_bank;
pub mod spatial;

pub use listener::AudioListener;
pub use music::{DynamicMusicDirector, ThemeType};
pub use sound_bank::{SoundBank, SoundId, SurfaceType};
pub use spatial::{AttenuationModel, AudioEmitter, SpatialAudioEvaluator, SurroundGains};
