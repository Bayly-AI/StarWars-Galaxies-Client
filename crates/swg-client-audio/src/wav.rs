// crates/swg-client-audio/src/wav.rs

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const RIFF_MAGIC: [u8; 4] = *b"RIFF";
pub const WAVE_MAGIC: [u8; 4] = *b"WAVE";
pub const FMT_CHUNK: [u8; 4] = *b"fmt ";
pub const DATA_CHUNK: [u8; 4] = *b"data";

pub const AUDIO_FORMAT_PCM: u16 = 1;
pub const AUDIO_FORMAT_IEEE_FLOAT: u16 = 3;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AudioError {
    #[error("Invalid RIFF header: expected 'RIFF', got {0:?}")]
    InvalidRiffMagic([u8; 4]),
    #[error("Invalid WAVE format: expected 'WAVE', got {0:?}")]
    InvalidWaveFormat([u8; 4]),
    #[error("Missing 'fmt ' chunk in WAV file")]
    MissingFmtChunk,
    #[error("Missing 'data' chunk in WAV file")]
    MissingDataChunk,
    #[error("Unsupported audio format code: {0}")]
    UnsupportedAudioFormat(u16),
    #[error("Unsupported bits per sample: {0}")]
    UnsupportedBitsPerSample(u16),
    #[error("Data payload truncated: expected {0} bytes, got {1}")]
    TruncatedData(usize, usize),
    #[error("Invalid SND template header: {0}")]
    InvalidSndHeader(String),
    #[error("Invalid MP3 frame header: {0}")]
    InvalidMp3Header(String),
}

/// A parsed and decoded audio clip containing normalized 32-bit floating point PCM samples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WavClip {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub duration_seconds: f32,
    /// Interleaved PCM samples normalized to [-1.0, 1.0].
    pub samples: Vec<f32>,
}

impl WavClip {
    /// Parses a standard RIFF WAVE audio stream from a raw byte buffer.
    pub fn parse(bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 12 {
            return Err(AudioError::TruncatedData(12, bytes.len()));
        }

        let riff = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if riff != RIFF_MAGIC {
            return Err(AudioError::InvalidRiffMagic(riff));
        }

        let wave = [bytes[8], bytes[9], bytes[10], bytes[11]];
        if wave != WAVE_MAGIC {
            return Err(AudioError::InvalidWaveFormat(wave));
        }

        let mut offset = 12;
        let mut audio_format = None;
        let mut channels = None;
        let mut sample_rate = None;
        let mut bits_per_sample = None;
        let mut raw_data = None;

        while offset + 8 <= bytes.len() {
            let chunk_id = [
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ];
            let chunk_size = u32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]) as usize;

            let chunk_data_start = offset + 8;
            let chunk_data_end = (chunk_data_start + chunk_size).min(bytes.len());

            if chunk_id == FMT_CHUNK {
                if chunk_size < 16 {
                    return Err(AudioError::MissingFmtChunk);
                }
                let fmt = &bytes[chunk_data_start..chunk_data_end];
                let fmt_code = u16::from_le_bytes([fmt[0], fmt[1]]);
                let ch = u16::from_le_bytes([fmt[2], fmt[3]]);
                let sr = u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]);
                let bits = u16::from_le_bytes([fmt[14], fmt[15]]);

                if fmt_code != AUDIO_FORMAT_PCM && fmt_code != AUDIO_FORMAT_IEEE_FLOAT {
                    return Err(AudioError::UnsupportedAudioFormat(fmt_code));
                }
                if bits != 8 && bits != 16 && bits != 24 && bits != 32 {
                    return Err(AudioError::UnsupportedBitsPerSample(bits));
                }

                audio_format = Some(fmt_code);
                channels = Some(ch);
                sample_rate = Some(sr);
                bits_per_sample = Some(bits);
            } else if chunk_id == DATA_CHUNK {
                raw_data = Some(&bytes[chunk_data_start..chunk_data_end]);
            }

            // RIFF chunks are word-aligned (padded to 2 bytes)
            offset = chunk_data_start + chunk_size + (chunk_size % 2);
        }

        let format_code = audio_format.ok_or(AudioError::MissingFmtChunk)?;
        let ch = channels.ok_or(AudioError::MissingFmtChunk)?;
        let sr = sample_rate.ok_or(AudioError::MissingFmtChunk)?;
        let bits = bits_per_sample.ok_or(AudioError::MissingFmtChunk)?;
        let payload = raw_data.ok_or(AudioError::MissingDataChunk)?;

        // Decode PCM bytes to normalized float samples
        let mut samples = Vec::new();
        match (format_code, bits) {
            (AUDIO_FORMAT_PCM, 8) => {
                samples.reserve(payload.len());
                for &b in payload {
                    samples.push((b as f32 - 128.0) / 128.0);
                }
            }
            (AUDIO_FORMAT_PCM, 16) => {
                let sample_count = payload.len() / 2;
                samples.reserve(sample_count);
                for i in 0..sample_count {
                    let sample = i16::from_le_bytes([payload[i * 2], payload[i * 2 + 1]]);
                    samples.push(sample as f32 / 32768.0);
                }
            }
            (AUDIO_FORMAT_PCM, 24) => {
                let sample_count = payload.len() / 3;
                samples.reserve(sample_count);
                for i in 0..sample_count {
                    let b0 = payload[i * 3];
                    let b1 = payload[i * 3 + 1];
                    let b2 = payload[i * 3 + 2];
                    // Sign extend 24-bit integer
                    let val = i32::from_le_bytes([b0, b1, b2, if b2 & 0x80 != 0 { 0xFF } else { 0x00 }]);
                    samples.push(val as f32 / 8388608.0);
                }
            }
            (AUDIO_FORMAT_IEEE_FLOAT, 32) => {
                let sample_count = payload.len() / 4;
                samples.reserve(sample_count);
                for i in 0..sample_count {
                    let val = f32::from_le_bytes([
                        payload[i * 4],
                        payload[i * 4 + 1],
                        payload[i * 4 + 2],
                        payload[i * 4 + 3],
                    ]);
                    samples.push(val);
                }
            }
            _ => return Err(AudioError::UnsupportedBitsPerSample(bits)),
        }

        let total_frames = samples.len() / ch.max(1) as usize;
        let duration_seconds = total_frames as f32 / sr.max(1) as f32;

        Ok(Self {
            channels: ch,
            sample_rate: sr,
            bits_per_sample: bits,
            duration_seconds,
            samples,
        })
    }

    /// Number of audio frames (time slices across all channels).
    #[must_use]
    pub fn frame_count(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }

    /// Retrieves sample for a specific channel and frame index.
    #[must_use]
    pub fn sample_at(&self, channel: usize, frame_idx: usize) -> f32 {
        let ch = self.channels as usize;
        if channel >= ch {
            return 0.0;
        }
        let index = frame_idx * ch + channel;
        self.samples.get(index).copied().unwrap_or(0.0)
    }

    /// Generates a synthetic sine wave clip for testing and sound synthesis.
    #[must_use]
    pub fn generate_sine(sample_rate: u32, channels: u16, frequency: f32, duration_seconds: f32) -> Self {
        let total_frames = (sample_rate as f32 * duration_seconds) as usize;
        let mut samples = Vec::with_capacity(total_frames * channels as usize);

        for f in 0..total_frames {
            let t = f as f32 / sample_rate as f32;
            let val = (2.0 * std::f32::consts::PI * frequency * t).sin();
            for _ in 0..channels {
                samples.push(val);
            }
        }

        Self {
            channels,
            sample_rate,
            bits_per_sample: 16,
            duration_seconds,
            samples,
        }
    }

    /// Serializes audio clip to valid 16-bit PCM RIFF WAVE bytes.
    #[must_use]
    pub fn to_wav_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();

        let num_ch = self.channels;
        let sr = self.sample_rate;
        let bits = 16u16;
        let block_align = num_ch * (bits / 8);
        let byte_rate = sr * block_align as u32;

        let pcm_payload_len = (self.samples.len() * 2) as u32;
        let riff_len = 36 + pcm_payload_len;

        // RIFF header
        out.extend_from_slice(&RIFF_MAGIC);
        out.extend_from_slice(&riff_len.to_le_bytes());
        out.extend_from_slice(&WAVE_MAGIC);

        // fmt chunk
        out.extend_from_slice(&FMT_CHUNK);
        out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
        out.extend_from_slice(&AUDIO_FORMAT_PCM.to_le_bytes());
        out.extend_from_slice(&num_ch.to_le_bytes());
        out.extend_from_slice(&sr.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());

        // data chunk
        out.extend_from_slice(&DATA_CHUNK);
        out.extend_from_slice(&pcm_payload_len.to_le_bytes());

        for &sample in &self.samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let s16 = (clamped * 32767.0) as i16;
            out.extend_from_slice(&s16.to_le_bytes());
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wav_sine_roundtrip() {
        let clip = WavClip::generate_sine(44100, 2, 440.0, 0.1);
        assert_eq!(clip.channels, 2);
        assert_eq!(clip.sample_rate, 44100);
        assert_eq!(clip.frame_count(), 4410);

        let bytes = clip.to_wav_bytes();
        let parsed = WavClip::parse(&bytes).expect("Failed to parse WAV bytes");

        assert_eq!(parsed.channels, 2);
        assert_eq!(parsed.sample_rate, 44100);
        assert_eq!(parsed.bits_per_sample, 16);
        assert_eq!(parsed.frame_count(), 4410);

        // Verify sample values match within 16-bit quantization epsilon
        for i in 0..100 {
            let diff = (clip.samples[i] - parsed.samples[i]).abs();
            assert!(diff < 0.001, "Sample {i} diff {diff} too high");
        }
    }
}
