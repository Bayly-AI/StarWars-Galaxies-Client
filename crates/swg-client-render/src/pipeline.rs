// crates/swg-client-render/src/pipeline.rs

use wgpu::{
    ColorTargetState, ColorWrites, Device, Face, FragmentState, FrontFace, MultisampleState,
    PipelineCompilationOptions, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource, TextureFormat,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

/// WGSL shader source code for multi-layer terrain splatting and lighting.
pub const TERRAIN_WGSL: &str = r#"
struct CameraUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct LightUniforms {
    sun_direction: vec4<f32>,
    sun_color: vec4<f32>,
    ambient_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;

@group(0) @binding(1)
var<uniform> lighting: LightUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
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
    out.world_pos = in.position;
    out.normal = normalize(in.normal);
    out.uv = in.uv;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let norm = normalize(in.normal);
    let slope = clamp(norm.y, 0.0, 1.0);
    let elevation = in.world_pos.y;

    // Palette albedo definitions
    let sand_color = vec3<f32>(0.85, 0.72, 0.50);
    let dirt_color = vec3<f32>(0.65, 0.48, 0.32);
    let rock_color = vec3<f32>(0.45, 0.38, 0.30);
    let mesa_color = vec3<f32>(0.70, 0.35, 0.22);

    // Splat weights computation
    var w_sand = 0.0;
    var w_dirt = 0.0;
    var w_rock = 0.0;

    if (slope > 0.85) {
        w_sand = 1.0;
    } else if (slope > 0.65) {
        let t = (slope - 0.65) / 0.20;
        w_sand = t;
        w_dirt = 1.0 - t;
    } else if (slope > 0.40) {
        let t = (slope - 0.40) / 0.25;
        w_dirt = t;
        w_rock = 1.0 - t;
    } else {
        w_rock = 1.0;
    }

    var w_mesa = 0.0;
    if (elevation > 50.0) {
        let e_factor = clamp((elevation - 50.0) / 30.0, 0.0, 1.0);
        w_mesa = e_factor * (w_sand + w_dirt * 0.5);
        w_sand *= (1.0 - e_factor);
        w_dirt *= (1.0 - e_factor * 0.5);
    }

    let total_w = w_sand + w_dirt + w_rock + w_mesa + 1e-4;
    let albedo = (sand_color * w_sand + dirt_color * w_dirt + rock_color * w_rock + mesa_color * w_mesa) / total_w;

    // Directional sun lighting
    let n_dot_l = max(dot(norm, -lighting.sun_direction.xyz), 0.0);
    let diffuse = albedo * lighting.sun_color.xyz * n_dot_l;
    let ambient = albedo * lighting.ambient_color.xyz;

    let final_color = diffuse + ambient;
    return vec4<f32>(final_color, 1.0);
}
"#;

pub struct TerrainPipeline;

impl TerrainPipeline {
    /// Returns the vertex buffer layout for TerrainVertex.
    #[must_use]
    pub fn vertex_layout<'a>() -> VertexBufferLayout<'a> {
        const ATTRIBS: [VertexAttribute; 3] = [
            VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3,
            },
            VertexAttribute {
                offset: 12,
                shader_location: 1,
                format: VertexFormat::Float32x3,
            },
            VertexAttribute {
                offset: 24,
                shader_location: 2,
                format: VertexFormat::Float32x2,
            },
        ];

        VertexBufferLayout {
            array_stride: 32, // 3 floats + 3 floats + 2 floats = 8 * 4 = 32 bytes
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
            label: Some("Terrain Shader"),
            source: ShaderSource::Wgsl(TERRAIN_WGSL.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Terrain Uniforms Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Terrain Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Terrain Render Pipeline"),
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
    fn test_terrain_shader_compilation() {
        let config = RenderConfig {
            backend: GraphicsBackend::Auto,
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        };

        match RenderContext::init_headless_sync(&config) {
            Ok(ctx) => {
                let (_shader, _pipeline) =
                    TerrainPipeline::create_pipeline(&ctx.device, TextureFormat::Rgba8Unorm);
                println!("Terrain pipeline compiled successfully on hardware adapter");
            }
            Err(RenderError::AdapterNotFound) => {
                println!("Headless runner has no GPU adapter (allowed in headless CI)");
            }
            Err(e) => panic!("Unexpected render error: {e:?}"),
        }
    }
}
