// crates/swg-client-customization/src/palette.rs

use glam::Vec4;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An RGBA color ramp extracted from a Star Wars Galaxies `.pal` asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorRamp {
    pub name: String,
    pub colors: Vec<[u8; 4]>,
}

impl ColorRamp {
    #[must_use]
    pub fn new(name: impl Into<String>, colors: Vec<[u8; 4]>) -> Self {
        Self {
            name: name.into(),
            colors,
        }
    }

    /// Retrieves the discrete color entry at `index`, clamped to bounds.
    #[must_use]
    pub fn get_color(&self, index: usize) -> [u8; 4] {
        if self.colors.is_empty() {
            return [255, 255, 255, 255];
        }
        let clamped = index.min(self.colors.len() - 1);
        self.colors[clamped]
    }

    /// Samples the color ramp using a continuous normalized parameter `t` in `0.0..=1.0`.
    #[must_use]
    pub fn sample_normalized(&self, t: f32) -> [u8; 4] {
        if self.colors.is_empty() {
            return [255, 255, 255, 255];
        }
        if self.colors.len() == 1 {
            return self.colors[0];
        }

        let clamped_t = t.clamp(0.0, 1.0);
        let max_idx = (self.colors.len() - 1) as f32;
        let pos = clamped_t * max_idx;
        let idx0 = pos.floor() as usize;
        let idx1 = (idx0 + 1).min(self.colors.len() - 1);
        let frac = pos - (idx0 as f32);

        let c0 = self.colors[idx0];
        let c1 = self.colors[idx1];

        let r = ((c0[0] as f32) * (1.0 - frac) + (c1[0] as f32) * frac) as u8;
        let g = ((c0[1] as f32) * (1.0 - frac) + (c1[1] as f32) * frac) as u8;
        let b = ((c0[2] as f32) * (1.0 - frac) + (c1[2] as f32) * frac) as u8;
        let a = ((c0[3] as f32) * (1.0 - frac) + (c1[3] as f32) * frac) as u8;

        [r, g, b, a]
    }

    /// Converts color to normalized `Vec4` suitable for GPU shader uniforms.
    #[must_use]
    pub fn sample_vec4(&self, index: usize) -> Vec4 {
        let [r, g, b, a] = self.get_color(index);
        Vec4::new(
            (r as f32) / 255.0,
            (g as f32) / 255.0,
            (b as f32) / 255.0,
            (a as f32) / 255.0,
        )
    }

    /// Encodes the palette into binary format.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.colors.len() * 4);
        // Header: count of colors (u32 LE)
        bytes.extend_from_slice(&(self.colors.len() as u32).to_le_bytes());
        for c in &self.colors {
            bytes.extend_from_slice(c);
        }
        bytes
    }

    /// Decodes a binary palette buffer.
    pub fn from_bytes(name: impl Into<String>, bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 4 {
            return Err("Palette buffer too short for header");
        }
        let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let expected_len = 4 + count * 4;
        if bytes.len() < expected_len {
            return Err("Palette buffer shorter than declared color count");
        }

        let mut colors = Vec::with_capacity(count);
        for i in 0..count {
            let offset = 4 + i * 4;
            colors.push([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
        }

        Ok(Self {
            name: name.into(),
            colors,
        })
    }
}

/// A character customization variable referencing a palette ramp and color index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomizationVariable {
    pub path: String,
    pub palette_path: String,
    pub color_index: u16,
}

/// Central catalog of loaded `.pal` palettes.
#[derive(Debug, Clone, Default)]
pub struct PaletteLibrary {
    palettes: HashMap<String, ColorRamp>,
}

impl PaletteLibrary {
    #[must_use]
    pub fn new() -> Self {
        let mut lib = Self {
            palettes: HashMap::new(),
        };
        lib.register_builtin_palettes();
        lib
    }

    pub fn insert(&mut self, path: impl Into<String>, ramp: ColorRamp) {
        self.palettes.insert(path.into(), ramp);
    }

    #[must_use]
    pub fn get(&self, path: &str) -> Option<&ColorRamp> {
        self.palettes.get(path)
    }

    #[must_use]
    pub fn sample_color(&self, palette_path: &str, index: usize) -> Option<[u8; 4]> {
        self.get(palette_path).map(|p| p.get_color(index))
    }

    /// Pre-populates canonical retail SWG palettes.
    fn register_builtin_palettes(&mut self) {
        // Human Skin
        let human_skin = ColorRamp::new(
            "human_skin",
            vec![
                [248, 224, 203, 255], // Fair
                [236, 198, 169, 255], // Peach
                [208, 162, 127, 255], // Tan
                [176, 128, 92, 255],  // Olive
                [136, 88, 54, 255],   // Brown
                [78, 48, 28, 255],    // Dark brown
            ],
        );
        self.insert("palette/human_skin.pal", human_skin);

        // Human Hair
        let human_hair = ColorRamp::new(
            "human_hair",
            vec![
                [240, 220, 150, 255], // Blonde
                [180, 120, 60, 255],  // Light brown
                [100, 60, 30, 255],   // Dark brown
                [30, 25, 20, 255],    // Jet black
                [170, 70, 40, 255],   // Auburn/Red
                [210, 210, 210, 255], // Platinum/White
            ],
        );
        self.insert("palette/human_hair.pal", human_hair);

        // Human Eyes
        let human_eyes = ColorRamp::new(
            "human_eyes",
            vec![
                [60, 120, 200, 255],  // Blue
                [60, 160, 90, 255],   // Green
                [110, 70, 40, 255],   // Brown
                [140, 130, 70, 255],  // Hazel
                [120, 130, 140, 255], // Gray
            ],
        );
        self.insert("palette/human_eyes.pal", human_eyes);

        // Twi'lek Skin (Rutian, Lethan, etc.)
        let twilek_skin = ColorRamp::new(
            "twilek_skin",
            vec![
                [70, 140, 220, 255],  // Rutian Blue
                [210, 50, 45, 255],   // Lethan Red
                [80, 190, 120, 255],  // Green
                [180, 110, 220, 255], // Violet
                [240, 210, 100, 255], // Yellow
                [245, 200, 185, 255], // Flesh/Tan
            ],
        );
        self.insert("palette/twilek_skin.pal", twilek_skin);

        // Zabrak Skin
        let zabrak_skin = ColorRamp::new(
            "zabrak_skin",
            vec![
                [230, 180, 140, 255],
                [195, 140, 95, 255],
                [160, 95, 60, 255],
                [185, 80, 60, 255], // Ochre tint
            ],
        );
        self.insert("palette/zabrak_skin.pal", zabrak_skin);

        // Trandoshan Scales
        let trando_scales = ColorRamp::new(
            "trando_scales",
            vec![
                [75, 110, 60, 255],  // Forest moss
                [130, 140, 65, 255], // Olive yellow
                [160, 135, 75, 255], // Desert sand
                [60, 75, 55, 255],   // Dark swamp green
            ],
        );
        self.insert("palette/trando_scales.pal", trando_scales);

        // Wookiee Fur
        let wookiee_fur = ColorRamp::new(
            "wookiee_fur",
            vec![
                [90, 55, 30, 255],   // Kashyyyk brown
                [140, 75, 35, 255],  // Cinnamon
                [40, 30, 25, 255],   // Charcoal
                [170, 110, 50, 255], // Tawny gold
            ],
        );
        self.insert("palette/wookiee_fur.pal", wookiee_fur);

        // Clothing Dyes
        let clothing_dyes = ColorRamp::new(
            "clothing_dyes",
            vec![
                [20, 50, 120, 255],   // Corellian Navy Blue
                [180, 30, 30, 255],   // Imperial Crimson
                [220, 180, 50, 255],  // Naboo Gold
                [190, 160, 120, 255], // Tatooine Dune
                [40, 100, 50, 255],   // Endor Olive
                [25, 25, 28, 255],    // Stealth Charcoal
            ],
        );
        self.insert("palette/clothing_dyes.pal", clothing_dyes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_binary_roundtrip() {
        let original = ColorRamp::new(
            "test_ramp",
            vec![[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]],
        );

        let bytes = original.to_bytes();
        let decoded = ColorRamp::from_bytes("test_ramp", &bytes).unwrap();

        assert_eq!(decoded.colors.len(), 3);
        assert_eq!(decoded.get_color(0), [255, 0, 0, 255]);
        assert_eq!(decoded.get_color(1), [0, 255, 0, 255]);
        assert_eq!(decoded.get_color(2), [0, 0, 255, 255]);
    }

    #[test]
    fn test_normalized_color_ramp_sampling() {
        let ramp = ColorRamp::new("black_to_white", vec![[0, 0, 0, 255], [200, 200, 200, 255]]);

        let mid = ramp.sample_normalized(0.5);
        assert_eq!(mid, [100, 100, 100, 255]);

        let v4 = ramp.sample_vec4(1);
        assert!((v4.x - (200.0 / 255.0)).abs() < 0.001);
    }

    #[test]
    fn test_palette_library_preloaded_species() {
        let lib = PaletteLibrary::new();
        let twilek_blue = lib.sample_color("palette/twilek_skin.pal", 0).unwrap();
        assert_eq!(twilek_blue, [70, 140, 220, 255]); // Rutian Blue

        let human_hair = lib.sample_color("palette/human_hair.pal", 3).unwrap();
        assert_eq!(human_hair, [30, 25, 20, 255]); // Jet black
    }
}
