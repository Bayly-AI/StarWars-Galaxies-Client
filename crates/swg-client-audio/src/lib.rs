// crates/swg-client-audio/src/lib.rs

pub mod listener;
pub mod mixer;
pub mod mp3;
pub mod music;
pub mod snd;
pub mod sound_bank;
pub mod spatial;
pub mod wav;

pub use listener::AudioListener;
pub use mixer::{ActiveVoice, AudioMixer};
pub use mp3::Mp3Track;
pub use music::{DynamicMusicDirector, ThemeType};
pub use snd::SoundTemplate;
pub use sound_bank::{SoundBank, SoundId, SurfaceType};
pub use spatial::{AttenuationModel, AudioEmitter, SpatialAudioEvaluator, SurroundGains};
pub use wav::{AudioError, WavClip};
