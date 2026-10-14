// crates/swg-client-render/src/dds.rs

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DDS_MAGIC: u32 = 0x2053_4444; // "DDS "

pub const FOURCC_DXT1: u32 = 0x3154_5844; // "DXT1"
pub const FOURCC_DXT3: u32 = 0x3354_5844; // "DXT3"
pub const FOURCC_DXT5: u32 = 0x3554_5844; // "DXT5"

pub const DDPF_ALPHAPIXELS: u32 = 0x0000_0001;
pub const DDPF_ALPHA: u32 = 0x0000_0002;
pub const DDPF_FOURCC: u32 = 0x0000_0004;
pub const DDPF_RGB: u32 = 0x0000_0040;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DdsError {
    #[error("Invalid DDS magic number: {0:#X}")]
    InvalidMagic(u32),
    #[error("Header buffer too small: expected at least 128 bytes, got {0}")]
    BufferTooSmall(usize),
    #[error("Unsupported texture format or FourCC: {0:#X}")]
    UnsupportedFormat(u32),
    #[error("Data payload truncated: expected {0} bytes, got {1}")]
    DataTruncated(usize, usize),
}

/// Pixel format specification of a DDS texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DdsFormat {
    Dxt1,
    Dxt3,
    Dxt5,
    Rgba8,
    Bgra8,
}

impl DdsFormat {
    #[must_use]
    pub const fn block_size(&self) -> usize {
        match self {
            Self::Dxt1 => 8,
            Self::Dxt3 | Self::Dxt5 => 16,
            Self::Rgba8 | Self::Bgra8 => 4,
        }
    }

    #[must_use]
    pub const fn is_compressed(&self) -> bool {
        matches!(self, Self::Dxt1 | Self::Dxt3 | Self::Dxt5)
    }
}

/// Parsed DDS image containing dimensional metadata, format, and raw pixel blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DdsImage {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mipmap_count: u32,
    pub format: DdsFormat,
    pub data: Vec<u8>,
}

impl DdsImage {
    /// Parses a DirectDraw Surface (.dds) from binary buffer.
    pub fn parse(bytes: &[u8]) -> Result<Self, DdsError> {
        if bytes.len() < 128 {
            return Err(DdsError::BufferTooSmall(bytes.len()));
        }

        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != DDS_MAGIC {
            return Err(DdsError::InvalidMagic(magic));
        }

        let height = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let width = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let depth = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]).max(1);
        let mipmap_count =
            u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]).max(1);

        // Pixel format starts at offset 76 (relative to start of file: 80)
        let pf_flags = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]);
        let four_cc = u32::from_le_bytes([bytes[84], bytes[85], bytes[86], bytes[87]]);
        let rgb_bit_count = u32::from_le_bytes([bytes[88], bytes[89], bytes[90], bytes[91]]);
        let r_mask = u32::from_le_bytes([bytes[92], bytes[93], bytes[94], bytes[95]]);

        let format = if (pf_flags & DDPF_FOURCC) != 0 {
            match four_cc {
                FOURCC_DXT1 => DdsFormat::Dxt1,
                FOURCC_DXT3 => DdsFormat::Dxt3,
                FOURCC_DXT5 => DdsFormat::Dxt5,
                _ => return Err(DdsError::UnsupportedFormat(four_cc)),
            }
        } else if (pf_flags & DDPF_RGB) != 0 {
            if rgb_bit_count == 32 {
                if r_mask == 0x00FF_0000 {
                    DdsFormat::Bgra8
                } else {
                    DdsFormat::Rgba8
                }
            } else {
                return Err(DdsError::UnsupportedFormat(rgb_bit_count));
            }
        } else {
            return Err(DdsError::UnsupportedFormat(pf_flags));
        };

        let data = bytes[128..].to_vec();

        Ok(Self {
            width,
            height,
            depth,
            mipmap_count,
            format,
            data,
        })
    }

    /// Decompresses the base mipmap layer to uncompressed 32-bit RGBA8 pixels.
    pub fn decode_to_rgba8(&self) -> Result<Vec<u8>, DdsError> {
        let w = self.width as usize;
        let h = self.height as usize;
        let mut out = vec![0u8; w * h * 4];

        match self.format {
            DdsFormat::Rgba8 => {
                let required = w * h * 4;
                if self.data.len() < required {
                    return Err(DdsError::DataTruncated(required, self.data.len()));
                }
                out.copy_from_slice(&self.data[..required]);
            }
            DdsFormat::Bgra8 => {
                let required = w * h * 4;
                if self.data.len() < required {
                    return Err(DdsError::DataTruncated(required, self.data.len()));
                }
                for i in 0..w * h {
                    let b = self.data[i * 4];
                    let g = self.data[i * 4 + 1];
                    let r = self.data[i * 4 + 2];
                    let a = self.data[i * 4 + 3];
                    out[i * 4] = r;
                    out[i * 4 + 1] = g;
                    out[i * 4 + 2] = b;
                    out[i * 4 + 3] = a;
                }
            }
            DdsFormat::Dxt1 => {
                let blocks_x = (w + 3) / 4;
                let blocks_y = (h + 3) / 4;
                let required = blocks_x * blocks_y * 8;
                if self.data.len() < required {
                    return Err(DdsError::DataTruncated(required, self.data.len()));
                }

                for by in 0..blocks_y {
                    for bx in 0..blocks_x {
                        let block_idx = (by * blocks_x + bx) * 8;
                        let block = &self.data[block_idx..block_idx + 8];
                        decompress_dxt1_block(block, bx * 4, by * 4, w, h, &mut out);
                    }
                }
            }
            DdsFormat::Dxt3 => {
                let blocks_x = (w + 3) / 4;
                let blocks_y = (h + 3) / 4;
                let required = blocks_x * blocks_y * 16;
                if self.data.len() < required {
                    return Err(DdsError::DataTruncated(required, self.data.len()));
                }

                for by in 0..blocks_y {
                    for bx in 0..blocks_x {
                        let block_idx = (by * blocks_x + bx) * 16;
                        let block = &self.data[block_idx..block_idx + 16];
                        decompress_dxt3_block(block, bx * 4, by * 4, w, h, &mut out);
                    }
                }
            }
            DdsFormat::Dxt5 => {
                let blocks_x = (w + 3) / 4;
                let blocks_y = (h + 3) / 4;
                let required = blocks_x * blocks_y * 16;
                if self.data.len() < required {
                    return Err(DdsError::DataTruncated(required, self.data.len()));
                }

                for by in 0..blocks_y {
                    for bx in 0..blocks_x {
                        let block_idx = (by * blocks_x + bx) * 16;
                        let block = &self.data[block_idx..block_idx + 16];
                        decompress_dxt5_block(block, bx * 4, by * 4, w, h, &mut out);
                    }
                }
            }
        }

        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// DXT Decompression Helpers
// ---------------------------------------------------------------------------

fn unpack_rgb565(c: u16) -> [u8; 4] {
    let r = ((c >> 11) & 0x1F) as u8;
    let g = ((c >> 5) & 0x3F) as u8;
    let b = (c & 0x1F) as u8;
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
        255,
    ]
}

fn decompress_dxt1_block(block: &[u8], bx: usize, by: usize, width: usize, height: usize, out: &mut [u8]) {
    let c0_raw = u16::from_le_bytes([block[0], block[1]]);
    let c1_raw = u16::from_le_bytes([block[2], block[3]]);
    let c0 = unpack_rgb565(c0_raw);
    let c1 = unpack_rgb565(c1_raw);

    let mut palette = [[0u8; 4]; 4];
    palette[0] = c0;
    palette[1] = c1;

    if c0_raw > c1_raw {
        palette[2] = [
            ((2 * c0[0] as u16 + c1[0] as u16) / 3) as u8,
            ((2 * c0[1] as u16 + c1[1] as u16) / 3) as u8,
            ((2 * c0[2] as u16 + c1[2] as u16) / 3) as u8,
            255,
        ];
        palette[3] = [
            ((c0[0] as u16 + 2 * c1[0] as u16) / 3) as u8,
            ((c0[1] as u16 + 2 * c1[1] as u16) / 3) as u8,
            ((c0[2] as u16 + 2 * c1[2] as u16) / 3) as u8,
            255,
        ];
    } else {
        palette[2] = [
            ((c0[0] as u16 + c1[0] as u16) / 2) as u8,
            ((c0[1] as u16 + c1[1] as u16) / 2) as u8,
            ((c0[2] as u16 + c1[2] as u16) / 2) as u8,
            255,
        ];
        palette[3] = [0, 0, 0, 0]; // 1-bit transparent
    }

    let lookup = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    for py in 0..4 {
        for px in 0..4 {
            let x = bx + px;
            let y = by + py;
            if x < width && y < height {
                let shift = (py * 4 + px) * 2;
                let idx = ((lookup >> shift) & 0x03) as usize;
                let color = palette[idx];
                let out_idx = (y * width + x) * 4;
                out[out_idx..out_idx + 4].copy_from_slice(&color);
            }
        }
    }
}

fn decompress_dxt3_block(block: &[u8], bx: usize, by: usize, width: usize, height: usize, out: &mut [u8]) {
    // First 8 bytes are 4-bit alpha values (16 pixels)
    // Next 8 bytes are standard DXT1 color block
    decompress_dxt1_block(&block[8..16], bx, by, width, height, out);

    for py in 0..4 {
        for px in 0..4 {
            let x = bx + px;
            let y = by + py;
            if x < width && y < height {
                let pixel_idx = py * 4 + px;
                let byte_idx = pixel_idx / 2;
                let nibble = if pixel_idx % 2 == 0 {
                    block[byte_idx] & 0x0F
                } else {
                    (block[byte_idx] >> 4) & 0x0F
                };
                let alpha = (nibble << 4) | nibble;
                let out_idx = (y * width + x) * 4 + 3;
                out[out_idx] = alpha;
            }
        }
    }
}

fn decompress_dxt5_block(block: &[u8], bx: usize, by: usize, width: usize, height: usize, out: &mut [u8]) {
    // First 8 bytes are 8-value interpolated alpha
    let a0 = block[0];
    let a1 = block[1];
    let mut alpha_palette = [0u8; 8];
    alpha_palette[0] = a0;
    alpha_palette[1] = a1;

    if a0 > a1 {
        alpha_palette[2] = ((6 * a0 as u16 + 1 * a1 as u16) / 7) as u8;
        alpha_palette[3] = ((5 * a0 as u16 + 2 * a1 as u16) / 7) as u8;
        alpha_palette[4] = ((4 * a0 as u16 + 3 * a1 as u16) / 7) as u8;
        alpha_palette[5] = ((3 * a0 as u16 + 4 * a1 as u16) / 7) as u8;
        alpha_palette[6] = ((2 * a0 as u16 + 5 * a1 as u16) / 7) as u8;
        alpha_palette[7] = ((1 * a0 as u16 + 6 * a1 as u16) / 7) as u8;
    } else {
        alpha_palette[2] = ((4 * a0 as u16 + 1 * a1 as u16) / 5) as u8;
        alpha_palette[3] = ((3 * a0 as u16 + 2 * a1 as u16) / 5) as u8;
        alpha_palette[4] = ((2 * a0 as u16 + 3 * a1 as u16) / 5) as u8;
        alpha_palette[5] = ((1 * a0 as u16 + 4 * a1 as u16) / 5) as u8;
        alpha_palette[6] = 0;
        alpha_palette[7] = 255;
    }

    // 48-bit alpha indices (6 bytes)
    let a_bits = u64::from_le_bytes([
        block[2], block[3], block[4], block[5], block[6], block[7], 0, 0,
    ]);

    // Decompress colors from second 8 bytes
    decompress_dxt1_block(&block[8..16], bx, by, width, height, out);

    // Apply interpolated alpha
    for py in 0..4 {
        for px in 0..4 {
            let x = bx + px;
            let y = by + py;
            if x < width && y < height {
                let pixel_idx = py * 4 + px;
                let shift = pixel_idx * 3;
                let a_idx = ((a_bits >> shift) & 0x07) as usize;
                let alpha = alpha_palette[a_idx];
                let out_idx = (y * width + x) * 4 + 3;
                out[out_idx] = alpha;
            }
        }
    }
}

/// Helper builder for creating valid synthetic DDS texture files for tests and assets.
pub struct DdsBuilder {
    width: u32,
    height: u32,
    format: DdsFormat,
    data: Vec<u8>,
}

impl DdsBuilder {
    #[must_use]
    pub fn new(width: u32, height: u32, format: DdsFormat) -> Self {
        Self {
            width,
            height,
            format,
            data: Vec::new(),
        }
    }

    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    #[must_use]
    pub fn build(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(128 + self.data.len());
        out.extend_from_slice(&DDS_MAGIC.to_le_bytes()); // 0..4
        out.extend_from_slice(&124u32.to_le_bytes()); // header size (4..8)
        out.extend_from_slice(&0x0002_1007u32.to_le_bytes()); // flags: caps, height, width, pixelformat, mipmapcount (8..12)
        out.extend_from_slice(&self.height.to_le_bytes()); // 12..16
        out.extend_from_slice(&self.width.to_le_bytes()); // 16..20
        out.extend_from_slice(&0u32.to_le_bytes()); // pitch / linear size (20..24)
        out.extend_from_slice(&1u32.to_le_bytes()); // depth (24..28)
        out.extend_from_slice(&1u32.to_le_bytes()); // mipmap count (28..32)
        out.extend_from_slice(&[0u8; 44]); // reserved 11 dwords (32..76)

        // Pixel Format (32 bytes at 76..108, file offset 80..112)
        out.extend_from_slice(&32u32.to_le_bytes()); // pf size
        match self.format {
            DdsFormat::Dxt1 => {
                out.extend_from_slice(&DDPF_FOURCC.to_le_bytes());
                out.extend_from_slice(&FOURCC_DXT1.to_le_bytes());
                out.extend_from_slice(&[0u8; 20]);
            }
            DdsFormat::Dxt3 => {
                out.extend_from_slice(&DDPF_FOURCC.to_le_bytes());
                out.extend_from_slice(&FOURCC_DXT3.to_le_bytes());
                out.extend_from_slice(&[0u8; 20]);
            }
            DdsFormat::Dxt5 => {
                out.extend_from_slice(&DDPF_FOURCC.to_le_bytes());
                out.extend_from_slice(&FOURCC_DXT5.to_le_bytes());
                out.extend_from_slice(&[0u8; 20]);
            }
            DdsFormat::Rgba8 => {
                out.extend_from_slice(&(DDPF_RGB | DDPF_ALPHAPIXELS).to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes()); // fourCC
                out.extend_from_slice(&32u32.to_le_bytes()); // rgb bit count
                out.extend_from_slice(&0x0000_00FFu32.to_le_bytes()); // r
                out.extend_from_slice(&0x0000_FF00u32.to_le_bytes()); // g
                out.extend_from_slice(&0x00FF_0000u32.to_le_bytes()); // b
                out.extend_from_slice(&0xFF00_0000u32.to_le_bytes()); // a
            }
            DdsFormat::Bgra8 => {
                out.extend_from_slice(&(DDPF_RGB | DDPF_ALPHAPIXELS).to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes()); // fourCC
                out.extend_from_slice(&32u32.to_le_bytes()); // rgb bit count
                out.extend_from_slice(&0x00FF_0000u32.to_le_bytes()); // r
                out.extend_from_slice(&0x0000_FF00u32.to_le_bytes()); // g
                out.extend_from_slice(&0x0000_00FFu32.to_le_bytes()); // b
                out.extend_from_slice(&0xFF00_0000u32.to_le_bytes()); // a
            }
        }

        // Caps (16 bytes: 108..124)
        out.extend_from_slice(&0x0000_1000u32.to_le_bytes()); // DDSCAPS_TEXTURE
        out.extend_from_slice(&[0u8; 12]);
        out.extend_from_slice(&0u32.to_le_bytes()); // dwReserved2 (4 bytes, bringing header to 124 bytes + 4 magic = 128)

        // Payload
        out.extend_from_slice(&self.data);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dds_rgba8_roundtrip() {
        let mut builder = DdsBuilder::new(2, 2, DdsFormat::Rgba8);
        let pixels = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            255, 255, 0, 128, // Yellow semi-transparent
        ];
        builder.set_data(pixels.clone());

        let bytes = builder.build();
        let image = DdsImage::parse(&bytes).expect("Failed to parse DDS RGBA8");

        assert_eq!(image.width, 2);
        assert_eq!(image.height, 2);
        assert_eq!(image.format, DdsFormat::Rgba8);

        let decoded = image.decode_to_rgba8().unwrap();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn test_dds_dxt1_decompression() {
        let mut builder = DdsBuilder::new(4, 4, DdsFormat::Dxt1);
        // 4x4 block: 8 bytes
        // c0: 0xF800 (Red in RGB565)
        // c1: 0x001F (Blue in RGB565)
        // lookup: 0x00000000 (all pixels index 0 = c0)
        let block = vec![0x00, 0xF8, 0x1F, 0x00, 0x00, 0x00, 0x00, 0x00];
        builder.set_data(block);

        let bytes = builder.build();
        let image = DdsImage::parse(&bytes).expect("Failed to parse DDS DXT1");

        assert_eq!(image.format, DdsFormat::Dxt1);
        let decoded = image.decode_to_rgba8().unwrap();
        assert_eq!(decoded.len(), 4 * 4 * 4);

        // Every pixel should be red [255, 0, 0, 255]
        assert_eq!(decoded[0], 255);
        assert_eq!(decoded[1], 0);
        assert_eq!(decoded[2], 0);
        assert_eq!(decoded[3], 255);
    }

    #[test]
    fn test_dds_dxt5_decompression() {
        let mut builder = DdsBuilder::new(4, 4, DdsFormat::Dxt5);
        // 16-byte DXT5 block:
        // 8 bytes alpha: a0=255, a1=0, indices all 0
        // 8 bytes color: DXT1 block
        let mut block = vec![255, 0, 0, 0, 0, 0, 0, 0];
        block.extend_from_slice(&[0x00, 0xF8, 0x1F, 0x00, 0x00, 0x00, 0x00, 0x00]);
        builder.set_data(block);

        let bytes = builder.build();
        let image = DdsImage::parse(&bytes).expect("Failed to parse DDS DXT5");

        assert_eq!(image.format, DdsFormat::Dxt5);
        let decoded = image.decode_to_rgba8().unwrap();
        assert_eq!(decoded.len(), 4 * 4 * 4);
        assert_eq!(decoded[0], 255);
        assert_eq!(decoded[3], 255); // Alpha = 255
    }
}
