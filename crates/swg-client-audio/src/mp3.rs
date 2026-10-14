// crates/swg-client-audio/src/mp3.rs

use crate::wav::AudioError;
use serde::{Deserialize, Serialize};

/// MPEG-1 Layer III audio soundtrack track metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mp3Track {
    pub name: String,
    pub sample_rate: u32,
    pub bitrate_kbps: u32,
    pub channels: u16,
    pub duration_seconds: f32,
    pub size_bytes: usize,
}

impl Mp3Track {
    /// Parses MPEG audio frame headers from an authentic Star Wars Galaxies MP3 file.
    pub fn parse(name: &str, bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 4 {
            return Err(AudioError::InvalidMp3Header(
                "Buffer too small for MP3 header".to_string(),
            ));
        }

        // Bitrate lookup table for MPEG-1 Layer III (in kbps)
        const MPEG1_LAYER3_BITRATES: [u32; 16] = [
            0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
        ];
        // Sample rate lookup table for MPEG-1 (in Hz)
        const MPEG1_SAMPLERATES: [u32; 4] = [44100, 48000, 32000, 0];

        // Search for first valid MPEG audio frame sync: 11 bits set (0xFF followed by 0xE0)
        let mut frame_header = None;

        for offset in 0..bytes.len().saturating_sub(4) {
            let b0 = bytes[offset];
            let b1 = bytes[offset + 1];

            if b0 == 0xFF && (b1 & 0xE0) == 0xE0 {
                let mpeg_ver = (b1 >> 3) & 0x03;
                let layer = (b1 >> 1) & 0x03;

                // We expect MPEG-1 (ver=3) Layer III (layer=1)
                if mpeg_ver == 3 && layer == 1 {
                    let b2 = bytes[offset + 2];
                    let b3 = bytes[offset + 3];

                    let br_idx = ((b2 >> 4) & 0x0F) as usize;
                    let sr_idx = ((b2 >> 2) & 0x03) as usize;
                    let channel_mode = (b3 >> 6) & 0x03;

                    let bitrate = MPEG1_LAYER3_BITRATES[br_idx];
                    let sample_rate = MPEG1_SAMPLERATES[sr_idx];

                    if bitrate > 0 && sample_rate > 0 {
                        let channels = if channel_mode == 3 { 1 } else { 2 };
                        frame_header = Some((bitrate, sample_rate, channels));
                        break;
                    }
                }
            }
        }

        let (bitrate_kbps, sample_rate, channels) = frame_header.ok_or_else(|| {
            AudioError::InvalidMp3Header("Valid MPEG-1 Layer 3 frame sync not found".to_string())
        })?;

        // Estimate duration based on file size and bitrate
        let bits_total = bytes.len() as f32 * 8.0;
        let bitrate_bps = bitrate_kbps as f32 * 1000.0;
        let duration_seconds = bits_total / bitrate_bps;

        Ok(Self {
            name: name.to_string(),
            sample_rate,
            bitrate_kbps,
            channels,
            duration_seconds,
            size_bytes: bytes.len(),
        })
    }

    /// Generates a synthetic MP3 byte stream with valid MPEG-1 Layer III frame sync headers.
    #[must_use]
    pub fn generate_synthetic(bitrate_kbps: u32, duration_seconds: f32) -> Vec<u8> {
        let sample_rate = 44100u32;
        // Frame size formula for Layer III: 144 * BitRate / SampleRate + Padding
        let frame_size = (144 * (bitrate_kbps * 1000) / sample_rate) as usize;
        let frames_per_second = sample_rate as f32 / 1152.0;
        let total_frames = (duration_seconds * frames_per_second).max(1.0) as usize;

        let mut out = Vec::with_capacity(total_frames * frame_size);

        // Header byte 0: 0xFF
        // Header byte 1: 0xFB (MPEG-1, Layer 3, No Protection)
        // Header byte 2: (bitrate_idx << 4) | (0 << 2) | (0 << 1) | 0
        let br_idx = match bitrate_kbps {
            128 => 9,
            160 => 10,
            192 => 11,
            320 => 14,
            _ => 9,
        };
        let b2 = (br_idx << 4) as u8; // 44.1kHz, no padding
        let b3 = 0x40; // Joint stereo

        for _ in 0..total_frames {
            out.push(0xFF);
            out.push(0xFB);
            out.push(b2);
            out.push(b3);
            // Payload
            for _ in 4..frame_size {
                out.push(0xAA);
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mp3_metadata_parsing() {
        let bytes = Mp3Track::generate_synthetic(128, 2.5);
        let track = Mp3Track::parse("music/mus_combat_b_lp.mp3", &bytes)
            .expect("Failed to parse synthetic MP3");

        assert_eq!(track.sample_rate, 44100);
        assert_eq!(track.bitrate_kbps, 128);
        assert_eq!(track.channels, 2);
        assert!(
            (track.duration_seconds - 2.5).abs() < 0.2,
            "Duration {} deviated too much from 2.5s",
            track.duration_seconds
        );
    }
}
