// crates/swg-client-core/src/render.rs

use crate::window::{DisplayMetrics, PresentModeSetting};
use std::sync::Arc;
use thiserror::Error;
use wgpu::{
    Backends, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, Operations, PowerPreference, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RequestAdapterOptions, Surface, SurfaceConfiguration, SurfaceError,
    TextureFormat, TextureUsages, TextureViewDescriptor,
};

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("Failed to find suitable graphics adapter")]
    AdapterNotFound,
    #[error("Device request failed: {0}")]
    DeviceRequestFailed(String),
    #[error("Surface creation failed: {0}")]
    SurfaceCreationFailed(String),
    #[error("Surface error: {0}")]
    Surface(#[from] SurfaceError),
}

/// Graphics backend preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphicsBackend {
    #[default]
    Auto,
    Vulkan,
    Metal,
    Dx12,
    Gl,
}

impl From<GraphicsBackend> for Backends {
    fn from(b: GraphicsBackend) -> Self {
        match b {
            GraphicsBackend::Auto => Backends::PRIMARY,
            GraphicsBackend::Vulkan => Backends::VULKAN,
            GraphicsBackend::Metal => Backends::METAL,
            GraphicsBackend::Dx12 => Backends::DX12,
            GraphicsBackend::Gl => Backends::GL,
        }
    }
}

/// Configuration parameters for graphics device initialization.
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub backend: GraphicsBackend,
    pub power_preference: PowerPreference,
    pub present_mode: PresentModeSetting,
    pub clear_color: Color,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            backend: GraphicsBackend::Auto,
            power_preference: PowerPreference::HighPerformance,
            present_mode: PresentModeSetting::Mailbox,
            clear_color: Color {
                r: 0.05,
                g: 0.05,
                b: 0.08,
                a: 1.0,
            },
        }
    }
}

/// Render context managing device, queue, and surface state.
pub struct RenderContext {
    pub instance: Instance,
    pub adapter: wgpu::Adapter,
    pub device: Device,
    pub queue: Queue,
    pub surface: Option<Surface<'static>>,
    pub surface_config: Option<SurfaceConfiguration>,
    pub clear_color: Color,
}

impl RenderContext {
    /// Initializes a headless render context without surface presentation (ideal for tests / compute).
    pub async fn init_headless(config: &RenderConfig) -> Result<Self, RenderError> {
        let instance = Instance::new(&InstanceDescriptor {
            backends: config.backend.into(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: config.power_preference,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::AdapterNotFound)?;

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("SWG Modern Client Device (Headless)"),
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| RenderError::DeviceRequestFailed(e.to_string()))?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface: None,
            surface_config: None,
            clear_color: config.clear_color,
        })
    }

    /// Synchronous helper for headless initialization.
    pub fn init_headless_sync(config: &RenderConfig) -> Result<Self, RenderError> {
        pollster::block_on(Self::init_headless(config))
    }

    /// Initializes a windowed render context bound to a target surface.
    pub async fn init_windowed<W>(
        window: Arc<W>,
        metrics: DisplayMetrics,
        config: &RenderConfig,
    ) -> Result<Self, RenderError>
    where
        W: raw_window_handle::HasWindowHandle
            + raw_window_handle::HasDisplayHandle
            + Send
            + Sync
            + 'static,
    {
        let instance = Instance::new(&InstanceDescriptor {
            backends: config.backend.into(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(window)
            .map_err(|e| RenderError::SurfaceCreationFailed(e.to_string()))?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: config.power_preference,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::AdapterNotFound)?;

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("SWG Modern Client Device (Windowed)"),
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| RenderError::DeviceRequestFailed(e.to_string()))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(TextureFormat::is_srgb)
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: metrics.physical_width,
            height: metrics.physical_height,
            present_mode: config.present_mode.into(),
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface: Some(surface),
            surface_config: Some(surface_config),
            clear_color: config.clear_color,
        })
    }

    /// Resizes the surface framebuffer.
    pub fn resize(&mut self, physical_width: u32, physical_height: u32) {
        if physical_width > 0
            && physical_height > 0
            && let (Some(surface), Some(config)) = (&self.surface, &mut self.surface_config)
        {
            config.width = physical_width;
            config.height = physical_height;
            surface.configure(&self.device, config);
        }
    }

    /// Switches the presentation mode (e.g. Fifo to Mailbox for 144Hz+).
    pub fn set_present_mode(&mut self, mode: PresentModeSetting) {
        if let (Some(surface), Some(config)) = (&self.surface, &mut self.surface_config) {
            config.present_mode = mode.into();
            surface.configure(&self.device, config);
        }
    }

    /// Executes a clear render pass on the surface texture.
    pub fn render_clear_pass(&self) -> Result<(), RenderError> {
        let Some(surface) = &self.surface else {
            return Ok(());
        };

        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Clear Pass Encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Clear Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_config_default() {
        let config = RenderConfig::default();
        assert_eq!(config.backend, GraphicsBackend::Auto);
        assert_eq!(config.present_mode, PresentModeSetting::Mailbox);
        assert_eq!(config.clear_color.a, 1.0);
    }

    #[test]
    fn test_headless_render_context() {
        let config = RenderConfig::default();
        match RenderContext::init_headless_sync(&config) {
            Ok(ctx) => {
                let info = ctx.adapter.get_info();
                println!(
                    "Found graphics adapter: {:?} ({:?})",
                    info.name, info.backend
                );
                assert!(ctx.surface.is_none());
            }
            Err(RenderError::AdapterNotFound) => {
                // Allowed in environments without GPU drivers
                println!("No GPU adapter found in test environment (allowed in headless CI)");
            }
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }
}
