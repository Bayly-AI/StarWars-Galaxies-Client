// crates/swg-client-audio/src/snd.rs

use crate::wav::AudioError;
use serde::{Deserialize, Serialize};

pub const FORM_MAGIC: [u8; 4] = *b"FORM";
pub const SD2D_TAG: [u8; 4] = *b"SD2D";
pub const SD3D_TAG: [u8; 4] = *b"SD3D";

/// Authentic Star Wars Galaxies sound template describing 2D/3D playback properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundTemplate {
    pub name: String,
    pub sample_paths: Vec<String>,
    pub volume: f32,
    pub pitch: f32,
    pub is_3d: bool,
    pub min_distance: f32,
    pub max_distance: f32,
    pub is_looping: bool,
}

impl Default for SoundTemplate {
    fn default() -> Self {
        Self {
            name: String::new(),
            sample_paths: Vec::new(),
            volume: 1.0,
            pitch: 1.0,
            is_3d: false,
            min_distance: 1.0,
            max_distance: 60.0,
            is_looping: false,
        }
    }
}

impl SoundTemplate {
    /// Creates a 2D non-spatial sound template (e.g. UI sounds, music stingers).
    pub fn new_2d(sample_path: &str, volume: f32) -> Self {
        Self {
            name: sample_path.to_string(),
            sample_paths: vec![sample_path.to_string()],
            volume,
            pitch: 1.0,
            is_3d: false,
            min_distance: 1.0,
            max_distance: 100.0,
            is_looping: false,
        }
    }

    /// Creates a 3D spatial sound template (e.g. weapons, footsteps, creatures, machinery).
    pub fn new_3d(sample_path: &str, volume: f32, min_distance: f32, max_distance: f32) -> Self {
        Self {
            name: sample_path.to_string(),
            sample_paths: vec![sample_path.to_string()],
            volume,
            pitch: 1.0,
            is_3d: true,
            min_distance,
            max_distance,
            is_looping: false,
        }
    }

    /// Primary audio sample asset path referenced by this sound template.
    #[must_use]
    pub fn primary_sample(&self) -> Option<&str> {
        self.sample_paths.first().map(|s| s.as_str())
    }

    /// Parses an authentic SOE IFF .snd template from a raw byte buffer.
    pub fn parse(bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 12 {
            return Err(AudioError::InvalidSndHeader(
                "Buffer too short for IFF container".to_string(),
            ));
        }

        let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if magic != FORM_MAGIC {
            return Err(AudioError::InvalidSndHeader(format!(
                "Expected 'FORM', got {magic:?}"
            )));
        }

        let tag = [bytes[8], bytes[9], bytes[10], bytes[11]];
        let is_3d = tag == SD3D_TAG;
        let is_2d = tag == SD2D_TAG;

        if !is_3d && !is_2d {
            return Err(AudioError::InvalidSndHeader(format!(
                "Expected 'SD2D' or 'SD3D', got {tag:?}"
            )));
        }

        // Search for null-terminated sample paths matching standard prefixes
        let mut sample_paths = Vec::new();
        let mut offset = 12;

        while offset < bytes.len() {
            // Check for sample path strings
            if let Some(pos) = bytes[offset..].iter().position(|&b| b == b'.' ) {
                let dot_idx = offset + pos;
                if dot_idx + 4 <= bytes.len() {
                    let ext = &bytes[dot_idx..dot_idx + 4];
                    if ext == b".wav" || ext == b".mp3" {
                        // Scan backwards to find start of path
                        let mut start_idx = dot_idx;
                        while start_idx > 0 && bytes[start_idx - 1] >= 0x20 && bytes[start_idx - 1] <= 0x7E && bytes[start_idx - 1] != 0 {
                            start_idx -= 1;
                        }
                        let path_slice = &bytes[start_idx..dot_idx + 4];
                        if let Ok(path_str) = std::str::from_utf8(path_slice) {
                            if path_str.contains('/') || path_str.contains('\\') {
                                let norm = path_str.replace('\\', "/");
                                if !sample_paths.contains(&norm) {
                                    sample_paths.push(norm);
                                }
                            }
                        }
                        offset = dot_idx + 4;
                        continue;
                    }
                }
            }
            offset += 1;
        }

        // Extract 3D distance parameters from tail if SD3D
        let mut min_distance = 1.0;
        let mut max_distance = 60.0;

        if is_3d && bytes.len() >= 24 {
            let tail = &bytes[bytes.len() - 24..];
            for i in (0..tail.len() - 4).step_by(4) {
                let val = f32::from_le_bytes([tail[i], tail[i + 1], tail[i + 2], tail[i + 3]]);
                if val >= 0.5 && val <= 5000.0 && val.is_finite() {
                    if min_distance == 1.0 {
                        min_distance = val;
                    } else if val > min_distance {
                        max_distance = val;
                    }
                }
            }
        }

        // Check for looping in sample name or flags
        let is_looping = sample_paths.iter().any(|p| p.contains("_lp") || p.contains("loop"));

        let name = sample_paths
            .first()
            .cloned()
            .unwrap_or_else(|| "unnamed_sound".to_string());

        Ok(Self {
            name,
            sample_paths,
            volume: 1.0,
            pitch: 1.0,
            is_3d,
            min_distance,
            max_distance,
            is_looping,
        })
    }

    /// Serializes sound template to authentic SOE IFF byte representation.
    #[must_use]
    pub fn to_snd_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let sample = self.primary_sample().unwrap_or("sample/default.wav");

        if self.is_3d {
            // Outer SD3D FORM
            out.extend_from_slice(&FORM_MAGIC);
            let outer_size_pos = out.len();
            out.extend_from_slice(&0u32.to_be_bytes()); // placeholder
            out.extend_from_slice(&SD3D_TAG);

            // Inner SD2D FORM
            let inner_start = out.len();
            out.extend_from_slice(&FORM_MAGIC);
            let inner_size_pos = out.len();
            out.extend_from_slice(&0u32.to_be_bytes()); // placeholder
            out.extend_from_slice(&SD2D_TAG);
            out.extend_from_slice(b"0003");
            out.extend_from_slice(&144u32.to_be_bytes()); // chunk size

            // Sample payload
            out.extend_from_slice(&1u32.to_le_bytes()); // count
            out.extend_from_slice(sample.as_bytes());
            out.push(0); // null terminator

            // Pad to 144 bytes chunk size
            while out.len() - inner_start < 164 {
                out.push(0);
            }
            let inner_size = (out.len() - inner_start - 8) as u32;
            out[inner_size_pos..inner_size_pos + 4].copy_from_slice(&inner_size.to_be_bytes());

            // 3D properties tail
            out.extend_from_slice(&self.min_distance.to_le_bytes());
            out.extend_from_slice(&self.max_distance.to_le_bytes());
            out.extend_from_slice(b"0001");
            out.extend_from_slice(&0u32.to_le_bytes());

            let outer_size = (out.len() - 8) as u32;
            out[outer_size_pos..outer_size_pos + 4].copy_from_slice(&outer_size.to_be_bytes());
        } else {
            // SD2D FORM
            out.extend_from_slice(&FORM_MAGIC);
            let size_pos = out.len();
            out.extend_from_slice(&0u32.to_be_bytes()); // placeholder
            out.extend_from_slice(&SD2D_TAG);
            out.extend_from_slice(b"0003");
            out.extend_from_slice(&144u32.to_be_bytes());

            out.extend_from_slice(&1u32.to_le_bytes());
            out.extend_from_slice(sample.as_bytes());
            out.push(0);

            while out.len() < 164 {
                out.push(0);
            }
            let size = (out.len() - 8) as u32;
            out[size_pos..size_pos + 4].copy_from_slice(&size.to_be_bytes());
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_template_2d_roundtrip() {
        let t = SoundTemplate::new_2d("sample/ui_select_rotate.wav", 0.85);
        assert!(!t.is_3d);
        assert_eq!(t.primary_sample(), Some("sample/ui_select_rotate.wav"));

        let bytes = t.to_snd_bytes();
        let parsed = SoundTemplate::parse(&bytes).expect("Failed to parse 2D sound template");

        assert!(!parsed.is_3d);
        assert_eq!(
            parsed.primary_sample(),
            Some("sample/ui_select_rotate.wav")
        );
    }

    #[test]
    fn test_sound_template_3d_roundtrip() {
        let t = SoundTemplate::new_3d("sample/wep_blaster_fire_2.wav", 1.0, 5.0, 120.0);
        assert!(t.is_3d);
        assert_eq!(t.primary_sample(), Some("sample/wep_blaster_fire_2.wav"));

        let bytes = t.to_snd_bytes();
        let parsed = SoundTemplate::parse(&bytes).expect("Failed to parse 3D sound template");

        assert!(parsed.is_3d);
        assert_eq!(
            parsed.primary_sample(),
            Some("sample/wep_blaster_fire_2.wav")
        );
        assert_eq!(parsed.min_distance, 5.0);
        assert_eq!(parsed.max_distance, 120.0);
    }
}
