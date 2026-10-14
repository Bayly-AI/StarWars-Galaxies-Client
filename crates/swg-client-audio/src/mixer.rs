// crates/swg-client-audio/src/mixer.rs

use crate::listener::AudioListener;
use crate::spatial::{AudioEmitter, SpatialAudioEvaluator};
use crate::wav::WavClip;
use std::sync::Arc;

/// A running audio playback voice being evaluated and mixed.
#[derive(Debug, Clone)]
pub struct ActiveVoice {
    pub id: u64,
    pub clip: Arc<WavClip>,
    pub cursor_frame: usize,
    pub volume: f32,
    pub pitch: f32,
    pub is_looping: bool,
    pub emitter: Option<AudioEmitter>,
    pub is_finished: bool,
}

/// Dynamic realtime multi-channel software audio mixer supporting 2D and 3D spatial sound.
#[derive(Debug, Clone)]
pub struct AudioMixer {
    pub listener: AudioListener,
    pub evaluator: SpatialAudioEvaluator,
    pub voices: Vec<ActiveVoice>,
    pub next_voice_id: u64,
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
    pub sample_rate: u32,
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new(44100)
    }
}

impl AudioMixer {
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            listener: AudioListener::default(),
            evaluator: SpatialAudioEvaluator,
            voices: Vec::new(),
            next_voice_id: 1,
            master_volume: 1.0,
            sfx_volume: 1.0,
            music_volume: 1.0,
            sample_rate,
        }
    }

    /// Updates listener 3D state (position and orientation).
    pub fn set_listener(&mut self, listener: AudioListener) {
        self.listener = listener;
    }

    /// Triggers 2D non-spatial playback (UI sound, narration, etc.).
    pub fn play_2d(&mut self, clip: Arc<WavClip>, volume: f32, looping: bool) -> u64 {
        let id = self.next_voice_id;
        self.next_voice_id += 1;

        self.voices.push(ActiveVoice {
            id,
            clip,
            cursor_frame: 0,
            volume: volume.clamp(0.0, 2.0),
            pitch: 1.0,
            is_looping: looping,
            emitter: None,
            is_finished: false,
        });

        id
    }

    /// Triggers 3D spatial sound playback relative to the active listener.
    pub fn play_3d(
        &mut self,
        clip: Arc<WavClip>,
        emitter: AudioEmitter,
        volume: f32,
        looping: bool,
    ) -> u64 {
        let id = self.next_voice_id;
        self.next_voice_id += 1;

        self.voices.push(ActiveVoice {
            id,
            clip,
            cursor_frame: 0,
            volume: volume.clamp(0.0, 2.0),
            pitch: 1.0,
            is_looping: looping,
            emitter: Some(emitter),
            is_finished: false,
        });

        id
    }

    /// Stops an active voice by ID.
    pub fn stop_voice(&mut self, voice_id: u64) {
        if let Some(voice) = self.voices.iter_mut().find(|v| v.id == voice_id) {
            voice.is_finished = true;
        }
    }

    /// Stops all running sound voices.
    pub fn stop_all(&mut self) {
        self.voices.clear();
    }

    /// Returns the number of actively playing sound voices.
    #[must_use]
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| !v.is_finished).count()
    }

    /// Renders stereo interleaved PCM audio frames (e.g. [L0, R0, L1, R1, ...]).
    pub fn render_stereo_frames(&mut self, num_frames: usize) -> Vec<f32> {
        let mut buffer = vec![0.0f32; num_frames * 2];
        let master = self.master_volume.max(0.0);
        let sfx = self.sfx_volume.max(0.0);

        for voice in &mut self.voices {
            if voice.is_finished {
                continue;
            }

            let clip_frames = voice.clip.frame_count();
            if clip_frames == 0 {
                voice.is_finished = true;
                continue;
            }

            // Calculate gain parameters
            let (gain_l, gain_r) = if let Some(emitter) = &voice.emitter {
                let gains = SpatialAudioEvaluator::evaluate_spatial(emitter, &self.listener);
                let (pair_l, pair_r) = gains.stereo_pair();
                (
                    pair_l * voice.volume * sfx * master,
                    pair_r * voice.volume * sfx * master,
                )
            } else {
                let g = voice.volume * sfx * master;
                (g, g)
            };

            // If completely inaudible, just advance cursor
            if gain_l.abs() < 1e-4 && gain_r.abs() < 1e-4 {
                voice.cursor_frame += num_frames;
                if voice.cursor_frame >= clip_frames {
                    if voice.is_looping {
                        voice.cursor_frame %= clip_frames;
                    } else {
                        voice.is_finished = true;
                    }
                }
                continue;
            }

            for f in 0..num_frames {
                if voice.cursor_frame >= clip_frames {
                    if voice.is_looping {
                        voice.cursor_frame = 0;
                    } else {
                        voice.is_finished = true;
                        break;
                    }
                }

                let (src_l, src_r) = if voice.clip.channels >= 2 {
                    (
                        voice.clip.sample_at(0, voice.cursor_frame),
                        voice.clip.sample_at(1, voice.cursor_frame),
                    )
                } else {
                    let mono = voice.clip.sample_at(0, voice.cursor_frame);
                    (mono, mono)
                };

                buffer[f * 2] += src_l * gain_l;
                buffer[f * 2 + 1] += src_r * gain_r;

                voice.cursor_frame += 1;
            }
        }

        // Clean up finished voices
        self.voices.retain(|v| !v.is_finished);

        // Soft limit master output to prevent digital clipping
        for sample in &mut buffer {
            *sample = sample.clamp(-1.0, 1.0);
        }

        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_audio_mixer_2d_and_3d_spatial_rendering() {
        let mut mixer = AudioMixer::new(44100);
        let clip = Arc::new(WavClip::generate_sine(44100, 1, 440.0, 0.2));

        // 1. Play 2D voice
        let v2d = mixer.play_2d(clip.clone(), 0.8, false);
        assert_eq!(mixer.active_voice_count(), 1);

        // 2. Play 3D voice positioned 10m to the right
        let emitter = AudioEmitter::new(1, Vec3::new(10.0, 0.0, 0.0), 1.0, 50.0);
        let _v3d = mixer.play_3d(clip, emitter, 1.0, false);
        assert_eq!(mixer.active_voice_count(), 2);

        // Render 1024 stereo frames
        let frames = mixer.render_stereo_frames(1024);
        assert_eq!(frames.len(), 2048); // 1024 * 2

        // Verify non-silent rendering
        let has_audio = frames.iter().any(|&s| s.abs() > 0.01);
        assert!(has_audio, "Rendered audio buffer should contain non-zero samples");

        // Stop 2D voice
        mixer.stop_voice(v2d);
        assert_eq!(mixer.active_voice_count(), 1);
    }
}
