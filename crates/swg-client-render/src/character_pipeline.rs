// crates/swg-client-render/src/character_pipeline.rs

use serde::{Deserialize, Serialize};
use wgpu::{
    ColorTargetState, ColorWrites, Device, Face, FragmentState, FrontFace, MultisampleState,
    PipelineCompilationOptions, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource, TextureFormat,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

/// WGSL shader source code for 4-bone skeletal skinning and multi-channel palette mask customization.
pub const CHARACTER_WGSL: &str = r#"
struct CameraUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct LightUniforms {
    sun_direction: vec4<f32>,
    sun_color: vec4<f32>,
    ambient_color: vec4<f32>,
};

struct CustomizationTints {
    skin_color: vec4<f32>,
    hair_color: vec4<f32>,
    cloth_primary: vec4<f32>,
    cloth_secondary: vec4<f32>,
};

struct BoneMatrices {
    bones: array<mat4x4<f32>, 96>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;

@group(0) @binding(1)
var<uniform> lighting: LightUniforms;

@group(1) @binding(0)
var<uniform> tints: CustomizationTints;

@group(1) @binding(1)
var<uniform> skeleton: BoneMatrices;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) bone_indices: vec4<u32>,
    @location(4) bone_weights: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // 4-Bone linear blend skinning
    let m0 = skeleton.bones[in.bone_indices.x];
    let m1 = skeleton.bones[in.bone_indices.y];
    let m2 = skeleton.bones[in.bone_indices.z];
    let m3 = skeleton.bones[in.bone_indices.w];

    let skin_mat = m0 * in.bone_weights.x
                 + m1 * in.bone_weights.y
                 + m2 * in.bone_weights.z
                 + m3 * in.bone_weights.w;

    let skinned_pos = skin_mat * vec4<f32>(in.position, 1.0);
    let skinned_norm = normalize((skin_mat * vec4<f32>(in.normal, 0.0)).xyz);

    out.world_pos = skinned_pos.xyz;
    out.normal = skinned_norm;
    out.uv = in.uv;
    out.clip_position = camera.view_proj * skinned_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let norm = normalize(in.normal);

    // Default base diffuse color
    let base_albedo = vec3<f32>(0.8, 0.8, 0.8);

    // Multi-channel palette mask simulation:
    // Red channel   -> skin color
    // Green channel -> hair / markings color
    // Blue channel  -> cloth primary color
    // Alpha channel -> cloth secondary color
    let mask_r = 0.6; // Primary skin
    let mask_g = 0.0;
    let mask_b = 0.2; // Secondary cloth

    let skin_contrib = tints.skin_color.rgb * mask_r;
    let hair_contrib = tints.hair_color.rgb * mask_g;
    let cloth_contrib = tints.cloth_primary.rgb * mask_b;
    let unmasked_factor = max(1.0 - (mask_r + mask_g + mask_b), 0.0);

    let albedo = base_albedo * unmasked_factor + skin_contrib + hair_contrib + cloth_contrib;

    // Directional sun lighting
    let n_dot_l = max(dot(norm, -lighting.sun_direction.xyz), 0.0);
    let diffuse = albedo * lighting.sun_color.xyz * n_dot_l;
    let ambient = albedo * lighting.ambient_color.xyz;

    let final_color = diffuse + ambient;
    return vec4<f32>(final_color, 1.0);
}
"#;

/// Uniform buffer payload for character customization color tints.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CustomizationTintsRaw {
    pub skin_color: [f32; 4],
    pub hair_color: [f32; 4],
    pub cloth_primary: [f32; 4],
    pub cloth_secondary: [f32; 4],
}

impl Default for CustomizationTintsRaw {
    fn default() -> Self {
        Self {
            skin_color: [0.95, 0.80, 0.70, 1.0],      // Fair human
            hair_color: [0.15, 0.12, 0.10, 1.0],      // Dark brown
            cloth_primary: [0.70, 0.15, 0.15, 1.0],   // Crimson
            cloth_secondary: [0.15, 0.30, 0.60, 1.0], // Navy
        }
    }
}

pub struct CharacterPipeline;

impl CharacterPipeline {
    /// Returns vertex layout for 4-bone skinned mesh vertices.
    #[must_use]
    pub fn vertex_layout<'a>() -> VertexBufferLayout<'a> {
        const ATTRIBS: [VertexAttribute; 5] = [
            VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3, // position (12B)
            },
            VertexAttribute {
                offset: 12,
                shader_location: 1,
                format: VertexFormat::Float32x3, // normal (12B)
            },
            VertexAttribute {
                offset: 24,
                shader_location: 2,
                format: VertexFormat::Float32x2, // uv (8B)
            },
            VertexAttribute {
                offset: 32,
                shader_location: 3,
                format: VertexFormat::Uint32x4, // bone_indices (16B)
            },
            VertexAttribute {
                offset: 48,
                shader_location: 4,
                format: VertexFormat::Float32x4, // bone_weights (16B)
            },
        ];

        VertexBufferLayout {
            array_stride: 64, // 12 + 12 + 8 + 16 + 16 = 64 bytes
            step_mode: VertexStepMode::Vertex,
            attributes: &ATTRIBS,
        }
    }

    /// Compiles shader module and creates the wgpu render pipeline.
    pub fn create_pipeline(
        device: &Device,
        format: TextureFormat,
    ) -> (ShaderModule, RenderPipeline) {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Character Skinning Shader"),
            source: ShaderSource::Wgsl(CHARACTER_WGSL.into()),
        });

        let scene_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Character Scene Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let char_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Character Customization Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Character Pipeline Layout"),
            bind_group_layouts: &[&scene_bgl, &char_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Character Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Self::vertex_layout()],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        (shader, pipeline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swg_client_core::{GraphicsBackend, RenderConfig, RenderContext, RenderError};

    #[test]
    fn test_character_shader_compilation() {
        let config = RenderConfig {
            backend: GraphicsBackend::Auto,
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        };

        match RenderContext::init_headless_sync(&config) {
            Ok(ctx) => {
                let (_shader, _pipeline) =
                    CharacterPipeline::create_pipeline(&ctx.device, TextureFormat::Rgba8Unorm);
                println!("Character skinning pipeline compiled successfully on GPU adapter");
            }
            Err(RenderError::AdapterNotFound) => {
                println!("Headless runner has no GPU adapter (allowed in headless CI)");
            }
            Err(e) => panic!("Unexpected render error: {e:?}"),
        }
    }
}
