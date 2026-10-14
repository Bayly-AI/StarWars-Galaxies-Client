// crates/swg-client-mesh/src/shader_template.rs

use crate::material::PbrMaterial;
use glam::{Vec3, Vec4};
use serde::{Deserialize, Serialize};
use swg_client_iff::{IffError, IffNode, IffWriter};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ShaderError {
    #[error("IFF error: {0}")]
    Iff(#[from] IffError),
    #[error("Invalid root form type: expected SHDR, CSHT, or SSHT")]
    InvalidRootForm,
    #[error("Missing material parameters or texture definitions")]
    MalformedShader,
}

/// Blend modes for surface rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Opaque,
    AlphaBlend,
    AlphaTest,
    Additive,
}

/// Type of shader template in Star Wars Galaxies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShaderType {
    /// Static shader for world geometry, buildings, vehicles, and weapons.
    Static,
    /// Character shader supporting multi-channel palette mask customization.
    Character,
}

/// Binding between a customization variable and a texture palette channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaletteBinding {
    pub variable_path: String,
    pub default_palette_path: String,
    pub mask_channel: u8, // 0 = Red (Skin), 1 = Green (Hair), 2 = Blue (Clothes Primary), 3 = Alpha (Clothes Secondary)
}

/// Complete Shader Template (.sht) representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShaderTemplate {
    pub name: String,
    pub shader_type: ShaderType,
    pub blend_mode: BlendMode,
    pub ambient: Vec4,
    pub diffuse: Vec4,
    pub specular: Vec4,
    pub emissive: Vec4,
    pub shininess: f32,
    pub diffuse_texture: Option<String>,
    pub palette_mask_texture: Option<String>,
    pub normal_texture: Option<String>,
    pub palette_bindings: Vec<PaletteBinding>,
}

impl Default for ShaderTemplate {
    fn default() -> Self {
        Self {
            name: "default_shader".to_string(),
            shader_type: ShaderType::Static,
            blend_mode: BlendMode::Opaque,
            ambient: Vec4::new(0.2, 0.2, 0.2, 1.0),
            diffuse: Vec4::ONE,
            specular: Vec4::new(0.5, 0.5, 0.5, 1.0),
            emissive: Vec4::ZERO,
            shininess: 32.0,
            diffuse_texture: None,
            palette_mask_texture: None,
            normal_texture: None,
            palette_bindings: Vec::new(),
        }
    }
}

impl ShaderTemplate {
    /// Parses an authentic SWG .sht file from IFF bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, ShaderError> {
        let root = IffNode::parse(bytes)?;
        let form_type = root.form_type().ok_or(ShaderError::InvalidRootForm)?;

        let shader_type = match &form_type {
            b"SHDR" | b"SSHT" => ShaderType::Static,
            b"CSHT" => ShaderType::Character,
            _ => return Err(ShaderError::InvalidRootForm),
        };

        let mut ambient = Vec4::new(0.2, 0.2, 0.2, 1.0);
        let mut diffuse = Vec4::ONE;
        let mut specular = Vec4::new(0.5, 0.5, 0.5, 1.0);
        let mut emissive = Vec4::ZERO;
        let mut shininess = 32.0;

        // Parse MATL chunk
        if let Some(matl_data) = root.find_chunk(b"MATL") {
            if matl_data.len() >= 68 {
                let mut off = 0;
                let mut read_v4 = || {
                    let x = f32::from_le_bytes([
                        matl_data[off],
                        matl_data[off + 1],
                        matl_data[off + 2],
                        matl_data[off + 3],
                    ]);
                    let y = f32::from_le_bytes([
                        matl_data[off + 4],
                        matl_data[off + 5],
                        matl_data[off + 6],
                        matl_data[off + 7],
                    ]);
                    let z = f32::from_le_bytes([
                        matl_data[off + 8],
                        matl_data[off + 9],
                        matl_data[off + 10],
                        matl_data[off + 11],
                    ]);
                    let w = f32::from_le_bytes([
                        matl_data[off + 12],
                        matl_data[off + 13],
                        matl_data[off + 14],
                        matl_data[off + 15],
                    ]);
                    off += 16;
                    Vec4::new(x, y, z, w)
                };

                ambient = read_v4();
                diffuse = read_v4();
                specular = read_v4();
                emissive = read_v4();
                shininess = f32::from_le_bytes([
                    matl_data[off],
                    matl_data[off + 1],
                    matl_data[off + 2],
                    matl_data[off + 3],
                ]);
            }
        }

        // Parse Blend Mode
        let mut blend_mode = BlendMode::Opaque;
        if let Some(blnd_data) = root.find_chunk(b"BLND") {
            if !blnd_data.is_empty() {
                blend_mode = match blnd_data[0] {
                    1 => BlendMode::AlphaBlend,
                    2 => BlendMode::AlphaTest,
                    3 => BlendMode::Additive,
                    _ => BlendMode::Opaque,
                };
            }
        }

        // Parse Texture maps (TXMS or TEXS chunks)
        let mut diffuse_texture = None;
        let mut palette_mask_texture = None;
        let mut normal_texture = None;

        if let Some(tex_data) = root.find_chunk(b"TXMS") {
            let mut off = 0;
            if tex_data.len() >= 4 {
                let count =
                    u32::from_le_bytes([tex_data[0], tex_data[1], tex_data[2], tex_data[3]])
                        as usize;
                off += 4;

                for i in 0..count {
                    if off + 4 > tex_data.len() {
                        break;
                    }
                    let str_len = u32::from_le_bytes([
                        tex_data[off],
                        tex_data[off + 1],
                        tex_data[off + 2],
                        tex_data[off + 3],
                    ]) as usize;
                    off += 4;
                    if off + str_len > tex_data.len() {
                        break;
                    }
                    let path = String::from_utf8_lossy(&tex_data[off..off + str_len])
                        .trim_end_matches('\0')
                        .to_string();
                    off += str_len;

                    match i {
                        0 => diffuse_texture = Some(path),
                        1 => {
                            if path.contains("_a.") || path.contains("_mask") {
                                palette_mask_texture = Some(path);
                            } else {
                                normal_texture = Some(path);
                            }
                        }
                        2 => {
                            if palette_mask_texture.is_none() {
                                palette_mask_texture = Some(path);
                            } else {
                                normal_texture = Some(path);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Parse Palette bindings: PVAR chunk
        let mut palette_bindings = Vec::new();
        if let Some(pvar_data) = root.find_chunk(b"PVAR") {
            let mut off = 0;
            if pvar_data.len() >= 4 {
                let count =
                    u32::from_le_bytes([pvar_data[0], pvar_data[1], pvar_data[2], pvar_data[3]])
                        as usize;
                off += 4;

                for _ in 0..count {
                    if off + 4 > pvar_data.len() {
                        break;
                    }
                    let var_len = u32::from_le_bytes([
                        pvar_data[off],
                        pvar_data[off + 1],
                        pvar_data[off + 2],
                        pvar_data[off + 3],
                    ]) as usize;
                    off += 4;
                    if off + var_len > pvar_data.len() {
                        break;
                    }
                    let variable_path = String::from_utf8_lossy(&pvar_data[off..off + var_len])
                        .trim_end_matches('\0')
                        .to_string();
                    off += var_len;

                    if off + 4 > pvar_data.len() {
                        break;
                    }
                    let pal_len = u32::from_le_bytes([
                        pvar_data[off],
                        pvar_data[off + 1],
                        pvar_data[off + 2],
                        pvar_data[off + 3],
                    ]) as usize;
                    off += 4;
                    if off + pal_len > pvar_data.len() {
                        break;
                    }
                    let default_palette_path =
                        String::from_utf8_lossy(&pvar_data[off..off + pal_len])
                            .trim_end_matches('\0')
                            .to_string();
                    off += pal_len;

                    let mask_channel = if off < pvar_data.len() {
                        let c = pvar_data[off];
                        off += 1;
                        c
                    } else {
                        0
                    };

                    palette_bindings.push(PaletteBinding {
                        variable_path,
                        default_palette_path,
                        mask_channel,
                    });
                }
            }
        }

        Ok(Self {
            name: "parsed_shader".to_string(),
            shader_type,
            blend_mode,
            ambient,
            diffuse,
            specular,
            emissive,
            shininess,
            diffuse_texture,
            palette_mask_texture,
            normal_texture,
            palette_bindings,
        })
    }

    /// Converts this legacy shader template into modern PBR material parameters.
    #[must_use]
    pub fn to_pbr_material(&self) -> PbrMaterial {
        let gloss_factor = (self.shininess / 128.0).clamp(0.0, 1.0);
        let is_metallic = self.specular.x > 0.7 && self.specular.y > 0.7 && self.specular.z > 0.7;

        let emissive_color = if self.emissive.length_squared() > 0.001 {
            Some(Vec3::new(self.emissive.x, self.emissive.y, self.emissive.z))
        } else {
            None
        };

        let mut pbr = PbrMaterial::from_legacy_parameters(
            &self.name,
            self.diffuse,
            gloss_factor,
            is_metallic,
            emissive_color,
        );

        pbr.diffuse_map = self.diffuse_texture.clone();
        pbr.normal_map = self.normal_texture.clone();
        pbr
    }
}

/// Builder helper to create valid binary .sht IFF templates.
pub struct ShaderTemplateWriter {
    shader_type: ShaderType,
    blend_mode: BlendMode,
    ambient: Vec4,
    diffuse: Vec4,
    specular: Vec4,
    emissive: Vec4,
    shininess: f32,
    textures: Vec<String>,
    palette_bindings: Vec<PaletteBinding>,
}

impl Default for ShaderTemplateWriter {
    fn default() -> Self {
        Self::new(ShaderType::Static)
    }
}

impl ShaderTemplateWriter {
    #[must_use]
    pub fn new(shader_type: ShaderType) -> Self {
        Self {
            shader_type,
            blend_mode: BlendMode::Opaque,
            ambient: Vec4::new(0.2, 0.2, 0.2, 1.0),
            diffuse: Vec4::ONE,
            specular: Vec4::new(0.5, 0.5, 0.5, 1.0),
            emissive: Vec4::ZERO,
            shininess: 32.0,
            textures: Vec::new(),
            palette_bindings: Vec::new(),
        }
    }

    pub fn set_colors(&mut self, ambient: Vec4, diffuse: Vec4, specular: Vec4, emissive: Vec4, shininess: f32) {
        self.ambient = ambient;
        self.diffuse = diffuse;
        self.specular = specular;
        self.emissive = emissive;
        self.shininess = shininess;
    }

    pub fn set_blend_mode(&mut self, mode: BlendMode) {
        self.blend_mode = mode;
    }

    pub fn add_texture(&mut self, path: impl Into<String>) {
        self.textures.push(path.into());
    }

    pub fn add_palette_binding(&mut self, var_path: &str, pal_path: &str, mask_channel: u8) {
        self.palette_bindings.push(PaletteBinding {
            variable_path: var_path.to_string(),
            default_palette_path: pal_path.to_string(),
            mask_channel,
        });
    }

    #[must_use]
    pub fn build(self) -> Vec<u8> {
        let mut writer = IffWriter::new();
        let tag = match self.shader_type {
            ShaderType::Static => b"SSHT",
            ShaderType::Character => b"CSHT",
        };
        writer.begin_form(tag);

        // MATL (68 bytes)
        let mut matl = Vec::with_capacity(68);
        for col in [self.ambient, self.diffuse, self.specular, self.emissive] {
            matl.extend_from_slice(&col.x.to_le_bytes());
            matl.extend_from_slice(&col.y.to_le_bytes());
            matl.extend_from_slice(&col.z.to_le_bytes());
            matl.extend_from_slice(&col.w.to_le_bytes());
        }
        matl.extend_from_slice(&self.shininess.to_le_bytes());
        writer.write_chunk(b"MATL", &matl);

        // BLND
        let blnd_byte = match self.blend_mode {
            BlendMode::Opaque => 0u8,
            BlendMode::AlphaBlend => 1u8,
            BlendMode::AlphaTest => 2u8,
            BlendMode::Additive => 3u8,
        };
        writer.write_chunk(b"BLND", &[blnd_byte]);

        // TXMS
        let mut txms = Vec::new();
        txms.extend_from_slice(&(self.textures.len() as u32).to_le_bytes());
        for tex in &self.textures {
            let bytes = tex.as_bytes();
            txms.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            txms.extend_from_slice(bytes);
        }
        writer.write_chunk(b"TXMS", &txms);

        // PVAR (if character shader)
        if !self.palette_bindings.is_empty() {
            let mut pvar = Vec::new();
            pvar.extend_from_slice(&(self.palette_bindings.len() as u32).to_le_bytes());
            for b in &self.palette_bindings {
                let v_bytes = b.variable_path.as_bytes();
                pvar.extend_from_slice(&(v_bytes.len() as u32).to_le_bytes());
                pvar.extend_from_slice(v_bytes);

                let p_bytes = b.default_palette_path.as_bytes();
                pvar.extend_from_slice(&(p_bytes.len() as u32).to_le_bytes());
                pvar.extend_from_slice(p_bytes);

                pvar.push(b.mask_channel);
            }
            writer.write_chunk(b"PVAR", &pvar);
        }

        writer.end_form();
        writer.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_shader_template_roundtrip() {
        let mut builder = ShaderTemplateWriter::new(ShaderType::Static);
        builder.set_colors(
            Vec4::new(0.1, 0.1, 0.1, 1.0),
            Vec4::new(0.8, 0.8, 0.8, 1.0),
            Vec4::new(0.9, 0.9, 0.9, 1.0),
            Vec4::ZERO,
            64.0,
        );
        builder.add_texture("texture/vehicle/xwing_hull.dds");
        builder.add_texture("texture/vehicle/xwing_hull_n.dds");

        let bytes = builder.build();
        let shader = ShaderTemplate::parse(&bytes).expect("Failed to parse static shader");

        assert_eq!(shader.shader_type, ShaderType::Static);
        assert_eq!(shader.blend_mode, BlendMode::Opaque);
        assert_eq!(
            shader.diffuse_texture.as_deref(),
            Some("texture/vehicle/xwing_hull.dds")
        );
        assert_eq!(
            shader.normal_texture.as_deref(),
            Some("texture/vehicle/xwing_hull_n.dds")
        );

        let pbr = shader.to_pbr_material();
        assert!(pbr.roughness < 0.8);
        assert_eq!(pbr.metallic, 0.95); // High specular -> metal
    }

    #[test]
    fn test_character_shader_template_palette_mask() {
        let mut builder = ShaderTemplateWriter::new(ShaderType::Character);
        builder.add_texture("appearance/character/twilek_f.dds");
        builder.add_texture("appearance/character/twilek_f_a.dds");
        builder.add_palette_binding("/shared_owner/index_color_skin", "palette/twilek_skin.pal", 0);
        builder.add_palette_binding("/shared_owner/index_color_cloth_primary", "palette/clothing_dyes.pal", 2);

        let bytes = builder.build();
        let shader = ShaderTemplate::parse(&bytes).expect("Failed to parse character shader");

        assert_eq!(shader.shader_type, ShaderType::Character);
        assert_eq!(
            shader.diffuse_texture.as_deref(),
            Some("appearance/character/twilek_f.dds")
        );
        assert_eq!(
            shader.palette_mask_texture.as_deref(),
            Some("appearance/character/twilek_f_a.dds")
        );
        assert_eq!(shader.palette_bindings.len(), 2);
        assert_eq!(
            shader.palette_bindings[0].variable_path,
            "/shared_owner/index_color_skin"
        );
        assert_eq!(shader.palette_bindings[0].mask_channel, 0);
    }
}
