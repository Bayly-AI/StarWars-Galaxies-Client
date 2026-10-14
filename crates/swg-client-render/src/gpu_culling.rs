// crates/swg-client-render/src/gpu_culling.rs

use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};
use wgpu::{
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType,
    ComputePipeline, ComputePipelineDescriptor, Device, PipelineCompilationOptions,
    ShaderModuleDescriptor, ShaderSource, ShaderStages,
};

/// WGSL compute shader for GPU frustum culling and indirect draw command compaction.
pub const CULLING_COMPUTE_WGSL: &str = r#"
struct CullableInstanceRaw {
    center: vec3<f32>,
    radius: f32,
    aabb_min: vec3<f32>,
    pad0: u32,
    aabb_max: vec3<f32>,
    instance_id: u32,
    lod: u32,
    pad1: u32,
    pad2: u32,
    pad3: u32,
};

struct FrustumUniforms {
    planes: array<vec4<f32>, 6>,
    camera_pos: vec4<f32>,
    total_instances: u32,
    max_draw_distance: f32,
    pad0: u32,
    pad1: u32,
};

struct DrawIndirectArgs {
    vertex_count: u32,
    instance_count: atomic<u32>,
    first_vertex: u32,
    first_instance: u32,
};

@group(0) @binding(0)
var<uniform> uniforms: FrustumUniforms;

@group(0) @binding(1)
var<storage, read> instances_in: array<CullableInstanceRaw>;

@group(0) @binding(2)
var<storage, read_write> visible_indices_out: array<u32>;

@group(0) @binding(3)
var<storage, read_write> draw_args: DrawIndirectArgs;

@compute @workgroup_size(64)
fn cs_cull_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= uniforms.total_instances) {
        return;
    }

    let inst = instances_in[index];

    // Distance culling check
    let dist_to_cam = distance(inst.center, uniforms.camera_pos.xyz);
    if (dist_to_cam > uniforms.max_draw_distance + inst.radius) {
        return;
    }

    // Frustum sphere test against all 6 clipping planes
    var is_visible = true;
    for (var p = 0u; p < 6u; p = p + 1u) {
        let plane = uniforms.planes[p];
        let dist = dot(plane.xyz, inst.center) + plane.w;
        if (dist < -inst.radius) {
            is_visible = false;
            break;
        }
    }

    if (is_visible) {
        let slot = atomicAdd(&draw_args.instance_count, 1u);
        visible_indices_out[slot] = inst.instance_id;
    }
}
"#;

/// Raw GPU-aligned layout for a cullable instance.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CullableInstanceRaw {
    pub center: [f32; 3],
    pub radius: f32,
    pub aabb_min: [f32; 3],
    pub pad0: u32,
    pub aabb_max: [f32; 3],
    pub instance_id: u32,
    pub lod: u32,
    pub pad1: u32,
    pub pad2: u32,
    pub pad3: u32,
}

impl CullableInstanceRaw {
    #[must_use]
    pub fn new(
        center: Vec3,
        radius: f32,
        aabb_min: Vec3,
        aabb_max: Vec3,
        instance_id: u32,
        lod: u32,
    ) -> Self {
        Self {
            center: center.to_array(),
            radius,
            aabb_min: aabb_min.to_array(),
            pad0: 0,
            aabb_max: aabb_max.to_array(),
            instance_id,
            lod,
            pad1: 0,
            pad2: 0,
            pad3: 0,
        }
    }
}

/// Raw GPU-aligned layout for frustum planes and camera position.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FrustumUniforms {
    pub planes: [[f32; 4]; 6],
    pub camera_pos: [f32; 4],
    pub total_instances: u32,
    pub max_draw_distance: f32,
    pub pad0: u32,
    pub pad1: u32,
}

/// GPU DrawIndirect argument buffer layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DrawIndirectArgs {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

/// GPU Compute Culling Pipeline manager.
pub struct GpuCullingPipeline;

impl GpuCullingPipeline {
    /// Creates the WGSL compute pipeline and bind group layout for GPU culling.
    #[must_use]
    pub fn create_pipeline(
        device: &Device,
    ) -> (wgpu::ShaderModule, ComputePipeline, wgpu::BindGroupLayout) {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("gpu_culling_compute_shader"),
            source: ShaderSource::Wgsl(CULLING_COMPUTE_WGSL.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("culling_bind_group_layout"),
            entries: &[
                // 0: Uniforms
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 1: Instances In
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 2: Visible Indices Out
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 3: Draw Indirect Args
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("culling_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("culling_compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_cull_main"),
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        });

        (shader, pipeline, bind_group_layout)
    }
}

/// High-performance CPU software reference evaluator guaranteeing identical mathematics
/// to the GPU compute shader for unit testing and fallback support.
pub struct CpuCullingEvaluator;

impl CpuCullingEvaluator {
    /// Evaluates visibility and compacts visible instance IDs.
    #[must_use]
    pub fn cull_instances(
        instances: &[CullableInstanceRaw],
        uniforms: &FrustumUniforms,
    ) -> Vec<u32> {
        let mut visible = Vec::with_capacity(instances.len());
        let camera_pos = Vec3::from_slice(&uniforms.camera_pos[0..3]);

        for inst in instances {
            let center = Vec3::from_slice(&inst.center);

            // Distance culling check
            if center.distance(camera_pos) > uniforms.max_draw_distance + inst.radius {
                continue;
            }

            // Frustum sphere test against all 6 clipping planes
            let mut is_visible = true;
            for p in 0..6 {
                let plane = Vec4::from_array(uniforms.planes[p]);
                let dist = plane.truncate().dot(center) + plane.w;
                if dist < -inst.radius {
                    is_visible = false;
                    break;
                }
            }

            if is_visible {
                visible.push(inst.instance_id);
            }
        }

        visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_reference_frustum_and_distance_culling() {
        // Frustum looking down -Z from origin (0, 0, 0)
        let planes = [
            [1.0, 0.0, 0.0, 50.0],  // Left (X > -50)
            [-1.0, 0.0, 0.0, 50.0], // Right (X < 50)
            [0.0, 1.0, 0.0, 50.0],  // Bottom (Y > -50)
            [0.0, -1.0, 0.0, 50.0], // Top (Y < 50)
            [0.0, 0.0, -1.0, 10.0], // Near (Z < 10)
            [0.0, 0.0, 1.0, 200.0], // Far (Z > -200)
        ];

        let uniforms = FrustumUniforms {
            planes,
            camera_pos: [0.0, 0.0, 0.0, 1.0],
            total_instances: 3,
            max_draw_distance: 150.0,
            pad0: 0,
            pad1: 0,
        };

        let instances = vec![
            // 1. In front of camera at (0, 0, -50), radius 5 (VISIBLE)
            CullableInstanceRaw::new(
                Vec3::new(0.0, 0.0, -50.0),
                5.0,
                Vec3::new(-5.0, -5.0, -55.0),
                Vec3::new(5.0, 5.0, -45.0),
                101,
                0,
            ),
            // 2. Behind camera at (0, 0, 50), radius 5 (CULLED)
            CullableInstanceRaw::new(
                Vec3::new(0.0, 0.0, 50.0),
                5.0,
                Vec3::new(-5.0, -5.0, 45.0),
                Vec3::new(5.0, 5.0, 55.0),
                102,
                0,
            ),
            // 3. Beyond max draw distance at (0, 0, -180), radius 5 (CULLED)
            CullableInstanceRaw::new(
                Vec3::new(0.0, 0.0, -180.0),
                5.0,
                Vec3::new(-5.0, -5.0, -185.0),
                Vec3::new(5.0, 5.0, -175.0),
                103,
                0,
            ),
        ];

        let visible = CpuCullingEvaluator::cull_instances(&instances, &uniforms);
        assert_eq!(visible, vec![101]);
    }
}
